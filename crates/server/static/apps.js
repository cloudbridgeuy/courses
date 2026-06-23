// Interactive-app widgets bundle. Loaded on pages that contain :::app blocks.
// Exposes globals: cbEvents (emit), cbState (read).
// Folds the notifications SSE toast client (formerly notifications.js).
//
// sessionStorage key: "cb-apps-secret"
//   Stores the instructor unlock secret used when gating is active on /events.
(function () {
  "use strict";

  // ---------------------------------------------------------------------------
  // Unlock secret — stored in sessionStorage under "cb-apps-secret"
  // ---------------------------------------------------------------------------

  var SECRET_KEY = "cb-apps-secret";

  function getSecret() {
    try {
      return sessionStorage.getItem(SECRET_KEY) || "";
    } catch (_) {
      return "";
    }
  }

  function setSecret(value) {
    try {
      sessionStorage.setItem(SECRET_KEY, value);
    } catch (_) {}
  }

  // ---------------------------------------------------------------------------
  // cbEvents.emit — POST /events with optional secret
  // ---------------------------------------------------------------------------

  window.cbEvents = {
    // id: string, type: string, payload: string (caller pre-stringifies)
    // Returns: Promise<number> (HTTP status code)
    emit: function (id, type, payload) {
      var url = "/events";
      var secret = getSecret();
      if (secret) {
        url += "?secret=" + encodeURIComponent(secret);
      }
      return fetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: id, type: type, payload: payload }),
      }).then(function (r) {
        return r.status;
      });
    },
  };

  // ---------------------------------------------------------------------------
  // cbState.read — GET /state/{collection}/{key}
  // ---------------------------------------------------------------------------

  window.cbState = {
    // Returns: Promise<any|null>
    read: function (collection, key) {
      return fetch(
        "/state/" + encodeURIComponent(collection) + "/" + encodeURIComponent(key)
      ).then(function (r) {
        if (r.status === 200) return r.json();
        if (r.status !== 404) {
          console.warn("cb apps: cbState.read returned", r.status, "for", collection + "/" + key);
        }
        return null;
      });
    },
  };

  // ---------------------------------------------------------------------------
  // App-status listener registry
  // ---------------------------------------------------------------------------

  var appStatusListeners = [];

  // ---------------------------------------------------------------------------
  // Toast notifications (ported from notifications.js)
  // ---------------------------------------------------------------------------

  var toastContainer;

  function stage() {
    if (toastContainer) return toastContainer;
    toastContainer = document.createElement("div");
    toastContainer.className = "cb-toasts";
    toastContainer.setAttribute("aria-live", "polite");
    document.body.appendChild(toastContainer);
    return toastContainer;
  }

  function statusClass(state) {
    var s = (state || "").toUpperCase();
    if (s.indexOf("FAIL") >= 0 || s.indexOf("ERROR") >= 0) return "cb-toast-fail";
    if (s.indexOf("SUCC") >= 0) return "cb-toast-ok";
    return "cb-toast-info";
  }

  function showToast(n) {
    var el = document.createElement("div");
    el.className = "cb-toast " + statusClass(n.state);

    var pod = document.createElement("strong");
    pod.className = "cb-toast-pod";
    pod.textContent = "Pod de " + (n.pod || "desconocido");

    var body = document.createElement("span");
    body.className = "cb-toast-body";
    var parts = [];
    if (n.detail) parts.push(n.detail);
    if (n.state) parts.push(n.state);
    if (n.source) parts.push("(" + n.source + ")");
    body.textContent = parts.join(" · ");

    el.appendChild(pod);
    el.appendChild(body);
    stage().appendChild(el);

    requestAnimationFrame(function () {
      el.classList.add("cb-toast-in");
    });
    var timer = setTimeout(dismiss, 7000);
    el.addEventListener("click", dismiss);
    function dismiss() {
      clearTimeout(timer);
      el.classList.remove("cb-toast-in");
      setTimeout(function () {
        if (el.parentNode) el.parentNode.removeChild(el);
      }, 300);
    }
  }

  // ---------------------------------------------------------------------------
  // Single EventSource multiplexer
  // ---------------------------------------------------------------------------

  // ---------------------------------------------------------------------------
  // uuid() helper — uses crypto.randomUUID when available (secure contexts),
  // falls back to crypto.getRandomValues or Math.random for plain-HTTP use.
  // ---------------------------------------------------------------------------

  function uuid() {
    if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
      return crypto.randomUUID();
    }
    // RFC4122 v4 using getRandomValues if available, else Math.random
    var bytes = new Uint8Array(16);
    if (typeof crypto !== "undefined" && typeof crypto.getRandomValues === "function") {
      crypto.getRandomValues(bytes);
    } else {
      for (var i = 0; i < 16; i++) bytes[i] = Math.floor(Math.random() * 256);
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    var hex = Array.from(bytes).map(function (b) {
      return ("0" + b.toString(16)).slice(-2);
    });
    return (
      hex.slice(0, 4).join("") + "-" +
      hex.slice(4, 6).join("") + "-" +
      hex.slice(6, 8).join("") + "-" +
      hex.slice(8, 10).join("") + "-" +
      hex.slice(10, 16).join("")
    );
  }

  if (typeof EventSource !== "undefined") {
    var source = new EventSource("/events/stream");
    source.onerror = function () {
      console.warn("cb apps: SSE connection error; browser will retry");
    };
    source.onmessage = function (e) {
      var envelope;
      try {
        envelope = JSON.parse(e.data);
      } catch (_) {
        return;
      }
      if (!envelope || typeof envelope.type !== "string") return;

      if (envelope.type === "notification") {
        var n;
        try {
          // payload is double-encoded: the Event carries a JSON string whose
          // value is itself a JSON-encoded Notification object.
          n = JSON.parse(envelope.payload);
        } catch (_) {
          return;
        }
        showToast(n);
      } else if (envelope.type === "app-status") {
        appStatusListeners.forEach(function (fn) { try { fn(envelope); } catch (_) {} });
      }
      // unknown types: ignore
    };
  }

  // ---------------------------------------------------------------------------
  // Unlock UI — small fixed panel letting an instructor paste the secret
  // ---------------------------------------------------------------------------

  function insertUnlockControl() {
    if (document.getElementById("cb-unlock-panel")) return;

    var panel = document.createElement("div");
    panel.id = "cb-unlock-panel";
    panel.setAttribute("aria-label", "Panel de acceso instructor");
    panel.innerHTML =
      '<button id="cb-unlock-toggle" type="button" aria-label="Configurar secreto de instructor">🔑</button>' +
      '<div id="cb-unlock-form" hidden>' +
        '<label for="cb-unlock-input">Secreto:</label>' +
        '<input id="cb-unlock-input" type="password" placeholder="Ingresa el secreto" autocomplete="off">' +
        '<button type="button" id="cb-unlock-save">Guardar</button>' +
        '<button type="button" id="cb-unlock-clear">Limpiar</button>' +
        '<span id="cb-unlock-status"></span>' +
      '</div>';
    document.body.appendChild(panel);

    var toggleBtn = document.getElementById("cb-unlock-toggle");
    var form      = document.getElementById("cb-unlock-form");
    var input     = document.getElementById("cb-unlock-input");
    var saveBtn   = document.getElementById("cb-unlock-save");
    var clearBtn  = document.getElementById("cb-unlock-clear");
    var status    = document.getElementById("cb-unlock-status");

    input.value = getSecret();
    status.textContent = getSecret() ? "✓ Secreto activo" : "";

    toggleBtn.addEventListener("click", function () {
      form.hidden = !form.hidden;
      if (!form.hidden) input.focus();
    });

    saveBtn.addEventListener("click", function () {
      var v = input.value.trim();
      setSecret(v);
      status.textContent = v ? "✓ Secreto activo" : "Secreto borrado";
      form.hidden = true;
    });

    clearBtn.addEventListener("click", function () {
      input.value = "";
      setSecret("");
      status.textContent = "Secreto borrado";
    });

    input.addEventListener("keydown", function (e) {
      if (e.key === "Enter") saveBtn.click();
      if (e.key === "Escape") {
        form.hidden = true;
      }
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", insertUnlockControl);
  } else {
    insertUnlockControl();
  }

  // ---------------------------------------------------------------------------
  // Shared helper: create the standard app button
  // ---------------------------------------------------------------------------

  function makeAppBtn(label) {
    var btn = document.createElement("button");
    btn.type = "button";
    btn.className = "cb-app-btn";
    btn.textContent = label;
    return btn;
  }

  // ---------------------------------------------------------------------------
  // Custom element: <cb-cpu-burst>
  // Attributes: seconds, intensity, label
  // ---------------------------------------------------------------------------

  customElements.define(
    "cb-cpu-burst",
    class extends HTMLElement {
      connectedCallback() {
        if (!this._rendered) {
          this._rendered = true;

          var seconds = this.getAttribute("seconds") || "30";
          var intensity = this.getAttribute("intensity") || "medium";
          var label = this.getAttribute("label") || "Generar carga de CPU";

          this._btn = makeAppBtn(label);
          this._status = document.createElement("span");
          this._status.className = "cb-app-status";

          this.appendChild(this._btn);
          this.appendChild(this._status);

          this._btn.addEventListener("click", () => {
            var id = uuid();
            this._btn.disabled = true;
            this._status.textContent = "Enviando…";
            cbEvents
              .emit(
                id,
                "cpu-burst",
                JSON.stringify({ seconds: Number(seconds), intensity: intensity })
              )
              .then((code) => {
                if (code === 202) {
                  this._status.textContent = "En curso…";
                } else if (code === 403) {
                  this._status.textContent = "🔒 Bloqueado: ingresa el secreto";
                  this._btn.disabled = false;
                } else {
                  this._status.textContent = "Error (" + code + ")";
                  this._btn.disabled = false;
                }
              })
              .catch(() => {
                this._status.textContent = "Error de red";
                this._btn.disabled = false;
              });
          });
        }

        this._statusListener = (envelope) => {
          if (
            typeof envelope.id === "string" &&
            envelope.id.indexOf("status-cpu-burst") === 0
          ) {
            this._status.textContent = envelope.payload || "";
            this._btn.disabled = false;
          }
        };
        appStatusListeners.push(this._statusListener);
      }

      disconnectedCallback() {
        var idx = appStatusListeners.indexOf(this._statusListener);
        if (idx !== -1) appStatusListeners.splice(idx, 1);
        this._statusListener = null;
      }
    }
  );

  // ---------------------------------------------------------------------------
  // Custom element: <cb-counter>
  // Attributes: key, label
  // ---------------------------------------------------------------------------

  customElements.define(
    "cb-counter",
    class extends HTMLElement {
      connectedCallback() {
        if (!this._rendered) {
          this._rendered = true;

          this._key = this.getAttribute("key") || "";
          var label = this.getAttribute("label") || "Incrementar contador";

          this._btn = makeAppBtn(label);
          this._valueDisplay = document.createElement("span");
          this._valueDisplay.className = "cb-app-value";
          this._valueDisplay.textContent = "0";

          this.appendChild(this._btn);
          this.appendChild(this._valueDisplay);

          // Load initial value from state
          if (this._key) {
            cbState.read("counters", this._key).then((item) => {
              if (item && item.value !== undefined) {
                this._valueDisplay.textContent = item.value;
              }
            });
          }

          this._btn.addEventListener("click", () => {
            var id = uuid();
            cbEvents
              .emit(id, "counter", JSON.stringify({ key: this._key }))
              .then((code) => {
                if (code === 403) {
                  this._valueDisplay.textContent = "🔒 Bloqueado: ingresa el secreto";
                }
              })
              .catch(function () {});
          });
        }

        this._statusListener = (envelope) => {
          if (envelope.id !== "status-counter-updated") return;
          var parsed;
          try {
            parsed = JSON.parse(envelope.payload);
          } catch (_) {
            return;
          }
          if (parsed && parsed.key === this._key) {
            this._valueDisplay.textContent = parsed.value;
          }
        };
        appStatusListeners.push(this._statusListener);
      }

      disconnectedCallback() {
        var idx = appStatusListeners.indexOf(this._statusListener);
        if (idx !== -1) appStatusListeners.splice(idx, 1);
        this._statusListener = null;
      }
    }
  );
})();
