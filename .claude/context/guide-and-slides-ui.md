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
- `hr` (Markdown `---`) is an 18rem centered bar carrying the `.cb-strip` teal→blue
  gradient, tapered to transparent at both ends. Slides use the same rule sized in
  `em` so it tracks the scaled deck.
- `render_session_page` emits `<hr class="cb-section-break">` between consecutive
  section files (never before the first), so the seams of a stitched session read
  like the `---` breaks inside one. Its `margin-bottom` is 0 on purpose — the
  following `h2` supplies the gap below through its own collapsing margin.

## Slides (`crates/server/static/slides.css`)

- CloudBridge reveal.js theme replaces stock themes; Montserrat, ink `#2d2926`,
  teal `#5fc8be`, blue `#5dc8f0`.
- Light variant via `section.cb-light`; background set with `data-background`
  (section background doesn't fill the viewport).
- Exercise hero cards: `.cb-ejercicio` with kicker, title, body, solution toggle.
  Reveal re-layouts when a solution toggle changes slide height.
- Anything that grows a slide after reveal has laid it out MUST call
  `Reveal.layout()`. With `center: true` reveal freezes an inline `top` computed
  from the height at layout time; mermaid renders later, so without the relayout
  the diagram is offset by a quarter of the deck and overflows the bottom
  (`mermaid-init.js` does this in `afterRender`).
- reveal's core stylesheet does NOT style anchors — link colour ships in the stock
  themes this file replaces, and the `--r-link-*` vars are inert without them. So
  `slides.css` styles `a:not(.cb-hlink)` itself: brand blue on dark, `--cb-link`
  on `section.cb-light` (brand blue is unreadable there). The `:not()` matters —
  `.reveal section.cb-light a` would otherwise outrank `.reveal .cb-hlink`.
- `.reveal pre` sets `text-align: left`; slides centre their text, which would
  otherwise destroy indentation in code blocks. `pre.mermaid` stays centred.
- Inline `code` is a tinted chip (background, hairline teal border, 5px radius),
  reset back to bare inside `pre`. The reset needs BOTH `.reveal pre code` and
  `.reveal section.cb-light pre code`, since `.reveal section.cb-light code`
  outranks the former.
- `li::marker` is teal; light slides use `--cb-teal-deep` (`#1f7a70`), because
  brand teal is ~1.9:1 on the light background.
- `.reveal` sets `line-height: 1.4` (core reveal sets none, leaving `normal`).
  Keep chip padding small — the inline-code box must fit inside the line box or
  it paints over the descenders above. Headings pin `line-height: 1.15`.
- A dense slide remains at Reveal's fixed canvas height and scrolls vertically inside
  its top-level `<section>` instead of being clipped or auto-shrunk. The scroll panel
  has `touch-action: pan-y`, so it also works with a touch gesture. Its scrollbar is
  hidden (`scrollbar-width: none` + `::-webkit-scrollbar { display: none }`) so no
  rail shows during the presentation; wheel/touch/key scrolling still works.
- Fenced code blocks that declare a language load Shiki on demand. The client-side
  initializer uses the local TextMate definition of `tokyonight-storm`; Mermaid and
  untyped blocks keep their dedicated/plain rendering.
- Top-level slide headings make the content hierarchy visible: `h1` uses a wide
  teal→blue divider, `h2` a tapered underline, `h3` a teal marker and rule, and `h4`
  a compact uppercase label. `.cb-title-slide` and exercise-card headings retain
  their specialized styles.
- A slide's first child gets `margin-top: 0` so content starts flush with the
  canvas top (the UA h2 margin otherwise pushes the slide down and eats canvas
  height). `.cb-title-slide` is exempt — the hero has nothing below it.
- Heading self-links come from the shared Markdown renderer; on slides they're inert
  (`pointer-events: none`) — hash clicks would fight reveal.js `#/n` navigation.
- Last viewed slide persists in `sessionStorage` key `'cb-slide:' + location.pathname`.
- Close button `.cb-slides-close` (top-right, inline SVG cross) returns to the guide;
  stays dark on hover so it's visible on light slides.
