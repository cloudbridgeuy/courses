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
| `:::inline-slide` … `:::` | Like `:::slide`, but the content ALSO renders inline in the guide as document content (dark slide). Accepts `light` and `with-title` (any order); unknown modifiers make it plain Markdown. `with-title` prepends the nearest heading above the block (fallback: the section title) to the SLIDE copy only — the guide copy is untouched. Keep content self-contained — `{{name}}` references are NOT expanded in the guide copy |
| `:::title-slide` … `:::` | Title-only slide. Shows the section heading (frontmatter `title`) by default, or a custom label: `:::title-slide Semana 1`. Body MUST be empty; non-empty body is an error |
| `:::skip` … `:::` | Guide-only content: renders transparently in the guide (no wrapper markup), dropped from every slide — except `:::add` blocks nested inside it with visibility `both`/`slide`, which still reach the slide. Main use: inside `:::inline-slide`, to keep detail out of the slide copy. Takes NO arguments — `:::skip anything` is plain Markdown |
| `:::add` … `:::` / `:::add visibility=slide` | Visibility filter, transparent (no wrapper markup). `visibility=both` (default): guide AND slide, overriding any enclosing `:::skip`. `visibility=guide`: guide only (like `:::skip`). `visibility=slide`: slide only, also overriding `:::skip`. Any other argument makes it plain Markdown. Top-level content only reaches slides through slide directives, so a top-level `visibility=slide` block renders nowhere |
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

#### Service icons

Node labels take inline HTML, so a diagram names a service with its icon:
`node["<img src='/static/aws-sns.svg' width='20' height='20' /> <b>Tema</b><br/>Amazon
SNS"]`. The house style is icon + bold role + service name, one `classDef` per service
family whose stroke is that family's icon colour on a white fill (`#c925d1` for the
Developer Tools icons, `#e7157b` for CloudFormation and SNS, `#ed7100` for ECR), and an
emoji for anything that is not an AWS service. Compare
`17-codepipeline-en-la-practica.md` and `18-notificaciones-teams.md`.

Available: `aws-codecommit`, `aws-codebuild`, `aws-codepipeline`,
`aws-codestar-notifications`, `aws-cloudformation`, `aws-ecr`, `aws-sns`,
`aws-chatbot`, `docker`. A new icon is **not** just a file: `crates/server/src/routes.rs`
embeds each asset with its own `include_str!` constant and match arm, so an
unregistered file answers 404. Icons are binary assets, which `CB_DEV_ROOT` does not
hot-serve — a running `cargo xtask dev` keeps serving the embedded copy until it is
rebuilt and restarted.

### Nesting

`::: solucion`, `::: warning`, `::: info`, `::: extra`, `:::skip`, and `:::add` nest freely — inside
each other and inside `:::slide`/`:::inline-slide` (slides carry matching reveal.js
CSS). A bare `:::` closes the innermost open block, so balance fences carefully. A
closer may carry a `#` comment (ignored; runs to end of line) to label deep nesting:

```
:::slide
## Título
::: warning
Cuidado.
::: # </warning>    <- closes the warning
::: # </slide>      <- closes the slide
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
("Abra **CloudWatch → Logs → Log Analytics**"), link the screen name to that window:

```
Abra [**CloudWatch → Logs → Log Analytics**](https://console.aws.amazon.com/cloudwatch/home#logsV2:).
```

- Prefer deep-link fragments to the exact window: `#logsV2:` (Logs, which opens
  Log Analytics), `#logsV2:log-groups` (Log Management), `#metricsV2:` (Metrics,
  which opens Classic metrics), `#alarmsV2:`, `#dashboards:`,
  `#container-insights:` (CloudWatch), `#TargetGroups:` (EC2). Use region-less
  service landings (`ecs/home`, `ec2/home`, `ecr/home`,
  `codesuite/codebuild/home`) when no stable fragment fits.
- **Use the console's current menu names.** CloudWatch reorganised both menus:
  Logs holds **Log Management**, **Log Analytics** (Logs Insights, Live Tail, and
  Contributor Insights in one screen), and **Log Anomalies**; Metrics holds
  **Query Studio**, **Classic metrics**, **Explorer**, and **Streams**. The old
  per-tool fragments (`#logsV2:logs-insights`, `#logsV2:live-tail`) and the old
  names ("Log groups", "All metrics") are retired — a tool that now lives inside
  a screen is named as such: "**Log Analytics**, en **Logs Insights**".
- Link only true "open this window" instructions. Leave **sub-navigation inside an
  already-open screen** as bold text — e.g. `Entre a **ECS → por servicio**` is the
  metric picker inside the CloudWatch Metrics window, and form fields like
  `Environment → Environment image` are not separate windows.
