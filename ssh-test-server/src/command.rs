use crate::session::ProgramsMap;
use crate::{SshExecuteContext, UsersMap};
use russh::server::Handle;
use russh::{ChannelId, CryptoVec};
use tracing::debug;

async fn send_stderr(channel: ChannelId, handle: &Handle, msg: &str) {
    let mut stderr = CryptoVec::from_slice(msg.as_bytes());
    stderr.push(b'\r');
    stderr.push(b'\n');
    handle.extended_data(channel, 1, stderr).await.unwrap();
}

async fn send_stdout(channel: ChannelId, handle: &Handle, msg: &str) {
    let mut stdout = CryptoVec::from_slice(msg.as_bytes());
    stdout.push(b'\r');
    stdout.push(b'\n');
    handle.data(channel, stdout).await.unwrap();
}

pub async fn execute_command(
    command: Vec<u8>,
    channel: ChannelId,
    handle: &Handle,
    session_user: &str,
    users: &UsersMap,
    programs: &ProgramsMap,
) {
    let cmd = String::from_utf8_lossy(&command);
    let mut cmdline = cmd.to_string();
    let mut parse = cmdline_words_parser::parse_posix(&mut cmdline);
    let Some(program) =  parse.next() else {
        // just enter
        return;
    };
    let args: Vec<_> = parse.collect();

    debug!("command: {cmd}, program {program} args: {args:?}");

    if let Some(handler) = programs.get(program) {
        let context = SshExecuteContext {
            users,
            current_user: session_user,
        };

        let r = handler(&context, program, &args);

        if !r.stderr.is_empty() {
            send_stderr(channel, handle, &r.stderr).await;
        }
        if !r.stdout.is_empty() {
            send_stdout(channel, handle, &r.stdout).await;
        }

        handle
            .exit_status_request(channel, r.status_code)
            .await
            .unwrap();
    } else if program == "echo" {
        let mut stdout = String::new();
        for a in args {
            stdout.push_str(a);
        }
        send_stdout(channel, handle, &stdout).await;
        handle.exit_status_request(channel, 0).await.unwrap();
    } else if program == "change_password" {
        match args.first() {
            Some(new_password) => {
                {
                    let mut users = users.lock().unwrap();
                    let user = users.get_mut(session_user).unwrap();
                    user.set_password(new_password);
                }
                send_stdout(channel, handle, "password changed").await;
                handle.exit_status_request(channel, 0).await.unwrap();
            }
            None => {
                send_stdout(
                    channel,
                    handle,
                    "no password Usage: change_password <new_password>",
                )
                .await;
                handle.exit_status_request(channel, 1).await.unwrap();
            }
        }
    } else if program == "exit" {
        handle.exit_status_request(channel, 0).await.unwrap();
        handle.close(channel).await.unwrap();
    } else {
        let msg = format!("{program}: command not found");
        send_stderr(channel, handle, &msg).await;
        handle.exit_status_request(channel, 127).await.unwrap();
    }
}
