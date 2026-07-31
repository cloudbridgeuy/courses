# Courses Platform

Web platform for hands-on AWS workshops: an axum server serves each course's lab
guides, and a scenario console, while staying course-agnostic. Course content lives
under `content/`, one subdirectory per course.

## Language

The domain speaks Spanish only in course content: guide text, and in-guide labels
(e.g. the «Ver solución» toggle). Everything else — code, identifiers, comments,
developer docs, URLs, and platform chrome (index page, error pages) — is English.
The glossary maps both.

**Course** (es: *taller*):
One deliverable workshop — a slug, a title, and its guide content.
_Avoid_: workshop (in code), training, class

**CourseSlug**:
The lowercase-kebab identifier of a Course (e.g. `aws-devops`); the only constructor is `parse`.
_Avoid_: id, name, key

**GuideSection** (es: *sección de la guía*):
One titled block of a Course's guide, carrying trusted, authored HTML.
_Avoid_: chapter, page, module

**Content root** (`content/`):
The directory holding each Course's authored content, keyed by CourseSlug.
_Avoid_: assets, static

**Scenario** (es: *escenario*) — deferred:
A user-triggered action that provokes observable infrastructure behavior (CPU burst, error-log burst, custom metric). Not in the backbone.
_Avoid_: demo, simulation, button

## Relationships

- A **Course** is identified by exactly one **CourseSlug**
- A **Course** has one or more **GuideSections**
- The **Content root** holds one subdirectory per **Course**, named by its **CourseSlug**
- A **Course** will own its **Scenarios** (future)

## Example dialogue

> **Dev:** "Is the *taller* a different thing from a **Course**?"
> **Domain expert:** "No — *taller* is the Spanish, user-facing name; in code it is always **Course**. One Course is one deliverable workshop."
> **Dev:** "And a **GuideSection** — is that a week of the workshop?"
> **Domain expert:** "Not necessarily. It is one titled block of the guide; how sections map to weeks, sessions, or labs is a content decision, not a type decision — for now."

## Flagged ambiguities

- Page language: guide pages use `lang="es"` in `render_guide_page` — resolved: acceptable while every Course is Spanish; it becomes a **Course** field the day a non-Spanish Course exists. The index page, and the 404 page, are platform chrome and use `lang="en"`.

## Project state & parameters

Living record of decisions and important parameters. Update whenever a durable fact
changes — shared memory across sessions.

### Active course: `aws-devops`

A 4-week, mostly hands-on AWS DevOps workshop. Total 18 h: three 5 h weeks + one 3 h
week. Single narrative end to end: **CodeCommit → CodeBuild → ECR → ECS/Fargate →
CodePipeline → CloudWatch**.

| Sem | Hrs | Título | Contenido |
|-----|-----|--------|-----------|
| 1 | 5 | Del código a la imagen desplegada | Intro DevOps · CodeCommit (repos, branching, git) · CodeBuild+ECR (build Docker, versionado) · despliegue inicial con CloudFormation como caja negra |
| 2 | 5 | Infraestructura como código y los primeros contenedores | CloudFormation (templates YAML, deploy/update, buenas prácticas, troubleshooting) · cierra con ECS/Fargate: task definitions y services |
| 3 | 5 | Operar, automatizar y observar | ECS/Fargate restante (networking, escalabilidad, troubleshooting) · CodePipeline (rol, leer pipelines, stages, integración, pipeline básico, aprobación manual + trigger) · inicio de Observabilidad (CloudWatch metrics y logs) |
| 4 | 3 | Observabilidad y cierre del curso | Termina Observabilidad (dashboards, alarmas, Container Insights, trazabilidad) · cierre: repaso del flujo CI/CD, troubleshooting operacional, próximos pasos |

**Content status.** Written (Week 1): `01`–`06`. Written (Week 2):
`07-cloudformation-anatomia`, `08-leer-el-template`, `09-actualizar-stacks`,
`10-preguntas-puente`, `11-buenas-practicas-troubleshooting`,
`12-primeros-contenedores` (ej. 9–11). Written (Week 3): `13-operar-contenedores`,
`14-cicd-y-el-pipeline`, `15-preguntas-puente`, `16-codepipeline-en-la-practica`,
`17-notificaciones-teams`, `18-observabilidad-metrics-logs` (ej. 12–15). Written
(Week 4): `19-dashboards-y-alarmas`, `20-container-insights-trazabilidad`,
`21-cierre-del-curso` (ej. 16–17 + optional capstone ej. 18). **All 4 weeks
authored.**

**Note:** new content files are not served until added to a `[[session]]` in
`content/<course>/course.toml`, in either mode. On the embedded path, `include_dir!`
also does not re-embed on new files alone — `touch crates/server/src/content.rs`
before `cargo build -p courses_server`. Dev mode reads the directory on every
reload, so it needs neither step.

### Conventions

- **CloudFormation term: `template`** (Spanish masc., "el template"), not "plantilla".
- **Register: impersonal manual** (2026-07). Course prose avoids first person and
  direct address ("se crea el repositorio", not "vas a crear" / "creamos"). All 21
  files of `content/aws-devops/` were rewritten to this register; keep new content
  consistent with it.
- **Console-action links** point at the window they act on — see
  `.claude/context/content-authoring.md`.
- **Bridge questions** (`preguntas-puente`) split each 5 h week into two sessions
  (presencial + remota); they sit mid-week, not as a trailing module. The 3 h week
  (Week 4) is single-session, no bridge.
- **Week-closing sections** (last file of Weeks 1–3) end with "Dónde estamos" (recap
  of what works now) + "Qué sigue en la Semana N+1" (what's missing / comes next).
  Closers: `06-preguntas-puente`, `12-primeros-contenedores`,
  `18-observabilidad-metrics-logs`. Week 4 closes the course, no preview.
- Full per-file section plan for all 4 weeks:
  `.claude/designs/2026-06-16-aws-devops-syllabus-breakdown.md`.

### Week 1 breakdown (target 5 h)

| File | Topic | Est. |
|------|-------|------|
| `01-introduccion` | DevOps, pipeline narrative, services table, how-to-use | 15–20 min |
| `02-codecommit` | Pre-reqs (HTTPS/SSH/Identity Center) · versioning · clone/remote/push · branching · ej. 1–2 | 60–75 min |
| `03-codebuild-ecr` | Build problem · CodeBuild · ECR · `buildspec.yml` · `Dockerfile` deep-dive (re-tagging, monorepo, hadolint, cache, pull-through cache) · create ECR + CodeBuild project + IAM · run build · ECR beyond push (lifecycle, replication, cross-account/public access, scanning) · ej. 3–6 | 100–120 min |
| `04-despliegue` | CloudFormation as black box · launch stack · ej. 7 | 30–45 min |
| `05-teardown` | Tear down + recreate ("el seguro del taller") · ej. 8 | 30–40 min |
| `06-preguntas-puente` | 3 bridge questions to Week 2 | 15–20 min |

Active content ≈ 3.5 h; reaches ~5 h counting passive waits (first build 10–20 min —
the Rust release compile runs inside `docker build` — stack 3–8 min, teardown
3–6 min) and discussion. Margin to raise density without padding:
connective theory before building, guided discussion during waits, and a **Teams
notifications teaser** anchored on the CodeBuild build (full treatment in Week 3).

**CodeStar / Teams placement.** Week 1 contains only the CodeCommit deprecation
`::: warning` (top of `02-codecommit`). The CodeStar clarification (projects service
discontinued; CodeStar Notifications alive) and the Teams flow (CodeStar
Notifications → SNS → AWS Chatbot → Teams) belong to the **CodePipeline module
(Week 3)**, where the lab toast mechanism makes the production path concrete. A Teams teaser is
planted in Week 1 (`03-codebuild-ecr`, "Un adelanto" section + `::: extra`): it
shows the full flow and the CodeStar naming clarification, deferring the build to
Week 3.

**Lab parameters.**
- Lab code source repo: `https://github.com/cloudbridgeuy/courses`.
- The repo root ships the lab build files (added 2026-07-30, reworked 2026-07-31):
  a 4-phase `buildspec.yml` (install verifies tools incl. buildx · pre_build
  hadolint + ECR login + `docker buildx create --use` · build `docker buildx build`
  with ECR registry cache (`:cache` tag, `mode=max`, `--provenance=false`) and
  `--push` · post_build `aws ecr describe-images` verification) and a multi-stage
  `Dockerfile` that compiles `courses_server` (content embeds at compile time),
  pinned by base-image digest and apt package versions (hadolint-clean). Exercise 5
  (session 03) enables CodeBuild local cache. Exercise 6 (session 03) sets up an
  ECR Public pull-through cache (prefix `ecr-public`, inline policy
  `ecr:BatchImportUpstreamImage` + `ecr:CreateRepository` on the CodeBuild role)
  and repoints both Dockerfile `FROM` lines at
  `<account>.dkr.ecr.<region>.amazonaws.com/ecr-public/docker/library/…` — same
  digests as Docker Hub (verified identical 2026-07-31), so the `@sha256:` pins
  don't change. Exercises renumbered course-wide to be continuous 1–18 (04: ej. 7,
  05: ej. 8, week 2+ shifted +2). The GitHub snapshot must be
  republished so students actually clone them.
- Per-participant resource naming: `taller-aws-<su-nombre>` (CodeCommit repo, ECR
  repo, CodeBuild project `…-build`).
- Week-1 CloudFormation template (instructor-provided): `taller-semana1.yaml`.
- Recovery mechanism: tear down and recreate the stack to reset a pod to a
  known-good state ("el seguro del taller").
- Each participant has their own AWS account / "pod".

### AWS service status (as of 2026-06)

- **CodeCommit**: closed to new customers Jul 2024, then **returned to GA
  2025-11-24**. Lab account retains access regardless.
- **CodePipeline**: never deprecated. Active. Part of the syllabus.
- **AWS CodeStar** (projects/dashboards): **discontinued 2024-07-31**. Do not
  reference "the CodeStar service".
- **CodeStar Connections** → renamed **AWS CodeConnections** (2024-03); old APIs
  gone after Apr 2025. External Git providers; not used here.
- **CodeStar Notifications** (prefix `codestar-notifications`): **still available**,
  not renamed. Source of our pipeline notifications.

### Apps event contract

Generic, app-agnostic event bus for in-guide interactive scenarios (**built**
2026-06-23; metric/toast-demo handlers and the validating lock UI added 2026-07).
Browser custom elements emit typed events to their own pod's server;
the server runs gated side-effecting handlers and broadcasts feedback on a unified
SSE bus.

- **Envelope**: `Event { id, type, payload }` — same struct both directions.
- **Endpoints**:
  - `POST /events` — parse → dedup → gate → dispatch; 202/200/403/400.
  - `GET /events/config` — reports whether a secret is configured (`{ gated }`).
  - `GET /events/verify?secret=` — validates a secret; 204 match / 403 mismatch.
  - `GET /events/stream` — unified SSE bus (unauthenticated, read-only).
  - `GET /state/{collection}/{key}` — read-only DynamoDB query side.
- **Env vars**: `CB_APPS_SECRET` (gate unlock), `CB_APPS_GATED` (`"all"` or kind
  list — `cpu-burst,counter,metric,toast-demo`), `CB_APPS_TABLE` (DynamoDB table,
  default `courses-apps`), `CB_APPS_PUBLIC_COLLECTIONS` (default `counters`).
- **Crates**: pure logic in `courses_core::events`; I/O (DynamoDB, CloudWatch, CPU
  load, handler dispatch) in the `courses_apps` crate; routes wired in
  `courses_server`.
- **Handlers**: `cpu-burst`, `counter`, `metric`, `toast-demo`.
  - `metric` publishes to namespace `Taller/Custom` by one of two methods
    (dimension `method`): **EMF** — a structured log line on stdout, no SDK call
    and no extra IAM, works locally; or **API** — a real `PutMetricData` call,
    needs `cloudwatch:PutMetricData` on the ECS task role and real credentials.
  - `toast-demo` broadcasts a demo `Notification` on the bus, so a guide preview
    toast renders through the exact path real SNS events use. No AWS call.
- **Client**: `static/apps.js` — `<cb-cpu-burst>`, `<cb-counter>`, `<cb-metric>`,
  `<cb-toast-demo>`, and the read-only `<cb-file>` source viewer, plus lock UI and
  multiplexed `EventSource`. `<cb-file>` accepts a repository-relative UTF-8 path;
  the server embeds the source during rendering rather than exposing a filesystem
  route. The bundle loads when `uses_apps` is set (via `:::app`).
- **Lock UI**: when `/events/config` reports gated, emitting widgets render dimmed
  behind a 🔒 overlay; clicking opens an unlock modal that validates the secret
  against `/events/verify` before storing it in `sessionStorage`. A stored secret
  is re-validated on load and cleared if rejected — fails closed. View-only
  widgets (e.g. a `mode="view"` counter) read open `/state` and stay visible.
- **Notifications folded in**: notifications arrive on `/events/stream` as
  `Event{type:"notification"}`. The old `/hooks/stream` route is **retired**.
- Topic guide: `.claude/context/apps-events.md`.

### Notifications / chat integration

- Client uses **Microsoft Teams**, not Slack. Production path (CodeStar
  Notifications → SNS → AWS Chatbot → Teams) is taught as theory only.
- Lab mechanism (**built** 2026-06-23): participant pipeline events → SNS HTTPS
  subscription → `courses_server` → SSE → per-pod toasts in guide + slides.
  - Pure parser: `courses_core::notifications` (`parse_sns_message`).
  - Shell: `POST /hooks/notifications` (confirms subscription, broadcasts onto
    `/events/stream` as `type: "notification"`). **SSE endpoint is now
    `GET /events/stream`** (the old `GET /hooks/stream` is retired). Client is
    `static/apps.js` (the old `notifications.js` is retired).
  - Auth: shared-secret token via `CB_HOOK_TOKEN` env — required as `?token=` on
    `/hooks/notifications` when set, open (with a startup warning) when unset.
    Emulates the unguessable-URL secret of real chat webhooks; not a hardened
    signature. Pod attribution prefers a baked-in `pod`, falls back to account id.
  - Topic guide: `.claude/context/notifications.md`. Design:
    `.claude/designs/2026-06-16-lab-notifications-toasts-design.md`.

### Server / build notes

- Run: `cargo run -p courses_server`; local dev port `8090`.
- **Content and static assets are embedded at build time** (`include_dir!` for
  `content/`, `include_str!` for CSS/JS, and a generated `include_str!` registry
  for each repository file referenced by `<cb-file>`). Any referenced source,
  content, CSS, or JS change requires `cargo build -p courses_server` before it is
  served. This production path is unaffected by dev mode below — unchanged behavior,
  bad content still aborts startup.
- Lint gate before done: `cargo xtask lint` (fallback `cargo run -p xtask -- lint`).

### Dev mode / hot reload

- **`CB_DEV_ROOT`** (repo root; trimmed, empty counts as absent) switches the
  server to read `content/` and six text assets (`guide.css`, `slides.css`,
  `cb-widgets.css`, `apps.js`, `toggle.js`, `mermaid-init.js`) from disk, with a
  `notify` watcher that hot reloads on save. It also watches each source referenced
  by `<cb-file>`. `cargo xtask dev` sets it automatically; `CB_DEV_ROOT=$PWD cargo
  run -p courses_server` is the Docker-free equivalent. Deliberately excluded from
  `.env.example` (a machine-specific absolute path).
- **`GET /dev/reload`** — dev-only SSE route the browser client
  (`static/dev-reload.js`) subscribes to; answers 404 in production.
- Topic guide: `.claude/context/dev-workflow.md`.
