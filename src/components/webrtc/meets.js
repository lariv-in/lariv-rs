if (!window.LarivMeets) {
  const sessions = new Map();
  const stageRoots = new WeakMap();
  const sourceRoots = new WeakMap();

  const VIDEO_MIME_ORDER = [
    "video/AV1",
    "video/H265",
    "video/HEVC",
    "video/VP9",
    "video/VP8",
    "video/H264",
  ];

  function session(roomCode) {
    let s = sessions.get(roomCode);
    if (!s) {
      s = {
        roomCode,
        pc: null,
        ws: null,
        localStream: null,
        screenStream: null,
        remotes: new Map(),
        stageEl: null,
        sourceEl: null,
        joinedUserId: null,
        iceServers: [],
        signalingUrl: "",
        closed: false,
        joinSent: false,
        pendingIce: [],
        signaling: Promise.resolve(),
      };
      s.whenLocalReady = new Promise((resolve) => {
        s.resolveLocalReady = resolve;
      });
      sessions.set(roomCode, s);
    }
    return s;
  }

  function markLocalReady(s) {
    if (s.resolveLocalReady) {
      s.resolveLocalReady();
      s.resolveLocalReady = null;
    }
  }

  function enqueueSignal(s, fn) {
    s.signaling = s.signaling.then(fn, fn);
    return s.signaling;
  }

  function iceInit(msg) {
    return {
      candidate: msg.candidate,
      sdpMid: msg.sdp_mid,
      sdpMLineIndex: msg.sdp_mline_index,
    };
  }

  function iceServersFrom(el) {
    try {
      return JSON.parse(el.getAttribute("data-ice-servers") || "[]");
    } catch (_) {
      return [];
    }
  }

  function preferCodecs(pc) {
    if (!window.RTCRtpSender || !RTCRtpSender.getCapabilities) return;
    const caps = RTCRtpSender.getCapabilities("video");
    if (!caps || !caps.codecs) return;
    const preferred = [];
    for (const mime of VIDEO_MIME_ORDER) {
      for (const c of caps.codecs) {
        if (c.mimeType && c.mimeType.toLowerCase() === mime.toLowerCase()) {
          preferred.push(c);
        }
      }
    }
    for (const t of pc.getTransceivers()) {
      if (t.receiver && t.receiver.track && t.receiver.track.kind === "video") {
        try {
          t.setCodecPreferences(preferred);
        } catch (_) {}
      }
      if (t.sender && t.sender.track && t.sender.track.kind === "video") {
        try {
          t.setCodecPreferences(preferred);
        } catch (_) {}
      }
    }
  }

  function listedCodecs() {
    if (!window.RTCRtpSender || !RTCRtpSender.getCapabilities) {
      return ["VP8", "H264"];
    }
    const caps = RTCRtpSender.getCapabilities("video");
    const out = [];
    if (!caps) return ["VP8", "H264"];
    for (const c of caps.codecs || []) {
      const m = (c.mimeType || "").toUpperCase();
      if (m.includes("AV1") && !out.includes("AV1")) out.push("AV1");
      else if ((m.includes("H265") || m.includes("HEVC")) && !out.includes("H265")) out.push("H265");
      else if (m.includes("VP9") && !out.includes("VP9")) out.push("VP9");
      else if (m.includes("VP8") && !out.includes("VP8")) out.push("VP8");
      else if (m.includes("H264") && !out.includes("H264")) out.push("H264");
    }
    return out.length ? out : ["VP8", "H264"];
  }

  function ensureRemoteTile(s, joinedUserId) {
    if (!s.stageEl) return;
    const grid = s.stageEl.querySelector("[data-remote-grid]");
    if (!grid) return;
    let tile = grid.querySelector(`[data-joined-user="${joinedUserId}"]`);
    if (tile) return tile;
    tile = document.createElement("div");
    tile.className = "meets-remote-tile rounded-lg overflow-hidden bg-base-300 relative";
    tile.setAttribute("data-joined-user", String(joinedUserId));
    const video = document.createElement("video");
    video.playsInline = true;
    video.autoplay = true;
    video.className = "w-full h-full object-cover";
    const audio = document.createElement("audio");
    audio.autoplay = true;
    tile.appendChild(video);
    tile.appendChild(audio);
    grid.appendChild(tile);
    return tile;
  }

  function attachRemoteTrack(s, track, streams) {
    const stream = streams && streams[0];
    const id = (stream && stream.id) || track.id;
    let joined = id;
    const m = String(id).match(/(\d+)$/);
    if (m) joined = m[1];
    const tile = ensureRemoteTile(s, joined);
    if (!tile) return;
    const el = track.kind === "audio" ? tile.querySelector("audio") : tile.querySelector("video");
    if (!el) return;
    let ms = el.srcObject;
    if (!ms) {
      ms = new MediaStream();
      el.srcObject = ms;
    }
    if (!ms.getTracks().includes(track)) ms.addTrack(track);
    el.play && el.play().catch(() => {});
  }

  async function ensurePc(s) {
    if (s.pc) return s.pc;
    s.pc = new RTCPeerConnection({ iceServers: s.iceServers });
    s.pc.onicecandidate = (ev) => {
      if (!ev.candidate || !s.ws || s.ws.readyState !== 1) return;
      s.ws.send(
        JSON.stringify({
          type: "ice",
          candidate: ev.candidate.candidate,
          sdp_mid: ev.candidate.sdpMid,
          sdp_mline_index: ev.candidate.sdpMLineIndex,
        })
      );
    };
    s.pc.ontrack = (ev) => attachRemoteTrack(s, ev.track, ev.streams);
    return s.pc;
  }

  async function flushPendingIce(s) {
    if (!s.pc || !s.pc.remoteDescription) return;
    const queued = s.pendingIce.splice(0);
    for (const msg of queued) {
      try {
        await s.pc.addIceCandidate(iceInit(msg));
      } catch (_) {}
    }
  }

  async function answerOffer(s, ws, sdp) {
    if (!s.pc) return;
    if (s.localStream) await publishLocal(s);
    preferCodecs(s.pc);
    await s.pc.setRemoteDescription({ type: "offer", sdp });
    await flushPendingIce(s);
    const answer = await s.pc.createAnswer();
    await s.pc.setLocalDescription(answer);
    if (ws.readyState === 1) ws.send(JSON.stringify({ type: "answer", sdp: answer.sdp }));
  }

  async function sendJoin(s) {
    if (s.joinSent || s.closed) return;
    if (!s.ws || s.ws.readyState !== 1) return;
    s.joinSent = true;
    s.ws.send(
      JSON.stringify({
        type: "join",
        joined_user_id: s.joinedUserId,
        video_codecs: listedCodecs(),
      })
    );
  }

  function liveSourceEl(roomCode) {
    const nodes = document.querySelectorAll("[data-webrtc-source]");
    for (const el of nodes) {
      if (el.getAttribute("data-room-code") === roomCode && !el.hasAttribute("data-preview-only")) {
        return el;
      }
    }
    return null;
  }

  async function waitForLocalPublish(s) {
    if (!liveSourceEl(s.roomCode)) return;
    await Promise.race([
      s.whenLocalReady,
      new Promise((resolve) => setTimeout(resolve, 30000)),
    ]);
    if (s.localStream) await publishLocal(s);
  }

  async function ensureWs(s) {
    if (s.ws && (s.ws.readyState === 0 || s.ws.readyState === 1)) return s.ws;
    const proto = location.protocol === "https:" ? "wss:" : "ws:";
    const url = s.signalingUrl.startsWith("ws")
      ? s.signalingUrl
      : proto + "//" + location.host + s.signalingUrl;
    s.joinSent = false;
    const ws = new WebSocket(url);
    s.ws = ws;
    ws.addEventListener("message", (ev) => {
      let msg;
      try {
        msg = JSON.parse(ev.data);
      } catch (_) {
        return;
      }
      if (msg.type === "offer") {
        enqueueSignal(s, () => answerOffer(s, ws, msg.sdp)).catch((err) =>
          console.error("LarivMeets answer failed", err)
        );
      } else if (msg.type === "ice" && msg.candidate) {
        enqueueSignal(s, async () => {
          if (!s.pc || !s.pc.remoteDescription) {
            s.pendingIce.push(msg);
            return;
          }
          try {
            await s.pc.addIceCandidate(iceInit(msg));
          } catch (_) {}
        });
      } else if (msg.type === "codec") {
        preferCodecs(s.pc);
      } else if (msg.type === "participant_joined" && s.stageEl) {
        ensureRemoteTile(s, msg.joined_user_id);
      } else if (msg.type === "tracks_changed" && s.stageEl) {
        ensureRemoteTile(s, msg.joined_user_id);
      } else if (msg.type === "participant_left" && s.stageEl) {
        const tile = s.stageEl.querySelector(`[data-joined-user="${msg.joined_user_id}"]`);
        if (tile) tile.remove();
      }
    });
    await new Promise((resolve, reject) => {
      ws.addEventListener("open", resolve, { once: true });
      ws.addEventListener("error", () => reject(new Error("signaling failed")), { once: true });
    });
    return ws;
  }

  async function publishLocal(s) {
    if (!s.localStream || !s.pc) return;
    for (const track of s.localStream.getTracks()) {
      const exists = s.pc.getSenders().some((snd) => snd.track === track);
      if (!exists) s.pc.addTrack(track, s.localStream);
    }
    preferCodecs(s.pc);
  }

  async function fillDevices(root) {
    if (!navigator.mediaDevices || !navigator.mediaDevices.enumerateDevices) return;
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

  async function startPreview(root, s) {
    const videoSel = root.querySelector("[data-video-device]");
    const audioSel = root.querySelector("[data-audio-device]");
    const constraints = {
      audio: audioSel && audioSel.value ? { deviceId: { exact: audioSel.value } } : true,
      video: videoSel && videoSel.value ? { deviceId: { exact: videoSel.value } } : true,
    };
    if (s.localStream) {
      s.localStream.getTracks().forEach((t) => t.stop());
    }
    s.localStream = await navigator.mediaDevices.getUserMedia(constraints);
    const preview = root.querySelector("[data-local-preview]");
    if (preview) {
      preview.srcObject = s.localStream;
      preview.muted = true;
      preview.play && preview.play().catch(() => {});
    }
    await fillDevices(root);
    if (!root.hasAttribute("data-preview-only")) {
      await ensurePc(s);
      await publishLocal(s);
      markLocalReady(s);
      await ensureWs(s);
      await sendJoin(s);
    }
  }

  function destroySession(s) {
    if (s.closed) return;
    s.closed = true;
    markLocalReady(s);
    try {
      if (s.ws && s.ws.readyState === 1) s.ws.send(JSON.stringify({ type: "leave" }));
      if (s.ws) s.ws.close();
    } catch (_) {}
    if (s.pc) {
      try {
        s.pc.close();
      } catch (_) {}
    }
    if (s.localStream) s.localStream.getTracks().forEach((t) => t.stop());
    if (s.screenStream) s.screenStream.getTracks().forEach((t) => t.stop());
    sessions.delete(s.roomCode);
  }

  window.LarivMeets = {
    async mountStage(root) {
      if (!root || stageRoots.get(root)) return;
      stageRoots.set(root, true);
      const roomCode = root.getAttribute("data-room-code") || "";
      const s = session(roomCode);
      s.stageEl = root;
      s.signalingUrl = root.getAttribute("data-signaling-url") || s.signalingUrl;
      s.joinedUserId = Number(root.getAttribute("data-joined-user-id") || s.joinedUserId || 0);
      s.iceServers = iceServersFrom(root);
      s.closed = false;
      await ensurePc(s);
      await ensureWs(s);
      await waitForLocalPublish(s);
      await sendJoin(s);
    },
    async mountSource(root) {
      if (!root || sourceRoots.get(root)) return;
      sourceRoots.set(root, true);
      const roomCode = root.getAttribute("data-room-code") || "";
      const s = session(roomCode);
      s.sourceEl = root;
      s.signalingUrl = root.getAttribute("data-signaling-url") || s.signalingUrl;
      s.joinedUserId = Number(root.getAttribute("data-joined-user-id") || s.joinedUserId || 0);
      s.iceServers = iceServersFrom(root);
      s.closed = false;
      const previewOnly = root.hasAttribute("data-preview-only");
      if (previewOnly) root.setAttribute("data-preview-only", "true");
      try {
        await startPreview(root, s);
      } catch (err) {
        console.error("LarivMeets preview failed", err);
      } finally {
        markLocalReady(s);
      }
      root.querySelector("[data-video-device]")?.addEventListener("change", () => startPreview(root, s));
      root.querySelector("[data-audio-device]")?.addEventListener("change", () => startPreview(root, s));
      root.querySelector("[data-mute-audio]")?.addEventListener("click", () => {
        if (!s.localStream) return;
        s.localStream.getAudioTracks().forEach((t) => (t.enabled = !t.enabled));
      });
      root.querySelector("[data-mute-video]")?.addEventListener("click", () => {
        if (!s.localStream) return;
        s.localStream.getVideoTracks().forEach((t) => (t.enabled = !t.enabled));
      });
      root.querySelector("[data-share-screen]")?.addEventListener("click", async () => {
        try {
          const display = await navigator.mediaDevices.getDisplayMedia({ video: true, audio: false });
          s.screenStream = display;
          const track = display.getVideoTracks()[0];
          const sender = s.pc && s.pc.getSenders().find((x) => x.track && x.track.kind === "video");
          if (sender && track) await sender.replaceTrack(track);
          const preview = root.querySelector("[data-local-preview]");
          if (preview) preview.srcObject = display;
          track.addEventListener("ended", () => startPreview(root, s));
        } catch (err) {
          console.error("LarivMeets screen share failed", err);
        }
      });
    },
    destroy(root) {
      if (!root) return;
      const roomCode = root.getAttribute("data-room-code");
      if (!roomCode) return;
      const s = sessions.get(roomCode);
      if (!s) return;
      if (s.stageEl === root) s.stageEl = null;
      if (s.sourceEl === root) s.sourceEl = null;
      if (!s.stageEl && !s.sourceEl) destroySession(s);
    },
    destroyAll() {
      for (const s of Array.from(sessions.values())) destroySession(s);
    },
  };

  document.addEventListener("htmx:before-swap", (ev) => {
    const media = document.getElementById("meets-room-media");
    if (!media) return;
    const target = ev.detail && ev.detail.target;
    if (target === media) {
      ev.preventDefault();
      return;
    }
    if (target && target.contains && target.contains(media)) {
      const incoming = String(ev.detail.serverResponse || "");
      if (incoming.indexOf('id="meets-room-media"') === -1 && incoming.indexOf("id='meets-room-media'") === -1) {
        window.LarivMeets.destroyAll();
      }
    }
  });
  document.addEventListener("htmx:history:restore", () => window.LarivMeets.destroyAll());
  window.dispatchEvent(new Event("lariv-meets-ready"));
}
