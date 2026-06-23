# courses_apps

I/O crate: event handler execution, DynamoDB reads and writes, and CPU load
generation for the apps event contract.

## Role

This crate is the imperative shell for the `/events` subsystem. It depends on the
AWS DynamoDB SDK (the first AWS SDK in this repo) and on `tokio`. No pure crate
depends on it.

Pure logic — the `Event` type, deduplication, gating, collection allowlist checks —
lives in `courses_core::events`. This crate adds the I/O: it calls DynamoDB, burns
CPU on the Fargate task, and drives the handler dispatch loop.

## Public surface

| Symbol | Description |
|--------|-------------|
| `AppsCtx` | Shared context: DynamoDB client, table name, SSE broadcast sender |
| `Outcome` | Result type returned by `dispatch` |
| `dispatch(event, ctx)` | Execute the handler selected by `event.kind`; returns an `Outcome` |
| `read_item(collection, key, ctx)` | DynamoDB `GetItem` for the CQRS query side (`GET /state`) |

Internal helpers (`cpu_burst`, `counter`) are not part of the public API.

## Wiring

`courses_server` builds an `AppsCtx` at startup and passes it into the route
handlers. The routes call `parse_event` + gating from `courses_core::events`,
then hand off to `dispatch` here.
