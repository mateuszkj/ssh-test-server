use crate::user::User;
use russh_keys::key::PublicKey;
use russh_keys::PublicKeyBase64;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

pub mod builder;
mod command;
mod session;
pub mod user;

pub type UsersMap = Arc<Mutex<HashMap<String, User>>>;

/// Running SSH server
#[derive(Debug)]
pub struct SshServer {
    listener: JoinHandle<()>,
    users: UsersMap,
    port: u16,
    host: String,
    server_public_key: PublicKey,
}

impl SshServer {
    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.host(), self.port())
    }

    pub fn server_public_key(&self) -> String {
        format!(
            "{} {}",
            self.server_public_key.name(),
            self.server_public_key.public_key_base64()
        )
    }

    pub fn users(&self) -> UsersMap {
        self.users.clone()
    }
}

impl Drop for SshServer {
    fn drop(&mut self) {
        self.listener.abort();
    }
}
