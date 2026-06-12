# courses

Web platform for hands-on AWS workshops. An axum server serves each course's lab
guides, and a scenario console that provokes observable infrastructure behavior
(CPU bursts, error-log bursts, custom metrics).

Each course's content lives under `content/<slug>/`; the server is course-agnostic.
Course content, and in-guide labels, are Spanish; code, developer docs, URLs, and
platform chrome are English (see `CONTEXT.md`).

## Layout

| Path | Crate | Role |
|------|-------|------|
| `crates/core` | `courses_core` | Functional core: domain model, and HTML rendering. Pure — no I/O, no async. |
| `crates/server` | `courses_server` | Imperative shell: axum binary. |
| `xtask` | `xtask` | Repository task runner. |
| `content/` | — | Course content, one subdirectory per slug. |

## Routes

| Method | Path | Response |
|--------|------|----------|
| `GET` | `/` | Index page: full course→session→section tree (`lang="en"`). |
| `GET` | `/courses/{slug}` | Course landing page: title + scoped tree (`lang="es"`); unknown slug → 404. |
| `GET` | `/courses/{slug}/{session}` | Session page: section nav, sections, prev/next footer nav (`lang="es"`); unknown slug or session → 404. |
| `GET` | `/static/{file}` | Embedded platform assets (`toggle.js`, `guide.css`, `montserrat.ttf`, `cloudbridge.png`), each with its Content-Type; unknown file returns 404. |
| `GET` | `/health` | Plain-text `ok` — used by load-balancer health checks. |

## Embedded content

Content under `content/` compiles into the binary via `include_dir`. The server
parses and pre-renders every course once at startup; a content error aborts the boot,
naming the file and reason. Editing content requires a rebuild. See
`content/README.md` for the authoring format.

## Brand assets

The pages follow the CloudBridge brand. `crates/server/static/` holds the embedded
Montserrat variable font (`montserrat.ttf`), the brand logo (`cloudbridge.png`), and
the stylesheet (`guide.css`). `OFL.txt` ships the font license beside the font, as the
SIL Open Font License requires; it stays in the repo and is not served.

## Development

```sh
cargo xtask lint            # fmt, check, clippy, test, builtin checks — silent unless it fails
cargo xtask lint --fix      # apply fmt + clippy fixes first
cargo run -p courses_server # serve on http://localhost:8080 (PORT to override)
```

`cargo xtask` requires the globally installed `cargo-xtask` wrapper; without it, use
`cargo run -p xtask -- lint`.

Repository conventions live in `CLAUDE.md`; the domain glossary in `CONTEXT.md`.
