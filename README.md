# courses

Web platform for hands-on AWS workshops. An axum server serves each course's lab
guides, and a scenario console that provokes observable infrastructure behavior
(CPU bursts, error-log bursts, custom metrics).

Each course's content lives under `content/<slug>/`; the server is course-agnostic.
Course content, and every user-facing string, is Spanish; code, and developer docs,
are English (see `CONTEXT.md`).

## Layout

| Path | Crate | Role |
|------|-------|------|
| `crates/core` | `courses_core` | Functional core: domain model, and HTML rendering. Pure — no I/O, no async. |
| `crates/server` | `courses_server` | Imperative shell: axum binary. |
| `xtask` | `xtask` | Repository task runner. |
| `content/` | — | Course content, one subdirectory per slug. |

## Development

```sh
cargo xtask lint            # fmt, check, clippy, test, builtin checks — silent unless it fails
cargo xtask lint --fix      # apply fmt + clippy fixes first
cargo run -p courses_server # serve on http://localhost:8080 (PORT to override)
```

`cargo xtask` requires the globally installed `cargo-xtask` wrapper; without it, use
`cargo run -p xtask -- lint`.

Repository conventions live in `CLAUDE.md`; the domain glossary in `CONTEXT.md`.
