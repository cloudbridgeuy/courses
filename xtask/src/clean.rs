use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use color_eyre::eyre::{Context as _, Result, bail};
use duct::cmd;

/// Default artifact age, in days, above which pruning kicks in.
const DEFAULT_DAYS: u32 = 7;

#[derive(Debug, Args)]
pub struct CleanArgs {
    /// Prune build artifacts untouched for more than this many days.
    #[arg(long, default_value_t = DEFAULT_DAYS)]
    pub days: u32,

    /// Also delete the incremental-compilation caches (`target/*/incremental`).
    #[arg(long)]
    pub incremental: bool,

    /// Delete the whole target directory instead of pruning selectively.
    #[arg(long, conflicts_with_all = ["days", "incremental"])]
    pub all: bool,

    /// Report what would be deleted without deleting anything.
    #[arg(long)]
    pub dry_run: bool,
}

// ---------------------------------------------------------------------------
// Which steps a run performs, and how it reports what they recovered
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CleanStep {
    /// Prune artifacts by age, via the `cargo-sweep` subcommand.
    Sweep,
    /// Delete the incremental-compilation caches outright.
    Incremental,
    /// `cargo clean` — remove the entire target directory.
    Full,
}

impl CleanStep {
    fn name(self) -> &'static str {
        match self {
            Self::Sweep => "sweep",
            Self::Incremental => "incremental",
            Self::Full => "full",
        }
    }
}

/// Decides which steps run, in order, from the parsed flags.
///
/// `--all` is exclusive: a full clean subsumes every selective step.
fn plan_steps(args: &CleanArgs) -> Vec<CleanStep> {
    if args.all {
        return vec![CleanStep::Full];
    }
    let mut steps = vec![CleanStep::Sweep];
    if args.incremental {
        steps.push(CleanStep::Incremental);
    }
    steps
}

/// Builds the `cargo sweep` invocation for an age threshold.
fn sweep_args(days: u32, dry_run: bool) -> Vec<String> {
    let mut args = vec!["sweep".to_owned(), "--time".to_owned(), days.to_string()];
    if dry_run {
        args.push("--dry-run".to_owned());
    }
    args
}

/// Renders a byte count in binary units, to one decimal place.
fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    #[allow(clippy::cast_precision_loss)]
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

/// Summarizes the space a clean run recovered.
fn reclaimed_line(before: u64, after: u64) -> String {
    let freed = before.saturating_sub(after);
    format!(
        "reclaimed {} ({} → {})",
        format_size(freed),
        format_size(before),
        format_size(after)
    )
}

// ---------------------------------------------------------------------------
// Running those steps against target/
// ---------------------------------------------------------------------------

/// Runs the clean pipeline, reporting the space each step recovered.
pub fn run(args: &CleanArgs) -> Result<()> {
    let root = workspace_root();
    let target = root.join("target");

    if !target.exists() {
        println!("target/ does not exist — nothing to clean");
        return Ok(());
    }

    let steps = plan_steps(args);
    if steps.contains(&CleanStep::Sweep) {
        ensure_cargo_sweep()?;
    }

    let before = dir_size(&target)?;
    println!("target/ is {}", format_size(before));

    for step in steps {
        execute_step(step, &root, &target, args)?;
    }

    if args.dry_run {
        println!("dry run — nothing deleted");
    } else {
        let after = dir_size(&target)?;
        println!("{}", reclaimed_line(before, after));
    }
    Ok(())
}

/// Workspace root: the parent of this crate's manifest directory.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn ensure_cargo_sweep() -> Result<()> {
    let found = cmd("cargo", ["sweep", "--version"])
        .stdout_null()
        .stderr_null()
        .unchecked()
        .run()
        .is_ok_and(|out| out.status.success());
    if found {
        return Ok(());
    }
    bail!("`cargo sweep` is not installed — run `cargo install cargo-sweep`");
}

fn execute_step(step: CleanStep, root: &Path, target: &Path, args: &CleanArgs) -> Result<()> {
    match step {
        CleanStep::Sweep => {
            println!("→ pruning artifacts older than {} days", args.days);
            cmd("cargo", sweep_args(args.days, args.dry_run))
                .dir(root)
                .run()
                .wrap_err("cargo sweep failed")?;
        }
        CleanStep::Incremental => {
            for dir in incremental_dirs(target)? {
                let size = dir_size(&dir)?;
                println!("→ {} ({})", dir.display(), format_size(size));
                if !args.dry_run {
                    fs::remove_dir_all(&dir)
                        .wrap_err_with(|| format!("failed to remove {}", dir.display()))?;
                }
            }
        }
        CleanStep::Full => {
            println!("→ cargo clean");
            if !args.dry_run {
                cmd("cargo", ["clean"])
                    .dir(root)
                    .run()
                    .wrap_err("cargo clean failed")?;
            }
        }
    }
    println!("✓ {}", step.name());
    Ok(())
}

/// Finds every per-profile incremental cache, e.g. `target/debug/incremental`.
fn incremental_dirs(target: &Path) -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    for entry in fs::read_dir(target).wrap_err("failed to read target/")? {
        let profile = entry.wrap_err("failed to read target/ entry")?.path();
        let incremental = profile.join("incremental");
        if incremental.is_dir() {
            dirs.push(incremental);
        }
    }
    dirs.sort();
    Ok(dirs)
}

/// Total size of a directory tree, following no symlinks.
fn dir_size(dir: &Path) -> Result<u64> {
    let mut total = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        // Racing builds, and permission quirks, must not abort a size report.
        let Ok(entries) = fs::read_dir(&path) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                stack.push(entry.path());
            } else if meta.is_file() {
                total += meta.len();
            }
        }
    }
    Ok(total)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn args_default() -> CleanArgs {
        CleanArgs {
            days: DEFAULT_DAYS,
            incremental: false,
            all: false,
            dry_run: false,
        }
    }

    #[test]
    fn plan_defaults_to_sweep_only() {
        assert_eq!(plan_steps(&args_default()), vec![CleanStep::Sweep]);
    }

    #[test]
    fn plan_adds_incremental_when_flagged() {
        let mut args = args_default();
        args.incremental = true;
        assert_eq!(
            plan_steps(&args),
            vec![CleanStep::Sweep, CleanStep::Incremental]
        );
    }

    #[test]
    fn plan_all_subsumes_other_steps() {
        let mut args = args_default();
        args.all = true;
        args.incremental = true;
        assert_eq!(plan_steps(&args), vec![CleanStep::Full]);
    }

    #[test]
    fn sweep_args_carry_the_age_threshold() {
        assert_eq!(sweep_args(3, false), vec!["sweep", "--time", "3"]);
    }

    #[test]
    fn sweep_args_pass_through_dry_run() {
        assert_eq!(
            sweep_args(7, true),
            vec!["sweep", "--time", "7", "--dry-run"]
        );
    }

    #[test]
    fn format_size_uses_plain_bytes_below_a_kibibyte() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn format_size_scales_to_binary_units() {
        assert_eq!(format_size(1024), "1.0 KiB");
        assert_eq!(format_size(1024 * 1024), "1.0 MiB");
        assert_eq!(format_size(3 * 1024 * 1024 * 1024), "3.0 GiB");
    }

    #[test]
    fn format_size_saturates_at_the_largest_unit() {
        // 2 PiB has no unit above TiB to scale into, so it stays in TiB.
        assert_eq!(format_size(2 * 1024_u64.pow(5)), "2048.0 TiB");
    }

    #[test]
    fn reclaimed_line_reports_the_delta() {
        let line = reclaimed_line(2 * 1024 * 1024, 1024 * 1024);
        assert!(line.contains("reclaimed 1.0 MiB"));
        assert!(line.contains("2.0 MiB → 1.0 MiB"));
    }

    #[test]
    fn reclaimed_line_handles_growth_without_underflow() {
        let line = reclaimed_line(1024, 2048);
        assert!(line.contains("reclaimed 0 B"));
    }
}
