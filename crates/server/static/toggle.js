// Guide/slides client behavior: solution toggles and code-copy buttons.

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
