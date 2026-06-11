# Courses Platform — Backbone Design

Date: 2026-06-11
Status: Validated

## Purpose

Set up the backbone of the `courses` repository: a Rust workspace, modeled on the
practices of `~/Projects/Rust/forgeguard`, that will host the course-platform web
application described in the AWS DevOps workshop proposal. The backbone proves the
architecture end to end with one thin vertical slice: a pure core crate renders a
placeholder guide page, and a thin axum shell serves it.

## Decisions

1. **Multiple courses, content layer only.** The server, and the future scenario
   console, stay course-agnostic. Each course is a directory of content under
   `content/`. No course registry, no multi-tenancy.
2. **Two crates plus xtask.** `crates/core` (package `courses_core`, pure), and
   `crates/server` (package `courses_server`, axum binary). The dependency points one
   way: server → core.
3. **Scope: infrastructure plus one thin vertical slice.** No DynamoDB, no AWS SDK, no
   scenarios, no JavaScript, no content pipeline.
4. **Toolchain 1.95.0, edition 2024.** Pinned in `rust-toolchain.toml`.
5. **No template engine.** The core renders HTML through pure functions that return
   `String`, with a small `escape_html` helper.
6. **Scenarios deferred.** The `Scenario` domain arrives with the scenario console,
   not with the backbone.

## Workspace Layout

```
courses/
├── Cargo.toml              # [workspace] members = ["xtask/", "crates/*"], resolver 2
├── rust-toolchain.toml     # channel 1.95.0, components clippy + rustfmt
├── clippy.toml             # too-many-arguments 5, max-fn-params-bools 1, cognitive-complexity 15
├── .cargo/config.toml      # [env] CARGO_WORKSPACE_DIR = { value = "", relative = true }
├── .gitignore              # /target/, **/xtask-lint.log, .claude/* (keep context/, commands/, designs/, plans/)
├── CLAUDE.md               # conventions: FCIS rule, error rule, xtask usage
├── README.md
├── xtask/
├── crates/
│   ├── core/               # package courses_core   (pure)
│   └── server/             # package courses_server (axum binary)
└── content/                # course content root; empty, .gitkeep + one-line README
```

The root manifest carries `[workspace.package]` (edition 2024, rust-version 1.95.0),
`[workspace.lints.clippy]` with forgeguard's deny/warn set (`enum_glob_use`, and
`wildcard_imports` denied; `manual_let_else`, `large_enum_variant`, `implicit_clone`,
`cloned_instead_of_copied`, `redundant_closure_for_method_calls`, and
`needless_pass_by_value` warned), all third-party dependencies in
`[workspace.dependencies]`, and `debug = 0` in both profiles. Every crate inherits
`authors`, `edition`, and `license` from the workspace, sets `[lints] workspace =
true`, and opens `lib.rs`, or `main.rs`, with `#![deny(clippy::unwrap_used,
clippy::expect_used)]` so production code cannot unwrap, while tests may.

## Functional Core — Imperative Shell Rule

Recorded in `CLAUDE.md`, verbatim in spirit from forgeguard: any crate with `tokio`,
`axum`, AWS SDKs, or any other I/O dependency is an I/O crate. I/O crates MUST NOT be
depended on by pure crates. If a type in an I/O crate is needed elsewhere, it MUST
move down to a pure crate. Unit tests apply only to pure functions; shell code stays
thin, and untested, until the project matures.

## xtask

A single `xtask/` crate (clap derive, `duct`, `color-eyre`). The globally installed
`cargo-xtask` wrapper discovers the workspace through the `.cargo/config.toml` env
entry, so `cargo xtask …` works with no aliases, and no wrapper duplication.

One subcommand: `cargo xtask lint`, a pipeline that stops at the first failure:

1. `cargo fmt --check`
2. `cargo check --workspace --all-targets`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace --all-targets`
5. File-length check: every `.rs` under `crates/*/src/`, and `xtask/src/`, at most
   1000 lines.
6. Forbid `#[allow(clippy::too_many_arguments)]`; `#[cfg(test)]` modules exempt.

Flags: `--fix` (fmt plus `clippy --fix`), `--verbose`, and `--no-<check>` skips.
Output is silent unless a check fails: `duct` captures with
`.stderr_to_stdout().stdout_capture().unchecked()`, prints captured output only on
failure, or under `--verbose`, and always appends everything to
`target/xtask-lint.log`. On success, the only output is one summary line per check.

The crate itself follows FCIS with section comments: the pure side holds `CheckId`,
`Check`, `CheckOutcome`, `determine_outcome`, `evaluate_file_lengths`,
`scan_for_forbidden_allows`, and argument-to-plan resolution, all unit-tested inline;
the shell side holds `run`, `execute_check`, file collection, and log writing.

## courses_core (pure)

Dependencies: `thiserror`, and `serde` only. Modules:

- `error.rs` — `Error` enum via `thiserror`, plus a `Result<T>` alias. Initial
  variants: `CourseNotFound`, and `InvalidSlug`. Names follow the forgeguard
  convention: always `Error`, and `Result<T>`, disambiguated by path at use sites.
- `course.rs` — initial domain ADTs. `CourseSlug` is a newtype whose only constructor
  is `CourseSlug::parse(&str) -> Result<CourseSlug>`, enforcing lowercase-kebab form
  (parse, don't validate). `Course { slug, title, sections: Vec<GuideSection> }`.
  `GuideSection { title, body_html }`.
- `render.rs` — pure HTML rendering: `render_guide_page(&Course) -> String`, and a
  pure `escape_html` helper.

Every pure function carries inline `#[cfg(test)] mod tests`, annotated with
`#[allow(clippy::unwrap_used, clippy::expect_used)]` at the module level: slug
accept/reject cases, escaped rendering output, and error `Display` coverage. `lib.rs`
re-exports the public surface explicitly, with no glob.

## courses_server (imperative shell)

Dependencies: `courses_core`, `axum`, `tokio` (rt-multi-thread, signal, macros),
`color-eyre`, `tracing`, and `tracing-subscriber`. Binary only. Modules:

- `main.rs` — `#[tokio::main]`, `color_eyre::install()`, tracing init, bind address
  from the `PORT` env var (default 8080), parsed at the boundary into a typed config,
  graceful shutdown on ctrl-c.
- `routes.rs` — `GET /health` returns `200 OK` plain text; `GET /` builds a
  hard-coded sample `Course`, calls `courses_core::render_guide_page`, and wraps the
  result in `Html<String>`. Handlers are glue, and contain no logic.

The server has no unit tests, per the testing rule.

## Acceptance

1. `cargo xtask lint` passes, silently.
2. `cargo run -p courses_server` serves `GET /` (placeholder guide page), and
   `GET /health`.
