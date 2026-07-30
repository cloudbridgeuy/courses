# Content authoring

Course content lives under `content/<course-slug>/`, one Markdown file per session,
ordered by filename prefix (`01-`, `02-`, …). Session URLs use slugs from the
session title (e.g. `del-codigo-a-la-imagen`), NOT filenames. All content is Spanish.

## Register (impersonal manual)

All content uses an impersonal, timeless manual register. No personal subjects, no
`usted`.

- **Instructions** (guided-practice steps, exercise tasks, solution steps) use the
  **infinitive** as the main verb: "Abrir CloudFormation", "Pulsar **Create stack**",
  "Seleccionar **Upload a template file**", "Guardar el archivo". Not "Abra", "Pulse",
  "Seleccione", "Guarde".
- **Explanatory prose** uses the impersonal present or the passive with `se`: "se
  describe en un archivo YAML", "el stack se lanza con dos parámetros y se obtiene un
  ambiente", "en la sección anterior se construyó la imagen". Not "usted describe".
- **Possessives that point at the student** drop to the article: "el stack", "el
  servicio", "el clúster", "la imagen", "el ambiente" — not "su stack", "su imagen".
- **Exercise titles** take the infinitive: "Ejercicio 5 — Desplegar la aplicación",
  not "Despliegue la aplicación".

Leave untouched: placeholder tokens (`<su-nombre>`, `taller-<su-nombre>`,
`cpu-alta-<su-nombre>`), English AWS console labels in bold (**Create stack**,
**Next**, **CREATE_COMPLETE**), code blocks, resource and file names, and all
directive fences and anchors.

## File format

- TOML frontmatter between `+++` fences: `title = "…"`.
- CommonMark plus tables, footnotes, strikethrough (pulldown-cmark 0.13).
- Raw HTML passes through verbatim — content is trusted.

## Directives

| Syntax | Effect |
|--------|--------|
| `::: solucion` … `:::` | Collapsible solution card (button toggle) in guide and slides |
| `::: warning` … `:::` | Always-visible amber admonition in guides and slides. Takes NO arguments — `::: warning anything` is plain Markdown |
| `::: info` … `:::` | Always-visible blue information admonition in guides and slides. Takes NO arguments — `::: info anything` is plain Markdown |
| `::: extra <title>` … `:::` | Collapsible deep-dive `<details>` block, closed by default (guide only). Title optional; empty falls back to "Contenido adicional" |
| `:::slide` … `:::` | reveal.js slide (dark, default theme); slides only |
| `:::slide light` … `:::` | Light slide variant (`<section class="cb-light">`) |
| `:::inline-slide` … `:::` | Like `:::slide`, but the content ALSO renders inline in the guide as document content (dark slide). Accepts `light`. Keep content self-contained — `{{name}}` references are NOT expanded in the guide copy |
| `:::title-slide` … `:::` | Title-only slide. Shows the section heading (frontmatter `title`) by default, or a custom label: `:::title-slide Semana 1`. Body MUST be empty; non-empty body is an error |
| `{#name}` on its own line | Anchors the following subsection (heading through next same-level heading) |
| `{{name}}` inside a `:::slide` | Embeds the anchored subsection as an exercise hero card |
| ```` ```mermaid ```` fenced block | Renders a mermaid.js diagram (`<pre class="mermaid">`). Works in guide and slides; mermaid.js loads only on pages that use it |

### Mermaid diagrams

A fenced code block tagged `mermaid` renders as a diagram instead of a code block.
The page loads `mermaid.min.js` + `mermaid-init.js` only when at least one diagram is
present (mirrors the `toggle.js` solution-script rule); `mermaid-init.js` handles both
guide and slide decks. Use the `neutral` theme — for slides, author diagrams on a
**light** slide (`:::slide light`) for contrast. Keep node labels short; use `<br/>`
for line breaks inside a label.

Each rendered diagram gets a maximize button (hover, top-right) that opens it in a
full-viewport overlay (click backdrop or Escape to close), shared by guide and slides.
On slides, a diagram renders when its slide first becomes active (mermaid must measure
a visible slide), so diagrams off the first slide appear on navigation, not before.

### Nesting

`::: solucion`, `::: warning`, `::: info`, and `::: extra` nest freely — inside each other and
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

## Console-action links

When an instruction tells the student to **open a specific AWS console screen**
("Abra **CloudWatch → Logs → Live Tail**"), link the screen name to that window:

```
Abra [**CloudWatch → Logs → Live Tail**](https://console.aws.amazon.com/cloudwatch/home#logsV2:live-tail).
```

- Prefer deep-link fragments to the exact window: `#logsV2:logs-insights`,
  `#logsV2:live-tail`, `#alarmsV2:`, `#container-insights:` (CloudWatch),
  `#TargetGroups:` (EC2). Use region-less service landings (`ecs/home`,
  `ec2/home`, `ecr/home`, `codesuite/codebuild/home`) when no stable fragment fits.
- Link only true "open this window" instructions. Leave **sub-navigation inside an
  already-open screen** as bold text — e.g. `Entre a **ECS → por servicio**` is the
  metric picker inside the CloudWatch Metrics window, and form fields like
  `Environment → Environment image` are not separate windows.
