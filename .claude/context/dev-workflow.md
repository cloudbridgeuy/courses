# Dev workflow notes

## Running the server locally

- Port 8080 is often taken by an unrelated Vite app on this machine. Test with:
  `PORT=8090 ./target/debug/courses_server`
- Kill the server after manual tests: `pkill -f courses_server`.
- Session URLs use title slugs, not filenames: discover them with
  `curl -s localhost:8090/ | grep -o 'href="/courses/[^"]*"'`.

## Verifying in the browser (Playwright MCP)

- Guides use `scroll-behavior: smooth`: poll `scrollY` until ~5 consecutive identical
  100ms samples before asserting positions; set `scrollBehavior = 'auto'` when you
  need deterministic initial positioning.

## Lint

- `cargo xtask lint` before declaring done; silent unless failing, full log at
  `target/xtask-lint.log`. `cargo xtask lint --fix` applies rustfmt fixes.

## Repo hygiene

- Plans and designs (`.claude/plans/`, `.claude/designs/`) stay OUT of git.
  Course content, `content/`, and fonts are committed. `.claude/context/` docs
  and `CLAUDE.md` are committed.
- Conventional-commit subjects (`feat(core): …`, `fix(slides): …`); no AI
  attribution trailers.
