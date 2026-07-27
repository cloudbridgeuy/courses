# courses

Course-platform web application: an axum server that serves course guides, and a
scenario console, for hands-on AWS workshops. Multiple courses live as content under
`content/`; the server stays course-agnostic.

## Commands

| Action | Command |
|--------|---------|
| Lint everything (fmt, check, clippy, test, builtin checks) | `cargo xtask lint` |
| Apply fixes | `cargo xtask lint --fix` |
| Run the server | `cargo run -p courses_server` |
| Run the local dev stack (requires Docker or Podman) | `cargo xtask dev` |
| Prune stale build artifacts from `target/` | `cargo xtask clean` |

`cargo xtask dev` reads `.env` (copy from `.env.example`) and needs Docker or Podman.

`cargo xtask` requires the globally installed `cargo-xtask` wrapper (from the
forgeguard repo). Fallback: `cargo run -p xtask -- lint`.

Run `cargo xtask lint` before considering any work done. It is silent unless a check
fails; full output is always written to `target/xtask-lint.log`.

## Unnegotiables

1. **Functional Core - Imperative Shell.** Any crate with `tokio`, `axum`, AWS SDKs,
   or any other I/O dependency is an I/O crate. I/O crates MUST NOT be depended on by
   pure crates. If a type in an I/O crate is needed elsewhere, it MUST move down to a
   pure crate. Within a file, separate pure logic from I/O with section comments.
2. **Testing.** Unit tests apply only to pure functions, inline in
   `#[cfg(test)] mod tests` blocks annotated with
   `#[allow(clippy::unwrap_used, clippy::expect_used)]`. Every pure function gets
   tests. Shell code gets none — keep it thin enough that nothing in it is worth
   testing. No integration, or E2E, tests until the project matures.
3. **Errors.** Every library crate defines a local `thiserror` `Error` enum, plus a
   `Result<T>` alias — always named `Error`, and `Result`, disambiguated by path at
   use sites. Binaries use `color-eyre`.
4. **Lints.** Every crate sets `[lints] workspace = true`. Every `lib.rs`, and
   `main.rs`, opens with `#![deny(clippy::unwrap_used, clippy::expect_used)]`.
   `#[allow(clippy::too_many_arguments)]` is forbidden outside test modules — use a
   Params struct.
5. **Files.** No `.rs` file under `crates/*/src/`, or `xtask/src/`, may exceed 1000
   lines.
6. **Language.** Code, identifiers, comments, developer docs, URLs, and platform
   chrome are English. Course content, and in-guide labels, are Spanish.

## Context docs

`CONTEXT.md` (repo root) is the living record of project state, decisions, and
important parameters (active course, week plan, AWS service status, lab parameters,
notifications design). Read it first; update it whenever a durable fact changes.

Topic-specific context lives under `.claude/context/` — read the relevant file
before working on that area:

- `.claude/context/content-authoring.md` — Markdown directives (`:::slide`,
  `::: solucion`, `{#name}`/`{{name}}`), frontmatter, heading slug rules.
- `.claude/context/guide-and-slides-ui.md` — guide/slides CSS conventions, floating
  buttons, sessionStorage keys, reveal.js theme rules.
- `.claude/context/dev-workflow.md` — local server ports, browser verification,
  lint, and repo hygiene (plans/designs stay out of git; conventional commits).
- `.claude/context/notifications.md` — live SSE toast subsystem: SNS webhook flow,
  `POST /hooks/notifications`, the `CB_HOOK_TOKEN` shared secret, pod attribution,
  and where the pure/shell/client code lives.
- `.claude/context/apps-events.md` — generic app event contract: `POST /events`,
  `GET /events/stream` (unified SSE bus), `GET /state`, gating env vars, FC/IS
  crate layout (`courses_core::events`, `courses_apps`), custom elements, `:::app`
  directive, and the `CB_APPS_SECRET` Secrets Manager teaching hook.

## Workspace

```
xtask/           — task runner (cargo xtask lint)
crates/core/     — courses_core: pure domain (course model, HTML rendering)
crates/apps/     — courses_apps: I/O crate (event handlers, DynamoDB, load gen)
crates/server/   — courses_server: axum binary, imperative shell
content/         — course content, one subdirectory per course slug
```
