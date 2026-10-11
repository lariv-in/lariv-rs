/* Task push notifications. Imported by /serviceworker.js. */
self.addEventListener("push", (event) => {
  let data = {};
  try {
    data = event.data ? event.data.json() : {};
  } catch (_) {}
  const title = typeof data.title === "string" && data.title ? data.title : "Task";
  const body = typeof data.body === "string" ? data.body : "";
  const url = typeof data.url === "string" && data.url ? data.url : "/dashboard/tasks";
  event.waitUntil(
    (async () => {
      if (Notification.permission !== "granted") return;
      await self.registration.showNotification(title, {
        body,
        data: { url },
      });
    })()
  );
});

self.addEventListener("notificationclick", (event) => {
  event.notification.close();
  const url =
    (event.notification.data && event.notification.data.url) || "/dashboard/tasks";
  event.waitUntil(
    self.clients
      .matchAll({ type: "window", includeUncontrolled: true })
      .then((windowClients) => {
        for (const client of windowClients) {
          if (client.url.endsWith(url) && "focus" in client) return client.focus();
        }
        if (self.clients.openWindow) return self.clients.openWindow(url);
      })
  );
});
