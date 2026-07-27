// Dev-mode hot reload client. Injected into every page only while the server
// runs with CB_DEV_ROOT set; it is never present on a production page.
//
// The server pushes one message per successful or failed content reload. The
// page then reloads, restoring the scroll position it held beforehand, so
// editing a section halfway down a long guide does not send you back to the top.
(function () {
  "use strict";

  const SCROLL_KEY = "cb-dev-scroll";

  // Restore the position saved just before the previous reload. Only this
  // script ever writes the key, so ordinary navigation is unaffected.
  const saved = sessionStorage.getItem(SCROLL_KEY);
  if (saved !== null) {
    sessionStorage.removeItem(SCROLL_KEY);
    const y = Number(saved);
    if (Number.isFinite(y)) {
      const restore = () => {
        // Guides set `scroll-behavior: smooth`, which would animate the jump
        // and lose the target when later content shifts the page height.
        const previous = document.documentElement.style.scrollBehavior;
        document.documentElement.style.scrollBehavior = "auto";
        window.scrollTo(0, y);
        document.documentElement.style.scrollBehavior = previous;
      };
      if (document.readyState === "loading") {
        document.addEventListener("DOMContentLoaded", restore);
      } else {
        restore();
      }
      // Images and mermaid diagrams settle after DOMContentLoaded and grow the
      // page, so the first restore can clamp short. Repeat once on load.
      window.addEventListener("load", restore);
    }
  }

  // EventSource reconnects on its own, so recompiling the server does not leave
  // a dead client. Reloading only on a message, never on a reconnect, keeps a
  // restarting server from driving a reload loop.
  const source = new EventSource("/dev/reload");
  source.onmessage = () => {
    sessionStorage.setItem(SCROLL_KEY, String(window.scrollY));
    location.reload();
  };
})();
