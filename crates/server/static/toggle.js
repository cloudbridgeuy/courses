// Guide/slides client behavior: solution toggles and code-copy buttons.

// On slides, reveal.js renders a fixed-size deck scaled to the viewport and
// centered, so tall viewports leave vertical whitespace above and below. When
// a slide has an open solution, top-align the deck to reclaim the top
// whitespace, then grow the solution's scroll panel to fill the available
// height (capping so the card never spills past the viewport). All no-ops
// outside the slideshow, where window.Reveal is undefined.
const CB_SLIDE_MARGIN = 24; // device px breathing room against viewport edges

function cbReveal() {
  return window.Reveal && typeof Reveal.getCurrentSlide === "function"
    ? Reveal
    : null;
}

// Pin the deck to the top of the viewport, preserving reveal's current scale.
function cbAlignDeckTop() {
  const slides = document.querySelector(".reveal .slides");
  if (!slides) return;
  const m = /scale\(([^)]+)\)/.exec(slides.style.transform);
  const scale = m ? m[1] : "1";
  slides.style.top = CB_SLIDE_MARGIN + "px";
  // Pivot the scale from the top so the deck grows downward from the anchor.
  slides.style.transformOrigin = "50% 0";
  slides.style.transform = `translate(-50%, 0) scale(${scale})`;
}

// Grow each open solution panel on `slide` to fill the available height.
function cbFitSlideSolutions(slide) {
  const scale = Reveal.getScale() || 1;
  for (const panel of slide.querySelectorAll(".solucion-cuerpo")) {
    if (panel.hasAttribute("hidden")) {
      panel.style.maxHeight = "";
      continue;
    }
    const card = panel.closest(".cb-ejercicio") || panel.closest(".solucion");
    panel.style.maxHeight = "none";
    // Converge in a few passes: shrinking the panel shifts the card's box.
    for (let pass = 0; pass < 6; pass++) {
      const rect = (card || panel).getBoundingClientRect();
      const over =
        Math.max(rect.bottom - (window.innerHeight - CB_SLIDE_MARGIN), 0) +
        Math.max(CB_SLIDE_MARGIN - rect.top, 0);
      if (over <= 1) break;
      const panelH = panel.getBoundingClientRect().height;
      panel.style.maxHeight = Math.max(60, (panelH - over) / scale) + "px";
    }
  }
}

// Undo cbAlignDeckTop, handing positioning back to reveal's centering.
function cbRestoreDeck() {
  const slides = document.querySelector(".reveal .slides");
  if (!slides) return;
  slides.style.top = "";
  slides.style.transformOrigin = "";
  Reveal.layout();
}

// Lay out the active slide: top-align + fill when a solution is open, else hand
// back to reveal's centering. Safe to call on every ready/slidechanged/resize.
function cbLayoutSlide() {
  const reveal = cbReveal();
  if (!reveal) return;
  const slide = reveal.getCurrentSlide();
  if (!slide) return;
  const hasOpen = [...slide.querySelectorAll(".solucion-cuerpo")].some(
    (p) => !p.hasAttribute("hidden"),
  );
  if (!hasOpen) {
    cbRestoreDeck();
    return;
  }
  cbAlignDeckTop();
  cbFitSlideSolutions(slide);
}

// Solution toggles: each .solucion block holds a button and a hidden panel.
// The button flips the panel's `hidden` attribute, and its own label.
document.addEventListener("DOMContentLoaded", () => {
  for (const block of document.querySelectorAll(".solucion")) {
    const button = block.querySelector(".solucion-toggle");
    const panel = block.querySelector(".solucion-cuerpo");
    if (!button || !panel) continue;
    button.addEventListener("click", () => {
      const reveal = panel.hasAttribute("hidden");
      if (reveal) {
        panel.removeAttribute("hidden");
      } else {
        panel.setAttribute("hidden", "");
      }
      button.setAttribute("aria-expanded", String(reveal));
      button.textContent = reveal ? "Ocultar solución" : "Ver solución";
      // Re-lay out the slide: top-align + fill when opening, restore on close.
      if (cbReveal()) requestAnimationFrame(cbLayoutSlide);
    });
  }

  if (window.Reveal && typeof Reveal.on === "function") {
    const relayout = () => requestAnimationFrame(cbLayoutSlide);
    Reveal.on("ready", relayout);
    Reveal.on("slidechanged", relayout);
    Reveal.on("resize", relayout);
  }
});

// Code-copy buttons: wrap every code fence (`<pre>` holding a `<code>`) in a
// positioned container and add a clipboard-icon button that copies the code.
const CB_CLIPBOARD_ICON =
  '<svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor"' +
  ' stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' +
  '<rect x="9" y="9" width="13" height="13" rx="2"/>' +
  '<path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>';
const CB_CHECK_ICON =
  '<svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor"' +
  ' stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' +
  '<path d="M20 6 9 17l-5-5"/></svg>';

document.addEventListener("DOMContentLoaded", () => {
  for (const pre of document.querySelectorAll("pre")) {
    const code = pre.querySelector("code");
    if (!code || pre.closest(".cb-code")) continue;

    const wrap = document.createElement("div");
    wrap.className = "cb-code";
    pre.parentNode.insertBefore(wrap, pre);
    wrap.appendChild(pre);

    const button = document.createElement("button");
    button.type = "button";
    button.className = "cb-copy";
    button.innerHTML = CB_CLIPBOARD_ICON;
    button.setAttribute("aria-label", "Copiar código");
    button.addEventListener("click", async () => {
      try {
        const text = code.textContent;
        if (navigator.clipboard && navigator.clipboard.writeText) {
          await navigator.clipboard.writeText(text);
        } else {
          const area = document.createElement("textarea");
          area.value = text;
          area.style.position = "fixed";
          area.style.opacity = "0";
          document.body.appendChild(area);
          area.select();
          document.execCommand("copy");
          document.body.removeChild(area);
        }
        button.innerHTML = CB_CHECK_ICON;
        button.classList.add("cb-copied");
        button.setAttribute("aria-label", "Código copiado");
      } catch {
        button.setAttribute("aria-label", "No se pudo copiar");
      }
      setTimeout(() => {
        button.innerHTML = CB_CLIPBOARD_ICON;
        button.classList.remove("cb-copied");
        button.setAttribute("aria-label", "Copiar código");
      }, 1500);
    });
    wrap.appendChild(button);
  }
});
