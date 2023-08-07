use ssh_test_server::builder::SshServerBuilder;
use ssh_test_server::user::User;
use ssh_test_server::{SshExecuteContext, SshExecuteResult};

mod common;

const USER_LOGIN: &str = "user1";
const USER_PASS: &str = "pass123";

const ROOT_LOGIN: &str = "root";
const ROOT_PASS: &str = "root1234";

#[tokio::test]
async fn test_run_echo_command() {
    let (stdout, stderr, status_code) = run_command("echo abc", true).await;
    assert_eq!(status_code, 0);
    assert_eq!(stdout.trim(), "abc");
    assert_eq!(stderr, "");
}

#[tokio::test]
async fn test_run_non_existing_command() {
    let (stdout, stderr, status_code) = run_command("x_echo abc", true).await;
    assert_eq!(stdout, "");
    assert_eq!(stderr.trim(), "x_echo: command not found");
    assert_eq!(status_code, 127);
}

#[tokio::test]
async fn test_run_change_password() {
    let (stdout, stderr, status_code) = run_command("change_password 54321", true).await;
    assert_eq!(status_code, 0);
    assert_eq!(stdout.trim(), "password changed");
    assert_eq!(stderr, "");
}

#[tokio::test]
async fn test_run_registered_whoami_root() {
    let (stdout, stderr, status_code) = run_command("whoami", true).await;
    assert_eq!(status_code, 0);
    assert_eq!(stdout, "root\r\n");
    assert_eq!(stderr, "");
}

#[tokio::test]
async fn test_run_registered_whoami_user() {
    let (stdout, stderr, status_code) = run_command("whoami", false).await;
    assert_eq!(status_code, 0);
    assert_eq!(stdout.trim(), USER_LOGIN);
    assert_eq!(stderr, "");
}

async fn run_command(command: &str, root: bool) -> (String, String, i32) {
    let server = SshServerBuilder::default()
        .add_user(User::new_admin(ROOT_LOGIN, ROOT_PASS))
        .add_user(User::new(USER_LOGIN, USER_PASS))
        .add_program("whoami", Box::new(cmd_whoami))
        .run()
        .await
        .unwrap();

    let (login, pass) = if root {
        (ROOT_LOGIN, ROOT_PASS)
    } else {
        (USER_LOGIN, USER_PASS)
    };

    let command = command.to_string();
    common::run_ssh_command(&server.addr(), login, pass, move |channel| {
        channel.exec(&command).unwrap();
    })
    .await
}

fn cmd_whoami(context: &SshExecuteContext, _program: &str, _args: &[&str]) -> SshExecuteResult {
    SshExecuteResult::stdout(0, context.current_user)
}
