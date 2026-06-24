# Dev workflow notes

## Running the server locally

- Port 8080 is often taken by an unrelated Vite app on this machine. Test with:
  `PORT=8090 ./target/debug/courses_server`
- Kill the server after manual tests: `pkill -f courses_server`.
- Session URLs use title slugs, not filenames: discover them with
  `curl -s localhost:8090/ | grep -o 'href="/courses/[^"]*"'`.

## Verifying in the browser (Playwright MCP)

- Guides use `scroll-behavior: smooth`: poll `scrollY` until ~5 consecutive identical
  100ms samples before asserting positions; set `scrollBehavior = 'auto'` when you
  need deterministic initial positioning.

## Lint

- `cargo xtask lint` before declaring done; silent unless failing, full log at
  `target/xtask-lint.log`. `cargo xtask lint --fix` applies rustfmt fixes.

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
   No server code changes are required.
6. On Ctrl-C: kills the server, then the container guard removes the container.

Useful flags: `--table`, `--region`, `--image`, `--container-name`.
