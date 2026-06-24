use std::path::Path;
use std::time::Duration;

use clap::Args;
use color_eyre::eyre::{Context as _, Result, bail, eyre};
use tokio::process::Command;

// ---------------------------------------------------------------------------
// Functional Core — pure parsing and data
// ---------------------------------------------------------------------------

/// Parses a single line from a `.env` file.
///
/// Returns `Some((key, value))` for valid `KEY=VALUE` lines,
/// `None` for blank lines and comments; logs a warning for malformed lines.
fn parse_env_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    // Strip optional `export ` / `export\t` prefix so `export KEY=VALUE` works.
    let trimmed = trimmed
        .strip_prefix("export ")
        .or_else(|| trimmed.strip_prefix("export\t"))
        .unwrap_or(trimmed);
    let (key, raw_value) = trimmed.split_once('=')?;
    let key = key.trim().to_owned();
    let value = strip_quotes(raw_value.trim());
    Some((key, value.to_owned()))
}

/// Strips surrounding single or double quotes from a value string.
fn strip_quotes(s: &str) -> &str {
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Parses a `docker port <id> 8000` output line to extract the host port.
///
/// Handles both `0.0.0.0:49153` and `[::]:49153` forms by splitting on the
/// last `:`.
fn parse_host_port(output: &str) -> Option<u16> {
    let line = output.lines().next()?.trim();
    let port_str = line.rsplit(':').next()?;
    port_str.parse().ok()
}

// ---------------------------------------------------------------------------
// Imperative Shell — I/O, side effects, orchestration
// ---------------------------------------------------------------------------

#[derive(Debug, Args)]
pub struct DevArgs {
    /// DynamoDB table name.
    #[arg(long, default_value = "courses-apps", env = "CB_APPS_TABLE")]
    pub table: String,

    /// AWS region for DynamoDB Local.
    #[arg(long, default_value = "us-east-1", env = "AWS_REGION")]
    pub region: String,

    /// DynamoDB Local Docker image.
    #[arg(long, default_value = "amazon/dynamodb-local")]
    pub image: String,

    /// Docker container name for DynamoDB Local.
    #[arg(long, default_value = "courses-dynamodb-local")]
    pub container_name: String,
}

/// RAII guard: force-removes the container on drop as a SIGKILL safety net.
///
/// The container is started with `--rm`, which handles cleanup on the happy
/// path (container exits cleanly). This guard exists solely to ensure
/// `docker rm -f` is called if the process is killed abnormally, so the
/// container is never left dangling.
struct ContainerGuard {
    runtime: String,
    container_id: String,
}

impl Drop for ContainerGuard {
    fn drop(&mut self) {
        let _ = duct::cmd!(
            self.runtime.as_str(),
            "rm",
            "-f",
            self.container_id.as_str()
        )
        .stdout_null()
        .stderr_null()
        .run();
    }
}

pub async fn run(args: DevArgs) -> Result<()> {
    let root = workspace_root();

    // 1. Load .env
    load_dot_env(&root);

    // 2. Detect container runtime
    let runtime = detect_runtime()?;
    println!("[dev] using container runtime: {runtime}");

    // 3. Start DynamoDB Local
    let container_id = start_dynamo(&runtime, &args)?;
    println!("[dev] DynamoDB Local container: {container_id}");

    // Bind RAII guard immediately so any early return tears down the container.
    let _guard = ContainerGuard {
        runtime: runtime.clone(),
        container_id: container_id.clone(),
    };

    // 4. Discover ephemeral port
    let port = discover_port(&runtime, &container_id)?;
    let endpoint = format!("http://127.0.0.1:{port}");
    println!("[dev] DynamoDB Local endpoint: {endpoint}");

    // 5. Wait for readiness
    wait_for_tcp(port).await?;
    println!("[dev] DynamoDB Local is ready");

    // 6. Create the table (with retry — HTTP layer can lag behind TCP)
    create_table_with_retry(&endpoint, &args.region, &args.table).await?;

    // 7. Build the server binary, then spawn it directly so start_kill() reaches
    //    the real process rather than an intermediate `cargo` wrapper.
    println!("[dev] building courses_server\u{2026}");
    duct::cmd!("cargo", "build", "-p", "courses_server")
        .dir(&root)
        .run()
        .wrap_err("failed to build courses_server")?;

    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("target"));
    let server_bin = target_dir.join("debug").join("courses_server");
    let server_port = std::env::var("PORT").unwrap_or_else(|_| "8090".to_owned());
    println!(
        "\n[dev] starting server → http://127.0.0.1:{server_port}\n\
         [dev] endpoint={endpoint}  table={}\n\
         [dev] press Ctrl-C to stop\n",
        args.table
    );

    let mut child = Command::new(&server_bin)
        .current_dir(&root)
        .env("AWS_ENDPOINT_URL_DYNAMODB", &endpoint)
        .env("AWS_REGION", &args.region)
        .env("AWS_ACCESS_KEY_ID", "test")
        .env("AWS_SECRET_ACCESS_KEY", "test")
        .env("CB_APPS_TABLE", &args.table)
        .spawn()
        .wrap_err("failed to spawn courses_server")?;

    // 8. Wait for server exit or Ctrl-C
    tokio::select! {
        status = child.wait() => {
            match status {
                Ok(s) if s.success() => println!("[dev] server exited cleanly"),
                Ok(s) => println!("[dev] server exited with status: {s}"),
                Err(e) => println!("[dev] error waiting for server: {e}"),
            }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\n[dev] Ctrl-C received — stopping server and container\u{2026}");
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
    }

    // _guard drops here; `--rm` already cleaned up on the happy path, but the
    // guard issues `rm -f` to cover abnormal exits.
    Ok(())
}

/// Workspace root: the parent of this crate's manifest directory.
fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| std::path::PathBuf::from("."), Path::to_path_buf)
}

/// Reads `.env` from the workspace root and injects each `KEY=VALUE` into the
/// current process environment so spawned children inherit them.
fn load_dot_env(root: &Path) {
    let path = root.join(".env");
    let Ok(content) = std::fs::read_to_string(&path) else {
        println!("[dev] no .env found \u{2014} copy .env.example to .env to configure");
        return;
    };

    let mut loaded = 0usize;
    for (lineno, line) in content.lines().enumerate() {
        match parse_env_line(line) {
            Some((k, v)) => {
                // SAFETY: `main` uses a current-thread Tokio runtime, so no
                // worker threads exist. `load_dot_env` is called at the very
                // top of `dev::run`, before any child process or signal-handler
                // thread is created, guaranteeing no concurrent readers of the
                // environment.
                #[allow(unsafe_code)]
                unsafe {
                    std::env::set_var(&k, &v);
                }
                loaded += 1;
            }
            None if line.trim().is_empty() || line.trim().starts_with('#') => {}
            None => {
                println!("[dev] .env line {}: ignoring malformed entry", lineno + 1);
            }
        }
    }
    println!("[dev] loaded {loaded} variable(s) from .env");
}

/// Returns `"docker"` or `"podman"`, whichever is found first.
fn detect_runtime() -> Result<String> {
    for rt in ["docker", "podman"] {
        if duct::cmd!(rt, "--version")
            .stdout_null()
            .stderr_null()
            .unchecked()
            .run()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(rt.to_owned());
        }
    }
    bail!("neither `docker` nor `podman` found \u{2014} install one to run `xtask dev`")
}

/// Removes a stale container if present, then starts a fresh one in detached
/// mode with an ephemeral host port. Returns the trimmed container ID.
fn start_dynamo(runtime: &str, args: &DevArgs) -> Result<String> {
    // Remove stale container, ignoring errors (it may not exist).
    let _ = duct::cmd!(runtime, "rm", "-f", args.container_name.as_str())
        .stdout_null()
        .stderr_null()
        .unchecked()
        .run();

    let id = duct::cmd!(
        runtime,
        "run",
        "-d",
        "--rm",
        "-p",
        "0:8000",
        "--name",
        args.container_name.as_str(),
        args.image.as_str(),
        "-jar",
        "DynamoDBLocal.jar",
        "-inMemory",
        "-sharedDb"
    )
    .read()
    .wrap_err("failed to start DynamoDB Local container")?;

    Ok(id.trim().to_owned())
}

/// Queries the Docker-assigned host port for container port 8000.
fn discover_port(runtime: &str, container_id: &str) -> Result<u16> {
    let output = duct::cmd!(runtime, "port", container_id, "8000")
        .read()
        .wrap_err("failed to run `docker port`")?;
    parse_host_port(&output)
        .ok_or_else(|| eyre!("could not parse host port from `docker port` output: {output:?}"))
}

/// Polls `127.0.0.1:<port>` with async TCP connect every 200ms up to 30s.
async fn wait_for_tcp(port: u16) -> Result<()> {
    let addr = format!("127.0.0.1:{port}");
    let attempt_timeout = Duration::from_millis(200);
    for _ in 0..150 {
        let connect = tokio::net::TcpStream::connect(&addr);
        if tokio::time::timeout(attempt_timeout, connect).await.is_ok() {
            return Ok(());
        }
        tokio::time::sleep(attempt_timeout).await;
    }
    bail!("DynamoDB Local did not become reachable on port {port} within 30s")
}

/// Calls `ensure_table` up to 10 times with 500ms between attempts.
async fn create_table_with_retry(endpoint: &str, region: &str, table: &str) -> Result<()> {
    let client = courses_apps::dev_dynamo_client(endpoint, region);
    for attempt in 1..=10u32 {
        match courses_apps::ensure_table(&client, table).await {
            Ok(()) => {
                println!("[dev] table '{table}' ready");
                return Ok(());
            }
            Err(e) => {
                println!("[dev] attempt {attempt}/10 failed: {e}");
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }
    bail!("could not create DynamoDB table '{table}' after 10 attempts")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn parse_env_line_basic() {
        let (k, v) = parse_env_line("PORT=8090").unwrap();
        assert_eq!(k, "PORT");
        assert_eq!(v, "8090");
    }

    #[test]
    fn parse_env_line_strips_double_quotes() {
        let (k, v) = parse_env_line(r#"SECRET="abc123""#).unwrap();
        assert_eq!(k, "SECRET");
        assert_eq!(v, "abc123");
    }

    #[test]
    fn parse_env_line_strips_single_quotes() {
        let (_, v) = parse_env_line("KEY='val'").unwrap();
        assert_eq!(v, "val");
    }

    #[test]
    fn parse_env_line_skips_comment() {
        assert!(parse_env_line("# comment").is_none());
    }

    #[test]
    fn parse_env_line_skips_blank() {
        assert!(parse_env_line("   ").is_none());
    }

    #[test]
    fn parse_env_line_rejects_no_equals() {
        assert!(parse_env_line("NOEQUALS").is_none());
    }

    #[test]
    fn parses_export_prefixed_line() {
        let (k, v) = parse_env_line("export PORT=8090").unwrap();
        assert_eq!(k, "PORT");
        assert_eq!(v, "8090");
    }

    #[test]
    fn keeps_equals_in_value() {
        let (k, v) = parse_env_line("KEY=a=b=c").unwrap();
        assert_eq!(k, "KEY");
        assert_eq!(v, "a=b=c");
    }

    #[test]
    fn parse_host_port_ipv4() {
        assert_eq!(parse_host_port("0.0.0.0:49153"), Some(49153));
    }

    #[test]
    fn parse_host_port_ipv6() {
        assert_eq!(parse_host_port("[::]:49153"), Some(49153));
    }

    #[test]
    fn parse_host_port_multi_line_takes_first() {
        assert_eq!(parse_host_port("0.0.0.0:49153\n[::]:49153"), Some(49153));
    }

    #[test]
    fn strip_quotes_double() {
        assert_eq!(strip_quotes(r#""hello""#), "hello");
    }

    #[test]
    fn strip_quotes_single() {
        assert_eq!(strip_quotes("'hello'"), "hello");
    }

    #[test]
    fn strip_quotes_no_quotes() {
        assert_eq!(strip_quotes("hello"), "hello");
    }

    #[test]
    fn strip_quotes_mismatched() {
        assert_eq!(strip_quotes("\"hello'"), "\"hello'");
    }
}
