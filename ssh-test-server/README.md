# In memory ssh server

`ssh-test-server` provides ssh server to that can be used in integration testing.

## Usage

```rust
use ssh_test_server::{SshServerBuilder, User};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let ssh = SshServerBuilder::default()
        .add_user(User::new_admin("root", "pass123"))
        .run()
        .await
        .unwrap();

    println!("ssh -p {} root@{}", ssh.port(), ssh.host());

    tokio::signal::ctrl_c().await.unwrap();
}
```

## Contributions

Contributions are welcome! Please open an issue or submit a pull request on Gitlab.

Before sending pull request please run lints and tests first:

```shell
cargo install just
just install-dev
just lint
just test
```

## License

This project is licensed under the MIT OR Apache-2.0 License.