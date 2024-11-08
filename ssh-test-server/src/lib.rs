//! This crate allow running mockup ssh server in integration tests.
//!
//! # Examples
//!
//! ```
//! use ssh_test_server::{SshServerBuilder, SshExecuteContext, SshExecuteResult, User};
//!
//! fn cmd_check_password(
//!     context: &SshExecuteContext,
//!     _program: &str,
//!     args: &[&str],
//! ) -> SshExecuteResult {
//!     let mut args = args.iter();
//!     let (Some(login), Some(password)) = (args.next(), args.next()) else {
//!         return SshExecuteResult::stderr(2, "Usage: check_password <login> <password>");
//!     };
//!
//!     if !context.current_admin() {
//!         return SshExecuteResult::stderr(1, "Permission denied.");
//!     }
//!
//!     let users = context.users.lock().unwrap();
//!     let Some(user) = users.get(*login) else {
//!         return SshExecuteResult::stderr(1, format!("Unknown user {login}."));
//!     };
//!
//!     if user.password() == *password {
//!         SshExecuteResult::stdout(0, "Password correct.")
//!     } else {
//!         SshExecuteResult::stderr(1, "Password does not match.")
//!     }
//! }
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() {
//! let ssh = SshServerBuilder::default()
//!     .add_user(User::new("user", "123"))
//!     .add_user(User::new_admin("root", "abc123"))
//!     .add_program("check_password", Box::new(cmd_check_password))
//!     .run()
//!     .await
//!     .unwrap();
//!
//! println!("ssh -p {} root@{} check_password user 123", ssh.port(), ssh.host());
//! # }
//! ```
//!
#![warn(missing_docs)]
use russh_keys::key::PublicKey;
use russh_keys::PublicKeyBase64;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

mod builder;
mod command;
mod session;
mod user;

pub use builder::SshServerBuilder;
pub use user::User;

/// Users required in ssh server context.
/// Key of the hash map is a user login.
pub type UsersMap = Arc<Mutex<HashMap<String, User>>>;

/// Function signature for custom commands.
pub type SshExecuteHandler =
    dyn Fn(&SshExecuteContext, &str, &[&str]) -> SshExecuteResult + Sync + Send;

/// Context of ssh server passed to every custom function.
///
/// For example, it's allows to implement program that modifies
/// user password.
pub struct SshExecuteContext<'a> {
    /// Users registered in server.
    pub users: &'a UsersMap,
    /// Current user's login.
    pub current_user: &'a str,
}

impl<'a> SshExecuteContext<'a> {
    /// Return true if current user has admin flag.
    pub fn current_admin(&self) -> bool {
        self.users
            .lock()
            .unwrap()
            .get(self.current_user)
            .map(|u| u.admin())
            .unwrap_or(false)
    }
}

/// Response that have to be returned by custom command handler.
pub struct SshExecuteResult {
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
    /// Program exit code. Usually 0 means success.
    pub status_code: u32,
}

impl SshExecuteResult {
    /// Create a stdout result.
    ///
    /// # Example
    /// ```
    /// use ssh_test_server::SshExecuteResult;
    /// let result = SshExecuteResult::stdout(0, "Password chained.");
    ///
    /// assert_eq!(result.status_code, 0);
    /// assert_eq!(result.stdout, "Password chained.");
    /// ```
    pub fn stdout(status_code: u32, stdout: impl Into<String>) -> Self {
        Self {
            stdout: stdout.into(),
            stderr: "".to_string(),
            status_code,
        }
    }

    /// Create a stderr result.
    ///
    /// # Example
    /// ```
    /// use ssh_test_server::SshExecuteResult;
    /// let result = SshExecuteResult::stderr(1, "Permission denied.");
    ///
    /// assert_eq!(result.status_code, 1);
    /// assert_eq!(result.stderr, "Permission denied.");
    /// ```
    pub fn stderr(status_code: u32, stderr: impl Into<String>) -> Self {
        Self {
            stdout: "".to_string(),
            stderr: stderr.into(),
            status_code,
        }
    }
}

/// Running SSH server.
///
/// When is dropped then ssh server stops.
#[derive(Debug)]
pub struct SshServer {
    listener: JoinHandle<()>,
    users: UsersMap,
    port: u16,
    host: String,
    server_public_key: PublicKey,
}

impl SshServer {
    /// IP or hostname of the ssh server.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Port number of the ssh server.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Host and port pair.
    ///
    /// Format:
    /// ```text
    /// <host>:<port>
    /// ```
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host(), self.port())
    }

    /// Ssh public key of the ssh server.
    pub fn server_public_key(&self) -> String {
        format!(
            "{} {}",
            self.server_public_key.name(),
            self.server_public_key.public_key_base64()
        )
    }

    /// Registered users in the ssh server.
    pub fn users(&self) -> UsersMap {
        self.users.clone()
    }
}

impl Drop for SshServer {
    fn drop(&mut self) {
        self.listener.abort();
    }
}
