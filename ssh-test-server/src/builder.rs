use crate::session::SshConnection;
use crate::user::User;
use crate::SshServer;
use anyhow::Result;
use rand::Rng;
use russh::{server, MethodSet};
use russh_keys::key::KeyPair;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpListener;
use tracing::info;

#[derive(Debug, Default)]
pub struct SshServerBuilder {
    port: Option<u16>,
    bind_addr: Option<String>,
    users: Vec<User>,
}

impl SshServerBuilder {
    pub fn add_user(mut self, user: User) -> Self {
        self.users.push(user);
        self
    }

    pub fn add_users(mut self, users: &[User]) -> Self {
        for u in users {
            self.users.push(u.clone());
        }
        self
    }

    /// Listen address
    pub fn bind_addr(mut self, bind_addr: &str) -> Self {
        self.bind_addr = Some(bind_addr.to_string());
        self
    }

    /// Listen port
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub async fn run(self) -> Result<SshServer> {
        let host = self
            .bind_addr
            .clone()
            .unwrap_or_else(|| "127.0.0.1".to_string());

        let port = self.port.unwrap_or_else(|| {
            portpicker::pick_unused_port().unwrap_or_else(|| {
                let mut rng = rand::thread_rng();
                rng.gen_range(15000..55000)
            })
        });
        let addr = format!("{host}:{port}");

        let server_keys = KeyPair::generate_ed25519().unwrap();
        let server_public_key = server_keys.clone_public_key().unwrap();

        let mut config = server::Config {
            methods: MethodSet::PASSWORD,
            auth_rejection_time: Duration::from_secs(0),
            ..Default::default()
        };
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
            let mut id = 0;
            while let Ok((socket, addr)) = socket.accept().await {
                let config = config.clone();
                info!("New connection from {addr:?}");
                let s = SshConnection::new(id, users2.clone());
                tokio::spawn(server::run_stream(config, socket, s));
                id += 1;
            }
            info!("ssh server stopped");
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
