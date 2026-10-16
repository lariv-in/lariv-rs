(function () {
  const cfgEl = document.getElementById("todo-config");
  if (!cfgEl) return;
  const cfg = JSON.parse(cfgEl.textContent);
  const storageKey = "lariv.todo.v1." + cfg.userId + "." + cfg.pageId;

  if (window.__larivTodo) window.__larivTodo.close();

  let state = load();
  let socket = null;
  let syncing = false;
  let syncBatchIds = new Set();
  let disposed = false;
  let reconnectTimer = null;
  let focusClientId = null;
  let drag = null;

  const session = {
    close: function () {
      disposed = true;
      window.clearTimeout(reconnectTimer);
      if (socket) {
        socket.close();
        socket = null;
      }
    },
  };
  window.__larivTodo = session;

  const list = document.getElementById("todo-list");
  const addButton = document.getElementById("todo-add");
  if (list) {
    list.addEventListener("input", onInput);
    list.addEventListener("change", onChange);
    list.addEventListener("click", onClick);
    list.addEventListener("pointerdown", onDragDown);
    list.addEventListener("pointermove", onDragMove);
    list.addEventListener("pointerup", onDragUp);
    list.addEventListener("pointercancel", onDragUp);
  }
  if (addButton) addButton.addEventListener("click", addTodo);

  render();
  setStatus("Offline");
  connect();

  function load() {
    try {
      const raw = localStorage.getItem(storageKey);
      if (!raw) return { todos: [], pending: [] };
      const parsed = JSON.parse(raw);
      const todos = Array.isArray(parsed.todos)
        ? parsed.todos.map(function (todo, index) { return normalize(todo, index); })
        : [];
      todos.sort(compareOrder);
      const pending = Array.isArray(parsed.pending) ? parsed.pending.filter(validOp) : [];
      for (const op of pending) op.sent = false;
      return { todos: todos, pending: pending };
    } catch (err) {
      return { todos: [], pending: [] };
    }
  }

  function validOp(op) {
    if (!op || typeof op.op_id !== "string" || typeof op.client_id !== "string" || typeof op.type !== "string") {
      return false;
    }
    if (op.type === "splice_title") {
      return isIndex(op.rev) && isIndex(op.index) && isIndex(op.delete) && typeof op.insert === "string";
    }
    if (op.type === "move") return isIndex(op.index);
    return op.type === "create" || op.type === "set_checked" || op.type === "delete" || op.type === "set_title";
  }

  function isIndex(n) {
    return typeof n === "number" && n >= 0 && Math.floor(n) === n;
  }

  function save() {
    localStorage.setItem(
      storageKey,
      JSON.stringify({ todos: state.todos, pending: state.pending })
    );
  }

  function setStatus(text) {
    const el = document.getElementById("todo-status");
    if (el) el.textContent = text;
  }

  function normalize(todo, index) {
    const title = todo.title || "";
    return {
      id: typeof todo.id === "number" ? todo.id : null,
      client_id: todo.client_id || "",
      title: title,
      baseTitle: typeof todo.baseTitle === "string" ? todo.baseTitle : title,
      rev: typeof todo.rev === "number" ? todo.rev : 0,
      checked: !!todo.checked,
      order: typeof todo.order === "number" ? todo.order : (typeof index === "number" ? index : 0),
    };
  }

  function compareOrder(a, b) {
    const ao = typeof a.order === "number" ? a.order : 0;
    const bo = typeof b.order === "number" ? b.order : 0;
    if (ao !== bo) return ao - bo;
    const ai = typeof a.id === "number" ? a.id : 0;
    const bi = typeof b.id === "number" ? b.id : 0;
    return ai - bi;
  }

  function sortTodos() {
    state.todos.sort(compareOrder);
  }

  function nextOrder() {
    let max = -1;
    for (const todo of state.todos) {
      if (typeof todo.order === "number" && todo.order > max) max = todo.order;
    }
    return max + 1;
  }

  function sameRow(todo, id, clientId) {
    if (clientId && todo.client_id === clientId) return true;
    if (typeof id === "number" && todo.id === id) return true;
    return false;
  }

  function payload(op) {
    const body = { type: op.type, op_id: op.op_id, client_id: op.client_id };
    if (typeof op.id === "number") body.id = op.id;
    if (op.type === "create") body.title = op.title || "";
    if (op.type === "create" || op.type === "set_checked") body.checked = !!op.checked;
    if (op.type === "splice_title") {
      body.rev = op.rev;
      body.index = op.index;
      body.delete = op.delete;
      body.insert = op.insert || "";
    }
    if (op.type === "move") body.index = op.index;
    return body;
  }

  function connect() {
    if (disposed) return;
    if (
      socket &&
      (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING)
    ) {
      return;
    }
    const url = new URL(cfg.wsUrl, location.origin);
    const proto = url.protocol === "https:" ? "wss:" : "ws:";
    socket = new WebSocket(proto + "//" + url.host + url.pathname + url.search);
    socket.addEventListener("open", function () {
      startSync();
    });
    socket.addEventListener("message", function (event) {
      onMessage(event.data);
    });
    socket.addEventListener("close", function () {
      if (disposed) return;
      socket = null;
      syncing = false;
      syncBatchIds = new Set();
      for (const op of state.pending) op.sent = false;
      save();
      setStatus("Offline");
      window.clearTimeout(reconnectTimer);
      reconnectTimer = window.setTimeout(connect, 1000);
    });
    socket.addEventListener("error", function () {
      setStatus("Offline");
    });
  }

  function startSync() {
    if (!socket || socket.readyState !== WebSocket.OPEN) return;
    syncing = true;
    setStatus("Syncing");
    const ops = state.pending.filter(function (op) { return op.type !== "set_title"; });
    syncBatchIds = new Set(ops.map(function (op) { return op.op_id; }));
    socket.send(JSON.stringify({ type: "sync", ops: ops.map(payload) }));
    for (const op of ops) op.sent = true;
    save();
  }

  function sendOp(op) {
    if (op.sent || op.type === "set_title") return;
    if (syncing || !socket || socket.readyState !== WebSocket.OPEN) return;
    socket.send(JSON.stringify(payload(op)));
    op.sent = true;
    save();
  }

  function onMessage(raw) {
    let msg;
    try {
      msg = JSON.parse(raw);
    } catch (err) {
      return;
    }
    if (!msg || typeof msg !== "object") return;
    if (msg.type === "snapshot" && Array.isArray(msg.todos)) applySnapshot(msg.todos);
    else if (msg.type === "upsert" && msg.todo) applyUpsert(msg);
    else if (msg.type === "splice_title") applySpliceEvent(msg);
    else if (msg.type === "reject" && msg.todo) applyReject(msg);
    else if (msg.type === "delete") applyDelete(msg);
    else if (msg.type === "move") applyMove(msg);
    else if (msg.type === "error") {
      setStatus(msg.message || "Could not save");
      for (const op of state.pending) op.sent = false;
      save();
    }
  }

  function applySnapshot(todos) {
    const desired = new Map();
    for (const todo of state.todos) {
      const titleOps = state.pending.filter(function (op) {
        return op.client_id === todo.client_id && (op.type === "splice_title" || op.type === "set_title");
      });
      if (!titleOps.length) continue;
      const splices = titleOps.filter(function (op) { return op.type === "splice_title"; });
      if (splices.length) desired.set(todo.client_id, todo.title || "");
      else desired.set(todo.client_id, titleOps[titleOps.length - 1].title || "");
    }
    state.pending = state.pending.filter(function (op) {
      if (syncBatchIds.has(op.op_id)) return false;
      if ((op.type === "splice_title" || op.type === "set_title") && desired.has(op.client_id)) return false;
      return true;
    });
    syncBatchIds = new Set();
    state.todos = todos.map(function (todo, index) { return normalize(todo, index); });
    reapplyPending();
    sortTodos();
    for (const entry of desired) {
      const clientId = entry[0];
      const title = entry[1];
      const todo = state.todos.find(function (item) { return item.client_id === clientId; });
      if (!todo || todo.title === title) continue;
      const sp = diffSplice(todo.title, title);
      if (!sp) continue;
      state.pending.push(spliceOp(todo, sp));
      todo.title = applySplice(todo.title, sp);
    }
    syncing = false;
    save();
    setStatus("Saved");
    render();
    for (const op of state.pending) sendOp(op);
  }

  function applyUpsert(msg) {
    if (msg.op_id) {
      state.pending = state.pending.filter(function (op) { return op.op_id !== msg.op_id; });
    }
    const server = normalize(msg.todo);
    const idx = state.todos.findIndex(function (todo) {
      return sameRow(todo, server.id, server.client_id);
    });
    const clientId = server.client_id || (idx >= 0 ? state.todos[idx].client_id : "");
    state.pending = state.pending.filter(function (op) {
      if (op.type !== "splice_title" || op.client_id !== clientId) return true;
      return op.rev >= server.rev;
    });
    if (idx >= 0) state.todos[idx] = server;
    else state.todos.push(server);
    reapplyPending();
    sortTodos();
    save();
    render();
    const todo = state.todos.find(function (item) { return item.client_id === server.client_id; });
    if (todo) syncFocused(todo);
    if (!syncing) setStatus("Saved");
  }

  function applySpliceEvent(msg) {
    const todo = state.todos.find(function (item) {
      return sameRow(item, msg.id, msg.client_id);
    });
    const mine = msg.op_id
      ? state.pending.find(function (op) { return op.op_id === msg.op_id; })
      : null;
    if (!todo) {
      if (mine) {
        state.pending = state.pending.filter(function (op) { return op.op_id !== msg.op_id; });
        save();
      }
      return;
    }
    if (!isIndex(msg.rev)) return;
    if (mine) {
      if (msg.rev <= todo.rev) {
        state.pending = state.pending.filter(function (op) { return op.op_id !== msg.op_id; });
        save();
        if (!syncing) setStatus("Saved");
        return;
      }
      const head = pendingSplices(todo.client_id)[0];
      if (msg.rev !== todo.rev + 1 || !head || head.op_id !== mine.op_id) {
        resync();
        return;
      }
      todo.baseTitle = applySplice(todo.baseTitle, mine);
      todo.rev = msg.rev;
      if (typeof msg.id === "number") todo.id = msg.id;
      state.pending = state.pending.filter(function (op) { return op.op_id !== mine.op_id; });
      save();
      if (!syncing) setStatus("Saved");
      return;
    }
    if (msg.rev <= todo.rev) return;
    if (msg.rev !== todo.rev + 1) {
      resync();
      return;
    }
    applyRemoteSplice(todo, msg);
  }

  function applyRemoteSplice(todo, msg) {
    const remote = { index: msg.index || 0, delete: msg.delete || 0, insert: msg.insert || "" };
    const ops = pendingSplices(todo.client_id);
    let displaySp = { index: remote.index, delete: remote.delete, insert: remote.insert };
    for (const op of ops) displaySp = transformSplice(displaySp, op, false);
    const text = focusedField(todo);
    const caret = text ? caretOffset(text) : null;
    const nextCaret = caret == null ? null : transformPos(caret, displaySp, true);
    todo.baseTitle = applySplice(todo.baseTitle, remote);
    todo.rev = msg.rev;
    if (typeof msg.id === "number") todo.id = msg.id;
    retargetPending(todo, remote);
    todo.title = foldTitle(todo);
    save();
    render();
    if (text && text.isConnected) {
      setFieldValue(text, todo.title);
      if (document.activeElement !== text) text.focus();
      if (nextCaret != null) setCaret(text, nextCaret);
    }
    for (const op of pendingSplices(todo.client_id)) sendOp(op);
    if (!syncing) setStatus("Saved");
  }

  function applyReject(msg) {
    const had = state.pending.some(function (op) { return op.op_id === msg.op_id; });
    state.pending = state.pending.filter(function (op) { return op.op_id !== msg.op_id; });
    if (!had) {
      save();
      return;
    }
    const server = normalize(msg.todo);
    const todo = state.todos.find(function (item) {
      return sameRow(item, server.id, server.client_id);
    });
    if (!todo || server.rev < todo.rev) {
      save();
      return;
    }
    const text = focusedField(todo);
    const desired = text ? fieldValue(text) : todo.title;
    state.pending = state.pending.filter(function (op) {
      return !(op.type === "splice_title" && op.client_id === todo.client_id);
    });
    const checked = state.pending.find(function (op) {
      return op.type === "set_checked" && op.client_id === todo.client_id;
    });
    todo.id = server.id;
    todo.client_id = server.client_id || todo.client_id;
    todo.rev = server.rev;
    todo.baseTitle = server.title;
    todo.title = server.title;
    todo.checked = checked ? !!checked.checked : server.checked;
    if (desired !== server.title) {
      const sp = diffSplice(server.title, desired);
      if (sp) {
        state.pending.push(spliceOp(todo, sp));
        todo.title = applySplice(server.title, sp);
      }
    }
    save();
    render();
    if (text && text.isConnected) {
      if (fieldValue(text) !== todo.title) setFieldValue(text, todo.title);
      if (document.activeElement !== text) text.focus();
    }
    for (const op of state.pending) sendOp(op);
  }

  function applyDelete(msg) {
    if (msg.op_id) {
      state.pending = state.pending.filter(function (op) { return op.op_id !== msg.op_id; });
    }
    state.todos = state.todos.filter(function (todo) {
      return !sameRow(todo, msg.id, msg.client_id);
    });
    reapplyPending();
    save();
    render();
  }

  function applyMove(msg) {
    if (msg.op_id) {
      state.pending = state.pending.filter(function (op) { return op.op_id !== msg.op_id; });
    }
    moveInList(msg.client_id, msg.id, msg.index);
    reapplyPending();
    sortTodos();
    save();
    render();
    if (!syncing) setStatus("Saved");
  }

  function moveInList(clientId, id, index) {
    const from = state.todos.findIndex(function (todo) {
      return sameRow(todo, id, clientId);
    });
    if (from < 0 || !isIndex(index)) return;
    const item = state.todos.splice(from, 1)[0];
    const at = Math.min(index, state.todos.length);
    state.todos.splice(at, 0, item);
    state.todos.forEach(function (todo, position) { todo.order = position; });
  }

  function resync() {
    if (syncing || !socket) return;
    socket.close();
  }

  function reapplyPending() {
    for (const op of state.pending) {
      if (op.type === "create") {
        if (!state.todos.some(function (todo) { return todo.client_id === op.client_id; })) {
          state.todos.push({
            id: typeof op.id === "number" ? op.id : null,
            client_id: op.client_id,
            title: op.title || "",
            baseTitle: op.title || "",
            rev: 0,
            checked: !!op.checked,
            order: nextOrder(),
          });
        }
      } else if (op.type === "set_checked") {
        const todo = state.todos.find(function (item) {
          return sameRow(item, op.id, op.client_id);
        });
        if (!todo) continue;
        todo.checked = !!op.checked;
      } else if (op.type === "delete") {
        state.todos = state.todos.filter(function (todo) {
          return !sameRow(todo, op.id, op.client_id);
        });
      } else if (op.type === "move") {
        moveInList(op.client_id, op.id, op.index);
      }
    }
    for (const todo of state.todos) {
      if (pendingSplices(todo.client_id).length) todo.title = foldTitle(todo);
    }
  }

  function addTodo() {
    const clientId = crypto.randomUUID();
    const op = {
      type: "create",
      op_id: crypto.randomUUID(),
      id: null,
      client_id: clientId,
      title: "",
      checked: false,
      sent: false,
    };
    state.todos.push({
      id: null,
      client_id: clientId,
      title: "",
      baseTitle: "",
      rev: 0,
      checked: false,
      order: nextOrder(),
    });
    state.pending.push(op);
    focusClientId = clientId;
    save();
    render();
    sendOp(op);
  }

  function pendingSplices(clientId) {
    return state.pending.filter(function (op) {
      return op.type === "splice_title" && op.client_id === clientId;
    });
  }

  function spliceOp(todo, sp) {
    return {
      type: "splice_title",
      op_id: crypto.randomUUID(),
      id: todo.id,
      client_id: todo.client_id,
      rev: (todo.rev || 0) + pendingSplices(todo.client_id).length,
      index: sp.index,
      delete: sp.delete,
      insert: sp.insert,
      sent: false,
    };
  }

  function enqueueSplice(todo, sp) {
    const op = spliceOp(todo, sp);
    state.pending.push(op);
    save();
    sendOp(op);
  }

  function retargetPending(todo, remote) {
    let rev = todo.rev;
    for (const op of pendingSplices(todo.client_id)) {
      const next = transformSplice(op, remote, true);
      op.index = next.index;
      op.delete = next.delete;
      op.insert = next.insert;
      op.rev = rev;
      rev += 1;
      if (op.sent) {
        op.op_id = crypto.randomUUID();
        op.sent = false;
      }
    }
  }

  function foldTitle(todo) {
    let title = todo.baseTitle || "";
    for (const op of pendingSplices(todo.client_id)) title = applySplice(title, op);
    return title;
  }

  function deleteTodo(clientId) {
    const todo = state.todos.find(function (item) { return item.client_id === clientId; });
    const create = state.pending.find(function (op) {
      return op.type === "create" && op.client_id === clientId;
    });
    state.todos = state.todos.filter(function (item) { return item.client_id !== clientId; });
    if (create && !create.sent) {
      state.pending = state.pending.filter(function (op) { return op.client_id !== clientId; });
      save();
      render();
      return;
    }
    state.pending = state.pending.filter(function (op) {
      if (op.client_id !== clientId) return true;
      if (op.type === "delete") return true;
      return op.sent;
    });
    const op = {
      type: "delete",
      op_id: crypto.randomUUID(),
      id: todo ? todo.id : null,
      client_id: clientId,
      sent: false,
    };
    state.pending.push(op);
    save();
    render();
    sendOp(op);
  }

  function codePoints(text) {
    return Array.from(text || "");
  }

  // Same rules as `transform_pos` / `apply_splice` in sync.rs. Indexes are Unicode scalars.
  function applySplice(title, sp) {
    const chars = codePoints(title || "");
    const index = Math.max(0, Math.min(sp.index, chars.length));
    const del = Math.max(0, Math.min(sp.delete, chars.length - index));
    chars.splice(index, del, ...codePoints(sp.insert || ""));
    return chars.join("");
  }

  function diffSplice(before, after) {
    const oldChars = codePoints(before || "");
    const nextChars = codePoints(after || "");
    let start = 0;
    while (start < oldChars.length && start < nextChars.length && oldChars[start] === nextChars[start]) start++;
    let endOld = oldChars.length;
    let endNext = nextChars.length;
    while (endOld > start && endNext > start && oldChars[endOld - 1] === nextChars[endNext - 1]) {
      endOld--;
      endNext--;
    }
    if (start === endOld && start === endNext) return null;
    return { index: start, delete: endOld - start, insert: nextChars.slice(start, endNext).join("") };
  }

  function transformPos(pos, already, insertAfter) {
    const start = already.index;
    const end = start + already.delete;
    const inserted = codePoints(already.insert || "").length;
    if (already.delete === 0 && pos === start) return insertAfter ? pos + inserted : pos;
    if (pos <= start) return pos;
    if (pos >= end) {
      const shifted = pos + inserted - already.delete;
      return shifted < 0 ? 0 : shifted;
    }
    return start;
  }

  function transformSplice(op, already, insertAfter) {
    const start = transformPos(op.index, already, insertAfter);
    const end = transformPos(op.index + op.delete, already, insertAfter);
    return {
      index: start,
      delete: end > start ? end - start : 0,
      insert: op.insert || "",
    };
  }

  function todoFrom(input) {
    const li = input.closest("li");
    if (!li) return null;
    return state.todos.find(function (todo) { return todo.client_id === li.dataset.clientId; }) || null;
  }

  function fieldOf(target) {
    if (!target || !target.closest) return null;
    const text = target.closest("[data-autogrow]");
    if (!text || !list.contains(text)) return null;
    return text;
  }

  function focusedField(todo) {
    const editing = document.activeElement;
    if (!editing || !list || !list.contains(editing) || !editing.matches("[data-autogrow]")) return null;
    const li = editing.closest("li");
    if (!li || li.dataset.clientId !== todo.client_id) return null;
    return editing;
  }

  function syncFocused(todo) {
    const text = focusedField(todo);
    if (!text) return;
    const current = fieldValue(text);
    if (current === todo.title) return;
    const caret = caretOffset(text);
    const sp = diffSplice(current, todo.title);
    setFieldValue(text, todo.title);
    if (document.activeElement !== text) text.focus();
    if (caret != null && sp) setCaret(text, transformPos(caret, sp, true));
  }

  function fieldValue(text) {
    if (window.larivAutogrow) return window.larivAutogrow.read(text);
    return (text.textContent || "").replace(/\u00a0/g, " ");
  }

  function setFieldValue(text, value) {
    const next = value || "";
    if (window.larivAutogrow) window.larivAutogrow.write(text, next);
    else if (fieldValue(text) !== next) text.textContent = next;
  }

  function caretOffset(el) {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) return null;
    const range = sel.getRangeAt(0);
    if (!el.contains(range.endContainer)) return null;
    const pre = range.cloneRange();
    pre.selectNodeContents(el);
    pre.setEnd(range.endContainer, range.endOffset);
    return codePoints(pre.toString()).length;
  }

  function setCaret(el, offset) {
    const sel = window.getSelection();
    if (!sel) return;
    const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
    let remaining = offset;
    let node = walker.nextNode();
    const range = document.createRange();
    if (!node) {
      range.selectNodeContents(el);
      range.collapse(false);
    } else {
      while (node) {
        const chars = codePoints(node.textContent);
        if (remaining <= chars.length) {
          range.setStart(node, chars.slice(0, remaining).join("").length);
          range.collapse(true);
          break;
        }
        remaining -= chars.length;
        const next = walker.nextNode();
        if (!next) {
          range.setStart(node, node.textContent.length);
          range.collapse(true);
          break;
        }
        node = next;
      }
    }
    sel.removeAllRanges();
    sel.addRange(range);
  }

  function focusField(el) {
    el.focus();
    const range = document.createRange();
    range.selectNodeContents(el);
    range.collapse(false);
    const sel = window.getSelection();
    if (!sel) return;
    sel.removeAllRanges();
    sel.addRange(range);
  }

  function onInput(event) {
    const text = fieldOf(event.target);
    if (!text) return;
    const todo = todoFrom(text);
    if (!todo) return;
    const next = fieldValue(text);
    const sp = diffSplice(todo.title, next);
    todo.title = next;
    if (!sp) return;
    enqueueSplice(todo, sp);
  }

  function onChange(event) {
    const box = event.target;
    if (!(box instanceof HTMLInputElement) || box.type !== "checkbox") return;
    const todo = todoFrom(box);
    if (!todo) return;
    todo.checked = box.checked;
    queueChecked(todo);
  }

  function onClick(event) {
    const btn = event.target.closest("[data-delete]");
    if (!btn || !list.contains(btn)) return;
    const li = btn.closest("li");
    if (!li) return;
    deleteTodo(li.dataset.clientId);
  }

  function queueChecked(todo) {
    let op = state.pending.find(function (item) {
      return item.type === "set_checked" && item.client_id === todo.client_id && !item.sent;
    });
    if (!op) {
      op = {
        type: "set_checked",
        op_id: crypto.randomUUID(),
        id: todo.id,
        client_id: todo.client_id,
        checked: todo.checked,
        sent: false,
      };
      state.pending.push(op);
    } else {
      op.checked = todo.checked;
      op.id = todo.id;
    }
    save();
    sendOp(op);
  }

  function queueMove(todo, index) {
    let op = state.pending.find(function (item) {
      return item.type === "move" && item.client_id === todo.client_id && !item.sent;
    });
    if (!op) {
      op = {
        type: "move",
        op_id: crypto.randomUUID(),
        id: todo.id,
        client_id: todo.client_id,
        index: index,
        sent: false,
      };
      state.pending.push(op);
    } else {
      op.index = index;
      op.id = todo.id;
    }
    save();
    sendOp(op);
  }

  function onDragDown(event) {
    const handle = event.target.closest("[data-drag-handle]");
    if (!handle || !list.contains(handle) || event.button > 0) return;
    const li = handle.closest("li");
    if (!li) return;
    drag = {
      id: li.dataset.clientId,
      pointer: event.pointerId,
      active: false,
      y: event.clientY,
    };
    handle.setPointerCapture(event.pointerId);
  }

  function onDragMove(event) {
    if (!drag || event.pointerId !== drag.pointer) return;
    if (!drag.active && Math.abs(event.clientY - drag.y) < 4) return;
    drag.active = true;
    const dragged = rowElement(drag.id);
    if (!dragged) return;
    dragged.classList.add("opacity-40");
    const over = rowAt(event.clientY, dragged);
    if (!over) return;
    const rect = over.getBoundingClientRect();
    const after = event.clientY > rect.top + rect.height / 2;
    if (after) {
      if (dragged !== over.nextElementSibling) over.after(dragged);
    } else if (dragged !== over.previousElementSibling) {
      over.before(dragged);
    }
  }

  function onDragUp(event) {
    if (!drag || event.pointerId !== drag.pointer) return;
    const id = drag.id;
    const active = drag.active;
    drag = null;
    const dragged = rowElement(id);
    if (dragged) dragged.classList.remove("opacity-40");
    if (!active) return;
    const index = Array.from(list.children).findIndex(function (li) {
      return li.dataset.clientId === id;
    });
    const from = state.todos.findIndex(function (todo) { return todo.client_id === id; });
    if (from < 0 || index < 0 || from === index) {
      render();
      return;
    }
    const item = state.todos.splice(from, 1)[0];
    state.todos.splice(index, 0, item);
    state.todos.forEach(function (todo, position) { todo.order = position; });
    queueMove(item, index);
    render();
  }

  function rowAt(y, ignore) {
    for (const li of list.children) {
      if (li === ignore) continue;
      const rect = li.getBoundingClientRect();
      if (y >= rect.top && y <= rect.bottom) return li;
    }
    return null;
  }

  function heroIcon(name) {
    const icon = document.createElement("span");
    icon.className = "heroicon heroicon-sm";
    icon.style.setProperty(
      "--heroicon-url",
      "url('https://api.iconify.design/heroicons/" + name + ".svg')"
    );
    icon.setAttribute("aria-hidden", "true");
    return icon;
  }

  function buildRow(todo) {
    const li = document.createElement("li");
    li.className = "flex items-start gap-2";
    li.dataset.clientId = todo.client_id;
    const rail = document.createElement("div");
    rail.className = "flex flex-col items-center shrink-0";
    const box = document.createElement("input");
    box.type = "checkbox";
    box.className = "checkbox checkbox-sm mt-2";
    box.checked = !!todo.checked;
    box.setAttribute("aria-label", "Checked");
    const handle = document.createElement("button");
    handle.type = "button";
    handle.className = "btn btn-ghost btn-xs btn-square cursor-grab touch-none";
    handle.dataset.dragHandle = "1";
    handle.setAttribute("aria-label", "Drag to reorder");
    handle.append(heroIcon("bars-3"));
    rail.append(box, handle);
    const field = document.createElement("div");
    field.className = "autogrow-field min-w-0 flex-1";
    field.dataset.autogrowField = "";
    const text = document.createElement("span");
    text.className = "autogrow-text input input-bordered input-sm w-full";
    text.setAttribute("contenteditable", "plaintext-only");
    text.setAttribute("role", "textbox");
    text.setAttribute("aria-multiline", "true");
    text.setAttribute("aria-label", "Todo");
    text.dataset.autogrow = "";
    text.spellcheck = true;
    setFieldValue(text, todo.title);
    if (window.larivAutogrow) window.larivAutogrow.bind(text);
    field.append(text);
    const del = document.createElement("button");
    del.type = "button";
    del.className = "btn btn-ghost btn-sm btn-square shrink-0 mt-1";
    del.dataset.delete = "1";
    del.setAttribute("aria-label", "Delete");
    del.append(heroIcon("trash"));
    li.append(rail, field, del);
    return li;
  }

  function render() {
    if (!list || drag) return;
    const editing = document.activeElement;
    const keepFocus = editing && list.contains(editing) && editing.matches("[data-autogrow]");
    const caret = keepFocus ? caretOffset(editing) : null;
    const seen = new Set();
    for (const todo of state.todos) {
      if (!todo.client_id) continue;
      seen.add(todo.client_id);
      let li = rowElement(todo.client_id);
      if (!li) {
        li = buildRow(todo);
        list.appendChild(li);
      }
      const box = li.querySelector('input[type="checkbox"]');
      const text = li.querySelector("[data-autogrow]");
      if (box && box.checked !== !!todo.checked) box.checked = !!todo.checked;
      if (text && text !== editing && fieldValue(text) !== todo.title) setFieldValue(text, todo.title);
    }
    let previous = null;
    for (const todo of state.todos) {
      if (!todo.client_id) continue;
      const li = rowElement(todo.client_id);
      if (!li) continue;
      if (li.previousElementSibling !== previous) {
        if (previous) previous.after(li);
        else list.prepend(li);
      }
      previous = li;
    }
    for (const li of Array.from(list.children)) {
      if (!seen.has(li.dataset.clientId)) li.remove();
    }
    if (focusClientId) {
      const li = rowElement(focusClientId);
      const text = li && li.querySelector("[data-autogrow]");
      if (text) focusField(text);
      focusClientId = null;
      return;
    }
    if (keepFocus && editing.isConnected && document.activeElement !== editing) {
      editing.focus();
      if (caret != null) setCaret(editing, caret);
    }
  }

  function rowElement(clientId) {
    return list.querySelector('li[data-client-id="' + CSS.escape(clientId) + '"]');
  }
})();
