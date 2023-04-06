use anyhow::Result;
use async_trait::async_trait;
use russh::server::{Auth, Handle, Msg, Session};
use russh::{server, Channel, ChannelId, CryptoVec};
use std::mem;
use tracing::{debug, info};

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
    }

    let msg = format!("Command not found: {program}");
    send_stderr(channel, &handle, &msg).await;
    handle.exit_status_request(channel, 127).await.unwrap();
}

#[async_trait]
impl server::Handler for SshConnection {
    type Error = anyhow::Error;

    async fn auth_password(self, user: &str, password: &str) -> Result<(Self, Auth), Self::Error> {
        info!("auth_password user={user} password={password}");
        Ok((self, Auth::Accept))
    }

    async fn channel_open_session(
        self,
        channel: Channel<Msg>,
        session: Session,
    ) -> Result<(Self, bool, Session), Self::Error> {
        info!("channel_open_session channel={}", channel.id());
        Ok((self, true, session))
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
}
