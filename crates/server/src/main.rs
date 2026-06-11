#![deny(clippy::unwrap_used, clippy::expect_used)]

//! Imperative shell: axum server for the courses platform.

mod routes;

use std::net::{Ipv4Addr, SocketAddr};

use color_eyre::eyre::{Result, WrapErr};

const DEFAULT_PORT: u16 = 8080;

/// Runtime configuration, parsed once at the process boundary.
#[derive(Debug, Clone, Copy)]
struct Config {
    port: u16,
}

impl Config {
    /// Reads `PORT` from the environment; absent means [`DEFAULT_PORT`].
    fn from_env() -> Result<Self> {
        let port = match std::env::var("PORT") {
            Ok(raw) => raw
                .parse::<u16>()
                .wrap_err_with(|| format!("invalid PORT value: {raw:?}"))?,
            Err(std::env::VarError::NotPresent) => DEFAULT_PORT,
            Err(e) => return Err(e).wrap_err("PORT is not valid unicode"),
        };
        Ok(Self { port })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = Config::from_env()?;
    let addr = SocketAddr::from((Ipv4Addr::UNSPECIFIED, config.port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .wrap_err_with(|| format!("failed to bind {addr}"))?;
    tracing::info!("listening on http://{addr}");

    axum::serve(listener, routes::router())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .wrap_err("server error")?;
    Ok(())
}

/// Resolves on SIGINT (ctrl-c), or SIGTERM (what ECS sends on task stop).
///
/// If a handler fails to register, that arm logs, and then stays pending, so a
/// registration failure never triggers shutdown by itself.
async fn shutdown_signal() {
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
        () = ctrl_c => {}
        () = terminate => {}
    }
}
