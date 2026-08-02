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
| 2 | 5 | Infraestructura como código y los primeros contenedores | CloudFormation (templates YAML, deploy/update, buenas prácticas, troubleshooting) · separación de stacks por ciclo de vida + resource import (migración de la tabla) · cierra con ECS/Fargate: task definitions y services |
| 3 | 5 | Operar, automatizar y observar | ECS/Fargate restante (networking, escalabilidad, troubleshooting) · CodePipeline (rol, leer pipelines, stages, integración, pipeline básico, aprobación manual + trigger) · inicio de Observabilidad (CloudWatch metrics y logs) |
| 4 | 3 | Observabilidad y cierre del curso | Termina Observabilidad (dashboards, alarmas, Container Insights, trazabilidad) · cierre: repaso del flujo CI/CD, troubleshooting operacional, próximos pasos |

**Content status.** Written (Week 1): `01`–`06`. Written (Week 2):
`07-cloudformation-anatomia`, `08-leer-el-template`, `09-actualizar-stacks`,
`10-preguntas-puente`, `11-buenas-practicas-troubleshooting`, `12-separar-stacks`,
`13-primeros-contenedores` (ej. 9–12). Written (Week 3): `14-operar-contenedores`,
`15-cicd-y-el-pipeline`, `16-preguntas-puente`, `17-codepipeline-en-la-practica`,
`18-notificaciones-teams`, `19-observabilidad-metrics-logs` (ej. 13–16). Written
(Week 4): `20-dashboards-y-alarmas`, `21-container-insights-trazabilidad`,
`22-cierre-del-curso` (ej. 17–18 + optional capstone ej. 19). **All 4 weeks
authored.**

**CloudFormation coverage (2026-08-01).** A gap audit compared sections `07`–`12`
against the features actually used in `infra/templates/*.yaml`. Eleven features
appeared in templates the students read without ever being taught, and `06`'s
bridge answer promised `DependsOn` for Week 2 without delivering it. All are now
closed, with no new exercises (numbering stays 1–18):

- `07` — `!Join`/`!Select`/`!GetAZs`/`!Split`/`!FindInMap`, pseudo parameters
  (`AWS::Region`, `AWS::StackName`, `AWS::AccountId`, `AWS::Partition`,
  `AWS::NoValue`), and the `Mappings` section the sections table had promised.
  Also corrected `Fn::Ref` → `Ref` (the only intrinsic without the `Fn::` prefix).
- `08` — resource **attributes** versus `Properties` (table + the "never inside
  `Properties`" rule), implicit versus explicit ordering with `DependsOn`, and the
  network block (`!Select [0, !GetAZs ""]`, `Tags`). `::: extra` on
  `Metadata: AWS::CloudFormation::Interface`.
- `09` — *Update requires* as the source of truth for replacement, plus failed
  rollback: `UPDATE_ROLLBACK_FAILED`, *Continue update rollback* with skipped
  resources, and *preserve successfully provisioned resources* for diagnosis.
- `11` — `UpdateReplacePolicy` paired with `DeletionPolicy` (the data-loss trap),
  `Snapshot` as the third value, stack-level tags and cost allocation, termination
  protection, stack policies, and service quotas. `::: extra` on
  `aws cloudformation deploy`.
- `12` — the price of an export (a value cannot change while imported; single
  region and account). `::: extra` on nested stacks and on SSM Parameter Store
  with `AWS::SSM::Parameter::Value<String>`.
- `22` — `::: extra` positioning `Transform`/SAM, CDK, and Terraform.

**Stack-splitting method (2026-08-01).** `12` previously handed over the three
stacks as a given and taught only the *mechanics* of moving a resource
(`Retain` → orphan → import). It now **derives** the split first, with a
four-step drill run on the Week 1 monolith: list the 21 resources → group by
rate of change (11 network / 1 data / 9 app) → mark the references that cross a
group → each crossing becomes an export. The drill lands on exactly the 7
exports the shipped templates already have (5 in `-red`, 2 in `-datos`), so
students can verify the method against the files. Added with it: the other two
axes (owner, blast radius; lifecycle wins when they disagree), **where security
goes** — the rule is *a permission follows its consumer, not the resource it
protects*, which is why both IAM roles live in the app stack while `RolTarea`
imports the table ARN — the three cases that do justify a separate security
stack (shared roles, separate approver, account-wide governance), and **when not
to split** (always deployed together, huge contract, changes cross constantly;
start together and split when it hurts). No separate security stack: both IAM
roles stay with the app.

**The platform stack, and adding a second app (2026-08-01).** The three-way
split left the ECS cluster and the ALB inside the app stack, so a second app
would duplicate both. `12` now derives a **fourth** cut from a second question
asked of the nine app-group resources — *how many applications use it?* — which
separates cluster + ALB + listener (all) from service, task def, target group,
log group, and the two roles (one). Teaching points: a Fargate cluster reserves
no capacity and is free, which is exactly why the duplication goes unnoticed,
while an ALB bills per hour of existence; exports are single-account and
single-region, so cross-account sharing means RAM for subnets plus a cluster per
account. The app stack no longer imports a load balancer — it **adds** an
`AWS::ElasticLoadBalancingV2::ListenerRule` to the platform's listener, whose
default action is a `fixed-response` 404, so the platform stack never names an
application. Rules evaluate low-to-high `Priority`, first match wins, priority
unique per listener, so the catch-all `/*` carries the highest number. New
sections: adding a second app, and distributing the pattern (versioned template
in S3; `::: extra` on CloudFormation modules, Service Catalog, and StackSets).

The second app is concrete rather than hypothetical: the **echo server**
subcommand of the same binary (see Server / build notes). Same image, same
template, only `ComandoContenedor=courses_server,echo`, `RutaPath=/eco/*`, and
`Prioridad=10`. It carries three teaching points that needed a real second app —
one artifact running as two applications via the container `Command` (with
`AWS::NoValue` to delete the property rather than send an empty list), routing by
`host-header` instead of `path-pattern` through an `Fn::If` inside the rule's
`Conditions`, and the networking the guide could previously only draw —
`peer` (the ALB) vs `forwarded_for` (the chain) vs `client_ip` (what to log),
`local` as the task's own ENI IP, and the ECS task metadata turning the week-1
VPC diagram into measured AZ, subnet CIDR, gateway, and resolver.
Host routing is what forced the platform certificate to carry a `*.<dominio>`
SAN and a wildcard alias record: with one certificate per name, adding an app
would mean editing — and revalidating — the shared stack.

**Teardown of the five-stack layout, and the echo server's second use
(2026-08-01).** The four-way split plus the echo stack left the course ending with
**five stacks** — `-eco`, `-app`, `-datos`, `-plataforma`, `-red` — and a
DynamoDB table that `DeletionPolicy: Retain` orphans on the way out. `05` covers
only Week 1's single stack, so `22` gained a **"Desarmar el ambiente"** section:
reverse-dependency delete order (an export cannot be deleted while imported —
the `Export … cannot be deleted as it is in use by …` message is the guarantee,
not a failure), a CLI loop with `wait stack-delete-complete`, `DELETE_SKIPPED`
and deleting the retained table by hand (the cost side of `Retain`: it moves the
delete from CloudFormation to a person), and a table of what never lived in a
stack (CodeCommit repo, CodeBuild project, ECR repo, the CodePipeline artifact
bucket, dashboard + alarm, `/aws/codebuild/…` log group, console-made IAM roles)
— which is the most concrete argument for IaC, and only visible at teardown. No
new exercise; numbering stays 1–19.

Two follow-ups landed with it. The echo server is now used past `12`: `14` reads
its `network` block as the request chain seen from inside the task (`peer` is the
balancer, not the browser; `local` alternates across healthy targets), and `21`
teaches `X-Amzn-Trace-Id` — the ALB-injected identifier the echo server returns
for free, the `trace_id` field of the ALB access log, and the `Root=` prefix of
an X-Ray trace — as correlation by identifier instead of by clock. And since the
cluster now holds two services, `13` says so (diagram plus prose: the cluster is
the context, not the subject), and `13`/`14`/`21` name **which** service each
console step means. `14`'s traffic chain gained the listener **rule** hop, and
its health-check path was corrected from `GET /` to the template's `/health`,
with the note that the health check bypasses the rule and talks to the task
directly.

Design record: `.claude/designs/2026-08-01-cloudformation-gap-closure-design.md`.

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
- **Bridge questions** (`preguntas-puente`) close a session: they are pondered
  after it and discussed at the start of the following session. `10`/`16` sit
  mid-week; Week 1's (`06`) closes the week and bridges into Week 2 — its
  questions reason from what was built toward Week 2 topics (dependency order,
  stack updates/change sets, stack separation), not backward as review. The 3 h
  week (Week 4) is single-session, no bridge.
- **Week-closing sections** (last file of Weeks 1–3) end with "Dónde estamos" (recap
  of what works now) + "Qué sigue en la Semana N+1" (what's missing / comes next).
  Closers: `06-preguntas-puente`, `13-primeros-contenedores`,
  `19-observabilidad-metrics-logs`. Week 4 closes the course, no preview.
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
- Instructor-provided CloudFormation templates (in `infra/templates/`): Week 1
  monolith `taller-aws-devops-semana1.yaml` plus variant
  `…-semana1-vpc-existente.yaml` (takes `VpcId`/`SubredAId`/`SubredBId` instead
  of creating the network); Week 2 split
  `taller-aws-devops-semana2-{red,datos,plataforma,app}.yaml` (lifecycle
  separation; the table migrates via resource import in `12-separar-stacks`,
  ej. 11) plus `…-semana2-red-existente.yaml` (network from params, SGs only,
  same five exports as `-red`). `-plataforma` owns the shared ECS cluster, the
  ALB, and its listeners, takes `RedStackName` plus optional
  `NombreDominio`/`HostedZoneId` (a `ConHttps` condition adds the ACM cert —
  with a `*.<dominio>` SAN so each app can take its own subdomain — the 443
  listener, the apex and wildcard Route 53 aliases, and the 443 ingress), and
  exports
  `${StackName}-{cluster-nombre,listener-http-arn,listener-https-arn,alb-arn,alb-dns,alb-zona}`.
  `-app` takes nine params (`ImageUri`, `ComandoContenedor`, the three stack
  names, `NombreHost`, `RutaPath`, `Prioridad`, `UsarHttps`), has an
  `AWS::CloudFormation::Interface` block, and exports
  `${StackName}-grupo-destino-arn`; deploying it twice with a different command,
  route, and priority is the whole second-app story. `ComandoContenedor` is a
  `CommaDelimitedList` that overrides the container `Command` (empty →
  `AWS::NoValue`), and `NombreHost` switches the listener rule from
  `path-pattern` to `host-header` via `Fn::If`. All nine templates pass
  `cfn-lint` clean. Extras: `…-extra-subredes-publicas.yaml` (account admin
  deploys once: two public subnets + routing on an existing VPC, optional
  existing-IGW param) and `…-extra-https.yaml` (optional per participant: ACM
  cert DNS-validated in a Route 53 hosted zone — workshop zone
  `courses.cloudbridge.com.uy` — alias record, and a 443 listener on the week-1
  ALB via `Fn::ImportValue`; guide section at the end of `04-despliegue`).
  Both Week 1 templates export
  `${StackName}-{alb-arn,alb-dns,alb-zona,grupo-destino-arn,sg-alb-id}` for the
  HTTPS add-on, and take `RedirigirAHttps` (default `no`; `si` turns listener 80
  into a 301 to HTTPS — set only while the `-https` stack exists, and delete
  that stack first on teardown since its imports block the base stack's delete).
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
  route. It can start collapsed with `toggleable`, and provides content-size and copy
  controls. `full-path` disables truncation of the slide label. The bundle loads when
  `uses_apps` is set (via `:::app`).
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
- **Subcommands** (`clap`, optional — a bare `courses_server` still means
  `serve`, so the `Dockerfile` `CMD` and every existing task definition keep
  working). `courses_server echo [--port|PORT] [--name|CB_ECHO_NAME]` starts an
  echo server: an axum `fallback` route that answers **every** request with a
  pretty JSON description of it. Five top-level keys — `received_at`, `server`
  (identity, and whether `Host` matched `--name`), `request` (method, URI, path
  segments, decoded query, grouped headers), `network`, and `body` (json / text
  / base64 / omitted over 64 KiB; 413 over 1 MiB).
  - `network` holds `local` and `peer` (each split into address + port),
    `client_ip` (first `x-forwarded-for` hop, else the peer address), the
    forwarded headers, and `ecs`. `local` matters because in `awsvpc` mode it is
    the task's own ENI IP — reached through a custom `Connected` impl over
    `IncomingStream::io().local_addr()`, since the stock
    `ConnectInfo<SocketAddr>` only carries the peer.
  - `ecs` comes from `${ECS_CONTAINER_METADATA_URI_V4}/task`, fetched **once at
    startup** (2 s timeout, every failure → `null`): cluster, task id, family,
    revision, launch type, AZ, network mode, private IPv4 and DNS name, MAC,
    subnet CIDR, subnet gateway, VPC resolvers. Needs no IAM and no VPC
    endpoint — it is a link-local address.
  - All of the shaping is pure in `courses_core::echo` (`EchoRequest` /
    `EchoServer` / `EcsNetwork` → `echo_json`, plus `parse_ecs_task_metadata`,
    and hand-rolled RFC 3339, percent-decode, base64, and host:port splitting —
    37 unit tests); `crates/server/src/echo.rs` is a thin shell.
  - It is the workshop's **second app**: same image,
    `Command: [courses_server, echo]`. `/health` returns 200 like any other
    path, so the existing target group health check needs no change.
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
