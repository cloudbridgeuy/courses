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

  var TOAST_TTL_MS = 8000;
  var TOAST_MAX = 4;

  var toastContainer;
  // Live toasts, oldest first. Each entry: { key, el, count, badge, bump, dismiss }.
  var toasts = [];

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
    // CloudWatch alarm states, which share no vocabulary with the pipeline ones.
    if (s === "ALARM") return "cb-toast-fail";
    if (s === "OK") return "cb-toast-ok";
    if (s === "INSUFFICIENT_DATA") return "cb-toast-warn";
    if (s.indexOf("FAIL") >= 0 || s.indexOf("ERROR") >= 0) return "cb-toast-fail";
    if (s.indexOf("SUCC") >= 0) return "cb-toast-ok";
    if (s.indexOf("CANCEL") >= 0 || s.indexOf("STOP") >= 0 || s.indexOf("SUPERSEDED") >= 0) {
      return "cb-toast-warn";
    }
    return "cb-toast-info";
  }

  function statusGlyph(cls) {
    if (cls === "cb-toast-ok") return "✓";
    if (cls === "cb-toast-fail") return "✕";
    if (cls === "cb-toast-warn") return "!";
    return "●";
  }

  // aws.codepipeline → CodePipeline; anything unknown keeps its own shape.
  var SOURCE_LABELS = {
    "aws.codepipeline": "CodePipeline",
    "aws.codebuild": "CodeBuild",
    "aws.codecommit": "CodeCommit",
    "aws.codedeploy": "CodeDeploy",
    "aws.cloudformation": "CloudFormation",
    "aws.cloudwatch": "CloudWatch",
    "aws.ecs": "ECS",
    "aws.ecr": "ECR",
  };

  function sourceLabel(source) {
    var s = (source || "").trim();
    if (!s) return "";
    var known = SOURCE_LABELS[s.toLowerCase()];
    if (known) return known;
    if (s.toLowerCase().indexOf("aws.") === 0) s = s.slice(4);
    return s.charAt(0).toUpperCase() + s.slice(1);
  }

  // The event's own timestamp when it carries one, else arrival time.
  function clockLabel(stamp) {
    var d = stamp ? new Date(stamp) : new Date();
    if (isNaN(d.getTime())) d = new Date();
    function pad(n) {
      return n < 10 ? "0" + n : "" + n;
    }
    return pad(d.getHours()) + ":" + pad(d.getMinutes());
  }

  // The one-line answer to "what was this about": where it happened, then why.
  function contextLine(n) {
    var parts = [];
    if (n.stage) parts.push(n.stage);
    if (n.action) parts.push(n.action);
    if (n.phase) parts.push(n.phase);
    if (n.reason) parts.push(n.reason);
    return parts.join(" · ");
  }

  // Expanded rows, in reading order. Labels are guide-facing, so Spanish.
  var TOAST_FIELDS = [
    ["stage", "etapa"],
    ["action", "acción"],
    ["provider", "proveedor"],
    ["phase", "fase"],
    ["reason", "motivo"],
    ["execution", "ejecución"],
    ["region", "región"],
    ["time", "hora"],
    ["source", "origen"],
    ["pod", "pod"],
  ];

  function detailsPanel(n) {
    var panel = document.createElement("dl");
    panel.className = "cb-toast-details";
    TOAST_FIELDS.forEach(function (field) {
      var value = n[field[0]];
      if (!value) return;
      var dt = document.createElement("dt");
      dt.textContent = field[1];
      var dd = document.createElement("dd");
      dd.textContent = value;
      panel.appendChild(dt);
      panel.appendChild(dd);
    });
    return panel;
  }

  function showToast(n) {
    var cls = statusClass(n.state);
    var source = sourceLabel(n.source);
    var pod = n.pod || "desconocido";
    var title = n.detail || source || n.state || "Notificación";
    var context = contextLine(n);
    var key = pod + "|" + n.source + "|" + n.state + "|" + n.detail + "|" + context;

    // Repeated event while its toast is still up: bump the counter, don't stack.
    for (var i = 0; i < toasts.length; i++) {
      if (toasts[i].key === key) {
        toasts[i].bump();
        return;
      }
    }

    var el = document.createElement("div");
    el.className = "cb-toast " + cls;

    var icon = document.createElement("span");
    icon.className = "cb-toast-icon";
    icon.textContent = statusGlyph(cls);
    icon.setAttribute("aria-hidden", "true");

    var main = document.createElement("div");
    main.className = "cb-toast-main";

    var head = document.createElement("div");
    head.className = "cb-toast-head";

    // With a console link the subject becomes the link; the link only works for
    // whoever is signed into that pod's account, which is the owner.
    var titleEl = document.createElement(n.url ? "a" : "span");
    titleEl.className = "cb-toast-title";
    titleEl.textContent = title;
    titleEl.title = n.url ? title + " — abrir en la consola AWS" : title;
    if (n.url) {
      titleEl.href = n.url;
      titleEl.target = "_blank";
      titleEl.rel = "noopener noreferrer";
    }

    var count = document.createElement("span");
    count.className = "cb-toast-count";
    count.hidden = true;

    var time = document.createElement("span");
    time.className = "cb-toast-time";
    time.textContent = clockLabel(n.time);

    var close = document.createElement("button");
    close.className = "cb-toast-close";
    close.type = "button";
    close.textContent = "×";
    close.setAttribute("aria-label", "Cerrar aviso");

    head.appendChild(titleEl);
    head.appendChild(count);
    head.appendChild(time);
    head.appendChild(close);

    var meta = document.createElement("div");
    meta.className = "cb-toast-meta";

    if (n.state) {
      var badge = document.createElement("span");
      badge.className = "cb-toast-badge";
      badge.textContent = n.state;
      meta.appendChild(badge);
    }
    var where = document.createElement("span");
    where.className = "cb-toast-where";
    // The source is the title when nothing better was parsed; don't repeat it.
    var origin = source && source !== title ? source + " · " : "";
    where.textContent = origin + "pod " + pod;
    meta.appendChild(where);

    var contextEl;
    if (context) {
      contextEl = document.createElement("div");
      contextEl.className = "cb-toast-context";
      contextEl.textContent = context;
      contextEl.title = context;
    }

    var progress = document.createElement("span");
    progress.className = "cb-toast-progress";

    main.appendChild(head);
    main.appendChild(meta);
    if (contextEl) main.appendChild(contextEl);
    main.appendChild(detailsPanel(n));
    el.appendChild(icon);
    el.appendChild(main);
    el.appendChild(progress);
    stage().appendChild(el);

    var entry = { key: key, el: el, count: 1, bump: bump, dismiss: dismiss };
    toasts.push(entry);
    while (toasts.length > TOAST_MAX) toasts[0].dismiss();

    requestAnimationFrame(function () {
      el.classList.add("cb-toast-in");
    });
    var open = false;
    var timer = setTimeout(dismiss, TOAST_TTL_MS);
    close.addEventListener("click", function (ev) {
      ev.stopPropagation();
      dismiss();
    });
    // The card expands instead of dismissing; the link keeps its own click.
    el.addEventListener("click", function (ev) {
      if (ev.target.closest("a")) return;
      toggle();
    });

    function restart() {
      clearTimeout(timer);
      progress.classList.remove("cb-toast-progress-run");
      // Force a reflow so the countdown restarts from the top.
      void progress.offsetWidth;
      if (open) return;
      timer = setTimeout(dismiss, TOAST_TTL_MS);
      progress.classList.add("cb-toast-progress-run");
    }

    // Expanded toasts stay until dismissed: nobody can read a payload in 8 s.
    function toggle() {
      open = !open;
      el.classList.toggle("cb-toast-open", open);
      restart();
    }

    function bump() {
      entry.count += 1;
      count.hidden = false;
      count.textContent = "×" + entry.count;
      time.textContent = clockLabel(n.time);
      el.classList.remove("cb-toast-bump");
      void el.offsetWidth;
      el.classList.add("cb-toast-bump");
      restart();
    }

    function dismiss() {
      clearTimeout(timer);
      var at = toasts.indexOf(entry);
      if (at >= 0) toasts.splice(at, 1);
      el.classList.remove("cb-toast-in");
      setTimeout(function () {
        if (el.parentNode) el.parentNode.removeChild(el);
      }, 250);
    }

    restart();
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
  // Attributes: seconds (initial value of the duration field), intensity, label
  // ---------------------------------------------------------------------------

  // Mirrors MAX_BURST_SECONDS in courses_core::events; the server clamps too, so
  // this bound is only there to keep the field honest.
  var MAX_BURST_SECONDS = 120;

  var CPU_BURST_DESC =
    "Ejecuta bucles de cálculo en la tarea de ECS de tu pod durante el tiempo " +
    "indicado. La CPU sube en todos los núcleos disponibles, y la métrica " +
    "CPUUtilization de CloudWatch lo refleja uno o dos minutos después. La " +
    "carga se libera sola al terminar. El servidor limita la duración a " +
    MAX_BURST_SECONDS +
    " s.";

  customElements.define(
    "cb-cpu-burst",
    class extends HTMLElement {
      connectedCallback() {
        if (!this._rendered) {
          this._rendered = true;

          var seconds = this.getAttribute("seconds") || "30";
          var intensity = this.getAttribute("intensity") || "medium";
          var label = this.getAttribute("label") || "Generar carga de CPU";

          var desc = document.createElement("p");
          desc.className = "cb-app-desc";
          desc.textContent = CPU_BURST_DESC;

          this._seconds = document.createElement("input");
          this._seconds.type = "number";
          this._seconds.min = "1";
          this._seconds.max = String(MAX_BURST_SECONDS);
          this._seconds.step = "1";
          this._seconds.value = seconds;
          this._seconds.className = "cb-app-input";

          var field = document.createElement("label");
          field.className = "cb-app-label";
          field.appendChild(document.createTextNode("Duración (s)"));
          field.appendChild(this._seconds);

          this._btn = makeAppBtn(label);
          this._status = document.createElement("span");
          this._status.className = "cb-app-status";

          var controls = document.createElement("div");
          controls.className = "cb-app-controls";
          controls.appendChild(field);
          controls.appendChild(this._btn);

          this.appendChild(desc);
          this.appendChild(controls);
          this.appendChild(this._status);

          this._btn.addEventListener("click", () => {
            var n = Math.round(Number(this._seconds.value));
            if (!isFinite(n)) {
              this._status.textContent = "Duración inválida";
              return;
            }
            n = Math.max(1, Math.min(MAX_BURST_SECONDS, n));
            this._seconds.value = String(n);
            var id = uuid();
            this._btn.disabled = true;
            this._status.textContent = "Enviando…";
            cbEvents
              .emit(
                id,
                "cpu-burst",
                JSON.stringify({ seconds: n, intensity: intensity })
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
  // Attributes: mode ("emf" | "api"), label, interval (auto period, seconds)
  //
  // Two ways to publish: one shot with the value in the field, or the auto
  // button, which emits a random 0–100 value every `interval` seconds until it
  // is stopped. The auto run fills a CloudWatch graph with a series instead of
  // a single point, which is what a metric period or an alarm needs to react.
  // ---------------------------------------------------------------------------

  var METRIC_AUTO_SECONDS = 5;
  var MAX_METRIC_AUTO_SECONDS = 300;

  customElements.define(
    "cb-metric",
    class extends HTMLElement {
      connectedCallback() {
        if (!this._rendered) {
          this._rendered = true;

          this._method = this.getAttribute("mode") === "api" ? "api" : "emf";
          var interval = Math.round(Number(this.getAttribute("interval")));
          this._interval = isFinite(interval) && interval > 0
            ? Math.min(MAX_METRIC_AUTO_SECONDS, interval)
            : METRIC_AUTO_SECONDS;
          this._timer = null;
          this._left = 0;
          var label = this.getAttribute("label") || "Enviar métrica";

          this._input = document.createElement("input");
          this._input.type = "number";
          this._input.min = "0";
          this._input.max = "100";
          this._input.step = "1";
          this._input.value = "42";
          this._input.className = "cb-app-input";

          this._btn = makeAppBtn(label);

          // One toggle, two faces: ▶ starts the run, ⏸ stops it.
          this._autoBtn = makeAppBtn("");
          this._autoBtn.classList.add("cb-metric-auto");
          this._autoGlyph = document.createElement("span");
          this._autoGlyph.className = "cb-metric-glyph";
          this._autoGlyph.setAttribute("aria-hidden", "true");
          this._autoText = document.createElement("span");
          this._autoBtn.appendChild(this._autoGlyph);
          this._autoBtn.appendChild(this._autoText);

          // Only visible while the run is on: a pulsing dot and the countdown.
          this._live = document.createElement("span");
          this._live.className = "cb-metric-live";
          this._live.hidden = true;
          var dot = document.createElement("span");
          dot.className = "cb-metric-dot";
          dot.setAttribute("aria-hidden", "true");
          this._liveText = document.createElement("span");
          this._live.appendChild(dot);
          this._live.appendChild(this._liveText);

          this._status = document.createElement("span");
          this._status.className = "cb-app-status";

          var controls = document.createElement("div");
          controls.className = "cb-app-controls";
          controls.appendChild(this._input);
          controls.appendChild(this._btn);
          controls.appendChild(this._autoBtn);
          controls.appendChild(this._live);

          this.appendChild(controls);
          this.appendChild(this._status);

          this._paintAuto();

          this._btn.addEventListener("click", () => {
            var n = Math.round(Number(this._input.value));
            if (!isFinite(n)) {
              this._status.textContent = "Valor inválido";
              return;
            }
            this._send(Math.max(0, Math.min(100, n)), true);
          });

          this._autoBtn.addEventListener("click", () => {
            if (this._timer) this._stopAuto("Auto en pausa");
            else this._startAuto();
          });
        }

        registerLockable(this);

        this._statusListener = (envelope) => {
          if (
            typeof envelope.id === "string" &&
            envelope.id.indexOf("status-metric-submitted-" + this._method) === 0
          ) {
            // The server's own wording wins; the live chip carries the run state.
            this._status.textContent = envelope.payload || "";
            this._btn.disabled = false;
          }
        };
        appStatusListeners.push(this._statusListener);
      }

      disconnectedCallback() {
        this._stopAuto("");
        unregisterLockable(this);
        var idx = appStatusListeners.indexOf(this._statusListener);
        if (idx !== -1) appStatusListeners.splice(idx, 1);
        this._statusListener = null;
      }

      // Emits one value. A manual send holds its button until the server
      // answers on the bus; an auto send leaves the controls alone.
      _send(value, manual) {
        if (manual) this._btn.disabled = true;
        this._status.textContent = "Enviando " + value + "…";
        cbEvents
          .emit(
            uuid(),
            "metric",
            JSON.stringify({ value: value, method: this._method })
          )
          .then((code) => {
            if (code === 202) {
              this._status.textContent = "Enviado " + value;
            } else if (code === 403) {
              this._status.textContent = "";
              this._btn.disabled = false;
              this._stopAuto("");
              handleForbidden();
            } else {
              this._btn.disabled = false;
              this._stopAuto("");
              this._status.textContent = "Error (" + code + ")";
            }
          })
          .catch(() => {
            this._btn.disabled = false;
            this._stopAuto("");
            this._status.textContent = "Error de red";
          });
      }

      // Starts the repeating run. The first value goes out now, so the operator
      // sees an effect without waiting a full period. The ticker runs once a
      // second — the period is a countdown, which is what the chip shows.
      _startAuto() {
        if (this._timer) return;
        this._left = this._interval;
        this._fire();
        this._timer = setInterval(() => {
          this._left -= 1;
          if (this._left <= 0) {
            this._left = this._interval;
            this._fire();
          }
          this._paintAuto();
        }, 1000);
        this._paintAuto();
      }

      _fire() {
        var value = Math.floor(Math.random() * 101);
        this._input.value = String(value);
        this._send(value, false);
      }

      // `message` of "" stops without writing over whatever the caller shows.
      _stopAuto(message) {
        if (this._timer) {
          clearInterval(this._timer);
          this._timer = null;
        }
        this._paintAuto();
        if (message) this._status.textContent = message;
      }

      // Single place where the run state reaches the DOM: button face, the
      // pressed style, and the live chip with its countdown.
      _paintAuto() {
        var running = !!this._timer;
        this._autoGlyph.textContent = running ? "⏸" : "▶";
        this._autoText.textContent = running
          ? "Pausar"
          : "Auto (" + this._interval + " s)";
        this._autoBtn.title = running
          ? "Detener el envío automático"
          : "Envía un valor al azar (0–100) cada " +
            this._interval +
            " s hasta pausarlo";
        this._autoBtn.setAttribute("aria-pressed", String(running));
        this._autoBtn.classList.toggle("cb-app-btn-on", running);
        this._live.hidden = !running;
        if (running) {
          this._liveText.textContent = "Enviando · próximo en " + this._left + " s";
        }
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

  // ---------------------------------------------------------------------------
  // Custom element: <cb-health>
  // Attributes: dependency, seconds, label
  //
  // Two halves: a live board polling the three health endpoints, and a control
  // that breaks one dependency for a bounded time. Watching the board while the
  // outage runs is the whole point — readiness leaves rotation, liveness does
  // not, and the instance comes back on its own.
  // ---------------------------------------------------------------------------

  var HEALTH_CHECKS = [
    { key: "live", path: "/health/live" },
    { key: "ready", path: "/health/ready" },
    { key: "startup", path: "/health/startup" },
  ];

  var HEALTH_DEPENDENCIES = [
    { value: "dynamodb", label: "dynamodb (dura)" },
    { value: "content", label: "content (blanda)" },
  ];

  var HEALTH_POLL_MS = 1000;

  function healthCodeClass(code) {
    if (code === 200) return "cb-health-ok";
    if (code === 503) return "cb-health-fail";
    if (code === 404) return "cb-health-off";
    return "cb-health-unknown";
  }

  customElements.define(
    "cb-health",
    class extends HTMLElement {
      connectedCallback() {
        if (!this._rendered) {
          this._rendered = true;
          this._chips = {};
          this._until = 0;

          var dependency = this.getAttribute("dependency") || "dynamodb";
          var seconds = this.getAttribute("seconds") || "60";
          var label = this.getAttribute("label") || "Romper dependencia";

          // --- board: one chip per endpoint ---
          var board = document.createElement("div");
          board.className = "cb-health-board";
          HEALTH_CHECKS.forEach((check) => {
            var chip = document.createElement("span");
            chip.className = "cb-health-chip cb-health-unknown";
            var name = document.createElement("code");
            name.textContent = check.path;
            var code = document.createElement("b");
            code.textContent = "…";
            chip.appendChild(name);
            chip.appendChild(code);
            board.appendChild(chip);
            this._chips[check.key] = { chip: chip, code: code };
          });

          this._detail = document.createElement("div");
          this._detail.className = "cb-health-detail";

          // --- controls ---
          var controls = document.createElement("div");
          controls.className = "cb-health-controls";

          this._select = document.createElement("select");
          this._select.className = "cb-app-input cb-health-select";
          HEALTH_DEPENDENCIES.forEach(function (dep) {
            var option = document.createElement("option");
            option.value = dep.value;
            option.textContent = dep.label;
            this._select.appendChild(option);
          }, this);
          this._select.value = dependency;

          this._seconds = document.createElement("input");
          this._seconds.className = "cb-app-input cb-health-seconds";
          this._seconds.type = "number";
          this._seconds.min = "1";
          this._seconds.max = "600";
          this._seconds.value = seconds;

          this._break = makeAppBtn(label);
          this._restore = makeAppBtn("Restaurar");
          this._restore.classList.add("cb-health-restore");

          controls.appendChild(this._select);
          controls.appendChild(this._seconds);
          controls.appendChild(this._break);
          controls.appendChild(this._restore);

          this._status = document.createElement("span");
          this._status.className = "cb-app-status";

          this.appendChild(board);
          this.appendChild(this._detail);
          this.appendChild(controls);
          this.appendChild(this._status);

          this._break.addEventListener("click", () => {
            var requested = Number(this._seconds.value) || 60;
            this._send(requested, "Rompiendo…");
          });
          this._restore.addEventListener("click", () => {
            // Zero seconds is the restore command.
            this._send(0, "Restaurando…");
          });
        }

        registerLockable(this);

        this._statusListener = (envelope) => {
          if (envelope.id !== "status-health-fault") return;
          var status;
          try {
            status = JSON.parse(envelope.payload);
          } catch (_) {
            return;
          }
          if (status.state === "broken") {
            this._until = Date.now() + Number(status.seconds || 0) * 1000;
            this._status.textContent =
              status.dependency + ": en falla por " + status.seconds + "s";
          } else {
            this._until = 0;
            this._status.textContent = status.dependency + ": restaurada";
          }
        };
        appStatusListeners.push(this._statusListener);

        this._poll();
        this._timer = setInterval(() => this._poll(), HEALTH_POLL_MS);
      }

      disconnectedCallback() {
        unregisterLockable(this);
        clearInterval(this._timer);
        this._timer = null;
        var idx = appStatusListeners.indexOf(this._statusListener);
        if (idx !== -1) appStatusListeners.splice(idx, 1);
        this._statusListener = null;
      }

      // Emits one health-fault event. `seconds` of 0 restores.
      _send(seconds, pending) {
        this._break.disabled = true;
        this._restore.disabled = true;
        this._status.textContent = pending;
        cbEvents
          .emit(
            uuid(),
            "health-fault",
            JSON.stringify({ dependency: this._select.value, seconds: seconds })
          )
          .then((code) => {
            this._break.disabled = false;
            this._restore.disabled = false;
            if (code === 202) {
              // The authoritative text arrives on the bus; keep the board moving.
              this._poll();
            } else if (code === 403) {
              this._status.textContent = "";
              handleForbidden();
            } else {
              this._status.textContent = "Error (" + code + ")";
            }
          })
          .catch(() => {
            this._break.disabled = false;
            this._restore.disabled = false;
            this._status.textContent = "Error de red";
          });
      }

      // Reads the three endpoints and repaints the board.
      _poll() {
        if (document.hidden || !this.isConnected) return;
        HEALTH_CHECKS.forEach((check) => {
          fetch(check.path, { cache: "no-store" })
            .then((r) => {
              var slot = this._chips[check.key];
              slot.code.textContent = String(r.status);
              slot.chip.className = "cb-health-chip " + healthCodeClass(r.status);
              if (r.status === 404) {
                this._detail.textContent =
                  "Los endpoints no están habilitados en este servidor (CB_HEALTH_CHECKS).";
                return null;
              }
              return check.key === "ready" ? r.json() : null;
            })
            .then((body) => {
              if (body) this._paintDetail(body);
            })
            .catch(() => {
              var slot = this._chips[check.key];
              slot.code.textContent = "×";
              slot.chip.className = "cb-health-chip cb-health-unknown";
            });
        });
        this._paintCountdown();
      }

      // The readiness body carries the aggregate and the per-dependency reasons.
      // The body speaks the wire vocabulary; the guide reads Spanish.
      _paintDetail(body) {
        var parts = ["estado: " + body.status, "ciclo: " + body.lifecycle];
        (body.checks || []).forEach(function (dep) {
          if (dep.status === "fail") {
            var kind = dep.criticality === "hard" ? "dura" : "blanda";
            parts.push(dep.name + " (" + kind + "): " + (dep.error || "falla"));
          }
        });
        this._detail.textContent = parts.join(" · ");
      }

      _paintCountdown() {
        if (!this._until) return;
        var left = Math.ceil((this._until - Date.now()) / 1000);
        if (left <= 0) {
          this._until = 0;
          return;
        }
        this._status.textContent = "Restaura sola en " + left + "s";
      }
    }
  );

  // ---------------------------------------------------------------------------
  // Custom element: <cb-file>
  // Attributes: path, type, data-content, toggleable, open, full-path
  //
  // The server fills data-content from a repository file while rendering the
  // course. The component deliberately has no fetch URL: published courses do
  // not expose the repository filesystem to the browser.
  // ---------------------------------------------------------------------------

  customElements.define(
    "cb-file",
    class extends HTMLElement {
      connectedCallback() {
        if (this._rendered) return;
        this._rendered = true;

        var type = this.getAttribute("type") || "text";
        var content = this.getAttribute("data-content") || "";
        var path = this.getAttribute("path") || "archivo";
        var toggleable = this.hasAttribute("toggleable");
        var isOpen = !toggleable || this.hasAttribute("open");
        var header = document.createElement("span");
        header.className = "cb-file-header";
        var label = document.createElement("span");
        label.className = "cb-file-path";
        if (this.hasAttribute("full-path")) {
          label.classList.add("cb-file-path-full");
        }
        label.textContent = path;
        header.appendChild(label);
        var pre = document.createElement("pre");
        var code = document.createElement("code");
        var body = document.createElement("span");
        body.className = "cb-file-content";
        code.className = "language-" + type;
        code.textContent = content;
        pre.appendChild(code);
        body.appendChild(pre);
        if (toggleable) {
          var handle = document.createElement("button");
          handle.type = "button";
          handle.className = "cb-file-toggle";
          var updateState = () => {
            // Shiki replaces the inner <pre> while it highlights the code. The
            // wrapper remains in the DOM, so it is the stable visibility target.
            body.hidden = !isOpen;
            handle.textContent = isOpen ? "▾" : "▸";
            handle.title = isOpen ? "Ocultar archivo" : "Mostrar archivo";
            handle.setAttribute("aria-expanded", String(isOpen));
            this.toggleAttribute("open", isOpen);
            this.classList.toggle("cb-file-closed", !isOpen);
          };
          handle.addEventListener("click", () => {
            isOpen = !isOpen;
            updateState();
          });
          header.appendChild(handle);
          updateState();
        }
        this.appendChild(header);
        this.appendChild(body);
      }
    }
  );

  // ---------------------------------------------------------------------------
  // In-page HTTP console — the machinery behind <cb-http> and <cb-eco>
  //
  // Method selector, editable domain + endpoint fields, optional body, and a
  // response panel with status, latency, and the body (JSON pretty-printed).
  // The authored attributes are only the defaults — every field stays editable
  // in the page. An empty domain keeps the request same-origin (the guide and
  // the echo service share the ALB); a domain without a scheme inherits the
  // page's, which also avoids mixed content. The request is a plain browser
  // fetch(); no server handler is involved, so it never locks.
  //
  // Options: defaultEndpoint, extraQuery (called on every send, and returns the
  // pairs the widget adds on top of what the endpoint field carries), onChange
  // (called whenever a field changes, so a caller can refresh its own preview).
  // Nothing is in the DOM until mount() runs, which lets a caller build its own
  // controls against the returned handles first.
  // ---------------------------------------------------------------------------

  var HTTP_METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

  // Joins the domain and the endpoint into the fetch target. An empty domain
  // keeps the endpoint relative (same origin); a domain without a scheme gets
  // the page's own, so a plain host works from http and https alike.
  function joinUrl(base, endpoint) {
    base = (base || "").trim();
    endpoint = (endpoint || "").trim() || "/";
    if (!base) return endpoint;
    if (!/^[a-z][a-z0-9+.-]*:\/\//i.test(base)) {
      base = location.protocol + "//" + base;
    }
    base = base.replace(/\/+$/, "");
    return base + (endpoint.charAt(0) === "/" ? "" : "/") + endpoint;
  }

  // Appends `key=value` pairs to a URL that may already carry a query string.
  function withQuery(url, pairs) {
    var extra = (pairs || [])
      .filter(function (pair) {
        return pair;
      })
      .join("&");
    if (!extra) return url;
    return url + (url.indexOf("?") === -1 ? "?" : "&") + extra;
  }

  function httpConsole(host, options) {
    var opts = options || {};
    var initialMethod = (host.getAttribute("method") || "GET").toUpperCase();
    var label = host.getAttribute("label") || "Enviar";

    var row = document.createElement("div");
    row.className = "cb-http-row";

    var select = document.createElement("select");
    select.className = "cb-http-method";
    HTTP_METHODS.forEach(function (m) {
      var opt = document.createElement("option");
      opt.value = m;
      opt.textContent = m;
      select.appendChild(opt);
    });
    if (HTTP_METHODS.indexOf(initialMethod) !== -1) select.value = initialMethod;

    var domain = document.createElement("input");
    domain.type = "text";
    domain.className = "cb-http-domain";
    domain.value = host.getAttribute("domain") || "";
    domain.placeholder = "mismo origen";
    domain.spellcheck = false;

    var url = document.createElement("input");
    url.type = "text";
    url.className = "cb-http-url";
    url.value = host.getAttribute("endpoint") || opts.defaultEndpoint || "/";
    url.spellcheck = false;

    var btn = makeAppBtn(label);

    row.appendChild(select);
    row.appendChild(domain);
    row.appendChild(url);
    row.appendChild(btn);

    function buildUrl() {
      var target = joinUrl(domain.value, url.value);
      return opts.extraQuery ? withQuery(target, opts.extraQuery()) : target;
    }

    var body = document.createElement("textarea");
    body.className = "cb-http-body";
    body.rows = 3;
    body.placeholder = "Cuerpo del pedido (opcional)";
    body.spellcheck = false;
    body.value = host.getAttribute("body") || "";

    var statusLine = document.createElement("div");
    statusLine.className = "cb-http-status";
    statusLine.hidden = true;

    var responsePre = document.createElement("pre");
    var responseCode = document.createElement("code");
    responsePre.className = "cb-http-response";
    responsePre.appendChild(responseCode);
    responsePre.hidden = true;

    // Response render: plain text first (always correct), then swapped
    // for Shiki tokens when the page ships the highlighter (shiki-init.js
    // exposes window.cbShiki). The sequence guard drops a slow highlight
    // that finishes after a newer send already replaced the panel.
    var renderSeq = 0;
    function showResponse(text, isJson) {
      renderSeq++;
      responseCode.textContent = text;
      responsePre.style.removeProperty("background-color");
      responsePre.style.removeProperty("color");
      responsePre.hidden = false;
      if (!isJson || !window.cbShiki) return;
      var seq = renderSeq;
      window.cbShiki.highlight(text, "json").then(function (highlighted) {
        var code = highlighted && highlighted.querySelector("code");
        if (!code || seq !== renderSeq) return;
        responseCode.innerHTML = code.innerHTML;
        responsePre.style.backgroundColor = highlighted.style.backgroundColor || "";
        responsePre.style.color = highlighted.style.color || "";
      });
    }

    function changed() {
      if (opts.onChange) opts.onChange();
    }

    // GET and HEAD cannot carry a body — fetch() rejects the request.
    function syncBody() {
      body.hidden = select.value === "GET" || select.value === "HEAD";
    }
    select.addEventListener("change", function () {
      syncBody();
      changed();
    });
    syncBody();

    function send() {
      var init = { method: select.value };
      if (!body.hidden && body.value) init.body = body.value;
      btn.disabled = true;
      statusLine.hidden = false;
      statusLine.className = "cb-http-status";
      statusLine.textContent = "Enviando…";
      responsePre.hidden = true;
      var started = performance.now();
      fetch(buildUrl(), init)
        .then(function (r) {
          return r.text().then(function (text) {
            var ms = Math.round(performance.now() - started);
            statusLine.textContent =
              "HTTP " + r.status +
              (r.statusText ? " " + r.statusText : "") +
              " · " + ms + " ms";
            statusLine.classList.add(r.ok ? "cb-http-ok" : "cb-http-fail");
            if (text) {
              var isJson = false;
              try {
                text = JSON.stringify(JSON.parse(text), null, 2);
                isJson = true;
              } catch (_) {}
              showResponse(text, isJson);
            } else {
              responseCode.textContent = "";
              statusLine.textContent += " · sin cuerpo";
            }
          });
        })
        .catch(function (e) {
          statusLine.classList.add("cb-http-fail");
          statusLine.textContent =
            "Error de red: " + (e && e.message ? e.message : e);
        })
        .then(function () {
          btn.disabled = false;
        });
    }

    function sendOnEnter(e) {
      if (e.key === "Enter") send();
    }

    btn.addEventListener("click", send);
    domain.addEventListener("keydown", sendOnEnter);
    url.addEventListener("keydown", sendOnEnter);
    domain.addEventListener("input", changed);
    url.addEventListener("input", changed);

    return {
      method: select,
      buildUrl: buildUrl,
      send: send,
      sendOnEnter: sendOnEnter,
      // Puts the widget in the page. `extras` are the caller's own controls,
      // which sit between the request row and the body box.
      mount: function (extras) {
        host.appendChild(row);
        (extras || []).forEach(function (node) {
          host.appendChild(node);
        });
        host.appendChild(body);
        host.appendChild(statusLine);
        host.appendChild(responsePre);
        changed();
      },
    };
  }

  // ---------------------------------------------------------------------------
  // Custom element: <cb-http>
  // Attributes: method, domain, endpoint, body, label
  // ---------------------------------------------------------------------------

  customElements.define(
    "cb-http",
    class extends HTMLElement {
      connectedCallback() {
        if (this._rendered) return;
        this._rendered = true;
        httpConsole(this).mount();
      }
    }
  );

  // ---------------------------------------------------------------------------
  // Custom element: <cb-eco>
  // Attributes: method, domain, endpoint, status, query, body, label
  //
  // The <cb-http> console pointed at the echo service, with the two controls
  // that service reads from the query string: the status code it answers with
  // (`?status=503`), and any extra pairs, which come back parsed under
  // `request.query`. A preview line shows the URL those controls build, so the
  // query string stays visible instead of hiding inside the widget.
  // ---------------------------------------------------------------------------

  // Codes the workshop asks for. The field stays editable: these are shortcuts,
  // not the whole set the echo service accepts (200 to 599).
  var ECO_STATUS_CODES = [
    "200", "201", "204", "301", "400", "401", "403", "404",
    "418", "429", "500", "502", "503", "504",
  ];

  customElements.define(
    "cb-eco",
    class extends HTMLElement {
      connectedCallback() {
        if (this._rendered) return;
        this._rendered = true;

        var status = document.createElement("input");
        status.type = "text";
        status.className = "cb-eco-status-input";
        status.value = this.getAttribute("status") || "";
        status.placeholder = "200";
        status.spellcheck = false;
        status.setAttribute("aria-label", "Código de respuesta");
        status.setAttribute("list", "cb-eco-codes");

        // One shared datalist for every widget in the page.
        if (!document.getElementById("cb-eco-codes")) {
          var codes = document.createElement("datalist");
          codes.id = "cb-eco-codes";
          ECO_STATUS_CODES.forEach(function (code) {
            var opt = document.createElement("option");
            opt.value = code;
            codes.appendChild(opt);
          });
          document.body.appendChild(codes);
        }

        var query = document.createElement("input");
        query.type = "text";
        query.className = "cb-eco-query-input";
        query.value = this.getAttribute("query") || "";
        query.placeholder = "clave=valor&otra=2";
        query.spellcheck = false;
        query.setAttribute("aria-label", "Parámetros extra");

        var preview = document.createElement("div");
        preview.className = "cb-eco-preview";

        var client = httpConsole(this, {
          defaultEndpoint: "/eco/prueba",
          // `status` is only sent when the field holds something: an empty
          // field must leave the service on its own default, not send
          // `status=`, which the service would report as invalid.
          extraQuery: function () {
            var extra = query.value.trim().replace(/^[?&]+/, "");
            var code = status.value.trim();
            return [code ? "status=" + encodeURIComponent(code) : "", extra];
          },
          onChange: function () {
            preview.textContent =
              client.method.value + " " + client.buildUrl();
          },
        });

        var row = document.createElement("div");
        row.className = "cb-eco-row";
        row.appendChild(makeFieldLabel("status", status));
        row.appendChild(makeFieldLabel("query", query));

        [status, query].forEach(function (field) {
          field.addEventListener("keydown", client.sendOnEnter);
          field.addEventListener("input", function () {
            preview.textContent =
              client.method.value + " " + client.buildUrl();
          });
        });

        client.mount([row, preview]);
      }
    }
  );

  // A named field: the caption sits with its input, so the two wrap together.
  function makeFieldLabel(text, field) {
    var label = document.createElement("label");
    label.className = "cb-eco-field";
    var caption = document.createElement("span");
    caption.textContent = text;
    label.appendChild(caption);
    label.appendChild(field);
    return label;
  }

  // ---------------------------------------------------------------------------
  // Custom element: <cb-goto>
  // Attributes: path, label, data-target
  //
  // Navigation button to a heading on the session's guide page. The server
  // resolves the authored path (heading text, or "#anchor") to a heading id at
  // render time and stamps it as data-target — unknown targets fail the build,
  // so the element never has to guess. On a slides page the click leaves the
  // deck for the guide; on the guide it scrolls in place. Pure navigation, so
  // it never locks.
  // ---------------------------------------------------------------------------

  customElements.define(
    "cb-goto",
    class extends HTMLElement {
      connectedCallback() {
        if (this._rendered) return;
        this._rendered = true;

        var target = this.getAttribute("data-target") || "";
        var label =
          this.getAttribute("label") ||
          "Ir a: " + (this.getAttribute("path") || "");

        var btn = makeAppBtn(label);
        btn.classList.add("cb-goto-btn");
        btn.addEventListener("click", function () {
          // Slide headings carry the same ids as their guide copies, so the
          // page decides the behavior — not getElementById.
          var onSlides = /\/slides$/.test(location.pathname);
          var el = onSlides ? null : document.getElementById(target);
          if (el) {
            // Already on the guide: scroll in place, keep the hash honest.
            el.scrollIntoView({ behavior: "smooth" });
            history.replaceState(null, "", "#" + target);
          } else {
            var guidePath = location.pathname.replace(/\/slides$/, "");
            window.location.href = guidePath + "#" + target;
          }
        });
        this.appendChild(btn);
      }
    }
  );
})();
