/* lariv p_pwa default service worker */
function larivPwaCacheName(scopeUrl) {
  try {
    const path = new URL(scopeUrl, self.location.origin).pathname || "/";
    if (path === "/") return "lariv-pwa-v1";
    const key = path.replace(/^\/+|\/+$/g, "").replace(/\//g, "-") || "scoped";
    return "lariv-pwa-v1-" + key;
  } catch (_) {
    return "lariv-pwa-v1";
  }
}

function larivOfflineUrl(scopeUrl) {
  try {
    const path = new URL(scopeUrl, self.location.origin).pathname || "/";
    if (path === "/") return "/offline";
    const scope = path.endsWith("/") ? path.slice(0, -1) : path;
    return "/offline?scope=" + encodeURIComponent(scope || "/");
  } catch (_) {
    return "/offline";
  }
}

const CACHE_NAME = larivPwaCacheName(self.registration.scope);
const OFFLINE_URL = larivOfflineUrl(self.registration.scope);

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => cache.addAll([OFFLINE_URL]))
  );
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(self.clients.claim());
});

self.addEventListener("fetch", (event) => {
  const req = event.request;
  if (req.method !== "GET") return;

  event.respondWith(
    fetch(req).catch(async () => {
      if (req.mode === "navigate") {
        const cache = await caches.open(CACHE_NAME);
        const cached = await cache.match(OFFLINE_URL);
        if (cached) return cached;
      }
      throw new Error("Network error");
    })
  );
});
