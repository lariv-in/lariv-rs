import { Local, Room } from "https://esm.sh/@moq/room@0.1.0";
import { Connection, Path } from "https://esm.sh/@moq/net@0.3.5";
import { Effect } from "https://esm.sh/@moq/signals@0.2.3";

if (!window.LarivMeets) {
  const sessions = new Map();
  const stageRoots = new WeakMap();
  const sourceRoots = new WeakMap();

  function session(roomCode) {
    let s = sessions.get(roomCode);
    if (!s) {
      s = {
        roomCode,
        connection: null,
        local: null,
        room: null,
        remoteEffect: null,
        localStream: null,
        remotes: new Map(),
        stageEl: null,
        sourceEl: null,
        joinedUserId: null,
        relayUrl: "",
        moqJwt: "",
        closed: false,
        leaving: false,
        reconnectTimer: null,
        mutedAudio: false,
        mutedVideo: false,
      };
      sessions.set(roomCode, s);
    }
    return s;
  }

  function relayUrlFor(s) {
    const raw = (s.relayUrl || "").trim();
    if (!raw) {
      throw new Error(
        "Missing MoQ relay URL. Set meets.transport.publicUrl in Lariv config."
      );
    }
    const base = raw.startsWith("http")
      ? raw
      : location.protocol === "https:"
        ? location.origin + (raw.startsWith("/") ? raw : "/" + raw)
        : null;
    if (!base) {
      throw new Error("Meets requires HTTPS for MoQ media.");
    }
    const url = new URL(base);
    if (s.moqJwt) {
      url.searchParams.set("jwt", s.moqJwt);
    }
    return url;
  }

  function stageRoot(s) {
    if (s.stageEl) return s.stageEl;
    const nodes = document.querySelectorAll("[data-meets-stage]");
    for (const el of nodes) {
      if (el.getAttribute("data-room-code") === s.roomCode) {
        s.stageEl = el;
        return el;
      }
    }
    return null;
  }

  function ensureRemoteTile(s, identity) {
    const root = stageRoot(s);
    if (!root) return null;
    const grid = root.querySelector("[data-remote-grid]");
    if (!grid) return null;
    let tile = grid.querySelector(`[data-joined-user="${identity}"]`);
    if (tile) return tile;
    tile = document.createElement("div");
    tile.className = "meets-tile rounded-lg bg-base-300 overflow-hidden";
    tile.setAttribute("data-joined-user", identity);
    tile.innerHTML =
      '<canvas class="w-full aspect-video object-cover"></canvas>' +
      '<p class="text-xs p-2 opacity-70" data-participant-label></p>';
    grid.appendChild(tile);
    return tile;
  }

  function attachRemoteMember(s, identity, remote) {
    const tile = ensureRemoteTile(s, identity);
    if (!tile) return;
    const canvas = tile.querySelector("canvas");
    const label = tile.querySelector("[data-participant-label]");
    if (label) label.textContent = identity;

    const member = remote.camera?.peek?.() ?? remote.camera;
    if (!member) return;

    if (member.canvas && canvas) {
      const bound = member.canvas.peek?.() ?? member.canvas;
      if (bound && bound !== canvas) {
        const ctx = canvas.getContext("2d");
        const draw = () => {
          if (bound.width && bound.height) {
            canvas.width = bound.width;
            canvas.height = bound.height;
            ctx.drawImage(bound, 0, 0);
          }
          if (!s.closed) requestAnimationFrame(draw);
        };
        draw();
      } else if (member.canvas?.subscribe) {
        member.canvas.subscribe((c) => {
          if (c && canvas.parentNode) {
            canvas.replaceWith(c);
            c.className = "w-full aspect-video object-cover";
          }
        });
      }
    }

    if (member.muted) {
      member.muted.set(false);
    }
  }

  function watchRemotes(s) {
    if (!s.room || s.remoteEffect) return;
    s.remoteEffect = new Effect((ctx) => {
      const remotes = ctx.get(s.room.remotes);
      for (const [identity, remote] of remotes) {
        if (String(identity) === String(s.joinedUserId)) continue;
        attachRemoteMember(s, String(identity), remote);
      }
    });
  }

  async function ensureMoq(s) {
    if (s.connection && s.local) return;
    const url = relayUrlFor(s);
    const identity = Path.from(String(s.joinedUserId));

    const connection = new Connection.Reload({
      url,
      enabled: true,
    });

    const local = new Local({
      connection: connection.established,
      identity,
      user: { name: String(s.joinedUserId) },
    });

    s.connection = connection;
    s.local = local;
    s.room = new Room({ connection, identity });

    connection.closed?.catch?.(() => {
      if (s.closed || s.leaving) return;
      scheduleReconnect(s);
    });

    watchRemotes(s);
  }

  function scheduleReconnect(s) {
    if (s.reconnectTimer || s.closed || s.leaving) return;
    s.reconnectTimer = setTimeout(() => {
      s.reconnectTimer = null;
      if (!liveSourceEl(s.roomCode)) return;
      joinStreams(s).catch((err) => console.error("LarivMeets reconnect", err));
    }, 2000);
  }

  function liveSourceEl(roomCode) {
    const nodes = document.querySelectorAll("[data-meets-source]");
    for (const el of nodes) {
      if (el.getAttribute("data-room-code") === roomCode && !el.hasAttribute("data-preview-only")) {
        return el;
      }
    }
    return null;
  }

  async function joinStreams(s) {
    if (!s.joinedUserId || !s.moqJwt) return;
    await ensureMoq(s);
    s.local.enabled.set(true);
    s.local.cameraEnabled.set(!s.mutedVideo);
    s.local.microphoneEnabled.set(!s.mutedAudio);
  }

  function mediaAvailable() {
    return !!(navigator.mediaDevices && navigator.mediaDevices.getUserMedia);
  }

  function mediaErrorMessage(err) {
    if (window.isSecureContext === false) {
      return "Camera and microphone require HTTPS.";
    }
    if (!mediaAvailable()) {
      return "This browser does not support camera or microphone access.";
    }
    if (err && err.name === "NotAllowedError") {
      return "Camera or microphone permission was denied.";
    }
    if (err && err.name === "NotFoundError") {
      return "No camera or microphone was found on this device.";
    }
    return "Could not access camera or microphone.";
  }

  function showMediaError(root, message) {
    const el = root && root.querySelector("[data-media-error]");
    if (!el) return;
    if (message) {
      el.textContent = message;
      el.classList.remove("hidden");
    } else {
      el.textContent = "";
      el.classList.add("hidden");
    }
  }

  async function fillDevices(root) {
    if (!navigator.mediaDevices?.enumerateDevices) return;
    const videoSel = root.querySelector("[data-video-device]");
    const audioSel = root.querySelector("[data-audio-device]");
    const devices = await navigator.mediaDevices.enumerateDevices();
    if (videoSel) {
      const cur = videoSel.value;
      videoSel.innerHTML = "";
      for (const d of devices.filter((x) => x.kind === "videoinput")) {
        const opt = document.createElement("option");
        opt.value = d.deviceId;
        opt.textContent = d.label || "Camera";
        videoSel.appendChild(opt);
      }
      if (cur) videoSel.value = cur;
    }
    if (audioSel) {
      const cur = audioSel.value;
      audioSel.innerHTML = "";
      for (const d of devices.filter((x) => x.kind === "audioinput")) {
        const opt = document.createElement("option");
        opt.value = d.deviceId;
        opt.textContent = d.label || "Microphone";
        audioSel.appendChild(opt);
      }
      if (cur) audioSel.value = cur;
    }
  }

  function prefsKey(roomCode) {
    return `lariv-meets-prefs:${roomCode}`;
  }

  function readPrefs(roomCode) {
    try {
      const raw = sessionStorage.getItem(prefsKey(roomCode));
      return raw ? JSON.parse(raw) : null;
    } catch (_) {
      return null;
    }
  }

  function currentPrefs(root) {
    const videoSel = root.querySelector("[data-video-device]");
    const audioSel = root.querySelector("[data-audio-device]");
    return {
      videoDeviceId: videoSel && videoSel.value,
      audioDeviceId: audioSel && audioSel.value,
    };
  }

  function savePrefs(roomCode, root) {
    try {
      sessionStorage.setItem(prefsKey(roomCode), JSON.stringify(currentPrefs(root)));
    } catch (_) {}
  }

  async function startPreview(root, s, previewOnly) {
    if (!mediaAvailable()) {
      showMediaError(root, mediaErrorMessage(null));
      return;
    }
    const prefs = readPrefs(s.roomCode) || {};
    const constraints = {
      video: prefs.videoDeviceId ? { deviceId: { exact: prefs.videoDeviceId } } : true,
      audio: prefs.audioDeviceId ? { deviceId: { exact: prefs.audioDeviceId } } : true,
    };
    try {
      s.localStream = await navigator.mediaDevices.getUserMedia(constraints);
      showMediaError(root, null);
    } catch (err) {
      showMediaError(root, mediaErrorMessage(err));
      return;
    }
    const preview = root.querySelector("[data-local-preview]");
    if (preview) preview.srcObject = s.localStream;
    await fillDevices(root);
    if (!previewOnly) {
      await joinStreams(s);
    }
  }

  function destroySession(s) {
    s.closed = true;
    s.leaving = true;
    if (s.reconnectTimer) clearTimeout(s.reconnectTimer);
    if (s.remoteEffect) {
      try {
        s.remoteEffect.close();
      } catch (_) {}
    }
    if (s.local) {
      try {
        s.local.enabled.set(false);
        s.local.close();
      } catch (_) {}
    }
    if (s.room) {
      try {
        s.room.close();
      } catch (_) {}
    }
    if (s.connection) {
      try {
        s.connection.close?.();
      } catch (_) {}
    }
    if (s.localStream) {
      for (const t of s.localStream.getTracks()) t.stop();
    }
    sessions.delete(s.roomCode);
  }

  function destroyAll() {
    for (const s of sessions.values()) destroySession(s);
  }

  async function mountSource(root) {
    const roomCode = root.getAttribute("data-room-code");
    const s = session(roomCode);
    sourceRoots.set(root, s);
    s.sourceEl = root;
    s.relayUrl = root.getAttribute("data-relay-url") || "";
    s.moqJwt = root.getAttribute("data-moq-jwt") || "";
    s.joinedUserId = Number(root.getAttribute("data-joined-user-id") || 0);
    const previewOnly = root.hasAttribute("data-preview-only");

    root.querySelector("[data-mute-audio]")?.addEventListener("click", () => {
      s.mutedAudio = !s.mutedAudio;
      if (s.local) s.local.microphoneEnabled.set(!s.mutedAudio);
      for (const t of s.localStream?.getAudioTracks() || []) {
        t.enabled = !s.mutedAudio;
      }
    });
    root.querySelector("[data-mute-video]")?.addEventListener("click", () => {
      s.mutedVideo = !s.mutedVideo;
      if (s.local) s.local.cameraEnabled.set(!s.mutedVideo);
      for (const t of s.localStream?.getVideoTracks() || []) {
        t.enabled = !s.mutedVideo;
      }
    });
    root.querySelector("[data-share-screen]")?.addEventListener("click", async () => {
      if (!s.local) return;
      s.local.screenEnabled.set(!s.local.screenEnabled.peek?.());
    });
    for (const sel of root.querySelectorAll("[data-video-device],[data-audio-device]")) {
      sel.addEventListener("change", () => {
        savePrefs(roomCode, root);
        startPreview(root, s, previewOnly).catch(() => {});
      });
    }
    try {
      await startPreview(root, s, previewOnly);
    } catch (err) {
      showMediaError(root, err && err.message ? err.message : "Meets setup failed.");
      throw err;
    }
  }

  async function mountStage(root) {
    const roomCode = root.getAttribute("data-room-code");
    const s = session(roomCode);
    stageRoots.set(root, s);
    s.stageEl = root;
    s.relayUrl = root.getAttribute("data-relay-url") || s.relayUrl;
    s.moqJwt = root.getAttribute("data-moq-jwt") || s.moqJwt || "";
    s.joinedUserId = Number(root.getAttribute("data-joined-user-id") || s.joinedUserId || 0);
    if (s.moqJwt) {
      await joinStreams(s);
    }
  }

  if (!window.__larivMeetsHtmxGuard) {
    window.__larivMeetsHtmxGuard = true;
    document.body.addEventListener("htmx:beforeSwap", (ev) => {
      const target = ev.detail && ev.detail.target;
      if (target && (target.id === "meets-room-media" || target.closest("#meets-room-media"))) {
        ev.preventDefault();
      }
    });
    window.addEventListener("htmx:historyRestore", () => destroyAll());
  }

  window.LarivMeets = { mountSource, mountStage, destroyAll };
  window.dispatchEvent(new Event("lariv-meets-ready"));
}
