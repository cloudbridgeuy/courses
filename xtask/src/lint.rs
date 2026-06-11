use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use clap::Args;
use color_eyre::eyre::{Context as _, Result, bail};
use duct::cmd;

const MAX_FILE_LINES: usize = 1000;

#[derive(Debug, Args)]
pub struct LintArgs {
    /// Apply fixes (cargo fmt, cargo clippy --fix) before checking.
    #[arg(long)]
    pub fix: bool,

    /// Print every check's output, even on success.
    #[arg(long)]
    pub verbose: bool,

    /// Skip `cargo fmt --check`.
    #[arg(long)]
    pub no_fmt: bool,

    /// Skip `cargo check`.
    #[arg(long)]
    pub no_check: bool,

    /// Skip `cargo clippy`.
    #[arg(long)]
    pub no_clippy: bool,

    /// Skip `cargo test`.
    #[arg(long)]
    pub no_test: bool,

    /// Skip the file-length check.
    #[arg(long)]
    pub no_file_length: bool,

    /// Skip the forbidden-allows check.
    #[arg(long)]
    pub no_forbidden_allows: bool,
}

// ---------------------------------------------------------------------------
// Functional Core — pure types and logic, no I/O
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckId {
    Fmt,
    Check,
    Clippy,
    Test,
    FileLength,
    ForbiddenAllows,
}

impl CheckId {
    fn name(self) -> &'static str {
        match self {
            Self::Fmt => "fmt",
            Self::Check => "check",
            Self::Clippy => "clippy",
            Self::Test => "test",
            Self::FileLength => "file-length",
            Self::ForbiddenAllows => "forbidden-allows",
        }
    }

    /// The cargo invocation backing this check, if it is command-based.
    fn command(self) -> Option<&'static [&'static str]> {
        match self {
            Self::Fmt => Some(&["fmt", "--check"]),
            Self::Check => Some(&["check", "--workspace", "--all-targets"]),
            Self::Clippy => Some(&[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ]),
            Self::Test => Some(&["test", "--workspace", "--all-targets"]),
            Self::FileLength | Self::ForbiddenAllows => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum CheckOutcome {
    Passed { output: String },
    Failed { output: String },
}

/// Decides which checks run, in pipeline order, from the parsed flags.
fn plan_checks(args: &LintArgs) -> Vec<CheckId> {
    let plan = [
        (CheckId::Fmt, args.no_fmt),
        (CheckId::Check, args.no_check),
        (CheckId::Clippy, args.no_clippy),
        (CheckId::Test, args.no_test),
        (CheckId::FileLength, args.no_file_length),
        (CheckId::ForbiddenAllows, args.no_forbidden_allows),
    ];
    plan.into_iter()
        .filter(|(_, skipped)| !skipped)
        .map(|(id, _)| id)
        .collect()
}

/// Maps a command's exit status, and captured output, to an outcome.
fn determine_outcome(success: bool, output: String) -> CheckOutcome {
    if success {
        CheckOutcome::Passed { output }
    } else {
        CheckOutcome::Failed { output }
    }
}

/// Flags files longer than [`MAX_FILE_LINES`] lines.
fn evaluate_file_lengths(files: &[(PathBuf, usize)]) -> CheckOutcome {
    let violations: Vec<String> = files
        .iter()
        .filter(|(_, lines)| *lines > MAX_FILE_LINES)
        .map(|(path, lines)| format!("{}: {lines} lines (max {MAX_FILE_LINES})", path.display()))
        .collect();

    if violations.is_empty() {
        CheckOutcome::Passed {
            output: String::new(),
        }
    } else {
        CheckOutcome::Failed {
            output: violations.join("\n") + "\n",
        }
    }
}

/// Flags the `too_many_arguments` clippy allow attribute in production code.
///
/// Production code is everything before the first `#[cfg(test)]` marker. This
/// assumes the repo convention of a single, trailing `#[cfg(test)] mod tests`
/// block per file (mandated by CLAUDE.md). A `#[cfg(test)]` placed before a
/// genuine production violation would let that violation pass unflagged.
fn scan_for_forbidden_allows(files: &[(PathBuf, String)]) -> CheckOutcome {
    // Built via concat! so this source file doesn't contain the literal it scans for.
    const FORBIDDEN: &str = concat!("#[allow(clippy::", "too_many_arguments)]");
    const TEST_MARKER: &str = "#[cfg(test)]";

    let violations: Vec<String> = files
        .iter()
        .filter(|(_, content)| {
            let production = match content.find(TEST_MARKER) {
                Some(idx) => &content[..idx],
                None => content.as_str(),
            };
            production.contains(FORBIDDEN)
        })
        .map(|(path, _)| {
            format!(
                "{}: {FORBIDDEN} is forbidden — use a Params struct",
                path.display()
            )
        })
        .collect();

    if violations.is_empty() {
        CheckOutcome::Passed {
            output: String::new(),
        }
    } else {
        CheckOutcome::Failed {
            output: violations.join("\n") + "\n",
        }
    }
}

// ---------------------------------------------------------------------------
// Imperative Shell — I/O, side effects, orchestration
// ---------------------------------------------------------------------------

/// Runs the lint pipeline; stops, and fails, at the first failing check.
pub fn run(args: &LintArgs) -> Result<()> {
    let root = workspace_root();

    if args.fix {
        run_fixes(&root)?;
    }

    let log_path = root.join("target/xtask-lint.log");
    let mut log = open_log(&log_path)?;

    for id in plan_checks(args) {
        let outcome = execute_check(id, &root)?;
        let (output, passed) = match &outcome {
            CheckOutcome::Passed { output } => (output, true),
            CheckOutcome::Failed { output } => (output, false),
        };

        writeln!(log, "=== {} ===", id.name())
            .and_then(|()| log.write_all(output.as_bytes()))
            .wrap_err("failed to write lint log")?;

        if passed {
            if args.verbose {
                print!("{output}");
            }
            println!("✓ {}", id.name());
        } else {
            print!("{output}");
            println!("✗ {}", id.name());
            bail!(
                "lint failed at `{}` (full log: {})",
                id.name(),
                log_path.display()
            );
        }
    }

    Ok(())
}

/// Workspace root: the parent of this crate's manifest directory.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn open_log(path: &Path) -> Result<fs::File> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).wrap_err("failed to create target dir")?;
    }
    fs::File::create(path).wrap_err("failed to create lint log")
}

fn run_fixes(root: &Path) -> Result<()> {
    cmd(
        "cargo",
        [
            "clippy",
            "--workspace",
            "--all-targets",
            "--fix",
            "--allow-dirty",
            "--allow-staged",
        ],
    )
    .dir(root)
    .run()
    .wrap_err("cargo clippy --fix failed")?;
    cmd("cargo", ["fmt"])
        .dir(root)
        .run()
        .wrap_err("cargo fmt failed")?;
    Ok(())
}

fn execute_check(id: CheckId, root: &Path) -> Result<CheckOutcome> {
    if let Some(cargo_args) = id.command() {
        let output = cmd("cargo", cargo_args)
            .dir(root)
            .stderr_to_stdout()
            .stdout_capture()
            .unchecked()
            .run()
            .wrap_err_with(|| format!("failed to spawn `cargo {}`", cargo_args.join(" ")))?;
        let text = String::from_utf8_lossy(&output.stdout).into_owned();
        return Ok(determine_outcome(output.status.success(), text));
    }

    match id {
        CheckId::FileLength => {
            let lengths: Vec<(PathBuf, usize)> = collect_rust_sources(root)?
                .into_iter()
                .map(|(path, content)| (path, content.lines().count()))
                .collect();
            Ok(evaluate_file_lengths(&lengths))
        }
        CheckId::ForbiddenAllows => {
            let sources = collect_rust_sources(root)?;
            Ok(scan_for_forbidden_allows(&sources))
        }
        _ => bail!("check `{}` has no executor", id.name()),
    }
}

/// Reads every `.rs` file under `crates/*/src/`, and `xtask/src/`.
fn collect_rust_sources(root: &Path) -> Result<Vec<(PathBuf, String)>> {
    let mut sources = Vec::new();
    let crates_dir = root.join("crates");
    for entry in fs::read_dir(&crates_dir).wrap_err("failed to read crates/")? {
        let src = entry
            .wrap_err("failed to read crates/ entry")?
            .path()
            .join("src");
        if src.is_dir() {
            collect_rs_files(&src, &mut sources)?;
        }
    }

    let xtask_src = root.join("xtask/src");
    if xtask_src.is_dir() {
        collect_rs_files(&xtask_src, &mut sources)?;
    }
    sources.sort_by(|(a, _), (b, _)| a.cmp(b));
    Ok(sources)
}

fn collect_rs_files(dir: &Path, out: &mut Vec<(PathBuf, String)>) -> Result<()> {
    for entry in fs::read_dir(dir).wrap_err_with(|| format!("failed to read {}", dir.display()))? {
        let path = entry.wrap_err("failed to read dir entry")?.path();
        if path.is_dir() {
            collect_rs_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let content = fs::read_to_string(&path)
                .wrap_err_with(|| format!("failed to read {}", path.display()))?;
            out.push((path, content));
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn args_none_skipped() -> LintArgs {
        LintArgs {
            fix: false,
            verbose: false,
            no_fmt: false,
            no_check: false,
            no_clippy: false,
            no_test: false,
            no_file_length: false,
            no_forbidden_allows: false,
        }
    }

    #[test]
    fn plan_runs_all_checks_in_order() {
        let plan = plan_checks(&args_none_skipped());
        assert_eq!(
            plan,
            vec![
                CheckId::Fmt,
                CheckId::Check,
                CheckId::Clippy,
                CheckId::Test,
                CheckId::FileLength,
                CheckId::ForbiddenAllows,
            ]
        );
    }

    #[test]
    fn plan_skips_flagged_checks() {
        let mut args = args_none_skipped();
        args.no_fmt = true;
        args.no_test = true;
        let plan = plan_checks(&args);
        assert!(!plan.contains(&CheckId::Fmt));
        assert!(!plan.contains(&CheckId::Test));
        assert_eq!(plan.len(), 4);
    }

    #[test]
    fn determine_outcome_maps_status() {
        assert!(matches!(
            determine_outcome(true, "out".to_owned()),
            CheckOutcome::Passed { .. }
        ));
        assert!(matches!(
            determine_outcome(false, "out".to_owned()),
            CheckOutcome::Failed { .. }
        ));
    }

    #[test]
    fn file_lengths_pass_at_limit() {
        let files = vec![(PathBuf::from("a.rs"), MAX_FILE_LINES)];
        assert!(matches!(
            evaluate_file_lengths(&files),
            CheckOutcome::Passed { .. }
        ));
    }

    #[test]
    fn file_lengths_fail_over_limit() {
        let files = vec![(PathBuf::from("a.rs"), MAX_FILE_LINES + 1)];
        let CheckOutcome::Failed { output } = evaluate_file_lengths(&files) else {
            panic!("expected failure");
        };
        assert!(output.contains("a.rs"));
        assert!(output.contains("1001 lines"));
    }

    #[test]
    fn forbidden_allows_flags_production_code() {
        let files = vec![(
            PathBuf::from("a.rs"),
            "#[allow(clippy::too_many_arguments)]\nfn f() {}".to_owned(),
        )];
        assert!(matches!(
            scan_for_forbidden_allows(&files),
            CheckOutcome::Failed { .. }
        ));
    }

    #[test]
    fn forbidden_allows_exempts_test_modules() {
        let files = vec![(
            PathBuf::from("a.rs"),
            "fn f() {}\n#[cfg(test)]\n#[allow(clippy::too_many_arguments)]\nmod tests {}"
                .to_owned(),
        )];
        assert!(matches!(
            scan_for_forbidden_allows(&files),
            CheckOutcome::Passed { .. }
        ));
    }

    #[test]
    fn forbidden_allows_passes_clean_files() {
        let files = vec![(PathBuf::from("a.rs"), "fn f() {}".to_owned())];
        assert!(matches!(
            scan_for_forbidden_allows(&files),
            CheckOutcome::Passed { .. }
        ));
    }

    #[test]
    fn forbidden_allows_misses_violation_after_early_cfg_test_marker() {
        // Documents the known limitation: a violation after a non-trailing
        // `#[cfg(test)]` marker is not caught by the first-marker heuristic.
        let forbidden = concat!("#[allow(clippy::", "too_many_arguments)]");
        let content =
            format!("#[cfg(test)]\nfn helper() {{}}\nfn real() {{}}\n{forbidden}\nfn g() {{}}");
        let files = vec![(PathBuf::from("a.rs"), content)];
        assert!(matches!(
            scan_for_forbidden_allows(&files),
            CheckOutcome::Passed { .. }
        ));
    }
}
