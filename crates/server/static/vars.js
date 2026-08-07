// Content variable tokens (`{%name%}`, rendered server-side as
// `<span class="cb-var" data-var="name">`): click-to-fill popover, page-wide
// substitution, and per-course persistence.
//
// Storage: localStorage key "cb-vars:<course-slug>", value a {name: value}
// JSON object. The course slug is the path segment right after "/courses/",
// read from location.pathname (same approach the guide page's own inline
// scripts use for their "cb-scroll:" / "cb-slide:" sessionStorage keys).
//
// Values are applied with textContent only — never innerHTML — so a stored
// value can never inject markup.

function cbVarsStorageKey() {
  const segments = location.pathname.split("/").filter(Boolean);
  const at = segments.indexOf("courses");
  const slug = at >= 0 && segments.length > at + 1 ? segments[at + 1] : "";
  return `cb-vars:${slug}`;
}

// Reads the stored values for the current course. A missing key, or JSON
// that fails to parse or isn't a plain object, both read as "no values yet"
// rather than throwing.
function cbVarsRead() {
  let raw;
  try {
    raw = localStorage.getItem(cbVarsStorageKey());
  } catch {
    return {};
  }
  if (!raw) return {};
  try {
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === "object" && !Array.isArray(parsed)
      ? parsed
      : {};
  } catch {
    return {};
  }
}

function cbVarsWrite(values) {
  try {
    localStorage.setItem(cbVarsStorageKey(), JSON.stringify(values));
  } catch {
    // Storage can be unavailable (private browsing, quota exceeded). The
    // edit still applies to the page in front of the user; it just won't
    // survive a reload.
  }
}

// Fills every token on the page from stored values, or restores the
// unfilled "<name>" display when a token has no stored value.
function cbVarsApply() {
  const values = cbVarsRead();
  for (const token of document.querySelectorAll(".cb-var[data-var]")) {
    const name = token.dataset.var;
    const value = values[name];
    if (value) {
      token.textContent = value;
      token.classList.add("cb-var-filled");
    } else {
      token.textContent = `<${name}>`;
      token.classList.remove("cb-var-filled");
    }
  }
}

window.cbVars = { apply: cbVarsApply };

// --- Popover: name label, value input, Guardar / Borrar ---

let cbVarsPopover = null;

function cbVarsClosePopover() {
  if (!cbVarsPopover) return;
  cbVarsPopover.remove();
  cbVarsPopover = null;
}

function cbVarsSave(name, rawValue) {
  const value = rawValue.trim();
  const values = cbVarsRead();
  if (value) {
    values[name] = value;
  } else {
    delete values[name];
  }
  cbVarsWrite(values);
  cbVarsApply();
  cbVarsClosePopover();
}

function cbVarsClear(name) {
  const values = cbVarsRead();
  delete values[name];
  cbVarsWrite(values);
  cbVarsApply();
  cbVarsClosePopover();
}

// Positions the popover just below the token, nudging it left when it would
// otherwise overflow the right edge of the viewport.
function cbVarsPlacePopover(popover, token) {
  const rect = token.getBoundingClientRect();
  const left = rect.left + window.scrollX;
  popover.style.top = `${rect.bottom + window.scrollY + 6}px`;
  popover.style.left = `${left}px`;
  const overflow =
    left + popover.offsetWidth - (window.scrollX + document.documentElement.clientWidth);
  if (overflow > 0) {
    popover.style.left = `${Math.max(8, left - overflow - 8)}px`;
  }
}

function cbVarsOpenPopover(token) {
  cbVarsClosePopover();

  const name = token.dataset.var;
  const values = cbVarsRead();

  const popover = document.createElement("div");
  popover.className = "cb-var-popover";

  const label = document.createElement("div");
  label.className = "cb-var-popover-label";
  label.textContent = name;
  popover.appendChild(label);

  const input = document.createElement("input");
  input.type = "text";
  input.className = "cb-var-popover-input";
  input.value = values[name] || "";
  input.addEventListener("keydown", (event) => {
    if (event.key === "Enter") {
      event.preventDefault();
      cbVarsSave(name, input.value);
    }
  });
  popover.appendChild(input);

  const actions = document.createElement("div");
  actions.className = "cb-var-popover-actions";

  const save = document.createElement("button");
  save.type = "button";
  save.className = "cb-var-popover-save";
  save.textContent = "Guardar";
  save.addEventListener("click", () => cbVarsSave(name, input.value));
  actions.appendChild(save);

  const clear = document.createElement("button");
  clear.type = "button";
  clear.className = "cb-var-popover-clear";
  clear.textContent = "Borrar";
  clear.addEventListener("click", () => cbVarsClear(name));
  actions.appendChild(clear);

  popover.appendChild(actions);
  document.body.appendChild(popover);
  cbVarsPlacePopover(popover, token);
  input.focus();
  input.select();

  cbVarsPopover = popover;
}

// One delegate handles opening a popover on a token click, switching to a
// different token, and closing on a click outside the popover — all from a
// single listener, since a click can only ever be one of those three things.
function cbVarsOnDocumentClick(event) {
  const token = event.target.closest(".cb-var");
  if (token) {
    cbVarsOpenPopover(token);
    return;
  }
  if (cbVarsPopover && !cbVarsPopover.contains(event.target)) {
    cbVarsClosePopover();
  }
}

function cbVarsOnDocumentKeydown(event) {
  if (event.key === "Escape") cbVarsClosePopover();
}

document.addEventListener("DOMContentLoaded", () => {
  cbVarsApply();

  // Slide decks are display-only: reveal.js owns the page, and editing UI
  // has no home there.
  if (window.Reveal) return;

  document.addEventListener("click", cbVarsOnDocumentClick);
  document.addEventListener("keydown", cbVarsOnDocumentKeydown);
});

// The storage event fires only in *other* tabs than the one that wrote the
// value, which is exactly what live-syncs an already-open guide tab and an
// already-open deck tab against each other with no polling. Re-reading and
// re-applying is enough: cbVarsApply() queries the DOM fresh each call, so
// it finds whatever tokens the current page has, guide or deck.
window.addEventListener("storage", (event) => {
  if (event.key !== cbVarsStorageKey()) return;
  cbVarsApply();
});
