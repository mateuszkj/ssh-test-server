use crate::{command, UsersMap};
use anyhow::Result;
use async_trait::async_trait;
use russh::server::{Auth, Handler, Msg, Response, Session};
use russh::{Channel, ChannelId, ChannelMsg, CryptoVec};
use russh_keys::key::PublicKey;
use std::mem;
use tracing::debug;

pub(crate) struct SshConnection {
    id: usize,
    users: UsersMap,
    user: Option<String>,
}

impl SshConnection {
    pub fn new(id: usize, users: UsersMap) -> Self {
        Self {
            id,
            users,
            user: None,
        }
    }
}

#[async_trait]
impl Handler for SshConnection {
    type Error = anyhow::Error;

    async fn auth_none(self, user: &str) -> Result<(Self, Auth), Self::Error> {
        debug!("auth_none user={user}");
        Ok((
            self,
            Auth::Reject {
                proceed_with_methods: None,
            },
        ))
    }

    async fn auth_password(
        mut self,
        user: &str,
        password: &str,
    ) -> Result<(Self, Auth), Self::Error> {
        let users = self.users.lock().unwrap();
        if let Some(u) = users.get(user) {
            if password == u.password() {
                self.user = Some(u.login().to_string());
                drop(users);
                debug!("auth_password user={user} password={password} Accepted");
                return Ok((self, Auth::Accept));
            }
        }

        drop(users);
        debug!("auth_password user={user} password={password} Rejected");
        Ok((
            self,
            Auth::Reject {
                proceed_with_methods: None,
            },
        ))
    }

    async fn auth_publickey(
        self,
        user: &str,
        public_key: &PublicKey,
    ) -> Result<(Self, Auth), Self::Error> {
        debug!("auth_publickey user={user} public_key={public_key:?}");

        Ok((
            self,
            Auth::Reject {
                proceed_with_methods: None,
            },
        ))
    }

    async fn auth_keyboard_interactive(
        self,
        user: &str,
        submethods: &str,
        _response: Option<Response<'async_trait>>,
    ) -> Result<(Self, Auth), Self::Error> {
        debug!("auth_keyboard_interactive user={user} submethods={submethods:?}");
        Ok((
            self,
            Auth::Reject {
                proceed_with_methods: None,
            },
        ))
    }

    async fn auth_succeeded(self, session: Session) -> Result<(Self, Session), Self::Error> {
        debug!("auth_succeeded");
        Ok((self, session))
    }

    async fn channel_close(
        self,
        channel: ChannelId,
        session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        debug!("channel_close channel={channel}");
        Ok((self, session))
    }

    async fn channel_eof(
        self,
        channel: ChannelId,
        session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        debug!("channel_eof channel={channel}");
        Handler::channel_eof(self, channel, session).await
    }

    async fn channel_open_session(
        self,
        mut channel: Channel<Msg>,
        session: Session,
    ) -> Result<(Self, bool, Session), Self::Error> {
        let session_id = self.id;
        debug!(session_id, "channel_open_session channel={}", channel.id());
        let handle = session.handle();
        let user = self.user.clone().unwrap();
        let users = self.users.clone();
        tokio::spawn(async move {
            let id = channel.id();
            let mut command_buf = vec![];

            while let Some(msg) = channel.wait().await {
                match msg {
                    ChannelMsg::RequestPty {
                        want_reply,
                        term,
                        col_width,
                        row_height,
                        pix_width,
                        pix_height,
                        terminal_modes,
                    } => {
                        debug!(session_id, "request-pty want_reply={want_reply} term={term} col/row={col_width}/{row_height} pix width/height={pix_width}/{pix_height} modes={terminal_modes:?}");
                        if want_reply {
                            handle.channel_success(id).await.unwrap();
                        }
                    }
                    ChannelMsg::RequestShell { want_reply } => {
                        debug!(session_id, "request-shell want_reply={want_reply}");
                        if want_reply {
                            handle.channel_success(id).await.unwrap();
                        }
                        handle.data(id, CryptoVec::from_slice(b"$ ")).await.unwrap();
                    }
                    ChannelMsg::Data { data } => {
                        debug!(session_id, "data={}", String::from_utf8_lossy(&data));

                        let mut stdout = CryptoVec::new();
                        for b in data.iter() {
                            if *b == 0x03 {
                                // Ctrl + C
                                handle.exit_status_request(id, 130).await.unwrap();
                                handle.close(id).await.unwrap();
                            } else if *b == b'\r' || *b == b'\n' {
                                stdout.push(b'\n');
                                stdout.push(b'\r');
                                handle.data(id, mem::take(&mut stdout)).await.unwrap();
                                let cmd = mem::take(&mut command_buf);
                                command::execute_command(cmd, id, &handle, &user, &users).await;
                                handle.data(id, CryptoVec::from_slice(b"$ ")).await.unwrap();
                            } else {
                                command_buf.push(*b);
                                stdout.push(*b);
                            }
                        }

                        if !stdout.is_empty() {
                            handle.data(id, mem::take(&mut stdout)).await.unwrap();
                        }
                    }
                    ChannelMsg::Exec {
                        want_reply,
                        command,
                    } => {
                        debug!(
                            session_id,
                            "exec want_reply={want_reply} command: {}",
                            String::from_utf8_lossy(&command)
                        );
                        if want_reply {
                            handle.channel_success(id).await.unwrap();
                        }

                        command::execute_command(command, id, &handle, &user, &users).await;
                        handle.close(id).await.unwrap();
                    }
                    _ => {
                        debug!(session_id, "msg={msg:?}");
                    }
                }
            }
            debug!(session_id, "closed");
        });

        Ok((self, true, session))
    }

    async fn channel_open_x11(
        self,
        channel: Channel<Msg>,
        originator_address: &str,
        originator_port: u32,
        session: Session,
    ) -> Result<(Self, bool, Session), Self::Error> {
        debug!("channel_open_x11 channel={} originator_address={originator_address} originator_port={originator_port}", channel.id());
        Ok((self, false, session))
    }
    async fn channel_open_direct_tcpip(
        self,
        channel: Channel<Msg>,
        host_to_connect: &str,
        port_to_connect: u32,
        originator_address: &str,
        originator_port: u32,
        session: Session,
    ) -> Result<(Self, bool, Session), Self::Error> {
        debug!("channel_open_direct_tcpip channel={} host_to_connect={host_to_connect} port_to_connect={port_to_connect} originator_address={originator_address} originator_port={originator_port}", channel.id());
        Ok((self, false, session))
    }

    async fn channel_open_forwarded_tcpip(
        self,
        channel: Channel<Msg>,
        host_to_connect: &str,
        port_to_connect: u32,
        originator_address: &str,
        originator_port: u32,
        session: Session,
    ) -> Result<(Self, bool, Session), Self::Error> {
        debug!("channel_open_forwarded_tcpip channel={} host_to_connect={host_to_connect} port_to_connect={port_to_connect} originator_address={originator_address} originator_port={originator_port}", channel.id());
        Ok((self, false, session))
    }

    fn adjust_window(&mut self, channel: ChannelId, current: u32) -> u32 {
        debug!("adjust_window {channel} current={current}");
        current
    }
}
