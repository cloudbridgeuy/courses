// Live toast client. Subscribes to the server's SSE stream and shows a toast
// per pipeline/build event, attributed to the pod. Loaded on guide and slide
// pages; idle (and invisible) until an event arrives. EventSource reconnects on
// its own, so a dropped connection recovers without page reload.
(function () {
  if (typeof EventSource === "undefined") return;

  var container;
  function stage() {
    if (container) return container;
    container = document.createElement("div");
    container.className = "cb-toasts";
    container.setAttribute("aria-live", "polite");
    document.body.appendChild(container);
    return container;
  }

  // Map a state word to a status class for coloring the toast accent.
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

    // Enter on next frame, auto-dismiss after a few seconds, click to dismiss.
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

  var source = new EventSource("/hooks/stream");
  source.onmessage = function (e) {
    var n;
    try {
      n = JSON.parse(e.data);
    } catch (err) {
      return;
    }
    showToast(n);
  };
})();
