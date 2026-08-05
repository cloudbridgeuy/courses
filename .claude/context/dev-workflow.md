# Dev workflow notes

## Running the server locally

- Port 8080 is often taken by an unrelated Vite app on this machine. Test with:
  `PORT=8090 ./target/debug/courses_server`
- Kill the server after manual tests by port, not by name:
  `kill $(lsof -t -nP -iTCP:8090 -sTCP:LISTEN)`. `pkill -f courses_server` also
  takes down every other instance, including an echo on another port.
- With `CB_HEALTH_CHECKS` set, SIGTERM starts the drain window, so the process
  keeps the port for `CB_HEALTH_DRAIN_SECS` before exiting. Wait for the port to
  free (`until ! lsof -nP -iTCP:<port> -sTCP:LISTEN; do sleep 2; done`) instead of
  assuming the kill was instant.
- Session URLs use title slugs, not filenames: discover them with
  `curl -s localhost:8090/ | grep -o 'href="/courses/[^"]*"'`. A file such as
  `12-separar-stacks.md` is one *section* of a session — look up which
  `[[session]]` lists it in `content/aws-devops/course.toml`.

## Subcommands

`courses_server` takes an optional subcommand. No subcommand means `serve`, so
the `Dockerfile` `CMD` and existing ECS task definitions are unaffected.

- `courses_server serve` — the courses platform. The default.
- `courses_server echo [--port <n>] [--name <dns-name>]` — the workshop's second
  app: answers every request with a JSON description of it. `--port` also reads
  `PORT` (default 8080), `--name` also reads `CB_ECHO_NAME`. Try it with
  `./target/debug/courses_server echo --port 8099 --name echo.example.com` and a
  `curl -s localhost:8099/a/b?x=1`. The shaping lives in `courses_core::echo`;
  `crates/server/src/echo.rs` only copies out of axum's types.
  `?status=<200..599>` chooses the code the answer carries
  (`curl -i "localhost:8099/eco?status=503"`); the `response` block of the body
  reports the code, where it came from, and, for a value the service cannot
  use, why it fell back to `200`. `courses_core::echo::status` holds that
  decision, and `<cb-eco>` is the widget over it.
  Every served request also writes one `info` access line to stdout —
  `GET /eco?status=503 -> 503 client=… peer=… host=… bytes=… ms=… trace=…` —
  built by `courses_core::echo::log::access_log_line`, which replaces
  whitespace, and control characters, inside client-controlled values, so a
  header cannot forge a second record or paint a terminal. A body over
  `MAX_BODY_BYTES` never becomes an `EchoRequest`, so it logs a `warn` instead.
- `courses_server healthcheck [--path <p>] [--port <n>] [--timeout-ms <ms>]` —
  requests one path over loopback (`127.0.0.1`, never `localhost`) and exits `0`
  on a success status, `1` otherwise, printing the status or the transport error
  to stderr. `--path` defaults to `/health/live`, `--port` also reads `PORT`,
  `--timeout-ms` defaults to 2000. It is what the ECS container health check
  runs: the runtime image has no `curl`.
- The `network.ecs` block needs the ECS task metadata endpoint. To exercise it
  off ECS, serve a static `/task` document and point
  `ECS_CONTAINER_METADATA_URI_V4` at it — the server reads it once at startup
  and answers `null` if anything fails.

## Hot reload (dev mode)

`CB_DEV_ROOT=<repo root>` switches the server off its compile-time embedded
content and onto disk, with a file watcher that reloads on save.

- `cargo xtask dev` sets it automatically, from the workspace root.
- Escape hatch without Docker: `CB_DEV_ROOT=$PWD cargo run -p courses_server`.
  The apps widgets on `18-notificaciones-teams.md` and
  `19-observabilidad-metrics-logs.md` will not work without DynamoDB Local.
- Watched: every non-hidden `.md`, `.toml`, `.css`, and `.js` file under
  `content/` and `crates/server/static/` fires the watcher — there is no
  filename allowlist, so vendored bundles like `reveal.min.js` and the
  dev-reload client itself also qualify.
- Each repository file referenced by a `<cb-file>` element is watched directly,
  so editing it reloads the current dev page and refreshes the rendered source.
- Hot-served: content (`*.md`, `course.toml`) is always re-read from disk;
  static assets are re-read only for the six files in `text_asset`
  (`guide.css`, `slides.css`, `cb-widgets.css`, `apps.js`, `toggle.js`,
  `mermaid-init.js`). Everything else — binary assets, `reveal.min.js`,
  `reveal.min.css`, `mermaid.min.js`, `dev-reload.js` — still triggers a
  reload notification but keeps serving the embedded copy, so the browser
  reloads without picking up the edit.
- Broken content does not stop the server: every page answers 500 with the
  parse error, and returns to normal on the next successful save.
- A `CB_DEV_ROOT` the watcher cannot watch is fatal: startup fails naming the
  variable and the offending path, rather than running without hot reload.
- The browser reloads over `GET /dev/reload` (SSE), preserving guide scroll
  through the `cb-dev-scroll` sessionStorage key. Slides keep their position
  through reveal's `hash:true`.
- `CB_DEV_ROOT` is absent in production, where `/dev/reload` answers 404 and
  pages carry no reload client. It is deliberately kept out of `.env.example`,
  since it is a machine-specific absolute path.

## Verifying in the browser (Playwright MCP)

- Guides use `scroll-behavior: smooth`: poll `scrollY` until ~5 consecutive identical
  100ms samples before asserting positions; set `scrollBehavior = 'auto'` when you
  need deterministic initial positioning.

## Lint

- `cargo xtask lint` before declaring done; silent unless failing, full log at
  `target/xtask-lint.log`. `cargo xtask lint --fix` applies rustfmt fixes.

## Keeping `target/` small

`target/` grows past 5 GB within weeks. Debug info is *not* the cause: the
workspace already sets `[profile.dev] debug = 0` (and `[profile.release]
debug = 0`), which emits no debug info at all, for dependencies as well as
workspace crates. The widely-shared `debug = "line-tables-only"` /
`split-debuginfo = "unpacked"` tip would therefore *increase* the size here —
do not apply it. The real drivers are stale artifacts from old dependency
versions and the incremental-compilation cache.

Only `dev` is incremental. `[profile.release] incremental = false`, because a
container build always starts with an empty incremental cache and pays the cost
for nothing — so `target/release/incremental` no longer exists. Export
`CARGO_INCREMENTAL=1` for a faster local `cargo run --release`.

`cargo xtask clean` prunes them. It needs `cargo install cargo-sweep`.

| Command | Effect |
|---------|--------|
| `cargo xtask clean` | Drops artifacts untouched for 7+ days |
| `cargo xtask clean --days 3` | Same, with a tighter threshold |
| `cargo xtask clean --incremental` | Also wipes `target/*/incremental` |
| `cargo xtask clean --all` | `cargo clean` — nukes `target/` entirely |
| `cargo xtask clean --dry-run` | Reports what would go, deletes nothing |

Every run prints the target size before and after. `--incremental` costs a full
recompile of the workspace crates on the next build, so reach for it only when
the age-based sweep is not enough.

## Repo hygiene

- Plans and designs (`.claude/plans/`, `.claude/designs/`) stay OUT of git.
  Course content, `content/`, and fonts are committed. `.claude/context/` docs
  and `CLAUDE.md` are committed.
- Conventional-commit subjects (`feat(core): …`, `fix(slides): …`); no AI
  attribution trailers.

## Local dev with `xtask dev`

`cargo xtask dev` starts a full local dev stack in one command:

1. Loads `.env` from the repo root (copy `.env.example` to `.env` to configure;
   the file is gitignored).
2. Detects `docker` or `podman` (prefers docker).
3. Starts DynamoDB Local in a container with an ephemeral host port (`-p 0:8000`)
   to avoid port conflicts; removes any stale container of the same name first.
4. Waits for the TCP port to be reachable, then creates the apps table
   (`courses-apps` by default) via `courses_apps::ensure_table` — idempotent.
5. Spawns `cargo run -p courses_server` with the following env injected so the
   server resolves credentials and endpoint without IMDS:
   - `AWS_ENDPOINT_URL_DYNAMODB=http://127.0.0.1:<ephemeral-port>`
   - `AWS_REGION=us-east-1`
   - `AWS_ACCESS_KEY_ID=test`
   - `AWS_SECRET_ACCESS_KEY=test`
   - `CB_APPS_TABLE=courses-apps`
   - `CB_DEV_ROOT=<workspace root>` — turns on hot reload; see above.
   No server code changes are required.
6. On Ctrl-C: kills the server, then the container guard removes the container.

Useful flags: `--table`, `--region`, `--image`, `--container-name`.
