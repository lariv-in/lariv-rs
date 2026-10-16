// Autogrowing text area: a contenteditable span that exposes .value and
// input/change the way a text field does. A sibling [data-autogrow-value]
// hidden input, when present, carries the form name.
(function () {
  function read(el) {
    if (el.querySelector("*")) {
      return (el.innerText || "").replace(/\u00a0/g, " ").replace(/\n$/, "");
    }
    return (el.textContent || "").replace(/\u00a0/g, " ");
  }

  function hiddenInput(el) {
    const field = el.closest("[data-autogrow-field]");
    if (!field) return null;
    return field.querySelector("[data-autogrow-value]");
  }

  function write(el, value) {
    const next = value == null ? "" : String(value);
    if (read(el) !== next) el.textContent = next;
    const input = hiddenInput(el);
    if (input && input.value !== next) input.value = next;
  }

  function caretOffset(el) {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) return null;
    const range = sel.getRangeAt(0);
    if (!el.contains(range.endContainer)) return null;
    const pre = range.cloneRange();
    pre.selectNodeContents(el);
    pre.setEnd(range.endContainer, range.endOffset);
    return pre.toString().length;
  }

  function setCaret(el, offset) {
    const sel = window.getSelection();
    if (!sel) return;
    const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
    let remaining = offset;
    let node = walker.nextNode();
    if (!node) {
      const range = document.createRange();
      range.selectNodeContents(el);
      range.collapse(false);
      sel.removeAllRanges();
      sel.addRange(range);
      return;
    }
    while (node) {
      const len = node.textContent.length;
      if (remaining <= len) {
        const range = document.createRange();
        range.setStart(node, remaining);
        range.collapse(true);
        sel.removeAllRanges();
        sel.addRange(range);
        return;
      }
      remaining -= len;
      node = walker.nextNode();
    }
  }

  function insertPlain(el, text) {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0 || !el.contains(sel.anchorNode)) {
      el.append(document.createTextNode(text));
    } else {
      const range = sel.getRangeAt(0);
      range.deleteContents();
      const node = document.createTextNode(text);
      range.insertNode(node);
      range.setStartAfter(node);
      range.collapse(true);
      sel.removeAllRanges();
      sel.addRange(range);
    }
    el.dispatchEvent(
      new InputEvent("input", { bubbles: true, inputType: "insertText", data: text })
    );
  }

  function mirror(el) {
    const input = hiddenInput(el);
    if (!input) return;
    const value = read(el);
    if (input.value !== value) input.value = value;
  }

  function bind(el) {
    if (!el || el.nodeType !== 1 || !el.matches("[data-autogrow]") || el.dataset.autogrowBound) {
      return;
    }
    el.dataset.autogrowBound = "1";
    let focusedValue = read(el);
    try {
      el.contentEditable = "plaintext-only";
    } catch (err) {
      el.contentEditable = "true";
    }
    if (el.contentEditable !== "plaintext-only") el.contentEditable = "true";
    Object.defineProperty(el, "value", {
      configurable: true,
      enumerable: true,
      get: function () {
        return read(el);
      },
      set: function (value) {
        write(el, value);
      },
    });
    el.addEventListener("keydown", function (event) {
      if (event.key !== "Enter" || event.isComposing) return;
      event.preventDefault();
      insertPlain(el, "\n");
    });
    el.addEventListener("paste", function (event) {
      event.preventDefault();
      const text = event.clipboardData ? event.clipboardData.getData("text/plain") : "";
      if (text) insertPlain(el, text);
    });
    el.addEventListener("beforeinput", function (event) {
      if (event.inputType && event.inputType.indexOf("format") === 0) event.preventDefault();
    });
    el.addEventListener("input", function () {
      if (el.querySelector("*")) {
        const offset = caretOffset(el);
        el.textContent = read(el);
        if (offset != null) setCaret(el, offset);
      }
      mirror(el);
    });
    el.addEventListener("focusin", function () {
      focusedValue = read(el);
    });
    el.addEventListener("focusout", function () {
      mirror(el);
      const value = read(el);
      if (value === focusedValue) return;
      focusedValue = value;
      el.dispatchEvent(new Event("change", { bubbles: true }));
      const input = hiddenInput(el);
      if (input) input.dispatchEvent(new Event("change", { bubbles: true }));
    });
  }

  function scan(root) {
    if (!root || !root.querySelectorAll) return;
    if (root.matches && root.matches("[data-autogrow]")) bind(root);
    root.querySelectorAll("[data-autogrow]").forEach(bind);
  }

  function start() {
    scan(document);
    document.addEventListener("focusin", function (event) {
      const el = event.target && event.target.closest && event.target.closest("[data-autogrow]");
      if (el) bind(el);
    });
    document.addEventListener("htmx:afterSwap", function (event) {
      scan(event.detail && event.detail.target ? event.detail.target : document);
    });
    if (typeof MutationObserver === "undefined") return;
    new MutationObserver(function (records) {
      for (const record of records) {
        for (const node of record.addedNodes) scan(node);
      }
    }).observe(document.documentElement, { childList: true, subtree: true });
  }

  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", start);
  else start();

  window.larivAutogrow = { bind: bind, read: read, write: write };
})();
