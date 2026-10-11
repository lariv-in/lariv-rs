(() => {
  const BUTTON_ID = "tasks-push-enable";
  const VAPID_URL = "__VAPID_URL__";
  const SUBSCRIBE_URL = "__SUBSCRIBE_URL__";
  const UNSUBSCRIBE_URL = "__UNSUBSCRIBE_URL__";

  function button() {
    return document.getElementById(BUTTON_ID);
  }

  function show(el, visible) {
    if (!el) return;
    if (visible) el.removeAttribute("hidden");
    else el.setAttribute("hidden", "");
  }

  function urlBase64ToUint8Array(base64String) {
    const padding = "=".repeat((4 - (base64String.length % 4)) % 4);
    const base64 = (base64String + padding).replace(/-/g, "+").replace(/_/g, "/");
    const raw = atob(base64);
    const output = new Uint8Array(raw.length);
    for (let i = 0; i < raw.length; i += 1) output[i] = raw.charCodeAt(i);
    return output;
  }

  async function existingSubscription() {
    if (!navigator.serviceWorker) return null;
    const reg = await navigator.serviceWorker.ready;
    return reg.pushManager.getSubscription();
  }

  async function refresh() {
    const el = button();
    if (!el || !("Notification" in window) || !navigator.serviceWorker) {
      show(el, false);
      return;
    }
    if (Notification.permission === "denied") {
      show(el, false);
      return;
    }
    let key = "";
    try {
      const res = await fetch(VAPID_URL, { credentials: "same-origin" });
      if (!res.ok) {
        show(el, false);
        return;
      }
      key = (await res.text()).trim();
    } catch (_) {
      show(el, false);
      return;
    }
    if (!key) {
      show(el, false);
      return;
    }
    el.dataset.vapid = key;
    const sub = await existingSubscription().catch(() => null);
    el.setAttribute(
      "aria-label",
      sub ? "Disable task notifications" : "Enable task notifications"
    );
    show(el, true);
    return key;
  }

  async function subscribe(key) {
    const reg = await navigator.serviceWorker.ready;
    const sub = await reg.pushManager.subscribe({
      userVisibleOnly: true,
      applicationServerKey: urlBase64ToUint8Array(key),
    });
    await fetch(SUBSCRIBE_URL, {
      method: "POST",
      credentials: "same-origin",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(sub.toJSON()),
    });
  }

  async function ensureSubscription(key) {
    if (Notification.permission === "denied") return;
    if (Notification.permission !== "granted") {
      const permission = await Notification.requestPermission();
      if (permission !== "granted") return;
    }
    const current = await existingSubscription().catch(() => null);
    if (!current) await subscribe(key);
  }

  document.addEventListener("click", async (event) => {
    const el = button();
    if (!el || !el.contains(event.target)) return;
    event.preventDefault();
    if (!el.dataset.vapid) return;
    const current = await existingSubscription().catch(() => null);
    if (current) {
      const endpoint = current.endpoint;
      try {
        await current.unsubscribe();
      } catch (_) {}
      await fetch(UNSUBSCRIBE_URL, {
        method: "POST",
        credentials: "same-origin",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ endpoint }),
      }).catch(() => {});
      await refresh();
      return;
    }
    await ensureSubscription(el.dataset.vapid).catch(() => {});
    await refresh();
  });

  function requestOnNextGesture(key) {
    const ask = () => {
      document.removeEventListener("pointerdown", ask, true);
      document.removeEventListener("keydown", ask, true);
      if (Notification.permission !== "default") return;
      ensureSubscription(key).then(() => refresh()).catch(() => {});
    };
    document.addEventListener("pointerdown", ask, true);
    document.addEventListener("keydown", ask, true);
  }

  document.addEventListener("DOMContentLoaded", refresh);
  document.addEventListener("htmx:after-swap", refresh);
  refresh().then((key) => {
    if (!key || Notification.permission === "denied") return;
    if (Notification.permission === "granted") {
      ensureSubscription(key).then(() => refresh()).catch(() => {});
      return;
    }
    Notification.requestPermission().then((permission) => {
      if (permission === "granted") {
        ensureSubscription(key).then(() => refresh()).catch(() => {});
        return;
      }
      if (permission === "default") requestOnNextGesture(key);
    }).catch(() => requestOnNextGesture(key));
  });
})();
