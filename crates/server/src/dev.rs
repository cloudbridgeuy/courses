//! Dev mode: reads course content from disk, and watches it for changes.
//!
//! Every decision about *what* to serve, or *when* to reload, comes from a
//! function in `courses_core`. What lives here is the file system, the watcher
//! thread, and the reload task.

use std::path::{Path, PathBuf};
use std::time::Duration;

use color_eyre::eyre::{OptionExt, Result, WrapErr};
use courses_core::{CourseInput, DEV_RELOAD_SCRIPT_TAG, LoadedCourse, RenderedSite};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::broadcast;

use crate::site::{SiteHandle, SiteState};

/// Parses every course under `<root>/content`, renders the site, and injects
/// the reload client.
///
/// Failure is a value, not an error: a broken Markdown file must leave the
/// server running so the author can fix it, with the message on screen.
pub fn load_from_disk(root: &Path) -> SiteState {
    match build_site(root) {
        Ok(site) => {
            let (site, skipped) = courses_core::with_dev_script(site, DEV_RELOAD_SCRIPT_TAG);
            for key in skipped {
                tracing::warn!("page {key:?} has no </head>: hot reload is inactive on it");
            }
            SiteState::Ready(site)
        }
        Err(e) => SiteState::Broken(format!("{e:#}")),
    }
}

/// Reads and renders every course directory under `<root>/content`.
fn build_site(root: &Path) -> Result<RenderedSite> {
    let content = root.join("content");
    let entries = std::fs::read_dir(&content)
        .wrap_err_with(|| format!("could not read {}", content.display()))?;

    let mut courses: Vec<LoadedCourse> = Vec::new();
    for entry in entries {
        let entry =
            entry.wrap_err_with(|| format!("could not read an entry of {}", content.display()))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let slug = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_eyre("content subdirectory with a non-UTF-8 name")?
            .to_owned();
        courses.push(load_course(root, &slug, &path).wrap_err_with(|| format!("course {slug:?}"))?);
    }

    courses.sort_by(|a, b| a.course.slug.as_str().cmp(b.course.slug.as_str()));
    Ok(courses_core::render_site(&courses))
}

/// Reads one course directory into the same `CourseInput` the embedded loader
/// builds in `crate::content`, then hands it to the pure parser.
fn load_course(root: &Path, slug: &str, dir: &Path) -> Result<LoadedCourse> {
    let manifest_path = dir.join("course.toml");
    let manifest = std::fs::read_to_string(&manifest_path)
        .wrap_err_with(|| format!("could not read {}", manifest_path.display()))?;

    // Shallow on purpose, matching the embedded loader: a .md file nested in a
    // subdirectory is not collected, and a manifest referencing it fails loudly
    // as MissingSection.
    let mut files: Vec<(String, String)> = Vec::new();
    for entry in
        std::fs::read_dir(dir).wrap_err_with(|| format!("could not read {}", dir.display()))?
    {
        let entry =
            entry.wrap_err_with(|| format!("could not read an entry of {}", dir.display()))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.ends_with(".md") {
            continue;
        }
        let contents = std::fs::read_to_string(&path)
            .wrap_err_with(|| format!("could not read {}", path.display()))?;
        let contents = crate::file_apps::expand_file_apps(&contents, |source| {
            std::fs::read_to_string(root.join(source)).ok()
        })
        .wrap_err_with(|| format!("could not expand cb-file references in {}", path.display()))?;
        files.push((name.to_owned(), contents));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let input = CourseInput {
        slug,
        manifest: &manifest,
        files: &files,
    };
    courses_core::parse_course(&input).wrap_err("invalid course content")
}

/// Quiet window after the last file event before a reload runs. A single save
/// in most editors produces a burst of events; this collapses the burst.
const DEBOUNCE: Duration = Duration::from_millis(200);

/// Directories watched for changes, relative to the repository root.
const WATCHED: [&str; 2] = ["content", "crates/server/static"];

/// Starts the file watcher, and the task that reloads on its events.
///
/// The returned watcher must be kept alive for as long as reloading is wanted:
/// dropping it stops the watch.
pub fn spawn_watcher(
    root: PathBuf,
    site: SiteHandle,
    reload: broadcast::Sender<()>,
    file_app_paths: &[&str],
) -> Result<RecommendedWatcher> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
    let source_paths: Vec<PathBuf> = file_app_paths.iter().map(|path| root.join(path)).collect();
    let source_paths_for_events = source_paths.clone();

    // notify calls this from its own thread. It does no work beyond filtering,
    // so a burst of events cannot stall the watcher.
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
        match result {
            Ok(event) => {
                let relevant = event.paths.iter().any(|path| {
                    path.to_str().is_some_and(courses_core::is_reload_trigger)
                        || source_paths_for_events.iter().any(|source| source == path)
                });
                if relevant {
                    // A send error only means the reload task is gone; ignore.
                    let _ = tx.send(());
                }
            }
            Err(e) => tracing::warn!("file watcher error: {e}"),
        }
    })
    .wrap_err("could not create the file watcher")?;

    for relative in WATCHED {
        let path = root.join(relative);
        watcher
            .watch(&path, RecursiveMode::Recursive)
            .wrap_err_with(|| format!("could not watch {}", path.display()))?;
        tracing::info!("watching {}", path.display());
    }
    for source in &source_paths {
        watcher
            .watch(source, RecursiveMode::NonRecursive)
            .wrap_err_with(|| format!("could not watch cb-file source {}", source.display()))?;
        tracing::info!(path = %source.display(), "watching cb-file source");
    }

    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            // Extend the quiet window for as long as events keep arriving.
            loop {
                match tokio::time::timeout(DEBOUNCE, rx.recv()).await {
                    Ok(Some(())) => continue,
                    Ok(None) => return,
                    Err(_elapsed) => break,
                }
            }

            // Reading and rendering the whole site is a few milliseconds over
            // roughly 150 KB of Markdown, so it runs inline rather than through
            // spawn_blocking.
            let state = load_from_disk(&root);
            match &state {
                SiteState::Ready(_) => tracing::info!("content reloaded"),
                SiteState::Broken(message) => {
                    tracing::error!("content reload failed: {message}");
                }
            }
            site.store(state);
            // A send error only means no browser is connected; ignore.
            let _ = reload.send(());
        }
    });

    Ok(watcher)
}
