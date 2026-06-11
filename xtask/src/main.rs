#![deny(clippy::unwrap_used, clippy::expect_used)]

//! Repository task runner. Invoked as `cargo xtask <command>` via the
//! globally installed `cargo-xtask` wrapper, or directly with
//! `cargo run -p xtask -- <command>`.

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
}

fn main() -> Result<()> {
    color_eyre::install()?;
    match App::parse().command {
        Commands::Lint(args) => lint::run(&args),
    }
}
