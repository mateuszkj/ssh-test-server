use crate::session::SshConnection;
use crate::user::User;
use crate::{SshExecuteHandler, SshServer};
use anyhow::Result;
use rand::Rng;
use random_port::PortPicker;
use russh::{server, MethodSet};
use russh_keys::key;
use russh_keys::key::KeyPair;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpListener;
use tracing::debug;

/// Builder for the ssh server.
///
/// # Example
///
/// ```
/// use ssh_test_server::{SshServerBuilder, User};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let _ssh = SshServerBuilder::default()
///     .add_user(User::new_admin("root", "pass123"))
///     .run()
///     .await
///     .unwrap();
/// # }
/// ```
#[derive(Default)]
pub struct SshServerBuilder {
    port: Option<u16>,
    bind_addr: Option<String>,
    users: Vec<User>,
    programs: HashMap<String, Box<SshExecuteHandler>>,
}

impl SshServerBuilder {
    /// Add a user to ssh server.
    ///
    /// # Example
    ///
    /// ```
    /// # use ssh_test_server::{SshServerBuilder, User};
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let _ssh = SshServerBuilder::default()
    ///     .add_user(User::new_admin("root", "pass"))
    ///     .add_user(User::new("luiza", "obrazy"))
    ///     .run()
    ///     .await
    ///     .unwrap();
    /// # }
    /// ```
    pub fn add_user(mut self, user: User) -> Self {
        self.users.push(user);
        self
    }

    /// Add list of users.
    ///
    ///
    /// ```
    /// # use ssh_test_server::{SshServerBuilder, User};
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let users = vec![User::new("a", "p"), User::new("b", "p")];
    /// let _ssh = SshServerBuilder::default()
    ///     .add_users(&users)
    ///     .run()
    ///     .await
    ///     .unwrap();
    /// # }
    /// ```
    pub fn add_users(mut self, users: &[User]) -> Self {
        for u in users {
            self.users.push(u.clone());
        }
        self
    }

    /// Add custom command/program.
    ///
    /// # Example
    ///
    /// ```
    /// # use ssh_test_server::{SshExecuteContext, SshServerBuilder, SshExecuteResult, User};
    /// fn cmd_print_message(
    ///     context: &SshExecuteContext,
    ///     program: &str,
    ///     args: &[&str],
    /// ) -> SshExecuteResult {
    ///     let stdout = format!(
    ///         "Program {program} run by {} has {} args.",
    ///         context.current_user,
    ///         args.len()
    ///     );
    ///     SshExecuteResult::stdout(0, stdout)
    /// }
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let _ssh = SshServerBuilder::default()
    ///     .add_program("print_message", Box::new(cmd_print_message))
    ///     .run()
    ///     .await
    ///     .unwrap();
    /// # }
    /// ```
    pub fn add_program(mut self, program: &str, handler: Box<SshExecuteHandler>) -> Self {
        self.programs.insert(program.to_string(), handler);
        self
    }

    /// Listen on address.
    ///
    /// # Example
    ///
    /// ```
    /// # use ssh_test_server::SshServerBuilder;
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let _ssh = SshServerBuilder::default()
    ///     .bind_addr("127.0.0.1")
    ///     .run()
    ///     .await
    ///     .unwrap();
    /// # }
    pub fn bind_addr(mut self, bind_addr: &str) -> Self {
        self.bind_addr = Some(bind_addr.to_string());
        self
    }

    /// Listen on port.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use ssh_test_server::SshServerBuilder;
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let _ssh = SshServerBuilder::default()
    ///     .port(9992)
    ///     .run()
    ///     .await
    ///     .unwrap();
    /// # }
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Build and run the ssh server.
    ///
    /// Server stops when [SshServer] is dropped.
    pub async fn run(self) -> Result<SshServer> {
        let host = self
            .bind_addr
            .clone()
            .unwrap_or_else(|| "127.0.0.1".to_string());

        let port = self.port.unwrap_or_else(|| {
            PortPicker::new().random(true).pick().unwrap_or_else(|_| {
                let mut rng = rand::thread_rng();
                rng.gen_range(15000..55000)
            })
        });
        let addr = format!("{host}:{port}");

        let server_keys = KeyPair::generate_ed25519();
        let server_public_key = server_keys.clone_public_key()?;

        let mut config = server::Config {
            methods: MethodSet::PASSWORD,
            auth_rejection_time: Duration::from_secs(0),
            ..Default::default()
        };
        config.preferred.key = Cow::Borrowed(&[key::ED25519]);
        config.keys.push(server_keys);
        let config = Arc::new(config);
        let users: Arc<Mutex<HashMap<String, User>>> = Arc::new(Mutex::new(
            self.users
                .into_iter()
                .map(|u| (u.login().to_string(), u))
                .collect(),
        ));

        let socket = TcpListener::bind(addr).await?;
        let users2 = users.clone();

        let listener = tokio::spawn(async move {
            let programs = Arc::new(self.programs);
            let mut id = 0;
            while let Ok((socket, addr)) = socket.accept().await {
                let config = config.clone();
                debug!("New connection from {addr:?}");
                let s = SshConnection::new(id, users2.clone(), programs.clone());
                tokio::spawn(server::run_stream(config, socket, s));
                id += 1;
            }
            debug!("ssh server stopped");
        });

        Ok(SshServer {
            listener,
            users,
            port,
            host,
            server_public_key,
        })
    }
}
