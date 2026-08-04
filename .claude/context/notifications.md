# Live notifications (SSE toasts)

The lab mirror of the production Teams flow: a participant's pipeline/build events
reach the instructor's public server and show up as toasts in the guide and slides,
attributed per pod. Read this before touching the notification path.

## Flow

```
Participant's CodePipeline / CodeBuild (state change)
  → CodeStar Notifications / EventBridge rule
  → SNS topic                           (in the participant's own account)
  → POST /hooks/notifications?token=…   (HTTPS subscription on the public server)
  → parse + broadcast
  → GET /events/stream                  (unified SSE bus, one per open guide/slide page)
  → toast in the browser (apps.js)
```

A notification rule can only target an SNS topic in **its own account and region**, and
each participant owns a pod account, so the topic cannot be the instructor's. Each pod
creates its own topic (**Create target → SNS topic** during rule creation, which applies
the publish policy for it) and points an HTTPS subscription at the shared endpoint. The
subscription — not the rule target — is what crosses accounts.

> **Note:** `GET /hooks/stream` is **retired**. All SSE consumers use
> `GET /events/stream`. Notifications ride the unified bus as
> `Event { type: "notification", payload: <notification-json> }`.
> See `.claude/context/apps-events.md` for the full event contract.

Production target is Microsoft Teams (CodeStar Notifications → SNS → AWS Chatbot →
Teams); the lab swaps only the last hop. See the CI/CD content (`18-notificaciones-teams`).

## Where the code lives

- **Pure (`crates/core/src/notifications.rs`)** — no I/O. `parse_sns_message(body)`
  returns `SnsMessage::Confirmation { subscribe_url }` or
  `SnsMessage::Event(Box<Notification>)` (boxed: a `Notification` dwarfs a subscribe
  URL, and clippy's `large_enum_variant` says so). Handles the SNS envelope and the
  inner EventBridge/CodeStar/CodeBuild/CloudWatch-alarm payload — an alarm reads
  `AlarmName` → detail, `NewStateValue` → state, `AWSAccountId` → pod, and an
  `AlarmName` with no `source` resolves the source to `aws.cloudwatch`, which is
  what `20-dashboards-y-alarmas` publishes. `Notification`
  carries `pod`, `source`, `state`, `detail` plus optional `stage`, `action`,
  `provider`, `phase`, `reason`, `execution`, `region`, `time`, `url`; the optional
  ones are `skip_serializing_if = "String::is_empty"`, so the SSE JSON stays small
  and the client can test truthiness. `reason` reads
  `external-execution-summary` → `error-code` → `NewStateReason` →
  `additional-information.message`. `console_url` prefers a URL the payload already
  supplies (`external-execution-url`, CodeBuild `logs.deep-link`) and otherwise
  builds the CodePipeline / CodeBuild / CloudWatch console link from `region` +
  `detail`; no region means no link. `token_matches(expected, provided)` is the
  constant-time-ish auth check. Fully unit-tested inline.
- **Shell (`crates/server/src/routes.rs`)** — `POST /hooks/notifications` (auth,
  parse, confirm-or-broadcast), `AppState` + `FromRef`. On a
  `SubscriptionConfirmation` it GETs the `SubscribeURL` (reqwest) to confirm; on an
  event it broadcasts onto the unified `GET /events/stream` SSE bus as
  `Event { type: "notification" }`. The old `GET /hooks/stream` route is retired.
- **Client (`crates/server/static/apps.js`)** — multiplexed
  `EventSource('/events/stream')` demultiplexed by `type`; the `"notification"`
  type renders/auto-dismisses toasts. The old `notifications.js` is retired. Toast
  CSS lives in both `guide.css` and `slides.css`.

## Toast layout

One compact card per event (reworked 2026-08-04, after a live run stacked five
near-identical toasts that filled the viewport):

- **Line 1** — the subject (`detail`: pipeline, project, or alarm name), single
  line with ellipsis, plus the `×N` repeat counter, the clock, and a `×` close
  button. The clock shows the event's own `time` when the payload carries one
  (`time`, `StateChangeTime`), else arrival time.
- **Line 2** — the raw AWS state as a colored badge (the console's own word, not a
  translation), then the friendly source (`aws.codepipeline` → `CodePipeline`) and
  `pod <account>`. The source is dropped when it is already the title.
- **Line 3 (context)** — `stage · action · phase · reason`, whichever are present:
  the one-line answer to "what was this about". Hidden while expanded.
- A status glyph, the left accent, the badge, and a bottom countdown bar all take
  the same accent colour: green ok / red fail / yellow warn (`SUPERSEDED`,
  `CANCELED`, `STOPPED`, `INSUFFICIENT_DATA`) / teal info. `ALARM`/`OK` map to
  fail/ok explicitly — CloudWatch shares no state vocabulary with CodePipeline.
- **Repeats coalesce**: `pod|source|state|detail|context` is the identity; a repeat
  while the toast is up bumps `×N` and restarts the 8 s timer instead of stacking.
- **At most 4** toasts are visible; the oldest is dropped.
- **Click expands** the card into a `<dl>` of every non-empty field (etapa, acción,
  proveedor, fase, motivo, ejecución, región, hora, origen, pod), scrollable at
  `max-height: 40vh`. An expanded toast stops its timer and stays until dismissed —
  nobody reads a payload in 8 s. `×` or a second click closes it.
- **The title is a link** when the parser found a console URL (`url`), marked with
  `↗` and opened in a new tab. Clicking the link does not toggle the card.

## Auth

A shared-secret token gates ingestion, emulating the unguessable-URL secret real chat
webhooks use — simple, not hardened.

- Set `CB_HOOK_TOKEN` on the server; subscribe SNS to
  `https://<host>/hooks/notifications?token=<secret>`. Live workshop values: endpoint
  `https://courses.cloudbridge.com.uy/hooks/notifications?token=cloudbridge`, with the
  token set on the task definition in
  `infra/templates/taller-aws-devops-semana3-app.yaml`. The token is published in the
  guide, so anyone holding the URL can inject toasts — accepted for a workshop.
- When set, a missing/wrong token → `401`. When unset, the endpoint is open and logs a
  startup warning.
- The SSE bus `GET /events/stream` is unauthenticated by design (read-only public toasts).

## Pod attribution

Every event should name a pod. Resolution order: a baked-in `pod` field (top level or
under `detail`) → the AWS account id (`account`, or `AWSAccountId` on an alarm
message) → `desconocido`. An unresolved pod still renders
(tagged with the account/`desconocido`) so misconfiguration is visible rather than
silently dropped.

Only an EventBridge rule with an input transformer can bake in `pod`; a CodeStar
Notifications rule has no transform step (`create-notification-rule` takes only name,
event types, resource, targets, status, detail type). The lab uses notification rules,
so its toasts always fall through to the account id.

## Open items

- No SNS message-signature verification or topic-ARN allowlist (token only).
- No persistence/replay: a toast missed while a page was closed is gone.
- The guide (`18-notificaciones-teams`) does not tell students a toast can be
  clicked open, or that its title links to the console.
