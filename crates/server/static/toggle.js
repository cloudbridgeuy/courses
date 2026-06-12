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
    });
  }
});
