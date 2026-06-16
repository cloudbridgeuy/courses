# Content authoring

Course content lives under `content/<course-slug>/`, one Markdown file per session,
ordered by filename prefix (`01-`, `02-`, …). Session URLs use slugs from the
session title (e.g. `del-codigo-a-la-imagen`), NOT filenames. All content is Spanish.

## File format

- TOML frontmatter between `+++` fences: `title = "…"`.
- CommonMark plus tables, footnotes, strikethrough (pulldown-cmark 0.13).
- Raw HTML passes through verbatim — content is trusted.

## Directives

| Syntax | Effect |
|--------|--------|
| `::: solucion` … `:::` | Collapsible solution card (button toggle) in guide and slides |
| `::: warning` … `:::` | Always-visible amber admonition (guide only). Takes NO arguments — `::: warning anything` is plain Markdown |
| `::: extra <title>` … `:::` | Collapsible deep-dive `<details>` block, closed by default (guide only). Title optional; empty falls back to "Contenido adicional" |
| `:::slide` … `:::` | reveal.js slide (dark, default theme); slides only |
| `:::slide light` … `:::` | Light slide variant (`<section class="cb-light">`) |
| `:::inline-slide` … `:::` | Like `:::slide`, but the content ALSO renders inline in the guide as document content (dark slide). Accepts `light`. Keep content self-contained — `{{name}}` references are NOT expanded in the guide copy |
| `:::title-slide` … `:::` | Title-only slide. Shows the section heading (frontmatter `title`) by default, or a custom label: `:::title-slide Semana 1`. Body MUST be empty; non-empty body is an error |
| `{#name}` on its own line | Anchors the following subsection (heading through next same-level heading) |
| `{{name}}` inside a `:::slide` | Embeds the anchored subsection as an exercise hero card |

### Nesting

`::: solucion`, `::: warning`, and `::: extra` nest freely — inside each other and
inside `:::slide`/`:::inline-slide` (slides carry matching reveal.js CSS). A bare
`:::` closes the innermost open block, so balance fences carefully:

```
:::slide
## Título
::: warning
Cuidado.
:::      <- closes the warning
:::      <- closes the slide
```

Slide-family directives (`:::slide`, `:::inline-slide`, `:::title-slide`) are
top-level only; nesting one inside any block is an error.

`{#name}`/`{{name}}` is custom syntax handled by the segment parser — do NOT enable
pulldown-cmark's heading-attributes option; it would collide with `{#name}`.

## Heading anchors (automatic)

Every heading gets a slug `id` and a self-link (`a.cb-hlink`), generated in
`crates/core/src/markdown.rs`:

- Lowercase; Spanish diacritics stripped (á→a … ñ→n); non-alphanumeric runs → one hyphen.
- Duplicates within a document get `-2`, `-3`, … suffixes.
- Empty result falls back to `titulo`.

Example: `## Práctica guiada: crear el repositorio` → `#practica-guiada-crear-el-repositorio`.
