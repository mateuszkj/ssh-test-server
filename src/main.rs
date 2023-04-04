use color_eyre::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thrussh::server::{Auth, Response, Session};
use thrussh::*;
use thrussh_keys::key::{KeyPair, PublicKey};
use thrussh_keys::*;
use tokio::time::sleep;
use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    color_eyre::install()?;

    let server_keys = KeyPair::generate_ed25519().unwrap();
    let pubkeys = server_keys.clone_public_key();

    let mut config = server::Config {
        methods: MethodSet::PASSWORD,
        ..Default::default()
    };
    config.keys.push(server_keys);

    let config = Arc::new(config);
    info!(
        "public key: {} {} test@test",
        pubkeys.name(),
        pubkeys.fingerprint()
    );

    let sh = Server {
        clients: Arc::new(Mutex::new(HashMap::new())),
        id: 0,
    };

    server::run(config, "0.0.0.0:2222", sh).await?;
    Ok(())
}

#[derive(Clone)]
struct Server {
    clients: Arc<Mutex<HashMap<(usize, ChannelId), thrussh::server::Handle>>>,
    id: usize,
}

impl server::Server for Server {
    type Handler = Self;
    fn new(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.id += 1;
        s
    }
}

impl server::Handler for Server {
    type Error = color_eyre::Report;
    type FutureAuth = futures::future::Ready<Result<(Self, server::Auth), color_eyre::Report>>;
    type FutureUnit = futures::future::Ready<Result<(Self, Session), color_eyre::Report>>;
    type FutureBool = futures::future::Ready<Result<(Self, Session, bool), color_eyre::Report>>;

    fn finished_auth(self, auth: Auth) -> Self::FutureAuth {
        futures::future::ready(Ok((self, auth)))
    }
    fn finished_bool(self, b: bool, s: Session) -> Self::FutureBool {
        futures::future::ready(Ok((self, s, b)))
    }
    fn finished(self, s: Session) -> Self::FutureUnit {
        futures::future::ready(Ok((self, s)))
    }

    fn auth_none(self, user: &str) -> Self::FutureAuth {
        info!("auth_none user={user}");
        self.finished_auth(Auth::Reject)
    }

    fn auth_password(self, user: &str, password: &str) -> Self::FutureAuth {
        info!("auth_password user={user} password={password}");
        self.finished_auth(Auth::Accept)
    }

    fn auth_publickey(self, user: &str, public_key: &key::PublicKey) -> Self::FutureAuth {
        info!("auth_publickey user={user} public_key={public_key:?}");
        self.finished_auth(Auth::Reject)
    }

    fn auth_keyboard_interactive(
        self,
        user: &str,
        submethods: &str,
        response: Option<Response>,
    ) -> Self::FutureAuth {
        info!("auth_keyboard_interactive user={user} submethods={submethods} respone={response:?}");
        self.finished_auth(Auth::Reject)
    }

    fn channel_open_session(self, channel: ChannelId, mut session: Session) -> Self::FutureUnit {
        session.data(channel, CryptoVec::from_slice(b"elo\n"));
        info!("channel_open_session channel={channel:?} id={}", self.id);
        self.finished(session)
    }
    fn data(self, channel: ChannelId, data: &[u8], mut session: Session) -> Self::FutureUnit {
        info!(
            "server channel: {channel:?} data = {:?} id={}",
            std::str::from_utf8(data),
            self.id,
        );
        session.data(channel, CryptoVec::from_slice(data));
        if data.contains(&b'b') {
            let mut handle = session.handle();
            tokio::task::spawn(async move {
                sleep(Duration::from_secs(2)).await;
                handle
                    .data(channel, CryptoVec::from_slice(b"ala ma kota\n"))
                    .await
                    .unwrap();
            });
        }
        self.finished(session)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        info!("dropped");
    }
}
