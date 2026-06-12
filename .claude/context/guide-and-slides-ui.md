# Guide and slides UI conventions

## Guide pages (`crates/server/static/guide.css`, `crates/core/src/render.rs`)

- Header is NOT sticky; anchor navigation lands headings at viewport top.
  `scroll-margin-top: 1rem` on sections/headings adds breathing room.
- `html { scroll-behavior: smooth }` — affects browser-automation tests (see
  dev-workflow.md).
- Heading self-links: `a.cb-hlink`, inherit color, show a teal ` #` suffix on hover.
- Section headings render as `<section id="seccion-N"><h2><a class="cb-hlink" …>`.
- Floating buttons (wide viewports only, ≥56rem):
  - `#cb-top-btn` (↑): appears past 300px scroll; clicking saves scrollY then goes top.
  - `#cb-back-btn` (↓): restores the saved position, then clears it.
  - Saved position lives in `sessionStorage` key `'cb-scroll:' + location.pathname`
    so it survives guide → slides → guide navigation. Visibility check runs once on
    load. Button bottom offset adjusts when the footer intersects.
- Ordered-list step numbering restarts at 1 in each subsection.

## Slides (`crates/server/static/slides.css`)

- CloudBridge reveal.js theme replaces stock themes; Montserrat, ink `#2d2926`,
  teal `#5fc8be`, blue `#5dc8f0`.
- Light variant via `section.cb-light`; background set with `data-background`
  (section background doesn't fill the viewport).
- Exercise hero cards: `.cb-ejercicio` with kicker, title, body, solution toggle.
  Reveal re-layouts when a solution toggle changes slide height.
- Heading self-links come from the shared Markdown renderer; on slides they're inert
  (`pointer-events: none`) — hash clicks would fight reveal.js `#/n` navigation.
- Last viewed slide persists in `sessionStorage` key `'cb-slide:' + location.pathname`.
- Close button `.cb-slides-close` (top-right, inline SVG cross) returns to the guide;
  stays dark on hover so it's visible on light slides.
