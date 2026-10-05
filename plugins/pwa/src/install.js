(() => {
  const BUTTON_ID = "pwa-install";
  let deferredPrompt = null;

  function installButton() {
    return document.getElementById(BUTTON_ID);
  }

  function setButtonVisible(visible) {
    const el = installButton();
    if (!el) return;
    if (visible) el.removeAttribute("hidden");
    else el.setAttribute("hidden", "");
  }

  async function relatedAppsInstalled() {
    if (typeof navigator.getInstalledRelatedApps !== "function") return false;
    try {
      const apps = await navigator.getInstalledRelatedApps();
      return Array.isArray(apps) && apps.length > 0;
    } catch {
      return false;
    }
  }

  async function maybeShow() {
    if (!deferredPrompt) {
      setButtonVisible(false);
      return;
    }
    if (await relatedAppsInstalled()) {
      setButtonVisible(false);
      return;
    }
    setButtonVisible(true);
  }

  window.addEventListener("beforeinstallprompt", (event) => {
    event.preventDefault();
    deferredPrompt = event;
    maybeShow();
  });

  window.addEventListener("appinstalled", () => {
    deferredPrompt = null;
    setButtonVisible(false);
  });

  document.addEventListener("click", async (event) => {
    const el = installButton();
    if (!el || !el.contains(event.target)) return;
    event.preventDefault();
    if (!deferredPrompt) return;
    deferredPrompt.prompt();
    try {
      await deferredPrompt.userChoice;
    } catch (_) {}
    deferredPrompt = null;
    setButtonVisible(false);
  });

  document.addEventListener("htmx:after-swap", maybeShow);
  document.addEventListener("htmx:after-settle", maybeShow);
  document.addEventListener("DOMContentLoaded", maybeShow);

  if (navigator.serviceWorker) {
    navigator.serviceWorker.register("/serviceworker.js").catch(() => {});
  }

  maybeShow();
})();
