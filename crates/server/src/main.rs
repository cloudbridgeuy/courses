#![deny(clippy::unwrap_used, clippy::expect_used)]

//! Imperative shell: axum server for the courses platform.

mod content;
mod dev;
mod echo;
mod file_apps;
mod health;
mod routes;
mod site;

use std::io::IsTerminal;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use color_eyre::eyre::{Result, WrapErr};
use tokio::sync::broadcast;

use crate::echo::EchoConfig;
use crate::health::HealthHandle;
use crate::routes::DevCtx;
use crate::site::{SiteHandle, SiteState};

pub const DEFAULT_PORT: u16 = 8080;

/// Environment variable holding the repository root. Its presence turns on dev
/// mode: content and text assets come from disk, and a watcher hot reloads them.
const DEV_ROOT_ENV: &str = "CB_DEV_ROOT";

/// Buffer of pending reload signals per browser before a slow client lags.
const RELOAD_CHANNEL_CAPACITY: usize = 16;

/// Command line of the binary.
///
/// The subcommand is optional, and no subcommand means `serve`: the container
/// image starts the courses platform with a bare `courses_server`, and any
/// other mode is selected by overriding the command.
#[derive(Debug, Parser)]
#[command(name = "courses_server", version, about = "Courses platform server")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Serve the course guides, and the scenario console. This is the default.
    Serve,
    /// Serve an echo server that answers every request with a JSON description
    /// of that request.
    Echo(EchoArgs),
    /// Probe this server over loopback, and report the result as an exit code.
    /// This is what the ECS container health check runs.
    Healthcheck(HealthcheckArgs),
}

#[derive(Debug, Args)]
struct EchoArgs {
    /// Port to listen on.
    #[arg(long, env = "PORT", default_value_t = DEFAULT_PORT)]
    port: u16,
    /// Public DNS name this server answers to, for example
    /// `echo.server.cloudbridge.com.uy`. The answer reports whether the request
    /// arrived under this name, or by some other route.
    #[arg(long, env = "CB_ECHO_NAME")]
    name: Option<String>,
}

#[derive(Debug, Args)]
struct HealthcheckArgs {
    /// Path to request.
    #[arg(long, default_value = "/health/live")]
    path: String,
    /// Port this server listens on.
    #[arg(long, env = "PORT", default_value_t = DEFAULT_PORT)]
    port: u16,
    /// Give up after this many milliseconds.
    #[arg(long, default_value_t = 2_000)]
    timeout_ms: u64,
}

/// Runtime configuration, parsed once at the process boundary.
#[derive(Debug, Clone)]
struct Config {
    port: u16,
    /// Repository root when dev mode is on, `None` in production.
    dev_root: Option<PathBuf>,
}

impl Config {
    /// Reads `PORT`, and `CB_DEV_ROOT`, from the environment.
    fn from_env() -> Result<Self> {
        let port = match std::env::var("PORT") {
            Ok(raw) => raw
                .parse::<u16>()
                .wrap_err_with(|| format!("invalid PORT value: {raw:?}"))?,
            Err(std::env::VarError::NotPresent) => DEFAULT_PORT,
            Err(e) => return Err(e).wrap_err("PORT is not valid unicode"),
        };
        let dev_root = match std::env::var(DEV_ROOT_ENV) {
            // An empty value means the same as absent, so clearing the variable
            // in a shell script does not half-enable dev mode.
            Ok(raw) if raw.trim().is_empty() => None,
            Ok(raw) => Some(PathBuf::from(raw.trim())),
            Err(std::env::VarError::NotPresent) => None,
            Err(e) => return Err(e).wrap_err(format!("{DEV_ROOT_ENV} is not valid unicode")),
        };
        Ok(Self { port, dev_root })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        // Colors help in a terminal, but CloudWatch keeps the raw escape codes
        // as text, so only use them when stdout really is a terminal.
        .with_ansi(std::io::stdout().is_terminal())
        .init();

    match Cli::parse().command {
        None | Some(Command::Serve) => serve_site(Config::from_env()?).await,
        Some(Command::Echo(args)) => {
            echo::serve(EchoConfig {
                port: args.port,
                public_name: args.name,
            })
            .await
        }
        Some(Command::Healthcheck(args)) => healthcheck(args).await,
    }
}

/// Requests one path over loopback, and turns the answer into an exit code:
/// `0` for a success status, `1` for anything else. It exists so the container
/// health check has a command to run without adding curl to the image.
async fn healthcheck(args: HealthcheckArgs) -> Result<()> {
    let url = format!(
        "http://127.0.0.1:{}/{}",
        args.port,
        args.path.trim_start_matches('/')
    );
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(args.timeout_ms))
        .build()
        .wrap_err("failed to build the health check client")?;
    match client.get(&url).send().await {
        Ok(response) if response.status().is_success() => Ok(()),
        Ok(response) => {
            eprintln!("{url} answered {}", response.status());
            std::process::exit(1)
        }
        Err(e) => {
            eprintln!("{url} failed: {e}");
            std::process::exit(1)
        }
    }
}

/// Binds every interface on `port`, and reports where the server listens.
pub async fn bind(port: u16) -> Result<tokio::net::TcpListener> {
    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .wrap_err_with(|| format!("failed to bind {addr}"))?;
    tracing::info!("listening on http://{addr}");
    Ok(listener)
}

async fn serve_site(config: Config) -> Result<()> {
    // `_watcher` must outlive the server: dropping the notify watcher ends the
    // watch, and hot reload stops silently.
    let (site, dev_ctx, _watcher) = match config.dev_root.as_deref() {
        Some(root) => {
            tracing::warn!(
                root = %root.display(),
                "{DEV_ROOT_ENV} is set: content and text assets are served from disk with hot \
                 reload. Never enable this in production."
            );
            // Bad content here leaves the server running with `Broken`, so it can
            // be fixed with the server running; a watcher that fails to start is
            // a fatal configuration error, handled below.
            let site = SiteHandle::new(dev::load_from_disk(root));
            let (reload, _rx) = broadcast::channel::<()>(RELOAD_CHANNEL_CAPACITY);
            let watcher = dev::spawn_watcher(
                root.to_path_buf(),
                site.clone(),
                reload.clone(),
                content::embedded_file_paths(),
            )
            .wrap_err_with(|| {
                format!(
                    "failed to watch {DEV_ROOT_ENV}={}: it must point at the repository root",
                    root.display()
                )
            })?;
            let ctx = DevCtx {
                reload,
                root: root.to_path_buf(),
            };
            (site, Some(ctx), Some(watcher))
        }
        // Production: bad content aborts startup, same as before dev mode existed.
        None => (
            SiteHandle::new(SiteState::Ready(content::load_site()?)),
            None,
            None,
        ),
    };

    let health = HealthHandle::new(health::settings_from_env());

    axum::serve(
        bind(config.port).await?,
        routes::router(site, dev_ctx, health.clone()).await,
    )
    .with_graceful_shutdown(drain_then_shutdown(health))
    .await
    .wrap_err("server error")
}

/// Waits for the shutdown signal, then holds the listener open while readiness
/// reports `503`, so the load balancer deregisters this target before in-flight
/// requests would be cut. Without the health feature, it resolves at once, and
/// the previous behaviour is unchanged.
async fn drain_then_shutdown(health: HealthHandle) {
    shutdown_signal().await;
    let settings = health.settings();
    if !settings.enabled || settings.drain_secs == 0 {
        return;
    }
    health.begin_draining().await;
    tracing::info!(
        drain_secs = settings.drain_secs,
        "draining: readiness reports 503 while the balancer deregisters this target"
    );
    tokio::time::sleep(std::time::Duration::from_secs(settings.drain_secs)).await;
    tracing::info!("drain window elapsed; closing connections");
}

/// Resolves on SIGINT (ctrl-c), or SIGTERM (what ECS sends on task stop).
///
/// If a handler fails to register, that arm logs, and then stays pending, so a
/// registration failure never triggers shutdown by itself.
pub async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!("failed to listen for ctrl-c: {e}");
            std::future::pending::<()>().await;
        }
    };

    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sigterm) => {
                sigterm.recv().await;
            }
            Err(e) => {
                tracing::error!("failed to listen for SIGTERM: {e}");
                std::future::pending::<()>().await;
            }
        }
    };

    tokio::select! {
        () = ctrl_c => tracing::info!(signal = "SIGINT", "graceful shutdown requested"),
        () = terminate => tracing::info!(signal = "SIGTERM", "graceful shutdown requested"),
    }
}
