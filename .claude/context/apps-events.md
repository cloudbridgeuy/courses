# Apps event contract

A generic, app-agnostic event bus for in-guide interactive scenarios. Browser
custom elements emit typed events to their own pod's server; the server runs
gated side-effecting handlers and broadcasts feedback on a unified SSE bus.
Read this before touching any code under `courses_core::events`, `courses_apps`,
or the `/events` routes.

## Envelope

One struct in both directions:

```rust
Event { id: EventId, kind: String, payload: String }
```

- `id` — idempotency key (UUID or similar opaque string), set by the emitting client.
- `kind` — selects the handler (inbound) or consumer (outbound). Serialised as
  `"type"` in JSON (`#[serde(rename = "type")]`); the field is named `kind` in
  Rust to avoid the keyword.
- `payload` — opaque text; meaning is handler-specific.

## Endpoints

| Method | Path | Auth | Notes |
|--------|------|------|-------|
| `POST /events` | inbound | optional (`?secret=`) | parse → dedup → gate → dispatch |
| `GET /events/config` | read-only | none | `{ "gated": bool }` — is a secret configured? |
| `GET /events/verify` | read-only | `?secret=` | validate a secret: `204` match, `403` mismatch |
| `GET /events/stream` | outbound SSE | none | unified bus; one `EventSource` per page |
| `GET /state/{collection}/{key}` | read-only | none | DynamoDB read, CQRS query side |

### POST /events — status codes

| Code | Condition |
|------|-----------|
| 202 | Accepted and dispatched (or duplicate `id` → accepted, no-op) |
| 200 | Unknown `kind` — no handler; event silently dropped |
| 403 | Handler is gated and the provided secret is wrong or missing |
| 400 | Malformed JSON |

Duplicate detection uses `RecentIds` (a fixed-size ring buffer in
`courses_core::events`). A duplicate returns 202 without re-dispatching.

### GET /events/stream

Unauthenticated SSE stream carrying `Event` values (JSON). The browser opens
exactly one `EventSource('/events/stream')` and demultiplexes by `type`.
Notifications and handler progress both ride this bus:

- Notifications arrive as `Event { type: "notification", payload: <notification-json> }`.
- Handler progress / feedback arrives as `Event { type: "app-status", payload }`.

**The old `/hooks/stream` route is retired.** All SSE consumers must use
`/events/stream`.

### GET /state/{collection}/{key}

Read-only DynamoDB lookup. Only collections listed in `CB_APPS_PUBLIC_COLLECTIONS`
are reachable; any other collection returns 404. This is the CQRS query side:
events are commands (`POST /events`), state reads are queries (`GET /state`).

## Gating

Handlers are optionally behind a secret. The gate is controlled by two env vars:

| Env var | Default | Meaning |
|---------|---------|---------|
| `CB_APPS_SECRET` | unset (open) | Unlock secret; when unset all handlers run ungated |
| `CB_APPS_GATED` | unset | `"all"` to gate every kind, or a comma-separated kind list |

A gated `POST /events` request must supply `?secret=<value>`. The check reuses
`token_matches` (constant-time comparison) from `courses_core`. The instructor
enters the secret once in the guide UI; it is stored in `sessionStorage` under
the key `cb-apps-secret` and attached automatically to gated emits as `?secret=`.

Two pure helpers on `Gate` back the client UX: `requires_secret()` (false only
for `Gate::Open`, drives whether the lock UI shows) and `accepts(provided)`
(constant-time validation, always true for `Gate::Open`, backs `/events/verify`).

### Lock UI (client)

When `/events/config` reports `gated`, interactive widgets render dimmed with a
🔒 overlay (`.cb-app-locked`). Clicking a locked widget opens a centered unlock
modal. Saving validates the secret against `/events/verify` before storing it —
a wrong value never unlocks. On load, any secret already in `sessionStorage` is
re-validated; a rejected or unverifiable secret is cleared (fail closed). Only
emitting widgets lock (`cb-cpu-burst` always, `cb-counter` in `increment`/`both`
mode); a view-only counter reads the open `/state` endpoint and stays visible.

## Same-origin topology

`courses_server` IS the workload each participant deploys to their own AWS account.
An app event hits that participant's own pod's server — there is no cross-pod
fan-out. Consequences:

- A CPU-burst handler burns CPU on that pod's ECS task → that pod's CloudWatch
  metrics respond.
- No CORS configuration is needed; browser and server share the same origin.
- The SSE bus (`/events/stream`) is per-pod; each participant sees only their own
  feedback.

`/hooks/notifications` ingress is unchanged — the instructor's public server
still receives SNS HTTPS subscription POSTs and broadcasts them on
`/events/stream` as `type: "notification"` events.

## Where the code lives

### Pure — `courses_core::events`

No I/O. Contains:

- `Event` / `EventId` types + JSON de/serialisation.
- `parse_event(body)` — deserialises and validates an inbound event.
- `RecentIds` — fixed-size ring buffer for deduplication.
- `select(kind)` — maps a kind string to a handler variant (or Unknown).
- `gate(kind, secret, config)` / `parse_gate(env)` — gating logic.
- `CpuBurstConfig` — pure config struct for the CPU-burst handler.
- `MetricConfig` / `MetricMethod` — pure config for the custom-metric handler.
  `MetricMethod` is `Emf | Api`; `MetricConfig::parse` clamps `value` to `0..=100`.
- `demo_notification(seed)` (in `courses_core::notifications`) — builds a demo
  `Notification` from a seed, cycling deterministically across the three pipeline
  events (`SUCCEEDED` / `FAILED` / pending approval). Backs the `toast-demo` handler.
- `is_public_collection(collection, whitelist)` — collection allowlist check.

`select` maps `"cpu-burst" | "counter" | "metric" | "toast-demo"` to the matching
`HandlerKind` (`CpuBurst | Counter | Metric | ToastDemo`); any other string is Unknown.

All of these are unit-tested inline.

### I/O — `courses_apps`

Depends on the AWS DynamoDB SDK and `aws-sdk-cloudwatch` (the repo's second AWS
SDK, pulled in for the metric handler's API path). Contains:

- `dispatch(event, ctx)` — executes the selected handler and returns an `Outcome`.
- `cpu_burst(config)` — spawns a timed CPU load on the Fargate task.
- `counter(key, table, client)` — atomic counter increment in DynamoDB.
- `metric(ctx, payload)` — publishes a custom metric to CloudWatch by one of two
  methods: an EMF log line on stdout (no SDK call, no extra IAM — CloudWatch
  extracts the metric from the log group) or a direct `PutMetricData` API call
  (needs `cloudwatch:PutMetricData` on the ECS task role). Both target namespace
  `Taller/Custom`, metric `CustomValue`, dimension `method` (`emf` | `api`).
- `toast_demo(ctx, seed)` — broadcasts a demo `Notification` (built by the pure
  `demo_notification`) on the bus as a `type: "notification"` event, so a guide
  preview toast renders through the exact path real SNS events use. No AWS call.
- `read_item(collection, key, ctx)` — DynamoDB `GetItem` for the query side.
- `AppsCtx` — shared context struct (DynamoDB client, CloudWatch client, table
  name, broadcast sender, gate, public collections).
- `Outcome` — result type returned by `dispatch`.

Nothing pure depends on `courses_apps`.

### Shell — `courses_server`

Wires routes, owns `Mutex<RecentIds>`, and builds `AppsCtx`:

- `POST /events` — calls `parse_event`, dedup check, gate check, `dispatch`.
- `GET /events/stream` — SSE over a `tokio::broadcast` channel.
- `GET /state/{collection}/{key}` — calls `read_item` after allowlist check.

## Client — `apps.js`

`crates/server/static/apps.js` provides:

- Custom elements `<cb-cpu-burst>`, `<cb-counter>`, `<cb-metric>`, and
  `<cb-toast-demo>` (a fire-and-forget button that emits a `toast-demo` event;
  feedback is the broadcast toast itself, so it carries no app-status listener;
  locks like `cb-cpu-burst` when gated).
  `<cb-counter>` takes a `mode` attribute — `increment` (button only), `view`
  (value only), or `both` (default); elements sharing a `key` stay in sync via the
  SSE bus, so an incrementer and a separate viewer can sit in different parts of the
  page. `<cb-metric>` takes `mode` (`emf` | `api`) and `label`; it renders a 0–100
  number input plus submit button and emits a `metric` event
  `{ value, method }`, locking like `cb-cpu-burst` when gated.
- Lock UI + unlock modal (see "Lock UI" above): dims gated widgets, validates the
  secret via `/events/verify`, stores it in `sessionStorage` under `cb-apps-secret`.
- `<cb-file path="./buildspec.yml" type="yaml">` is a read-only app. The server
  resolves its repository-relative path while rendering, embeds its UTF-8 source in
  the element, and the client renders it as a Shiki-highlighted code block. It never
  receives a filesystem endpoint, so a published course cannot expose arbitrary
  repository files. The authored path is preserved and rendered as a small label at
  the upper-left edge of the code block. `toggleable` hides the code at first;
  `open` makes a toggleable block start open. Each rendered file also has decrease,
  increase, and copy controls. The decrease and increase controls only change the
  file content size. Their own size stays fixed.
- A single multiplexed `EventSource('/events/stream')` demultiplexed by `type`.
- Toast renderer for `type: "notification"` events (replaces `notifications.js`).

`apps.js` is injected only when the guide/slide content sets `uses_apps: true`
(mirrors the `uses_mermaid` flag). The `:::app` Markdown directive wraps trusted
custom-element tags in `<div class="cb-app">` and sets `uses_apps`.

## Environment variables

| Var | Default | Purpose |
|-----|---------|---------|
| `CB_APPS_SECRET` | unset | Gate unlock secret |
| `CB_APPS_GATED` | unset | `"all"` or comma kind list (`cpu-burst,counter,metric,toast-demo`) |
| `CB_APPS_TABLE` | `"courses-apps"` | DynamoDB table name |
| `CB_APPS_PUBLIC_COLLECTIONS` | `"counters"` | Comma-separated readable collections |

## AWS Secrets Manager teaching hook

`CB_APPS_SECRET` is the concrete example used in the workshop to contrast
ECS `environment` (plaintext, visible via `DescribeTaskDefinition`) with
`secrets` + `valueFrom` (pulls from Secrets Manager or SSM at task launch,
never stored in the task definition). See the Week-2 ECS content
(`13-primeros-contenedores`) for the in-guide treatment.

## Open items

- No replay: events missed while the SSE connection is closed are gone.
- `RecentIds` ring buffer is in-process; a multi-replica deployment would need
  an external dedup store (DynamoDB conditional write or ElastiCache).
