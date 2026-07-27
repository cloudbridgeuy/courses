# Content authoring guide

Course content lives here. Each course occupies one subdirectory named by its slug
(lowercase-kebab, e.g. `aws-devops/`). Content is embedded at compile time, so an
ordinary `cargo run -p courses_server` needs a rebuild to pick up an edit; dev mode
reads it from disk instead, and reloads the open page on save (see "Picking up your
edits" below). Broken content fails the embedded-content test and aborts server
boot; in dev mode it turns every page into a 500 naming the file, and the next
successful save restores it.

## Directory layout

```
content/
  aws-devops/
    course.toml
    01-introduccion.md
    02-codecommit.md
    …
```

## course.toml

Every course directory must contain `course.toml` with a `title` key and one or
more `[[session]]` blocks:

```toml
title = "Taller AWS DevOps"

[[session]]
slug = "del-codigo-a-la-imagen"
title = "Del código a la imagen"
sections = [
  "01-introduccion.md",
  "02-codecommit.md",
]

[[session]]
slug = "la-aplicacion-en-linea"
title = "La aplicación en línea"
sections = [
  "03-despliegue.md",
]
```

`title` is the course display name shown in the index and on the landing page.

Each `[[session]]` block has three required fields:

| Field | Type | Description |
|-------|------|-------------|
| `slug` | string | Lowercase-kebab identifier, unique per course (e.g. `del-codigo-a-la-imagen`). Appears in the URL: `/courses/{course}/{session}`. |
| `title` | string | Display name shown in the tree and as the session page heading. |
| `sections` | array of strings | Section filenames in display order; every file must exist in the course directory. |

Sessions are displayed in the order they appear in the file. At least one session is required; each session must have at least one section.

## Section files

Each section file is a Markdown file with a TOML frontmatter block at the top.

### Frontmatter

The file must begin with `+++` alone on its own line, followed by TOML key-value
pairs, and a closing `+++` alone on its own line:

```
+++
title = "Introducción"
+++
```

`title` is required. Two optional arrays add extra assets to the page:

```
+++
title = "Integración continua"
scripts = ["/static/my-extra.js"]
styles  = ["/static/my-extra.css"]
+++
```

Both fences must sit alone on their lines with no leading or trailing whitespace.

### Markdown body

Write CommonMark Markdown. Tables and raw HTML pass through unchanged. Use `##` and
below for headings — the frontmatter `title` renders as the page-level `<h1>`.
Footnotes and strikethrough are also supported.

### Solution blocks

Wrap any answer panel in `::: solucion` … `:::` fences:

```
### Ejercicio 1 — Cree su repositorio

Describa la tarea aquí.

::: solucion
Pasos exactos de la solución.
:::
```

Rules:
- Each fence must sit alone on its line (no leading or trailing characters).
- Blocks may not nest.
- Every `::: solucion` must have a matching `:::`.

The block renders as a «Ver solución» / «Ocultar solución» toggle. The
`/static/toggle.js` script is injected automatically when any section in the
session contains at least one solution block (injection is per-session).

### Exercise convention

Number exercises continuously across the whole course using the heading pattern:

```
### Ejercicio N — <nombre>
```

where N increments from 1 across all sections.

## Assets

`/static/guide.css` is always included on every page. `/static/toggle.js` is
auto-injected when any section in a session uses a solution block (per-session). Both
ship inside the binary; no external files are served. Dev mode re-reads both from disk
on every request, so a styling edit shows up on the next refresh.

## Minimal example section

```markdown
+++
title = "El origen del código — CodeCommit"
+++

## Contexto

AWS CodeCommit es un servicio de control de versiones compatible con Git.

### Ejercicio 1 — Cree su repositorio

Cree un repositorio llamado `taller-aws-<su-nombre>`.

::: solucion
1. Abra la consola de AWS y busque **CodeCommit**.
2. Pulse **Create repository**.
3. En **Repository name**, escriba `taller-aws-<su-nombre>`.
4. Pulse **Create**.
:::
```

## Picking up your edits

Content is compiled into the binary. After editing any file here, rebuild with:

```sh
cargo run -p courses_server
```

or run the full lint gate:

```sh
cargo xtask lint
```

To skip the rebuild while authoring, run the server in dev mode instead. It reads
`content/` from disk and reloads the open page on save:

```sh
cargo xtask dev                                # + DynamoDB Local, for the apps widgets
CB_DEV_ROOT=$PWD cargo run -p courses_server   # without Docker
```

Details in `.claude/context/dev-workflow.md`.

A frontmatter or manifest error prints the file name and reason, then aborts.
