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

## Workspace

```
xtask/           — task runner (cargo xtask lint)
crates/core/     — courses_core: pure domain (course model, HTML rendering)
crates/server/   — courses_server: axum binary, imperative shell
content/         — course content, one subdirectory per course slug
```
