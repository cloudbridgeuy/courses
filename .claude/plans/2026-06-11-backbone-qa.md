# Manual QA Testing Plan: Courses Platform Backbone

**Source plan:** `.claude/plans/2026-06-11-backbone.md`
**Generated:** 2026-06-11

## Overview

This plan validates the scaffolded `courses` workspace: the `cargo xtask lint` gate
(silent unless it fails), the pure `courses_core` test suite, and the `courses_server`
axum slice that renders a placeholder guide and shuts down gracefully.

## Prerequisites

- Rust toolchain `1.95.0` installed with `clippy` and `rustfmt`. Verify:
  `rustup run 1.95.0 rustc --version` → `rustc 1.95.0`.
- A free TCP port 8080 (or set `PORT` to another).
- `curl` available.
- Run all commands from the workspace root: `/Users/guzmanmonne/Projects/Rust/courses`.
- This repo is not under git; nothing to stash or reset.

Note: the examples prefix cargo with `rustup run 1.95.0` to pin the toolchain
explicitly. Once `rustup` resolves `rust-toolchain.toml` in your shell, plain `cargo`
also works.

## How to Run

- **Solo:** Execute each step. Compare output to the "Expected output" block. Mark
  pass or fail.
- **With an agent:** Ask the agent to "walk me through the QA plan one step at a
  time." It should show each command, let you run it (or run it with your approval),
  display the output, compare to expected, and advance only after you confirm.

## Scenarios

### Scenario 1: The lint gate passes silently

**Purpose:** The repo is clean, and `cargo xtask lint` reports each check with one
line, printing no check body on success.

**Steps:**

1. Run:
   ```
   rustup run 1.95.0 cargo run -p xtask -- lint
   ```
   **Expected output** (after an optional one-time compile line):
   ```
   ✓ fmt
   ✓ check
   ✓ clippy
   ✓ test
   ✓ file-length
   ✓ forbidden-allows
   ```

2. Confirm the exit code is zero:
   ```
   echo $?
   ```
   **Expected output:**
   ```
   0
   ```

3. Confirm the full log was written:
   ```
   grep -c '^=== ' target/xtask-lint.log
   ```
   **Expected output:**
   ```
   6
   ```

**Pass criteria:** Six `✓` lines, no check body printed, exit 0, log holds six check
sections.
**Common failure modes:** A `✗` line means a check failed — its output prints above
the line, and the full text is in `target/xtask-lint.log`.

### Scenario 2: The pure core test suite

**Purpose:** Every pure function in `courses_core` is covered, and the whole workspace
test count matches the design.

**Steps:**

1. Run:
   ```
   rustup run 1.95.0 cargo test --workspace
   ```
   **Expected output** (a stable subset — three test binaries plus doc-tests):
   ```
   test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
   test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
   test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
   ```
   (18 = `courses_core`, 9 = `xtask`, 0 = `courses_server`, which is shell-only by
   design. Ordering of the binaries may vary.)

**Pass criteria:** 18 core tests and 9 xtask tests pass; the server has zero tests.
**Common failure modes:** A non-zero failure count names the failing test; the server
showing any test count means the FCIS rule was broken.

### Scenario 3: The server renders the guide

**Purpose:** The axum shell binds, serves `/health` and `/`, and renders the
placeholder guide through `courses_core`.

**Steps:**

1. Start the server in the background:
   ```
   PORT=8080 rustup run 1.95.0 cargo run -p courses_server &
   ```
   Wait until it logs (about 2 seconds):
   ```
   INFO courses_server: listening on http://0.0.0.0:8080
   ```

2. Run:
   ```
   curl -s -w '\n[%{http_code}]\n' http://127.0.0.1:8080/health
   ```
   **Expected output:**
   ```
   ok
   [200]
   ```

3. Run:
   ```
   curl -s http://127.0.0.1:8080/ | head -3
   ```
   **Expected output:**
   ```
   <!doctype html>
   <html lang="es">
   <head>
   ```

4. Confirm the rendered title appears twice (once in `<title>`, once in `<h1>`):
   ```
   curl -s http://127.0.0.1:8080/ | grep -c 'Taller AWS DevOps'
   ```
   **Expected output:**
   ```
   2
   ```

5. Stop the server (see Rollback / Cleanup for finding the PID if needed):
   ```
   kill %1
   ```

**Pass criteria:** `/health` returns `ok` with status 200; `/` returns a Spanish HTML
document whose title appears twice.
**Common failure modes:** `Connection refused` means the server has not bound yet —
wait and retry. A 500 on `/` means `sample_course()` failed to build.

## Edge Cases

- **Graceful shutdown on SIGTERM (the ECS task-stop path).** Start the server, find
  its PID, send SIGTERM, and confirm it exits cleanly without a panic:
  ```
  PORT=8080 rustup run 1.95.0 cargo run -p courses_server &
  sleep 3
  kill -TERM %1
  ```
  Expected: the process exits, and `jobs` shows it gone. No panic, no stack trace.

- **Ctrl-C shutdown.** Run the server in the foreground, press Ctrl-C, and confirm a
  clean exit (no panic).

- **Invalid PORT is rejected at the boundary.** Run:
  ```
  PORT=abc rustup run 1.95.0 cargo run -p courses_server
  ```
  Expected: the process exits non-zero, and prints an error containing
  `invalid PORT value: "abc"`. It does not bind.

- **Empty PORT is rejected.** `PORT= rustup run 1.95.0 cargo run -p courses_server`
  exits with the same `invalid PORT value` error (an empty string fails `u16` parse).

- **The lint gate catches a real failure.** Append a badly formatted line to a source
  file, run the lint, and confirm it fails at `fmt`:
  ```
  printf '\npub fn _scratch( ){let _x=1 ;}\n' >> crates/core/src/render.rs
  rustup run 1.95.0 cargo run -p xtask -- lint ; echo "exit=$?"
  ```
  Expected: a `Diff in .../render.rs` block prints, followed by `✗ fmt`, then
  `exit=1` (later checks do not run — the pipeline stops at the first failure).
  **Then revert** (see cleanup) before doing anything else.

## Rollback / Cleanup

- **Undo the lint-failure edit** from the last edge case. The safe revert (the
  interactive `mv`/`cp` aliases can silently decline) is to delete the appended line
  in your editor, or:
  ```
  rustup run 1.95.0 cargo fmt -p courses_core
  ```
  then manually delete the `_scratch` function from `crates/core/src/render.rs`.
  Confirm with `rustup run 1.95.0 cargo run -p xtask -- lint` showing six `✓` lines.
- **Stop any background server:** `kill %1` (or `pkill -f courses_server`). Confirm
  with `jobs`.
- **Reset build state (optional):** `rm -rf target/` forces a clean rebuild and clears
  `target/xtask-lint.log`.
