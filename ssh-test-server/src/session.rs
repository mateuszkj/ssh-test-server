use anyhow::Result;
use async_trait::async_trait;
use russh::server::{Auth, Handle, Handler, Msg, Response, Session};
use russh::{server, Channel, ChannelId, ChannelMsg, CryptoVec, Pty};
use russh_keys::key::PublicKey;
use std::env::var;
use std::mem;
use tracing::{debug, info, span, Level};

pub(crate) struct SshConnection {
    command: Vec<u8>,
    id: usize,
}

impl SshConnection {
    pub fn new(id: usize) -> Self {
        Self {
            command: vec![],
            id,
        }
    }
}

async fn send_stderr(channel: ChannelId, handle: &Handle, msg: &str) {
    let mut stderr = CryptoVec::from_slice(msg.as_bytes());
    stderr.push(b'\n');
    stderr.push(b'\r');
    handle.extended_data(channel, 1, stderr).await.unwrap();
}

async fn send_stdout(channel: ChannelId, handle: &Handle, msg: &str) {
    let mut stdout = CryptoVec::from_slice(msg.as_bytes());
    stdout.push(b'\n');
    stdout.push(b'\r');
    handle.data(channel, stdout).await.unwrap();
}

async fn execute_command(command: Vec<u8>, channel: ChannelId, handle: Handle) {
    let cmd = String::from_utf8_lossy(&command);
    let mut cmdline = cmd.to_string();
    let mut parse = cmdline_words_parser::parse_posix(&mut cmdline);
    let Some(program) =  parse.next() else {
        // just enter
        return;
    };
    let args: Vec<_> = parse.collect();

    info!("command: {cmd}, program {program} args: {args:?}");

    if program == "echo" {
        let mut stdout = String::new();
        for a in args {
            stdout.push_str(a);
        }
        send_stdout(channel, &handle, &stdout).await;
        handle.exit_status_request(channel, 0).await.unwrap();
    } else if program == "exit" {
        handle.exit_status_request(channel, 0).await.unwrap();
        handle.close(channel).await.unwrap();
    } else {
        let msg = format!("{program}: command not found");
        send_stderr(channel, &handle, &msg).await;
        handle.exit_status_request(channel, 127).await.unwrap();
    }
}

#[async_trait]
impl Handler for SshConnection {
    type Error = anyhow::Error;

    async fn auth_none(self, user: &str) -> Result<(Self, Auth), Self::Error> {
        info!("auth_none user={user}");
        Ok((
            self,
            Auth::Reject {
                proceed_with_methods: None,
            },
        ))
    }

    async fn auth_password(self, user: &str, password: &str) -> Result<(Self, Auth), Self::Error> {
        info!("auth_password user={user} password={password}");
        Ok((self, Auth::Accept))
    }

    async fn auth_publickey(
        self,
        user: &str,
        public_key: &PublicKey,
    ) -> Result<(Self, Auth), Self::Error> {
        info!("auth_publickey user={user} public_key={public_key:?}");

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
        info!("auth_keyboard_interactive user={user} submethods={submethods:?}");
        Ok((
            self,
            Auth::Reject {
                proceed_with_methods: None,
            },
        ))
    }

    async fn auth_succeeded(self, session: Session) -> Result<(Self, Session), Self::Error> {
        info!("auth_succeeded");
        Ok((self, session))
    }

    async fn channel_close(
        self,
        channel: ChannelId,
        session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        info!("channel_close channel={channel}");
        Ok((self, session))
    }

    async fn channel_eof(
        self,
        channel: ChannelId,
        session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        info!("channel_eof channel={channel}");
        Handler::channel_eof(self, channel, session).await
    }

    async fn channel_open_session(
        self,
        mut channel: Channel<Msg>,
        session: Session,
    ) -> Result<(Self, bool, Session), Self::Error> {
        info!("channel_open_session channel={}", channel.id());
        let handle = session.handle();

        tokio::spawn(async move {
            let id = channel.id();
            let span = span!(Level::INFO, "channel", id = id.to_string());
            let _enter = span.enter();

            while let Some(msg) = channel.wait().await {
                info!("msg={msg:?}");
                match msg {
                    ChannelMsg::RequestPty { want_reply, .. } => {
                        if want_reply {
                            handle.channel_success(id).await.unwrap();
                        }
                    }
                    ChannelMsg::RequestShell { want_reply, .. } => {
                        if want_reply {
                            handle.channel_success(id).await.unwrap();
                        }
                    }
                    _ => {}
                }
            }
            info!("closed");
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
        info!("channel_open_x11 channel={} originator_address={originator_address} originator_port={originator_port}", channel.id());
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
        info!("channel_open_direct_tcpip channel={} host_to_connect={host_to_connect} port_to_connect={port_to_connect} originator_address={originator_address} originator_port={originator_port}", channel.id());
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
        info!("channel_open_forwarded_tcpip channel={} host_to_connect={host_to_connect} port_to_connect={port_to_connect} originator_address={originator_address} originator_port={originator_port}", channel.id());
        Ok((self, false, session))
    }

    async fn data(
        mut self,
        channel: ChannelId,
        data: &[u8],
        mut session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        debug!(
            "server channel: {channel:?} data = {:?} id={}",
            std::str::from_utf8(data),
            self.id,
        );

        let mut stdout = CryptoVec::new();
        for b in data {
            if *b == 0x03 {
                // Ctrl + C
                session.exit_status_request(channel, 130);
                session.close(channel);
            } else if *b == b'\r' || *b == b'\n' {
                stdout.push(b'\n');
                stdout.push(b'\r');
                session.data(channel, mem::take(&mut stdout));
                let cmd = mem::take(&mut self.command);
                let handle = session.handle();
                execute_command(cmd, channel, handle).await;
            } else {
                self.command.push(*b);
                stdout.push(*b);
            }
        }

        if !stdout.is_empty() {
            session.data(channel, mem::take(&mut stdout));
        }
        Ok((self, session))
    }

    fn adjust_window(&mut self, channel: ChannelId, current: u32) -> u32 {
        info!("adjust_window {channel} current={current}");
        current
    }

    async fn exec_request(
        self,
        channel: ChannelId,
        data: &[u8],
        session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        info!(
            "exec_request channel: {channel:?} data = {:?} id={}",
            std::str::from_utf8(data),
            self.id,
        );

        let cmd = data.to_vec();
        let handle = session.handle();
        execute_command(cmd, channel, handle).await;
        let handle = session.handle();
        handle.channel_success(channel).await.unwrap();
        handle.close(channel).await.unwrap();

        Ok((self, session))
    }
}
