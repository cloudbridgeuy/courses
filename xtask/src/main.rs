#![deny(clippy::unwrap_used, clippy::expect_used)]

//! Repository task runner. Invoked as `cargo xtask <command>` via the
//! globally installed `cargo-xtask` wrapper, or directly with
//! `cargo run -p xtask -- <command>`.

mod clean;
mod dev;
mod lint;

use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

#[derive(Parser)]
#[command(name = "xtask", about = "Repository task runner")]
struct App {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the lint pipeline: fmt, check, clippy, test, builtin checks.
    Lint(lint::LintArgs),
    /// Start the full local dev stack (DynamoDB Local + server).
    Dev(dev::DevArgs),
    /// Prune stale build artifacts from target/.
    Clean(clean::CleanArgs),
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    color_eyre::install()?;
    match App::parse().command {
        Commands::Lint(args) => lint::run(&args),
        Commands::Dev(args) => dev::run(args).await,
        Commands::Clean(args) => clean::run(&args),
    }
}
