pub mod error;
pub mod game;
pub mod http;
pub mod player;
pub mod repo;
pub mod svc;

pub use self::error::Error;
pub use self::game::GameView;
pub use self::game::{Event, EventError};
pub use self::game::{Game, GameState, into_game_views};
pub use self::http::AxumApp;
pub use self::player::{Player, PlayerView, into_player_views};
pub use self::repo::{InMemoryRepo, Repo, SqliteRepo};
pub use self::svc::Service;

use clap::Parser;
use std::sync::Arc;
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const SQLITE_FILEPATH: &str = "subbers.sql";
const LISTEN_ADDR: &str = "0.0.0.0:8080";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CLIArgs {
    /// SQLite file path
    #[arg(short, long, default_value = SQLITE_FILEPATH)]
    sqlite_filepath: String,

    /// Listen Address for HTTP server
    #[arg(short, long, default_value = LISTEN_ADDR)]
    listen_addr: String,
}

pub enum RepoConfig {
    Sqlite(Option<String>),
    InMemory,
}

impl Default for RepoConfig {
    fn default() -> Self {
        Self::Sqlite(Some(String::from(SQLITE_FILEPATH)))
    }
}

pub struct Config {
    repo_config: RepoConfig,
    listen_addr: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen_addr: String::from(LISTEN_ADDR),
            repo_config: RepoConfig::default(),
        }
    }
}

impl Config {
    pub fn build(args: &[String]) -> Result<Config, std::io::Error> {
        let mut cfg = Config::default();

        let args = CLIArgs::try_parse_from(args)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        if !args.sqlite_filepath.is_empty() {
            cfg.repo_config = RepoConfig::Sqlite(Some(args.sqlite_filepath));
        }

        if !args.listen_addr.is_empty() {
            cfg.listen_addr = args.listen_addr;
        }

        return Ok(cfg);
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

    let repo: Arc<dyn Repo> = match cfg.repo_config {
        RepoConfig::InMemory => Arc::new(InMemoryRepo::new()),
        RepoConfig::Sqlite(p) => {
            let sqlite = SqliteRepo::new(p).map_err(|e| -> std::io::Error { e.into() })?;
            Arc::new(sqlite)
        }
    };
    let svc = Service::new(repo);
    let app = AxumApp::new(cfg.listen_addr, svc);

    app.run(shutdown_signal()).await
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
