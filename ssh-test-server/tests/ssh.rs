use cucumber::{given, then, when, World};
use ssh_test_server::{SshServer, SshServerBuilder, User};

mod common;

const SSH_USER: &str = "user1";
const SSH_PASS: &str = "pass123";

#[derive(Debug, Default, World)]
struct SshWorld {
    serer: Option<SshServer>,
    remote_addr: String,

    stdout: String,
    stderr: String,
    status_code: Option<i32>,
}

#[given("Running ssh server")]
async fn running_ssh_server(world: &mut SshWorld) {
    let server = SshServerBuilder::default()
        .add_user(User::new(SSH_USER, SSH_PASS))
        .run()
        .await
        .unwrap();

    world.remote_addr = server.addr();
    world.serer = Some(server);
}

#[when(expr = "Executed command {string} on remote ssh server")]
async fn execute_command(world: &mut SshWorld, command: String) {
    let r = common::run_ssh_command(&world.remote_addr, SSH_USER, SSH_PASS, move |channel| {
        channel.exec(&command).unwrap();
    })
    .await;
    world.stdout = r.0;
    world.stderr = r.1;
    world.status_code = Some(r.2);
}

#[when("Change user password via passwd command")]
async fn execute_passwd(world: &mut SshWorld) {
    let new_pass = format!("{SSH_PASS}_changed");
    common::change_password(&world.remote_addr, SSH_USER, SSH_PASS, &new_pass)
        .await
        .unwrap();
}

#[then(expr = "Got exit code {int} and response {string} and error containing {string}")]
fn got_command_result(world: &mut SshWorld, status_code: i32, stdout: String, stderr: String) {
    assert_eq!(world.stdout.trim(), stdout);
    assert!(
        world.stderr.contains(&stderr),
        "stderr should contain text '{stderr}' but got '{}'",
        world.stderr
    );
    assert_eq!(world.status_code, Some(status_code));
}

#[then("Password has been changed")]
async fn password_has_been_changed(world: &mut SshWorld) {
    let new_pass = format!("{SSH_PASS}_changed");
    let (_, _, status_code) =
        common::run_ssh_command(&world.remote_addr, SSH_USER, &new_pass, |channel| {
            channel.exec("echo abc").unwrap();
        })
        .await;
    assert_eq!(status_code, 0);
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let summary = SshWorld::cucumber()
        .max_concurrent_scenarios(1)
        .init_tracing()
        .run("tests/features/ssh.feature")
        .await;

    assert_eq!(summary.scenarios_stats().failed, 0);
    assert_eq!(summary.scenarios_stats().skipped, 0);
}
