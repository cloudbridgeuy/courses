// Mermaid initializer for both guide pages and slide decks.
//
// Guide: loaded with `defer` after mermaid.min.js, so the DOM is parsed and the
// global `mermaid` exists; `window.Reveal` is undefined, so it renders at once.
// Slides: loaded before the inline Reveal.initialize call, so it registers a
// 'ready' handler that renders once the (scaled) deck exists.
//
// After rendering, each diagram gets a maximize button that opens it in a
// full-viewport overlay (click backdrop or Escape to close).
(function () {
  if (typeof mermaid === "undefined") return;

  mermaid.initialize({
    startOnLoad: false,
    securityLevel: "loose",
    theme: "neutral",
  });

  // Render the unprocessed diagrams within `scope` (a slide section, or the
  // whole document on guide pages). Rendering only visible nodes matters on
  // slides: mermaid measures text geometry, and a hidden reveal.js slide yields
  // NaN transforms — so each slide's diagrams render when it becomes active.
  function runIn(scope) {
    var nodes = (scope || document).querySelectorAll(
      ".mermaid:not([data-processed])",
    );
    if (!nodes.length) return;
    try {
      var result = mermaid.run({ nodes: nodes });
      if (result && typeof result.then === "function") {
        result.then(setupZoom).catch(setupZoom);
      } else {
        setupZoom();
      }
    } catch (e) {
      console.error("mermaid render failed", e);
    }
  }

  // ── Maximize / lightbox ────────────────────────────────────────────────
  var overlay;

  function ensureOverlay() {
    if (overlay) return overlay;
    overlay = document.createElement("div");
    overlay.className = "cb-diagram-overlay";
    overlay.innerHTML = '<div class="cb-diagram-stage"></div>';
    overlay.addEventListener("click", closeOverlay);
    document.addEventListener("keydown", function (e) {
      if (e.key === "Escape") closeOverlay();
    });
    document.body.appendChild(overlay);
    return overlay;
  }

  function closeOverlay() {
    if (!overlay) return;
    overlay.classList.remove("cb-open");
    var stage = overlay.querySelector(".cb-diagram-stage");
    if (stage) stage.innerHTML = "";
  }

  function openOverlay(svg) {
    var ov = ensureOverlay();
    var stage = ov.querySelector(".cb-diagram-stage");
    stage.innerHTML = "";
    var clone = svg.cloneNode(true);
    clone.removeAttribute("width");
    clone.removeAttribute("height");
    // Mermaid pins an inline `max-width: <n>px` on the SVG, which would cap the
    // blow-up. Clear it so the viewBox scales to fill the overlay (aspect ratio
    // preserved by the SVG's default preserveAspectRatio="xMidYMid meet").
    clone.style.maxWidth = "none";
    clone.style.maxHeight = "none";
    clone.style.width = "100%";
    clone.style.height = "100%";
    stage.appendChild(clone);
    ov.classList.add("cb-open");
  }

  function setupZoom() {
    var diagrams = document.querySelectorAll("pre.mermaid");
    for (var i = 0; i < diagrams.length; i++) {
      var pre = diagrams[i];
      if (pre.querySelector(".cb-diagram-zoom")) continue; // already wired
      var svg = pre.querySelector("svg");
      if (!svg) continue;
      var btn = document.createElement("button");
      btn.type = "button";
      btn.className = "cb-diagram-zoom";
      btn.setAttribute("aria-label", "Ampliar diagrama");
      btn.title = "Ampliar";
      btn.textContent = "⤢";
      btn.addEventListener("click", makeOpener(pre));
      pre.appendChild(btn);
    }
  }

  function makeOpener(pre) {
    return function (e) {
      e.stopPropagation();
      var svg = pre.querySelector("svg");
      if (svg) openOverlay(svg);
    };
  }

  // ── When to render ─────────────────────────────────────────────────────
  if (window.Reveal && typeof Reveal.on === "function") {
    // Render the visible slide first, then each slide as it becomes active.
    Reveal.on("ready", function (e) {
      runIn(e.currentSlide || document);
    });
    Reveal.on("slidechanged", function (e) {
      runIn(e.currentSlide);
    });
  } else if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", function () {
      runIn(document);
    });
  } else {
    runIn(document);
  }
})();
