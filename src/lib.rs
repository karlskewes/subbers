pub mod error;
pub mod game;
pub mod http;
pub mod metrics;
pub mod player;
pub mod repo;
pub mod svc;

pub use self::error::{Error, ErrorKind};
pub use self::game::GameView;
pub use self::game::{Event, EventError};
pub use self::game::{Game, GameState, into_game_views};
pub use self::http::{AxumApp, User};
pub use self::player::{Player, PlayerView, into_player_views};
pub use self::repo::{InMemoryRepo, Repo, SqliteRepo};
pub use self::svc::Service;

use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const SQLITE_FILEPATH: &str = "subbers.sql";
const LISTEN_ADDR: &str = "0.0.0.0:8080";
const METRICS_LISTEN_ADDR: &str = "0.0.0.0:8081";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CLIArgs {
    /// SQLite file path
    #[arg(short, long, default_value = SQLITE_FILEPATH)]
    sqlite_filepath: Option<std::path::PathBuf>,

    /// Listen Address for HTTP server
    #[arg(short, long, default_value = LISTEN_ADDR)]
    listen_addr: String,

    /// Listen Address for Prometheus Metrics HTTP server
    #[arg(short, long, default_value = METRICS_LISTEN_ADDR)]
    metrics_listen_addr: String,

    /// Basic Auth 'user:pass' for HTTP server
    #[arg(short, long)]
    basic_auth: Option<String>,
}

pub enum RepoConfig {
    Sqlite(Option<std::path::PathBuf>),
    InMemory,
}

impl Default for RepoConfig {
    fn default() -> Self {
        Self::Sqlite(Some(std::path::PathBuf::from(SQLITE_FILEPATH)))
    }
}

pub struct Config {
    repo_config: RepoConfig,
    listen_addr: String,
    metrics_listen_addr: String,
    basic_auth: Option<User>,
    version: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen_addr: String::from(LISTEN_ADDR),
            metrics_listen_addr: String::from(METRICS_LISTEN_ADDR),
            repo_config: RepoConfig::default(),
            basic_auth: None,
            version: "devel".to_string(),
        }
    }
}

impl Config {
    pub fn build(args: &[String]) -> Result<Config, Error> {
        let mut cfg = Config::default();

        let args = CLIArgs::try_parse_from(args)
            .map_err(|e| Error::new(ErrorKind::InvalidInput(e.to_string())))?;

        cfg.repo_config = RepoConfig::Sqlite(args.sqlite_filepath);

        if !args.listen_addr.is_empty() {
            cfg.listen_addr = args.listen_addr;
        }

        if !args.metrics_listen_addr.is_empty() {
            cfg.metrics_listen_addr = args.metrics_listen_addr;
        }

        let version = env!("GIT_HASH");
        if !version.is_empty() {
            cfg.version = version.to_string();
        }

        cfg.basic_auth = args
            .basic_auth
            .as_ref()
            .map(|ba| {
                ba.split_once(':')
                    .ok_or_else(|| {
                        Error::new(ErrorKind::InvalidInput(
                            "basic auth must be in the form 'user:pass'".to_string(),
                        ))
                    })
                    .and_then(|(username, password)| {
                        if username.is_empty() {
                            Err(Error::new(ErrorKind::InvalidInput(
                                "basic auth username cannot be empty".to_string(),
                            )))
                        } else if password.is_empty() {
                            Err(Error::new(ErrorKind::InvalidInput(
                                "basic auth password cannot be empty".to_string(),
                            )))
                        } else {
                            Ok(User {
                                username: username.to_string(),
                                password: password.to_string(),
                            })
                        }
                    })
            })
            .transpose()?;

        Ok(cfg)
    }
}

pub async fn run(cfg: Config) -> Result<(), std::io::Error> {
    // TODO: logfmt or json for local/prod on fmt.json()
    // TODO: consider this a default, enable passing in.
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let metrics_listen_addr: SocketAddr = cfg.metrics_listen_addr.parse().map_err(|e| {
        Error::new(ErrorKind::InvalidInput(format!(
            "--metrics-listen-addr '{}' is not a valid <IP>:<PORT>",
            cfg.metrics_listen_addr
        )))
        .with_source(e)
    })?;

    metrics::init_prometheus_endpoint(metrics_listen_addr, cfg.version)?;

    let repo: Arc<dyn Repo> = match cfg.repo_config {
        RepoConfig::InMemory => Arc::new(InMemoryRepo::new()),
        RepoConfig::Sqlite(p) => {
            let sqlite = SqliteRepo::new(p).map_err(|e| -> std::io::Error { e.into() })?;
            Arc::new(sqlite)
        }
    };
    let svc = Service::new(repo);
    let app = AxumApp::new(cfg.listen_addr, cfg.basic_auth, svc);

    tracing::info!(message = "starting server");
    app.run(shutdown_signal()).await?;

    tracing::info!(message = "shutting down");
    Ok(())
}

// TODO: Should this live in AxumApp?
// Feels like something one passes in from main.rs though and lib.rs is our closest option..
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to register Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to register signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("received OS signal to shutdown, use Ctrl+C again to force");
        },
        _ = terminate => {
            tracing::info!("received OS signal to terminate");
        },
    }
}
