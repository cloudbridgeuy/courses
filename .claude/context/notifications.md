# Live notifications (SSE toasts)

The lab mirror of the production Teams flow: a participant's pipeline/build events
reach the instructor's public server and show up as toasts in the guide and slides,
attributed per pod. Read this before touching the notification path.

## Flow

```
Participant's CodePipeline / CodeBuild (state change)
  → CodeStar Notifications / EventBridge rule
  → SNS topic
  → POST /hooks/notifications?token=…   (HTTPS subscription on the public server)
  → parse + broadcast
  → GET /events/stream                  (unified SSE bus, one per open guide/slide page)
  → toast in the browser (apps.js)
```

> **Note:** `GET /hooks/stream` is **retired**. All SSE consumers use
> `GET /events/stream`. Notifications ride the unified bus as
> `Event { type: "notification", payload: <notification-json> }`.
> See `.claude/context/apps-events.md` for the full event contract.

Production target is Microsoft Teams (CodeStar Notifications → SNS → AWS Chatbot →
Teams); the lab swaps only the last hop. See the CI/CD content (`17-notificaciones-teams`).

## Where the code lives

- **Pure (`crates/core/src/notifications.rs`)** — no I/O. `parse_sns_message(body)`
  returns `SnsMessage::Confirmation { subscribe_url }` or `SnsMessage::Event(Notification)`.
  Handles the SNS envelope and the inner EventBridge/CodeStar payload (pod, source,
  state, detail). `token_matches(expected, provided)` is the constant-time-ish auth
  check. Fully unit-tested inline.
- **Shell (`crates/server/src/routes.rs`)** — `POST /hooks/notifications` (auth,
  parse, confirm-or-broadcast), `AppState` + `FromRef`. On a
  `SubscriptionConfirmation` it GETs the `SubscribeURL` (reqwest) to confirm; on an
  event it broadcasts onto the unified `GET /events/stream` SSE bus as
  `Event { type: "notification" }`. The old `GET /hooks/stream` route is retired.
- **Client (`crates/server/static/apps.js`)** — multiplexed
  `EventSource('/events/stream')` demultiplexed by `type`; the `"notification"`
  type renders/auto-dismisses toasts. The old `notifications.js` is retired. Toast
  CSS lives in both `guide.css` and `slides.css`.

## Auth

A shared-secret token gates ingestion, emulating the unguessable-URL secret real chat
webhooks use — simple, not hardened.

- Set `CB_HOOK_TOKEN` on the server; subscribe SNS to
  `https://<host>/hooks/notifications?token=<secret>`.
- When set, a missing/wrong token → `401`. When unset, the endpoint is open and logs a
  startup warning.
- The SSE bus `GET /events/stream` is unauthenticated by design (read-only public toasts).

## Pod attribution

Every event should name a pod. Resolution order: a baked-in `pod` field (top level or
under `detail`, added by the notification rule / EventBridge input transformer) →
the AWS `account` id → `desconocido`. An unresolved pod still renders (tagged with the
account/`desconocido`) so misconfiguration is visible rather than silently dropped.

## Open items

- No SNS message-signature verification or topic-ARN allowlist (token only).
- No persistence/replay: a toast missed while a page was closed is gone.
