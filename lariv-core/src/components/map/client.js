// Mounts one MapLibre map and tears it down on Alpine destroy, HTMX cleanup, or DOM removal.
// Read-only maps with `data-live-user` move their user marker as the punch bar
// broadcasts `lariv:geolocation` fixes.
const STYLE = "https://tiles.openfreemap.org/styles/liberty";
const MIN_RANGE = 1;
const MAX_RANGE = 100000;
const DEFAULT_RANGE = 100;
const GEO_EVENT = "lariv:geolocation";
const GEO_FIX_MAX_AGE_MS = 30000;

function num(value) {
  if (value == null || value === "") return null;
  const n = Number(value);
  return Number.isFinite(n) ? n : null;
}

function clampRange(meters) {
  if (!Number.isFinite(meters)) return DEFAULT_RANGE;
  return Math.min(MAX_RANGE, Math.max(MIN_RANGE, meters));
}

function destination(lng, lat, meters, bearingDeg) {
  const earth = 6378137;
  const br = (bearingDeg * Math.PI) / 180;
  const lat1 = (lat * Math.PI) / 180;
  const lng1 = (lng * Math.PI) / 180;
  const d = meters / earth;
  const lat2 = Math.asin(
    Math.sin(lat1) * Math.cos(d) + Math.cos(lat1) * Math.sin(d) * Math.cos(br),
  );
  const lng2 =
    lng1 +
    Math.atan2(
      Math.sin(br) * Math.sin(d) * Math.cos(lat1),
      Math.cos(d) - Math.sin(lat1) * Math.sin(lat2),
    );
  return [(lng2 * 180) / Math.PI, (lat2 * 180) / Math.PI];
}

function circleFeature(lng, lat, meters) {
  const steps = 64;
  const ring = [];
  for (let i = 0; i <= steps; i += 1) {
    ring.push(destination(lng, lat, meters, (i / steps) * 360));
  }
  return {
    type: "Feature",
    properties: {},
    geometry: { type: "Polygon", coordinates: [ring] },
  };
}

function emptyCollection() {
  return { type: "FeatureCollection", features: [] };
}

function readLastFix() {
  const fix = window.__larivGeoFix;
  if (!fix) return null;
  const at = num(fix.at);
  if (at == null || Date.now() - at > GEO_FIX_MAX_AGE_MS) return null;
  const lat = num(fix.latitude);
  const lng = num(fix.longitude);
  return lat == null || lng == null ? null : { lat, lng };
}

function readEventFix(event) {
  const detail = event && event.detail;
  if (!detail) return null;
  const lat = num(detail.latitude);
  const lng = num(detail.longitude);
  return lat == null || lng == null ? null : { lat, lng };
}

function mount(root) {
  if (!root || root._larivMap) return;
  const canvas = root.querySelector("[data-map-canvas]");
  if (!canvas || !maplibregl.Map) return;
  const readonly = root.dataset.readonly === "true";
  const liveUser = root.dataset.liveUser === "true";
  const latInput = root.querySelector("[data-lat-input]");
  const lngInput = root.querySelector("[data-lng-input]");
  const rangeInput = root.querySelector("[data-range-input]");
  const rangeVisible = root.querySelector("[data-range-visible]");
  let centerLat = num(root.dataset.lat);
  let centerLng = num(root.dataset.lng);
  let range = num(root.dataset.range);
  let userLat = num(root.dataset.userLat);
  let userLng = num(root.dataset.userLng);
  let dead = false;
  let centerMarker = null;
  let handleMarker = null;
  let userMarker = null;
  let draggingHandle = false;
  let userPlaced = false;
  let ignoreGps = false;
  let framed = false;

  const map = new maplibregl.Map({
    container: canvas,
    style: STYLE,
    center: centerLng == null || centerLat == null ? [0, 20] : [centerLng, centerLat],
    zoom: centerLng == null || centerLat == null ? 1.5 : 15,
    attributionControl: true,
  });
  map.addControl(new maplibregl.NavigationControl({ showCompass: false }), "top-right");

  const resize = new ResizeObserver(() => {
    if (!dead) map.resize();
  });
  resize.observe(canvas);

  function teardown() {
    if (dead) return;
    dead = true;
    resize.disconnect();
    observer.disconnect();
    document.removeEventListener("htmx:beforeCleanupElement", onCleanup);
    document.removeEventListener("htmx:before:cleanup", onCleanup);
    window.removeEventListener(GEO_EVENT, onGeolocation);
    if (centerMarker) centerMarker.remove();
    if (handleMarker) handleMarker.remove();
    if (userMarker) userMarker.remove();
    try {
      map.remove();
    } catch (_err) {
      /* already removed */
    }
    delete root._larivMap;
  }

  function onCleanup(ev) {
    const target = ev.target;
    if (!target || target.nodeType !== 1) return;
    if (target === root || target.contains(root)) teardown();
  }

  const observer = new MutationObserver(() => {
    if (!root.isConnected) teardown();
  });
  observer.observe(document.body || document.documentElement, {
    childList: true,
    subtree: true,
  });
  document.addEventListener("htmx:beforeCleanupElement", onCleanup);
  document.addEventListener("htmx:before:cleanup", onCleanup);
  root._larivMap = { teardown };

  function writeInputs() {
    if (latInput) latInput.value = centerLat == null ? "" : String(centerLat);
    if (lngInput) lngInput.value = centerLng == null ? "" : String(centerLng);
    if (rangeInput) rangeInput.value = range == null ? "" : String(range);
    if (rangeVisible && !draggingHandle) {
      rangeVisible.value = range == null ? "" : String(Math.round(range));
    }
  }

  function zoneData() {
    if (centerLat == null || centerLng == null || range == null) return emptyCollection();
    return { type: "FeatureCollection", features: [circleFeature(centerLng, centerLat, range)] };
  }

  function ensureZone() {
    const data = zoneData();
    if (map.getSource("lariv-zone")) {
      map.getSource("lariv-zone").setData(data);
      return;
    }
    map.addSource("lariv-zone", { type: "geojson", data });
    map.addLayer({
      id: "lariv-zone-fill",
      type: "fill",
      source: "lariv-zone",
      paint: { "fill-color": "#2563eb", "fill-opacity": 0.18 },
    });
    map.addLayer({
      id: "lariv-zone-line",
      type: "line",
      source: "lariv-zone",
      paint: { "line-color": "#1d4ed8", "line-width": 2 },
    });
  }

  function placeHandle() {
    if (readonly || centerLat == null || centerLng == null || range == null) {
      if (handleMarker) {
        handleMarker.remove();
        handleMarker = null;
      }
      return;
    }
    const at = destination(centerLng, centerLat, range, 90);
    if (!handleMarker) {
      const el = document.createElement("div");
      el.style.width = "14px";
      el.style.height = "14px";
      el.style.borderRadius = "9999px";
      el.style.background = "#2563eb";
      el.style.border = "2px solid #fff";
      el.style.boxShadow = "0 0 0 1px #1e3a8a";
      el.style.cursor = "ew-resize";
      handleMarker = new maplibregl.Marker({ element: el, draggable: true })
        .setLngLat(at)
        .addTo(map);
      handleMarker.on("dragstart", () => {
        draggingHandle = true;
      });
      handleMarker.on("drag", () => {
        if (centerLat == null || centerLng == null) return;
        const ll = handleMarker.getLngLat();
        const meters = maplibregl.LngLat
          ? distanceMeters(centerLng, centerLat, ll.lng, ll.lat)
          : DEFAULT_RANGE;
        range = clampRange(meters);
        writeInputs();
        if (rangeVisible) rangeVisible.value = String(Math.round(range));
        if (map.getSource("lariv-zone")) map.getSource("lariv-zone").setData(zoneData());
      });
      handleMarker.on("dragend", () => {
        draggingHandle = false;
        writeInputs();
      });
    } else if (!draggingHandle) {
      handleMarker.setLngLat(at);
    }
  }

  function placeCenter() {
    if (centerLat == null || centerLng == null) {
      if (centerMarker) {
        centerMarker.remove();
        centerMarker = null;
      }
      return;
    }
    const at = [centerLng, centerLat];
    if (!centerMarker) {
      centerMarker = new maplibregl.Marker({ draggable: !readonly }).setLngLat(at).addTo(map);
      if (!readonly) {
        centerMarker.on("drag", () => {
          const ll = centerMarker.getLngLat();
          centerLat = ll.lat;
          centerLng = ll.lng;
          if (range == null) range = DEFAULT_RANGE;
          writeInputs();
          if (map.getSource("lariv-zone")) map.getSource("lariv-zone").setData(zoneData());
          placeHandle();
        });
      }
    } else {
      centerMarker.setLngLat(at);
    }
    labelMarker(centerMarker, root.dataset.centerLabel);
  }

  function placeUser() {
    if (userLat == null || userLng == null) return;
    if (!userMarker) {
      userMarker = new maplibregl.Marker({ color: "#dc2626" })
        .setLngLat([userLng, userLat])
        .addTo(map);
    } else {
      userMarker.setLngLat([userLng, userLat]);
    }
    labelMarker(userMarker, root.dataset.userLabel);
  }

  function fitToLiveUser() {
    if (dead || userLat == null || userLng == null) return;
    try {
      if (map.getBounds().contains([userLng, userLat])) return;
    } catch (_err) {
      return;
    }
    fit();
  }

  function onGeolocation(event) {
    if (dead) return;
    const fix = readEventFix(event);
    if (!fix) return;
    userLat = fix.lat;
    userLng = fix.lng;
    if (!map.isStyleLoaded()) return;
    placeUser();
    fitToLiveUser();
  }

  function labelMarker(marker, label) {
    if (!label || !marker.getElement) return;
    const el = marker.getElement();
    if (!el) return;
    el.title = label;
    el.setAttribute("aria-label", label);
  }

  function draw() {
    if (!map.isStyleLoaded()) return;
    ensureZone();
    placeCenter();
    placeHandle();
    placeUser();
    writeInputs();
  }

  function reveal() {
    if (dead || centerLat == null || centerLng == null) return;
    placeCenter();
    placeUser();
    writeInputs();
    if (!framed) {
      try {
        map.jumpTo({ center: [centerLng, centerLat], zoom: 16 });
        framed = true;
      } catch (_err) {
        /* map camera is not ready yet; idle/load will retry */
      }
    }
    try {
      if (map.isStyleLoaded()) {
        ensureZone();
        placeHandle();
      }
    } catch (_err) {
      /* style sources are not ready yet */
    }
  }

  function fit() {
    if (!maplibregl.LngLatBounds) return;
    const bounds = new maplibregl.LngLatBounds();
    let any = false;
    if (centerLat != null && centerLng != null && range != null) {
      circleFeature(centerLng, centerLat, range).geometry.coordinates[0].forEach((pair) => {
        bounds.extend(pair);
        any = true;
      });
    } else if (centerLat != null && centerLng != null) {
      bounds.extend([centerLng, centerLat]);
      any = true;
    }
    if (userLat != null && userLng != null) {
      bounds.extend([userLng, userLat]);
      any = true;
    }
    if (any) map.fitBounds(bounds, { padding: 36, maxZoom: 17, animate: false });
  }

  map.on("load", () => {
    if (dead) return;
    map.resize();
    draw();
    reveal();
    if (readonly) fit();
    requestAnimationFrame(() => {
      if (dead) return;
      map.resize();
      reveal();
      if (readonly) fit();
    });
  });
  map.on("idle", () => reveal());

  let haveFine = false;
  let haveAny = false;

  function placeFix(pos, fine) {
    if (dead || userPlaced || ignoreGps || !pos || !pos.coords) return;
    if (!fine && haveFine) return;
    const lat = pos.coords.latitude;
    const lng = pos.coords.longitude;
    if (!Number.isFinite(lat) || !Number.isFinite(lng)) return;
    if (fine) haveFine = true;
    haveAny = true;
    centerLat = lat;
    centerLng = lng;
    if (range == null) range = DEFAULT_RANGE;
    range = clampRange(range);
    framed = false;
    setMapStatus("");
    reveal();
  }

  function setMapStatus(text) {
    const statusEl = root.querySelector("[data-map-status]");
    if (statusEl) statusEl.textContent = text;
  }

  // No saved office point. Rough and precise requests run together. The first
  // reading places the marker; a later precise reading replaces a rough one.
  // Success callbacks run only after permission is granted.
  if (!readonly && (centerLat == null || centerLng == null) && navigator.geolocation) {
    const precise = { enableHighAccuracy: true, maximumAge: 0, timeout: 20000 };
    const coarse = { enableHighAccuracy: false, maximumAge: 300000, timeout: 10000 };
    let asked = false;
    let roughSettled = false;
    let fineSettled = false;
    const noteError = (fine, err) => {
      if (fine) fineSettled = true;
      else roughSettled = true;
      if (!roughSettled || !fineSettled || haveAny || dead || userPlaced || ignoreGps) return;
      if (err && err.code === 1) setMapStatus("Allow location access to set the attendance point.");
      else setMapStatus("Could not read your location.");
    };
    const requestBoth = () => {
      if (asked || dead || userPlaced || ignoreGps) return;
      asked = true;
      roughSettled = false;
      fineSettled = false;
      navigator.geolocation.getCurrentPosition(
        (pos) => {
          roughSettled = true;
          placeFix(pos, false);
        },
        (err) => noteError(false, err),
        coarse,
      );
      navigator.geolocation.getCurrentPosition(
        (pos) => {
          fineSettled = true;
          placeFix(pos, true);
        },
        (err) => noteError(true, err),
        precise,
      );
    };
    setMapStatus("Waiting for location permission…");
    requestBoth();
    if (navigator.permissions && navigator.permissions.query) {
      navigator.permissions.query({ name: "geolocation" }).then((perm) => {
        perm.addEventListener("change", () => {
          if (perm.state === "granted" && !haveFine) {
            asked = false;
            requestBoth();
          }
        });
      }).catch(() => {});
    }
  }

  if (liveUser) {
    const last = readLastFix();
    if (last) {
      userLat = last.lat;
      userLng = last.lng;
    }
    window.addEventListener(GEO_EVENT, onGeolocation);
  }

  if (!readonly) {
    map.on("click", (ev) => {
      const target = ev.originalEvent && ev.originalEvent.target;
      if (target && target.closest && target.closest(".maplibregl-marker")) return;
      userPlaced = true;
      centerLat = ev.lngLat.lat;
      centerLng = ev.lngLat.lng;
      if (range == null) range = num(rangeVisible && rangeVisible.value) || DEFAULT_RANGE;
      range = clampRange(range);
      reveal();
      draw();
    });
    if (rangeVisible) {
      rangeVisible.addEventListener("input", () => {
        const next = num(rangeVisible.value);
        if (next == null) return;
        range = clampRange(next);
        if (centerLat != null && centerLng != null && map.getSource("lariv-zone")) {
          map.getSource("lariv-zone").setData(zoneData());
        }
        placeHandle();
        if (rangeInput) rangeInput.value = String(range);
      });
    }
    const clearButton = root.querySelector("[data-map-clear]");
    if (clearButton) {
      clearButton.addEventListener("click", () => {
        ignoreGps = true;
        userPlaced = true;
        centerLat = null;
        centerLng = null;
        range = null;
        if (centerMarker) {
          centerMarker.remove();
          centerMarker = null;
        }
        if (handleMarker) {
          handleMarker.remove();
          handleMarker = null;
        }
        if (map.getSource("lariv-zone")) map.getSource("lariv-zone").setData(emptyCollection());
        writeInputs();
        map.easeTo({ center: [0, 20], zoom: 1.5 });
      });
    }
  }
}

function distanceMeters(lng1, lat1, lng2, lat2) {
  const earth = 6378137;
  const p1 = (lat1 * Math.PI) / 180;
  const p2 = (lat2 * Math.PI) / 180;
  const dLat = ((lat2 - lat1) * Math.PI) / 180;
  const dLng = ((lng2 - lng1) * Math.PI) / 180;
  const h =
    Math.sin(dLat / 2) * Math.sin(dLat / 2) +
    Math.cos(p1) * Math.cos(p2) * Math.sin(dLng / 2) * Math.sin(dLng / 2);
  return 2 * earth * Math.asin(Math.min(1, Math.sqrt(h)));
}

window.LarivMap = { mount };
