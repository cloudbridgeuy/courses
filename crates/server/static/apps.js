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
  // Lock state — when the server gates handlers and no secret is stored, app
  // widgets render dimmed with a lock symbol. Clicking a locked widget opens
  // the unlock modal.
  // ---------------------------------------------------------------------------

  var gated = false;            // server reports CB_APPS_SECRET is set
  var lockables = [];           // registered app host elements

  function isLocked() {
    return gated && !getSecret();
  }

  function applyLockTo(el) {
    el.classList.toggle("cb-app-locked", isLocked());
  }

  function applyLockState() {
    lockables.forEach(applyLockTo);
  }

  function registerLockable(el) {
    if (lockables.indexOf(el) === -1) lockables.push(el);
    applyLockTo(el);
  }

  function unregisterLockable(el) {
    var idx = lockables.indexOf(el);
    if (idx !== -1) lockables.splice(idx, 1);
  }

  // Delegated, capture-phase: a click anywhere inside a locked widget opens the
  // modal instead of reaching the widget's own handlers.
  document.addEventListener(
    "click",
    function (e) {
      if (!isLocked()) return;
      var locked = e.target.closest && e.target.closest(".cb-app-locked");
      if (locked) {
        e.preventDefault();
        e.stopPropagation();
        openUnlockModal();
      }
    },
    true
  );

  // ---------------------------------------------------------------------------
  // Unlock modal — centered overlay letting an instructor paste the secret
  // ---------------------------------------------------------------------------

  function buildUnlockModal() {
    if (document.getElementById("cb-unlock-overlay")) return;

    var overlay = document.createElement("div");
    overlay.id = "cb-unlock-overlay";
    overlay.hidden = true;
    overlay.innerHTML =
      '<div id="cb-unlock-modal" role="dialog" aria-modal="true" aria-label="Acceso instructor">' +
        '<button type="button" id="cb-unlock-close" aria-label="Cerrar">×</button>' +
        '<h3 id="cb-unlock-title">🔒 Contenido bloqueado</h3>' +
        '<p id="cb-unlock-desc">Ingresa el secreto de instructor para habilitar las apps.</p>' +
        '<label for="cb-unlock-input">Secreto:</label>' +
        '<input id="cb-unlock-input" type="password" placeholder="Ingresa el secreto" autocomplete="off">' +
        '<div id="cb-unlock-actions">' +
          '<button type="button" id="cb-unlock-save">Guardar</button>' +
          '<button type="button" id="cb-unlock-clear">Limpiar</button>' +
        '</div>' +
        '<span id="cb-unlock-status"></span>' +
      '</div>';
    document.body.appendChild(overlay);

    var modal    = document.getElementById("cb-unlock-modal");
    var closeBtn = document.getElementById("cb-unlock-close");
    var input    = document.getElementById("cb-unlock-input");
    var saveBtn  = document.getElementById("cb-unlock-save");
    var clearBtn = document.getElementById("cb-unlock-clear");
    var status   = document.getElementById("cb-unlock-status");

    saveBtn.addEventListener("click", function () {
      var v = input.value.trim();
      if (!v) {
        setSecret("");
        applyLockState();
        status.textContent = "Secreto borrado";
        return;
      }
      // Validate against the server before storing — a wrong value must not unlock.
      saveBtn.disabled = true;
      status.textContent = "Verificando…";
      fetch("/events/verify?secret=" + encodeURIComponent(v))
        .then(function (r) {
          saveBtn.disabled = false;
          if (r.status === 204) {
            setSecret(v);
            applyLockState();
            status.textContent = "✓ Secreto activo";
            closeUnlockModal();
          } else {
            setSecret("");
            applyLockState();
            status.textContent = "✗ Secreto incorrecto";
          }
        })
        .catch(function () {
          saveBtn.disabled = false;
          status.textContent = "✗ Error de red";
        });
    });

    clearBtn.addEventListener("click", function () {
      input.value = "";
      setSecret("");
      applyLockState();
      status.textContent = "Secreto borrado";
    });

    input.addEventListener("keydown", function (e) {
      if (e.key === "Enter") saveBtn.click();
      if (e.key === "Escape") closeUnlockModal();
    });

    closeBtn.addEventListener("click", closeUnlockModal);
    overlay.addEventListener("click", function (e) {
      if (e.target === overlay) closeUnlockModal();
    });
    modal.addEventListener("click", function (e) {
      e.stopPropagation();
    });
  }

  function openUnlockModal() {
    buildUnlockModal();
    var overlay = document.getElementById("cb-unlock-overlay");
    var input   = document.getElementById("cb-unlock-input");
    var status  = document.getElementById("cb-unlock-status");
    input.value = getSecret();
    status.textContent = getSecret() ? "✓ Secreto activo" : "";
    overlay.hidden = false;
    input.focus();
  }

  function closeUnlockModal() {
    var overlay = document.getElementById("cb-unlock-overlay");
    if (overlay) overlay.hidden = true;
  }

  // Re-lock and prompt when the server rejects a stored secret as wrong.
  function handleForbidden() {
    setSecret("");
    applyLockState();
    openUnlockModal();
    var status = document.getElementById("cb-unlock-status");
    if (status) status.textContent = "✗ Secreto incorrecto";
  }

  // Validates a stored secret against the server; clears it if rejected.
  // Resolves once lock state can be applied authoritatively.
  function validateStoredSecret() {
    var stored = getSecret();
    if (!stored) return Promise.resolve();
    return fetch("/events/verify?secret=" + encodeURIComponent(stored))
      .then(function (r) {
        if (r.status !== 204) setSecret("");
      })
      .catch(function () {
        // On a network error, fail closed: drop the unverified secret.
        setSecret("");
      });
  }

  // Learn whether handlers are gated, drop any stale/wrong stored secret, then
  // reflect lock state on any registered widgets.
  fetch("/events/config")
    .then(function (r) { return r.ok ? r.json() : null; })
    .then(function (cfg) {
      gated = !!(cfg && cfg.gated);
      if (!gated) return applyLockState();
      return validateStoredSecret().then(applyLockState);
    })
    .catch(function () {});

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
                  this._status.textContent = "";
                  this._btn.disabled = false;
                  handleForbidden();
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

        registerLockable(this);

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
        unregisterLockable(this);
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
          // mode: "increment" (button only) | "view" (value only) | "both" (default).
          // Multiple elements sharing a key stay in sync via the SSE bus, so an
          // incrementer and a viewer can live in different parts of the page.
          this._mode = this.getAttribute("mode") || "both";
          var showButton = this._mode !== "view";
          var showValue = this._mode !== "increment";
          var label = this.getAttribute("label") || "Incrementar contador";

          if (showButton) {
            this._btn = makeAppBtn(label);
            this.appendChild(this._btn);
          }
          if (showValue) {
            this._valueDisplay = document.createElement("span");
            this._valueDisplay.className = "cb-app-value";
            this._valueDisplay.textContent = "0";
            this.appendChild(this._valueDisplay);
          }

          // Load initial value from state for any value-showing element.
          if (this._key && this._valueDisplay) {
            cbState.read("counters", this._key).then((item) => {
              if (item && item.value !== undefined) {
                this._valueDisplay.textContent = item.value;
              }
            });
          }

          if (this._btn) {
            // Only interactive (emitting) counters get locked; a view-only
            // counter reads the open /state endpoint and stays visible.
            registerLockable(this);
            this._btn.addEventListener("click", () => {
              var id = uuid();
              cbEvents
                .emit(id, "counter", JSON.stringify({ key: this._key }))
                .then((code) => {
                  if (code === 403) handleForbidden();
                })
                .catch(function () {});
            });
          }
        }

        // Only value-showing elements need to track updates.
        if (this._valueDisplay) {
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
      }

      disconnectedCallback() {
        unregisterLockable(this);
        if (!this._statusListener) return;
        var idx = appStatusListeners.indexOf(this._statusListener);
        if (idx !== -1) appStatusListeners.splice(idx, 1);
        this._statusListener = null;
      }
    }
  );

  // ---------------------------------------------------------------------------
  // Custom element: <cb-metric>
  // Attributes: mode ("emf" | "api"), label
  // ---------------------------------------------------------------------------

  customElements.define(
    "cb-metric",
    class extends HTMLElement {
      connectedCallback() {
        if (!this._rendered) {
          this._rendered = true;

          this._method = this.getAttribute("mode") === "api" ? "api" : "emf";
          var label = this.getAttribute("label") || "Enviar métrica";

          this._input = document.createElement("input");
          this._input.type = "number";
          this._input.min = "0";
          this._input.max = "100";
          this._input.step = "1";
          this._input.value = "42";
          this._input.className = "cb-app-input";

          this._btn = makeAppBtn(label);
          this._status = document.createElement("span");
          this._status.className = "cb-app-status";

          this.appendChild(this._input);
          this.appendChild(this._btn);
          this.appendChild(this._status);

          this._btn.addEventListener("click", () => {
            var n = Math.round(Number(this._input.value));
            if (!isFinite(n)) {
              this._status.textContent = "Valor inválido";
              return;
            }
            n = Math.max(0, Math.min(100, n));
            var id = uuid();
            this._btn.disabled = true;
            this._status.textContent = "Enviando…";
            cbEvents
              .emit(
                id,
                "metric",
                JSON.stringify({ value: n, method: this._method })
              )
              .then((code) => {
                if (code === 202) {
                  this._status.textContent = "Enviado";
                } else if (code === 403) {
                  this._status.textContent = "";
                  this._btn.disabled = false;
                  handleForbidden();
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

        registerLockable(this);

        this._statusListener = (envelope) => {
          if (
            typeof envelope.id === "string" &&
            envelope.id.indexOf("status-metric-submitted-" + this._method) === 0
          ) {
            this._status.textContent = envelope.payload || "";
            this._btn.disabled = false;
          }
        };
        appStatusListeners.push(this._statusListener);
      }

      disconnectedCallback() {
        unregisterLockable(this);
        var idx = appStatusListeners.indexOf(this._statusListener);
        if (idx !== -1) appStatusListeners.splice(idx, 1);
        this._statusListener = null;
      }
    }
  );

  // ---------------------------------------------------------------------------
  // Custom element: <cb-toast-demo>
  // Attributes: label
  // Fire-and-forget: the server broadcasts a random demo notification, so the
  // feedback is the toast itself appearing — no app-status listener needed.
  // ---------------------------------------------------------------------------

  customElements.define(
    "cb-toast-demo",
    class extends HTMLElement {
      connectedCallback() {
        if (!this._rendered) {
          this._rendered = true;

          var label = this.getAttribute("label") || "Mostrar un aviso de ejemplo";

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
              .emit(id, "toast-demo", "{}")
              .then((code) => {
                this._btn.disabled = false;
                if (code === 202) {
                  this._status.textContent = "Aviso enviado";
                } else if (code === 403) {
                  this._status.textContent = "";
                  handleForbidden();
                } else {
                  this._status.textContent = "Error (" + code + ")";
                }
              })
              .catch(() => {
                this._btn.disabled = false;
                this._status.textContent = "Error de red";
              });
          });
        }

        registerLockable(this);
      }

      disconnectedCallback() {
        unregisterLockable(this);
      }
    }
  );
})();
