#![allow(unused)]
use ssh2::{Channel, Session};
use std::io::{Read, Write};
use tracing::info;

pub async fn run_ssh_command<F>(
    addr: &str,
    username: &str,
    password: &str,
    f: F,
) -> (String, String, i32)
where
    F: FnOnce(&mut Channel) + Send + 'static,
{
    let addr = addr.to_string();
    let username = username.to_string();
    let password = password.to_string();

    tokio::task::spawn_blocking(move || {
        let tcp = std::net::TcpStream::connect(addr).unwrap();
        let mut sess = Session::new().unwrap();
        sess.set_tcp_stream(tcp);
        sess.handshake().unwrap();
        sess.set_timeout(5000);

        sess.userauth_password(&username, &password).unwrap();
        assert!(sess.authenticated());
        info!("ssh authenticated");

        let mut channel = sess.channel_session().unwrap();
        info!("channel");
        f(&mut channel);
        info!("f");

        let mut stdout = String::new();
        channel.read_to_string(&mut stdout).unwrap();
        info!("got stdout");

        let mut stderr = String::new();
        channel.stderr().read_to_string(&mut stderr).unwrap();
        info!("got stderr");

        let exit_code = channel.exit_status().unwrap();
        (stdout, stderr, exit_code)
    })
    .await
    .unwrap()
}

pub fn expect(channel: &mut Channel, expect: &str) {
    let mut content: Vec<u8> = vec![];
    let mut buf = vec![0u8; 8192];
    loop {
        let len = channel.read(&mut buf).unwrap();
        content.extend(&buf);

        let str = String::from_utf8_lossy(&content);
        if len == 0 {
            panic!("got str {str} expected {expect}");
        } else if str.contains(expect) {
            return;
        }
    }
}

pub async fn change_password(
    addr: &str,
    username: &str,
    password: &str,
    new_password: &str,
) -> Result<(), String> {
    let password_clone = password.to_string();
    let new_password = new_password.to_string();

    let command = "passwd";
    let password_prompt = "Current password:";
    let new_password_prompt = "New password:";
    let confirm_password_prompt = "Retype new password:";

    let (_stdout, stderr, exit_code) = run_ssh_command(addr, username, password, move |channel| {
        // Send the `passwd` command to start the password change process
        channel.exec(command).unwrap();

        // Use `expect` to wait for the password prompt and send the current password
        expect(channel, password_prompt);
        channel.write_all(password_clone.as_bytes()).unwrap();
        channel.write_all(b"\n").unwrap();

        // Use `expect` to wait for the new password prompt and send the new password
        expect(channel, new_password_prompt);
        channel.write_all(new_password.as_bytes()).unwrap();
        channel.write_all(b"\n").unwrap();

        // Use `expect` to wait for the password confirmation prompt and send the new password again
        expect(channel, confirm_password_prompt);
        channel.write_all(new_password.as_bytes()).unwrap();
        channel.write_all(b"\n").unwrap();

        // Use `exit_status` to retrieve the exit code of the `passwd` command
        channel.exit_status().unwrap();
    })
    .await;

    if exit_code != 0 {
        Err(stderr)
    } else {
        Ok(())
    }
}
