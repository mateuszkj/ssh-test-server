use crate::user::User;
use russh_keys::key::PublicKey;
use std::collections::HashMap;

pub mod builder;
pub mod session;
pub mod user;

/// Running SSH server
#[derive(Debug)]
pub struct SshServer {
    users: HashMap<String, User>,
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
            self.server_public_key.fingerprint()
        )
    }
}
