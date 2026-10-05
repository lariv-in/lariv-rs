function larivTextToIso(s, type) {
  s = String(s || "").trim();
  if (!s) return "";
  const dmy = s.match(/^(\d{2})\/(\d{2})\/(\d{4})(?:\s+(\d{2}):(\d{2})(?::(\d{2}))?)?$/);
  if (dmy) {
    const isoDate = dmy[3] + "-" + dmy[2] + "-" + dmy[1];
    if (type === "date") return isoDate;
    return isoDate + "T" + (dmy[4] || "00") + ":" + (dmy[5] || "00") + ":" + (dmy[6] || "00");
  }
  if (type === "date") {
    const iso = s.match(/^(\d{4})-(\d{2})-(\d{2})/);
    return iso ? iso[1] + "-" + iso[2] + "-" + iso[3] : "";
  }
  const isoDt = s.match(/^(\d{4}-\d{2}-\d{2})[T ](\d{2}:\d{2}(?::(\d{2}))?)/);
  if (!isoDt) return "";
  const tm = isoDt[2].length === 5 ? isoDt[2] + ":00" : isoDt[2].slice(0, 8);
  return isoDt[1] + "T" + tm;
}
function larivSplitPickersToText(wrap) {
  const text = wrap.querySelector("[data-lariv-date-text]");
  const datePicker = wrap.querySelector('[data-lariv-picker][type="date"]');
  const timePicker = wrap.querySelector('[data-lariv-picker][type="time"]');
  if (!text || !datePicker || !timePicker) return;
  const d = datePicker.value;
  if (!d) return;
  let tm = timePicker.value || "00:00:00";
  if (tm.length === 5) tm += ":00";
  else if (tm.length > 8) tm = tm.slice(0, 8);
  const dmy = d.split("-").reverse().join("/");
  text.value = dmy + " " + tm;
  text.dispatchEvent(new Event("input", { bubbles: true }));
  text.dispatchEvent(new Event("change", { bubbles: true }));
}
function larivPickerToText(picker) {
  const wrap = picker.closest("[data-lariv-date-wrap]");
  const text = wrap && wrap.querySelector("[data-lariv-date-text]");
  if (!wrap || !text) return;
  const timePicker = wrap.querySelector('[data-lariv-picker][type="time"]');
  const datePicker = wrap.querySelector('[data-lariv-picker][type="date"]');
  if (datePicker && timePicker) {
    larivSplitPickersToText(wrap);
    return;
  }
  const v = picker.value;
  // Native pickers often commit minute-precision values; with step="1" the
  // hidden input stays invalid and .value is "". Keep the visible field as-is.
  if (!v) return;
  if (picker.type === "date") { text.value = v.split("-").reverse().join("/"); }
  else {
    const parts = v.split("T");
    const dmy = (parts[0] || "").split("-").reverse().join("/");
    let tm = parts[1] || "00:00:00";
    if (tm.length === 5) tm += ":00";
    else tm = tm.slice(0, 8);
    text.value = dmy + " " + tm;
  }
  text.dispatchEvent(new Event("input", { bubbles: true }));
  text.dispatchEvent(new Event("change", { bubbles: true }));
}
function larivOpenPicker(btn) {
  const wrap = btn.closest("[data-lariv-date-wrap]");
  if (!wrap) return;
  const text = wrap.querySelector("[data-lariv-date-text]");
  const datePicker = wrap.querySelector('[data-lariv-picker][type="date"]');
  const timePicker = wrap.querySelector('[data-lariv-picker][type="time"]');
  if (datePicker && timePicker) {
    const iso = larivTextToIso(text && text.value, "datetime-local");
    const parts = iso.split("T");
    datePicker.value = parts[0] || "";
    let tm = parts[1] || "00:00:00";
    if (tm.length === 5) tm += ":00";
    else if (tm.length > 8) tm = tm.slice(0, 8);
    timePicker.value = tm;
    const openTime = () => {
      try { timePicker.showPicker(); } catch (err) { timePicker.click(); }
    };
    const onDateChange = () => {
      larivSplitPickersToText(wrap);
      openTime();
    };
    datePicker.addEventListener("change", onDateChange, { once: true });
    try { datePicker.showPicker(); } catch (err) { datePicker.click(); }
    return;
  }
  const picker = wrap.querySelector("[data-lariv-picker]");
  if (!picker) return;
  picker.value = larivTextToIso(text && text.value, picker.type);
  try { picker.showPicker(); } catch (err) { picker.click(); }
}
