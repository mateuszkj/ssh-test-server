use clap::{arg, Parser};
use ssh_test_server::{SshServerBuilder, User};
use tokio::signal;
use tracing::info;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

/// Run OpenLDAP server
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Bind server on address
    #[arg(long)]
    bind_addr: Option<String>,

    /// Port number
    #[arg(long)]
    port: Option<u16>,

    /// User login name
    #[arg(long)]
    login: Option<String>,

    /// User password
    #[arg(long)]
    password: Option<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let args = Args::parse();
    info!("args: {args:?}");

    let mut builder = SshServerBuilder::default();

    if let Some(bind_addr) = &args.bind_addr {
        builder = builder.bind_addr(bind_addr);
    }

    if let Some(port) = args.port {
        builder = builder.port(port);
    }

    let login = args.login.unwrap_or_else(|| "user".to_string());
    let pass = args.password.unwrap_or_else(|| "pass123".to_string());

    builder = builder.add_user(User::new(&login, &pass));

    let server = builder.run().await.unwrap();
    println!("Addr: {}", server.addr());
    println!("Login: {login}");
    println!("Password: {pass}");
    println!("Public Key: {}", server.server_public_key());
    println!("ssh -l {login} -p {} {}", server.port(), server.host());

    info!("waiting for ctrl-c");
    signal::ctrl_c().await.expect("failed to listen for event");
}
