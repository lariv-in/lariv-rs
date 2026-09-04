use crate::grapesjs::{
    GrapesJsBlock, GrapesJsCapability, GrapesJsComponent, GrapesJsRegistrar, GrapesJsTheme,
    GrapesJsTrait,
};
use serde_json::{Value, json};

const DEFAULT_THEME_FONTS_CSS: &str = "https://fonts.googleapis.com/css2?family=Fraunces:opsz,wght@9..144,500;600;700&family=Manrope:wght@400;500;600;700&display=swap";
const KDS_THEME_FONTS_CSS: &str = "https://fonts.googleapis.com/css2?family=Manrope:wght@400;500;600;700&family=Poppins:ital,wght@0,400;0,500;0,600;1,400;1,500&family=Rajdhani:wght@300;400;500;600;700&display=swap";

fn block_html(label: &str, category: &str, html: &str) -> GrapesJsBlock {
    GrapesJsBlock::html(label, category, html.trim())
}

fn is_component_type(type_id: &str) -> Value {
    Value::String(format!(
        "return !!(el && el.getAttribute && el.getAttribute('data-gjs-type') === {:?});",
        type_id
    ))
}

fn component_entry(type_id: &str, model: Value, view: Option<Value>) -> GrapesJsComponent {
    GrapesJsComponent {
        extend: String::new(),
        is_component: Some(is_component_type(type_id)),
        model: Some(model),
        view,
    }
}

fn src_url_trait(name: &str, label: &str) -> Value {
    json!({
        "type": "p_website.src-url",
        "name": name,
        "label": label,
    })
}

fn id_trait() -> Value {
    json!({"type": "text", "name": "id", "label": "ID"})
}

fn link_target_traits() -> Value {
    json!([
        {"type": "text", "name": "href", "label": "URL"},
        {"type": "text", "name": "title", "label": "Title"},
        {"type": "select", "name": "target", "label": "Target", "options": [
            {"id": "", "label": "Same tab"},
            {"id": "_blank", "label": "New tab"},
        ]},
    ])
}

fn counter_attrs_traits() -> Value {
    json!([
        {"type": "number", "name": "data-value", "label": "Value"},
        {"type": "number", "name": "data-duration", "label": "Duration (ms)"},
        {"type": "text", "name": "data-suffix", "label": "Suffix"},
    ])
}

const ACCORDION_SCRIPT: &str = r#"
var root = this;
root.addEventListener('click', function (ev) {
  var btn = ev.target.closest('.gjs-accordion-trigger');
  if (!btn || !root.contains(btn)) return;
  var item = btn.closest('.gjs-accordion-item');
  if (!item) return;
  var panel = item.querySelector('.gjs-accordion-panel');
  if (!panel) return;
  var open = !panel.hasAttribute('hidden');
  root.querySelectorAll('.gjs-accordion-panel').forEach(function (p) { p.setAttribute('hidden', ''); });
  if (!open) panel.removeAttribute('hidden');
});
"#;

const DROPDOWN_SCRIPT: &str = r#"
var root = this;
var trigger = root.querySelector('.gjs-dropdown-trigger');
var menu = root.querySelector('.gjs-dropdown-menu');
if (trigger && menu) {
  trigger.addEventListener('click', function () {
    if (menu.hasAttribute('hidden')) menu.removeAttribute('hidden');
    else menu.setAttribute('hidden', '');
  });
}
"#;

const SLIDER_SCRIPT: &str = r#"
var root = this;
var slides = Array.prototype.slice.call(root.querySelectorAll('.gjs-slider-slide'));
var idx = Math.max(0, slides.findIndex(function (s) { return s.classList.contains('is-active'); }));
function show(i) {
  if (!slides.length) return;
  idx = (i + slides.length) % slides.length;
  slides.forEach(function (s, n) {
    if (n === idx) { s.classList.add('is-active'); s.removeAttribute('hidden'); }
    else { s.classList.remove('is-active'); s.setAttribute('hidden', ''); }
  });
}
show(idx < 0 ? 0 : idx);
var prev = root.querySelector('.gjs-slider-prev');
var next = root.querySelector('.gjs-slider-next');
if (prev) prev.addEventListener('click', function () { show(idx - 1); });
if (next) next.addEventListener('click', function () { show(idx + 1); });
"#;

const TABS_SCRIPT: &str = r#"
var root = this;
var tabs = Array.prototype.slice.call(root.querySelectorAll('.gjs-tab'));
var panels = Array.prototype.slice.call(root.querySelectorAll('.gjs-tab-panel'));
function activate(i) {
  tabs.forEach(function (t, n) { t.classList.toggle('is-active', n === i); });
  panels.forEach(function (p, n) {
    if (n === i) { p.classList.add('is-active'); p.removeAttribute('hidden'); }
    else { p.classList.remove('is-active'); p.setAttribute('hidden', ''); }
  });
}
tabs.forEach(function (tab, i) {
  tab.addEventListener('click', function () { activate(i); });
});
"#;

const TOGGLEABLE_SCRIPT: &str = r#"
var root = this;
var trigger = root.querySelector('.gjs-toggleable-trigger');
var panel = root.querySelector('.gjs-toggleable-panel');
if (trigger && panel) {
  trigger.addEventListener('click', function () {
    if (panel.hasAttribute('hidden')) panel.removeAttribute('hidden');
    else panel.setAttribute('hidden', '');
  });
}
"#;

const NUMBER_COUNTER_SCRIPT: &str = r#"
var root = this;
var target = parseFloat(root.getAttribute('data-value') || '0') || 0;
var duration = parseInt(root.getAttribute('data-duration') || '1500', 10) || 1500;
var suffix = root.getAttribute('data-suffix') || '';
var el = root.querySelector('.gjs-number-counter-value');
if (!el) return;
var start = null;
function frame(ts) {
  if (start === null) start = ts;
  var p = Math.min(1, (ts - start) / duration);
  el.textContent = Math.round(target * p) + suffix;
  if (p < 1) requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
"#;

const BAR_COUNTER_SCRIPT: &str = r#"
var root = this;
var target = parseFloat(root.getAttribute('data-value') || '0') || 0;
var duration = parseInt(root.getAttribute('data-duration') || '1200', 10) || 1200;
var suffix = root.getAttribute('data-suffix') || '';
var fill = root.querySelector('.gjs-bar-counter-fill');
var valueEl = root.querySelector('.gjs-bar-counter-value');
var start = null;
function frame(ts) {
  if (start === null) start = ts;
  var p = Math.min(1, (ts - start) / duration);
  var v = Math.round(target * p);
  if (fill) fill.style.width = v + '%';
  if (valueEl) valueEl.textContent = v + suffix;
  if (p < 1) requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
"#;

const CIRCLE_COUNTER_SCRIPT: &str = r#"
var root = this;
var target = parseFloat(root.getAttribute('data-value') || '0') || 0;
var duration = parseInt(root.getAttribute('data-duration') || '1200', 10) || 1200;
var suffix = root.getAttribute('data-suffix') || '';
var fg = root.querySelector('.gjs-circle-counter-fg');
var valueEl = root.querySelector('.gjs-circle-counter-value');
var start = null;
function frame(ts) {
  if (start === null) start = ts;
  var p = Math.min(1, (ts - start) / duration);
  var v = Math.round(target * p);
  if (fg) fg.setAttribute('stroke-dasharray', v + ', 100');
  if (valueEl) valueEl.textContent = v + suffix;
  if (p < 1) requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
"#;

const COUNTDOWN_SCRIPT: &str = r#"
var root = this;
var targetAttr = root.getAttribute('data-target');
var target = targetAttr ? new Date(targetAttr).getTime() : (Date.now() + 7 * 24 * 60 * 60 * 1000);
var daysEl = root.querySelector('.gjs-countdown-days');
var hoursEl = root.querySelector('.gjs-countdown-hours');
var minsEl = root.querySelector('.gjs-countdown-mins');
var secsEl = root.querySelector('.gjs-countdown-secs');
function pad(n) { return (n < 10 ? '0' : '') + n; }
function tick() {
  var diff = Math.max(0, target - Date.now());
  var s = Math.floor(diff / 1000);
  var days = Math.floor(s / 86400); s -= days * 86400;
  var hours = Math.floor(s / 3600); s -= hours * 3600;
  var mins = Math.floor(s / 60); s -= mins * 60;
  if (daysEl) daysEl.textContent = pad(days);
  if (hoursEl) hoursEl.textContent = pad(hours);
  if (minsEl) minsEl.textContent = pad(mins);
  if (secsEl) secsEl.textContent = pad(s);
}
tick();
setInterval(tick, 1000);
"#;

const HEADING_INIT: &str = r#"
this.on('change:attributes:data-level', function () {
  var level = this.getAttributes()['data-level'] || '2';
  var tag = 'h' + level;
  if (this.get('tagName') !== tag) this.set('tagName', tag);
});
"#;

const HERO_MODEL_INIT: &str = r#"
var applyHero = function () {
  var attrs = this.getAttributes() || {};
  var showMedia = attrs['data-show-media'] === 'true' || attrs['data-show-media'] === true;
  var showButton = attrs['data-show-button'] === 'true' || attrs['data-show-button'] === true;
  var media = this.find('[data-gjs-type="p_website.hero-media"]')[0]
    || this.find('.gjs-hero-media')[0];
  var button = this.find('.gjs-hero-button')[0]
    || this.find('.gjs-button')[0];
  if (media) {
    if (showMedia) media.removeClass('hidden');
    else media.addClass('hidden');
  }
  if (button) {
    if (showButton) button.removeClass('hidden');
    else button.addClass('hidden');
  }
};
this.on('change:attributes:data-show-media change:attributes:data-show-button', applyHero);
applyHero.call(this);
"#;

const CTA_MODEL_INIT: &str = r#"
var applyCta = function () {
  var attrs = this.getAttributes() || {};
  var showButton = attrs['data-show-button'] === 'true' || attrs['data-show-button'] === true;
  var button = this.find('.gjs-cta-button')[0]
    || this.find('.gjs-button')[0];
  if (button) {
    if (showButton) button.removeClass('hidden');
    else button.addClass('hidden');
  }
};
this.on('change:attributes:data-show-button', applyCta);
applyCta.call(this);
"#;

const SECTION_MODEL_INIT: &str = r#"
var applySectionBg = function () {
  var attrs = this.getAttributes() || {};
  var bg = attrs['data-section-bg'] || 'base';
  var cls = this.getClasses() || [];
  cls = cls.filter(function (c) {
    return c.indexOf('bg-base-') !== 0 && c.indexOf('bg-primary') !== 0 && c.indexOf('bg-neutral') !== 0;
  });
  if (bg === 'base-200') cls.push('bg-base-200');
  else if (bg === 'primary') cls.push('bg-primary', 'text-primary-content');
  else if (bg === 'neutral') cls.push('bg-neutral', 'text-neutral-content');
  else if (bg === 'base-100') cls.push('bg-base-100');
  if (cls.indexOf('section') < 0) cls.push('section');
  this.setClass(cls);
};
this.on('change:attributes:data-section-bg', applySectionBg);
applySectionBg.call(this);
"#;

const SECTION_HEADER_INIT: &str = r#"
var applySectionHeader = function () {
  var attrs = this.getAttributes() || {};
  var align = attrs['data-align'] || 'center';
  var style = attrs['data-header-style'] || 'default';
  var cls = this.getClasses() || [];
  cls = cls.filter(function (c) {
    return c !== 'text-center' && c !== 'text-left' && c !== 'section-header--tight';
  });
  cls.push(align === 'left' ? 'text-left' : 'text-center');
  if (style === 'tight') cls.push('section-header--tight');
  if (cls.indexOf('section-header') < 0) cls.push('section-header');
  this.setClass(cls);
  var lead = this.find('.gjs-section-header-lead')[0];
  if (lead) {
    var text = (lead.get('content') || '').trim();
    if (!text) lead.addClass('hidden');
    else lead.removeClass('hidden');
  }
};
this.on('change:attributes:data-align change:attributes:data-header-style', applySectionHeader);
this.on('change:content', applySectionHeader);
applySectionHeader.call(this);
"#;

const NAVBAR_VARIANT_INIT: &str = r#"
var applyNavbarVariant = function () {
  var attrs = this.getAttributes() || {};
  var variant = attrs['data-variant'] || 'light';
  var cls = this.getClasses() || [];
  cls = cls.filter(function (c) {
    return c !== 'bg-base-100' && c !== 'bg-neutral' && c !== 'text-neutral-content'
      && c !== 'border-base-200' && c !== 'border-neutral';
  });
  if (variant === 'kds') {
    if (cls.indexOf('site-header') < 0) cls.push('site-header');
  } else if (variant === 'dark') {
    cls.push('bg-neutral', 'text-neutral-content', 'border-neutral');
  } else {
    cls.push('bg-base-100', 'border-base-200');
  }
  this.setClass(cls);
};
this.on('change:attributes:data-variant', applyNavbarVariant);
applyNavbarVariant.call(this);
"#;

const EXPAND_SECTION_SCRIPT: &str = r#"
var root = this;
if (root.dataset.kdsExpandBound) return;
root.dataset.kdsExpandBound = 'true';
var reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
var canHover = window.matchMedia('(hover: hover) and (pointer: fine)').matches;
function setBodyOpen(body, open) {
  if (!body) return;
  body.style.height = open ? 'auto' : '0px';
  body.style.opacity = open ? '1' : '0';
}
function syncToggle(section, open) {
  var toggle = section.querySelector('.expand-toggle');
  if (!toggle) return;
  toggle.setAttribute('aria-expanded', open ? 'true' : 'false');
}
function animateOpen(section) {
  if (section._expandOpen) return;
  section._expandOpen = true;
  var body = section.querySelector('.expand-body');
  section.classList.add('is-open');
  syncToggle(section, true);
  if (!body) return;
  if (reduceMotion) { setBodyOpen(body, true); return; }
  var start = body.getBoundingClientRect().height;
  body.style.height = start + 'px';
  body.style.opacity = start > 1 ? getComputedStyle(body).opacity : '0';
  body.offsetHeight;
  body.style.height = body.scrollHeight + 'px';
  body.style.opacity = '1';
  body.addEventListener('transitionend', function onEnd(e) {
    if (e.propertyName !== 'height') return;
    if (section._expandOpen) body.style.height = 'auto';
    body.removeEventListener('transitionend', onEnd);
  });
}
function animateClose(section) {
  if (!section._expandOpen) return;
  section._expandOpen = false;
  var body = section.querySelector('.expand-body');
  section.classList.remove('is-open');
  syncToggle(section, false);
  if (!body) return;
  if (reduceMotion) { setBodyOpen(body, false); return; }
  var start = body.getBoundingClientRect().height || body.scrollHeight;
  body.style.height = start + 'px';
  body.offsetHeight;
  body.style.height = '0px';
  body.style.opacity = '0';
}
var header = root.querySelector('.expand-header');
var body = root.querySelector('.expand-body');
if (header && body) {
  root._expandOpen = root.classList.contains('is-open');
  if (!root._expandOpen) setBodyOpen(body, false);
  syncToggle(root, root._expandOpen);
  header.addEventListener('click', function () {
    if (canHover) return;
    if (root._expandOpen) animateClose(root);
    else animateOpen(root);
  });
  if (canHover) {
    root.addEventListener('mouseenter', function () { animateOpen(root); });
    root.addEventListener('mouseleave', function () { animateClose(root); });
  }
}
"#;

const EXPAND_SECTION_INIT: &str = r#"
this.on('change:attributes:data-bg-src', function () {
  var src = (this.getAttributes() || {})['data-bg-src'] || '';
  var img = this.find('.gjs-expand-section-bg')[0] || this.find('.expand-section-bg img')[0];
  if (img && src) img.addAttributes({ src: src }, { silent: true });
});
"#;

const NAVBAR_LOGO_INIT: &str = r#"
var findNavbarParent = function (cmp) {
  if (cmp && typeof cmp.closestType === 'function') {
    var byType = cmp.closestType('p_website.navbar');
    if (byType) return byType;
  }
  var parent = cmp.collection && cmp.collection.parent;
  while (parent) {
    if (parent.get('type') === 'p_website.navbar') return parent;
    parent = parent.collection && parent.collection.parent;
  }
  return null;
};
var syncFromNavbar = function () {
  var nav = findNavbarParent(this);
  if (!nav || nav._renderingNavbar) return;
  var attrs = nav.getAttributes() || {};
  var logo = attrs['data-logo-src'] || '';
  var alt = attrs['data-logo-alt'] || 'Logo';
  if (!logo) return;
  var silent = { silent: true };
  var selfAttr = this.getAttributes() || {};
  if (this.get('tagName') !== 'img') this.set('tagName', 'img', silent);
  if (selfAttr.src !== logo || selfAttr.alt !== alt) {
    this.addAttributes({ src: logo, alt: alt }, silent);
  }
  if (this.getStyle && this.getStyle('display') === 'none') {
    this.removeStyle('display', silent);
  }
};
syncFromNavbar();
"#;

const NAVBAR_SCRIPT: &str = r#"
var root = this;
var decodeAttr = function (raw) {
  return String(raw == null ? '' : raw)
    .replace(/&quot;/g, '"')
    .replace(/&#34;/g, '"')
    .replace(/&apos;/g, "'")
    .replace(/&#39;/g, "'")
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&amp;/g, '&');
};
var logo = decodeAttr(root.getAttribute('data-logo-src') || '');
var alt = decodeAttr(root.getAttribute('data-logo-alt') || '') || 'Logo';
if (logo) {
  var img = root.querySelector('.gjs-navbar-logo');
  if (img) {
    if (img.tagName !== 'IMG') {
      var replacement = document.createElement('img');
      replacement.className = img.className;
      img.replaceWith(replacement);
      img = replacement;
    }
    img.setAttribute('src', logo);
    img.setAttribute('alt', alt);
    img.style.display = '';
  }
}
(function syncNavLinks() {
  var raw = root.getAttribute('data-nav-links');
  if (!raw) return;
  var links = [];
  try {
    var parsed = JSON.parse(decodeAttr(raw));
    if (Array.isArray(parsed)) links = parsed;
  } catch (e) {
    return;
  }
  var esc = function (s) {
    return String(s || '')
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/"/g, '&quot;');
  };
  var html = links
    .filter(function (l) { return l && (l.title || l.path); })
    .map(function (l) {
      return '<li><a class="nav-link" href="' + esc(l.path || '#') + '">' + esc(l.title || '') + '</a></li>';
    })
    .join('');
  var desktop = root.querySelector('.gjs-navbar-links');
  var mobile = root.querySelector('.gjs-navbar-mobile-links');
  if (desktop) desktop.innerHTML = html;
  if (mobile) mobile.innerHTML = html;
})();
(function initMobileNav() {
  var toggleBtn = root.querySelector('.mobile-nav-toggle');
  var navMenu = root.querySelector('.gjs-navbar-links');
  if (!toggleBtn || !navMenu || toggleBtn.dataset.bound) return;
  toggleBtn.dataset.bound = 'true';
  toggleBtn.addEventListener('click', function (e) {
    e.stopPropagation();
    var open = !navMenu.classList.contains('open');
    navMenu.classList.toggle('open', open);
    toggleBtn.classList.toggle('open', open);
    toggleBtn.setAttribute('aria-expanded', String(open));
  });
  document.addEventListener('click', function (e) {
    if (!navMenu.contains(e.target) && !toggleBtn.contains(e.target)) {
      navMenu.classList.remove('open');
      toggleBtn.classList.remove('open');
      toggleBtn.setAttribute('aria-expanded', 'false');
    }
  });
  navMenu.querySelectorAll('a').forEach(function (link) {
    link.addEventListener('click', function () {
      navMenu.classList.remove('open');
      toggleBtn.classList.remove('open');
      toggleBtn.setAttribute('aria-expanded', 'false');
    });
  });
})();
"#;

const NAVBAR_MODEL_INIT: &str = r#"
var decodeAttr = function (raw) {
  return String(raw == null ? '' : raw)
    .replace(/&quot;/g, '"')
    .replace(/&#34;/g, '"')
    .replace(/&apos;/g, "'")
    .replace(/&#39;/g, "'")
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&amp;/g, '&');
};
var parseLinks = function (raw) {
  try {
    var parsed = JSON.parse(decodeAttr(raw || '[]'));
    return Array.isArray(parsed) ? parsed : [];
  } catch (e) {
    return [];
  }
};
var esc = function (s) {
  return String(s || '')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/"/g, '&quot;');
};
var linkRow = function (title, path) {
  return '<li><a class="nav-link" href="' + esc(path || '#') + '">' + esc(title || '') + '</a></li>';
};
var findLogoComp = function (comp) {
  var found = null;
  comp.components().each(function (child) {
    if (found) return;
    var type = (child.get('attributes') || {})['data-gjs-type'];
    var classes = child.getClasses() || [];
    if (type === 'p_website.navbar-logo' || classes.indexOf('gjs-navbar-logo') >= 0) {
      found = child;
      return;
    }
    found = findLogoComp(child);
  });
  return found;
};
var setLinksHtml = function (comp, className, html) {
  var targets = (comp.find && comp.find('.' + className)) || [];
  for (var i = 0; i < targets.length; i++) {
    var target = targets[i];
    if (!target || typeof target.components !== 'function') continue;
    var current = '';
    try {
      current = target.components().map(function (c) { return c.toHTML(); }).join('');
    } catch (e) {
      current = '';
    }
    if (current === html) continue;
    target.components(html);
  }
};
this.renderNavbar = function () {
  if (this._renderingNavbar) return;
  this._renderingNavbar = true;
  try {
    var silent = { silent: true };
    var attrs = this.getAttributes() || {};
    var links = parseLinks(attrs['data-nav-links']);
    var logo = decodeAttr(attrs['data-logo-src'] || '');
    var alt = decodeAttr(attrs['data-logo-alt'] || '') || 'Logo';
    var el = this.getEl();
    var logoComp = findLogoComp(this);
    if (logoComp) {
      if (logoComp.get('tagName') !== 'img') {
        logoComp.set('tagName', 'img', silent);
      }
      var logoAttr = logoComp.getAttributes() || {};
      if (logo) {
        if (logoAttr.src !== logo || logoAttr.alt !== alt) {
          logoComp.addAttributes({ src: logo, alt: alt }, silent);
        }
        if (logoComp.getStyle && logoComp.getStyle('display') === 'none') {
          logoComp.removeStyle('display', silent);
        }
      } else {
        if (logoAttr.src) {
          logoComp.removeAttributes('src', silent);
        }
        if (!logoComp.getStyle || logoComp.getStyle('display') !== 'none') {
          logoComp.addStyle({ display: 'none' }, silent);
        }
      }
    }
    if (attrs['data-logo-display'] || attrs['data-logo-vnode-id']) {
      this.removeAttributes(['data-logo-display', 'data-logo-vnode-id'], silent);
    }
    var html = links
      .filter(function (l) { return l && (l.title || l.path); })
      .map(function (l) { return linkRow(l.title, l.path); })
      .join('');
    // Prefer the GrapesJS component tree so saved/exported HTML matches traits.
    setLinksHtml(this, 'gjs-navbar-links', html);
    setLinksHtml(this, 'gjs-navbar-mobile-links', html);
    if (!el) return;
    var img = el.querySelector('.gjs-navbar-logo');
    if (img) {
      if (img.tagName !== 'IMG') {
        var replacement = document.createElement('img');
        replacement.className = img.className;
        img.replaceWith(replacement);
        img = replacement;
      }
      if (logo) {
        if (img.getAttribute('src') !== logo) img.setAttribute('src', logo);
        if (img.getAttribute('alt') !== alt) img.setAttribute('alt', alt);
        if (img.style.display === 'none') img.style.display = '';
      } else {
        if (img.hasAttribute('src')) img.removeAttribute('src');
        img.style.display = 'none';
      }
    }
  } finally {
    this._renderingNavbar = false;
  }
};
this.on('change:attributes', function (_model, _value, opts) {
  if (this._renderingNavbar) return;
  if (opts && opts.silent) return;
  this.renderNavbar();
});
this.renderNavbar();
var navbarSelf = this;
setTimeout(function () { navbarSelf.renderNavbar(); }, 0);
"#;

const NAV_LINKS_CREATE_INPUT: &str = r#"
var root = document.createElement('div');
root.className = 'gjs-nav-links-trait';
root.style.cssText = 'display:flex;flex-direction:column;gap:8px;width:100%';
var label = document.createElement('div');
label.textContent = 'Navigation links';
label.style.cssText = 'font-size:11px;font-weight:600;opacity:0.85';
var rows = document.createElement('div');
rows.className = 'gjs-nav-links-rows';
rows.style.cssText = 'display:flex;flex-direction:column;gap:6px;width:100%';
var addBtn = document.createElement('button');
addBtn.type = 'button';
addBtn.className = 'gjs-nav-links-add';
addBtn.textContent = 'Add link';
addBtn.style.cssText =
  'width:100%;padding:6px 8px;border:1px solid rgba(255,255,255,0.25);border-radius:4px;background:transparent;color:inherit;cursor:pointer;font-size:12px';
function styleField(el) {
  el.style.flex = '1';
  el.style.minWidth = '0';
  el.style.padding = '4px 6px';
  el.style.border = '1px solid rgba(255,255,255,0.2)';
  el.style.borderRadius = '4px';
  el.style.background = 'rgba(0,0,0,0.2)';
  el.style.color = 'inherit';
  el.style.fontSize = '12px';
}
function appendRow(title, path) {
  var row = document.createElement('div');
  row.className = 'gjs-nav-links-row';
  row.style.cssText = 'display:flex;gap:4px;align-items:center;width:100%';
  var titleEl = document.createElement('input');
  titleEl.type = 'text';
  titleEl.className = 'gjs-nav-links-title';
  titleEl.placeholder = 'Title';
  styleField(titleEl);
  var pathEl = document.createElement('input');
  pathEl.type = 'text';
  pathEl.className = 'gjs-nav-links-path';
  pathEl.placeholder = '/path';
  styleField(pathEl);
  var removeBtn = document.createElement('button');
  removeBtn.type = 'button';
  removeBtn.className = 'gjs-nav-links-remove';
  removeBtn.setAttribute('aria-label', 'Remove link');
  removeBtn.textContent = '\u2715';
  removeBtn.style.cssText =
    'flex-shrink:0;width:28px;height:28px;padding:0;border:0;background:transparent;color:inherit;cursor:pointer;opacity:0.75';
  titleEl.value = title || '';
  pathEl.value = path || '';
  row.appendChild(titleEl);
  row.appendChild(pathEl);
  row.appendChild(removeBtn);
  rows.appendChild(row);
}
addBtn.addEventListener('click', function (e) {
  e.preventDefault();
  appendRow('', '/');
});
root.appendChild(label);
root.appendChild(rows);
root.appendChild(addBtn);
root.__larivAppendRow = appendRow;
root.__larivRows = rows;
return root;
"#;

const NAV_LINKS_ON_EVENT: &str = r#"
if (!elInput) return;
if (event && event.type === 'click') {
  var addBtn = event.target && event.target.closest('.gjs-nav-links-add');
  if (addBtn && elInput.__larivAppendRow) {
    elInput.__larivAppendRow('', '/');
  }
  var removeBtn = event.target && event.target.closest('.gjs-nav-links-remove');
  if (removeBtn) {
    var row = removeBtn.closest('.gjs-nav-links-row');
    if (row) row.remove();
  }
}
var rowEls = elInput.querySelectorAll('.gjs-nav-links-row');
var links = [];
for (var i = 0; i < rowEls.length; i++) {
  var row = rowEls[i];
  var titleEl = row.querySelector('.gjs-nav-links-title');
  var pathEl = row.querySelector('.gjs-nav-links-path');
  var title = (titleEl && titleEl.value ? titleEl.value : '').trim();
  var path = (pathEl && pathEl.value ? pathEl.value : '').trim();
  if (title || path) {
    links.push({ title: title || 'Link', path: path || '#' });
  }
}
component.addAttributes({ 'data-nav-links': JSON.stringify(links) });
"#;

const NAV_LINKS_ON_UPDATE: &str = r#"
if (!elInput || !elInput.__larivRows || !elInput.__larivAppendRow) return;
var rows = elInput.__larivRows;
rows.innerHTML = '';
var attrs = component.getAttributes() || {};
var links = [];
try {
  var raw = attrs['data-nav-links'] || '[]';
  raw = String(raw)
    .replace(/&quot;/g, '"')
    .replace(/&#34;/g, '"')
    .replace(/&amp;/g, '&');
  var parsed = JSON.parse(raw);
  if (Array.isArray(parsed)) links = parsed;
} catch (e) {
  links = [];
}
if (links.length === 0) {
  elInput.__larivAppendRow('Home', '/');
  elInput.__larivAppendRow('About', '/about');
} else {
  for (var i = 0; i < links.length; i++) {
    var l = links[i] || {};
    elInput.__larivAppendRow(l.title || '', l.path || '/');
  }
}
"#;

fn navbar_traits() -> Value {
    json!([
        id_trait(),
        src_url_trait("data-logo-src", "Logo image URL"),
        {"type": "text", "name": "data-logo-alt", "label": "Logo alt text"},
        {"type": "select", "name": "data-variant", "label": "Variant", "options": [
            {"id": "light", "label": "Light"},
            {"id": "dark", "label": "Dark"},
            {"id": "kds", "label": "KDS (fixed header)"},
        ]},
        {"type": "p_website.nav-links", "name": "data-nav-links", "label": "Navigation links", "changeProp": 0},
    ])
}

fn section_bg_trait() -> Value {
    json!({"type": "select", "name": "data-section-bg", "label": "Background", "options": [
        {"id": "base", "label": "Default (theme)"},
        {"id": "base-100", "label": "Surface"},
        {"id": "base-200", "label": "Muted"},
        {"id": "primary", "label": "Primary"},
        {"id": "neutral", "label": "Neutral"},
    ]})
}

fn header_style_trait() -> Value {
    json!({"type": "select", "name": "data-header-style", "label": "Style", "options": [
        {"id": "default", "label": "Default"},
        {"id": "tight", "label": "Tight"},
    ]})
}

fn show_media_trait() -> Value {
    json!({"type": "checkbox", "name": "data-show-media", "label": "Show background image"})
}

fn show_button_trait() -> Value {
    json!({"type": "checkbox", "name": "data-show-button", "label": "Show button"})
}

fn align_trait() -> Value {
    json!({"type": "select", "name": "data-align", "label": "Alignment", "options": [
        {"id": "center", "label": "Center"},
        {"id": "left", "label": "Left"},
    ]})
}

const VIDEO_SCRIPT: &str = r#"
var el = this;
if (!el || !el.tagName || el.tagName.toLowerCase() !== 'video') return;
if (el.dataset.larivVideoBound) return;
el.dataset.larivVideoBound = 'true';
function landscapeSrc() {
  return (el.getAttribute('data-src-landscape') || el.getAttribute('src') || '').trim();
}
function portraitSrc() {
  return (el.getAttribute('data-src-portrait') || '').trim();
}
function pickSrc() {
  var portrait = portraitSrc();
  if (portrait && window.matchMedia('(orientation: portrait)').matches) return portrait;
  return landscapeSrc();
}
function apply() {
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    el.pause();
    el.removeAttribute('autoplay');
    return;
  }
  var next = pickSrc();
  if (!next) return;
  if ((el.getAttribute('src') || '') === next) {
    var same = el.play();
    if (same && same.catch) same.catch(function () {});
    return;
  }
  el.setAttribute('src', next);
  el.load();
  var play = el.play();
  if (play && play.catch) play.catch(function () {});
}
apply();
if (window.matchMedia) {
  var mq = window.matchMedia('(orientation: portrait)');
  if (mq.addEventListener) mq.addEventListener('change', apply);
  else if (mq.addListener) mq.addListener(apply);
}
"#;

const DOTLOTTIE_ON_RENDER: &str = r#"
var doc = (this.el && this.el.ownerDocument) || document;
if (window.__larivEnsureDotLottie) {
  window.__larivEnsureDotLottie(doc);
}
"#;

const SRC_URL_CREATE_INPUT: &str = r#"
const el = document.createElement('input');
el.type = 'url';
el.placeholder = (trait && trait.get && trait.get('placeholder')) || 'https://…';
el.style.width = '100%';
return el;
"#;

const SRC_URL_ON_EVENT: &str = r#"
const name = (trait && trait.get && trait.get('name')) || 'src';
const value = elInput && elInput.value != null ? elInput.value : '';
if (trait && trait.get && trait.get('changeProp')) {
  component.set(name, value);
} else {
  const attrs = {};
  attrs[name] = value;
  component.addAttributes(attrs);
}
"#;

const SRC_URL_ON_UPDATE: &str = r#"
const name = (trait && trait.get && trait.get('name')) || 'src';
let value = '';
if (trait && trait.get && trait.get('changeProp')) {
  value = component.get(name) || '';
} else {
  const attrs = component.getAttributes() || {};
  value = attrs[name] || '';
}
if (elInput) elInput.value = value;
"#;

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl GrapesJsRegistrar for Hook {
    fn register_grapesjs(self, grapesjs: &mut GrapesJsCapability) {
        grapesjs
            // Layout blocks
            .register_block(
                "p_website.section",
                block_html("Section", "Layout", include_str!("assets/grapesjs_blocks/section.html")),
            )
            .register_block(
                "p_website.row-2",
                block_html(
                    "2 Columns",
                    "Layout",
                    include_str!("assets/grapesjs_blocks/2-columns.html"),
                ),
            )
            .register_block(
                "p_website.row-3",
                block_html(
                    "3 Columns",
                    "Layout",
                    include_str!("assets/grapesjs_blocks/3-columns.html"),
                ),
            )
            .register_block(
                "p_website.card",
                block_html("Card", "Layout", include_str!("assets/grapesjs_blocks/card.html")),
            )
            // Catalog blocks
            .register_block(
                "p_website.accordion",
                block_html(
                    "Accordion",
                    "Interactive",
                    include_str!("assets/grapesjs_components/accordion.html"),
                ),
            )
            .register_block(
                "p_website.blurb",
                block_html("Blurb", "Basic", include_str!("assets/grapesjs_components/blurb.html")),
            )
            .register_block(
                "p_website.button",
                block_html("Button", "Basic", include_str!("assets/grapesjs_components/button.html")),
            )
            .register_block(
                "p_website.contact-detail",
                block_html(
                    "Contact detail",
                    "Basic",
                    include_str!("assets/grapesjs_components/contact-detail.html"),
                ),
            )
            .register_block(
                "p_website.cta",
                block_html("CTA", "Basic", include_str!("assets/grapesjs_components/cta.html")),
            )
            .register_block(
                "p_website.code",
                block_html("Code", "Basic", include_str!("assets/grapesjs_components/code.html")),
            )
            .register_block(
                "p_website.expand-section",
                block_html(
                    "Expand section",
                    "Layout",
                    include_str!("assets/grapesjs_components/expand-section.html"),
                ),
            )
            .register_block(
                "p_website.feature-card",
                block_html(
                    "Feature card",
                    "Basic",
                    include_str!("assets/grapesjs_components/feature-card.html"),
                ),
            )
            .register_block(
                "p_website.divider",
                block_html("Divider", "Basic", include_str!("assets/grapesjs_components/divider.html")),
            )
            .register_block(
                "p_website.dropdown",
                block_html(
                    "Dropdown",
                    "Interactive",
                    include_str!("assets/grapesjs_components/dropdown.html"),
                ),
            )
            .register_block(
                "p_website.gallery",
                block_html("Gallery", "Media", include_str!("assets/grapesjs_components/gallery.html")),
            )
            .register_block(
                "p_website.heading",
                block_html(
                    "Heading",
                    "Basic",
                    include_str!("assets/grapesjs_components/heading.html"),
                ),
            )
            .register_block(
                "p_website.hero",
                block_html("Hero", "Layout", include_str!("assets/grapesjs_components/hero.html")),
            )
            .register_block(
                "p_website.navbar",
                block_html(
                    "Navbar",
                    "Layout",
                    include_str!("assets/grapesjs_components/navbar.html"),
                ),
            )
            .register_block(
                "p_website.icon",
                block_html("Icon", "Media", include_str!("assets/grapesjs_components/icon.html")),
            )
            .register_block(
                "p_website.icon-list",
                block_html(
                    "Icon list",
                    "Basic",
                    include_str!("assets/grapesjs_components/icon-list.html"),
                ),
            )
            .register_block(
                "p_website.image",
                block_html("Image", "Media", include_str!("assets/grapesjs_components/image.html")),
            )
            .register_block(
                "p_website.video",
                block_html("Video", "Media", include_str!("assets/grapesjs_components/video.html")),
            )
            .register_block(
                "p_website.link",
                block_html("Link", "Basic", include_str!("assets/grapesjs_components/link.html")),
            )
            .register_block(
                "p_website.dotlottie",
                block_html(
                    "DotLottie",
                    "Media",
                    include_str!("assets/grapesjs_components/dotlottie.html"),
                ),
            )
            .register_block(
                "p_website.person",
                block_html("Person", "Basic", include_str!("assets/grapesjs_components/person.html")),
            )
            .register_block(
                "p_website.pricing-tables",
                block_html(
                    "Pricing tables",
                    "Basic",
                    include_str!("assets/grapesjs_components/pricing-tables.html"),
                ),
            )
            .register_block(
                "p_website.section-header",
                block_html(
                    "Section header",
                    "Basic",
                    include_str!("assets/grapesjs_components/section-header.html"),
                ),
            )
            .register_block(
                "p_website.slider",
                block_html(
                    "Slider",
                    "Interactive",
                    include_str!("assets/grapesjs_components/slider.html"),
                ),
            )
            .register_block(
                "p_website.tabs",
                block_html("Tabs", "Interactive", include_str!("assets/grapesjs_components/tabs.html")),
            )
            .register_block(
                "p_website.testimonial",
                block_html(
                    "Testimonial",
                    "Basic",
                    include_str!("assets/grapesjs_components/testimonial.html"),
                ),
            )
            .register_block(
                "p_website.toggleable",
                block_html(
                    "Toggleable",
                    "Interactive",
                    include_str!("assets/grapesjs_components/toggleable.html"),
                ),
            )
            .register_block(
                "p_website.text",
                block_html("Text", "Basic", include_str!("assets/grapesjs_components/text.html")),
            )
            .register_block(
                "p_website.bar-counter",
                block_html(
                    "Bar counter",
                    "Counters",
                    include_str!("assets/grapesjs_components/bar-counter.html"),
                ),
            )
            .register_block(
                "p_website.circle-counter",
                block_html(
                    "Circle counter",
                    "Counters",
                    include_str!("assets/grapesjs_components/circle-counter.html"),
                ),
            )
            .register_block(
                "p_website.countdown-counter",
                block_html(
                    "Countdown",
                    "Counters",
                    include_str!("assets/grapesjs_components/countdown-counter.html"),
                ),
            )
            .register_block(
                "p_website.number-counter",
                block_html(
                    "Number counter",
                    "Counters",
                    include_str!("assets/grapesjs_components/number-counter.html"),
                ),
            )
            // Components
            .register_component(
                "p_website.lariv-region",
                component_entry(
                    "p_website.lariv-region",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "editable": false,
                            "selectable": false,
                            "draggable": false,
                            "droppable": false,
                            "highlightable": false,
                            "hoverable": false,
                            "copyable": false,
                            "removable": false,
                            "attributes": {
                                "data-gjs-type": "p_website.lariv-region",
                                "class": "lariv-region-locked",
                            },
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.lariv-content",
                component_entry(
                    "p_website.lariv-content",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "droppable": true,
                            "attributes": {
                                "data-gjs-type": "p_website.lariv-content",
                                "data-lariv-region": "content",
                            },
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.section",
                component_entry(
                    "p_website.section",
                    json!({
                        "defaults": {
                            "tagName": "section",
                            "droppable": false,
                            "attributes": {
                                "data-gjs-type": "p_website.section",
                                "class": "gjs-section section",
                                "data-section-bg": "base",
                            },
                            "traits": [id_trait(), section_bg_trait()],
                        },
                        "init": SECTION_MODEL_INIT,
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.row-2",
                component_entry(
                    "p_website.row-2",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "droppable": ".gjs-cell",
                            "attributes": {
                                "data-gjs-type": "p_website.row-2",
                                "class": "gjs-row grid grid-cols-1 md:grid-cols-2 gap-6 w-full items-stretch",
                            },
                            "traits": [id_trait()],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.row-3",
                component_entry(
                    "p_website.row-3",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "droppable": ".gjs-cell",
                            "attributes": {
                                "data-gjs-type": "p_website.row-3",
                                "class": "gjs-row feature-grid",
                            },
                            "traits": [id_trait()],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.card",
                component_entry(
                    "p_website.card",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "attributes": {
                                "data-gjs-type": "p_website.card",
                                "class": "gjs-card card bg-base-100 shadow-md border border-base-200 w-full h-full",
                            },
                            "traits": [id_trait()],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.accordion",
                component_entry(
                    "p_website.accordion",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "droppable": false,
                            "attributes": {
                                "data-gjs-type": "p_website.accordion",
                                "class": "gjs-accordion join join-vertical w-full bg-base-100 border border-base-200 rounded-box shadow-sm",
                            },
                            "traits": [id_trait()],
                            "script": ACCORDION_SCRIPT,
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.blurb",
                component_entry(
                    "p_website.blurb",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "attributes": {
                                "data-gjs-type": "p_website.blurb",
                                "class": "gjs-blurb card bg-base-100 shadow-md border border-base-200 w-full h-full",
                            },
                            "traits": [id_trait()],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.button",
                component_entry(
                    "p_website.button",
                    json!({
                        "defaults": {
                            "tagName": "a",
                            "editable": true,
                            "attributes": {
                                "data-gjs-type": "p_website.button",
                                "class": "gjs-button btn btn-primary",
                                "href": "#",
                            },
                            "traits": link_target_traits(),
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.contact-detail",
                component_entry(
                    "p_website.contact-detail",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "attributes": {
                                "data-gjs-type": "p_website.contact-detail",
                                "class": "gjs-contact-detail cta-row",
                            },
                            "traits": [
                                id_trait(),
                                {"type": "text", "name": "href", "label": "Link URL", "changeProp": 1},
                            ],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.cta",
                component_entry(
                    "p_website.cta",
                    json!({
                        "defaults": {
                            "tagName": "section",
                            "attributes": {
                                "data-gjs-type": "p_website.cta",
                                "class": "gjs-cta section",
                                "data-show-button": "false",
                            },
                            "traits": [id_trait(), show_button_trait()],
                        },
                        "init": CTA_MODEL_INIT,
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.code",
                component_entry(
                    "p_website.code",
                    json!({
                        "defaults": {
                            "tagName": "pre",
                            "attributes": {
                                "data-gjs-type": "p_website.code",
                                "class": "gjs-code mockup-code bg-neutral text-neutral-content rounded-box p-4 overflow-x-auto text-sm",
                            },
                            "traits": [
                                {"type": "text", "name": "data-language", "label": "Language"},
                            ],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.expand-section",
                component_entry(
                    "p_website.expand-section",
                    json!({
                        "defaults": {
                            "tagName": "section",
                            "droppable": false,
                            "attributes": {
                                "data-gjs-type": "p_website.expand-section",
                                "class": "gjs-expand-section expand-section",
                                "data-bg-src": "https://kdstagore.com/static/laser.jpg",
                            },
                            "traits": [
                                id_trait(),
                                src_url_trait("data-bg-src", "Background image"),
                            ],
                        },
                        "script": EXPAND_SECTION_SCRIPT,
                        "init": EXPAND_SECTION_INIT,
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.feature-card",
                component_entry(
                    "p_website.feature-card",
                    json!({
                        "defaults": {
                            "tagName": "article",
                            "attributes": {
                                "data-gjs-type": "p_website.feature-card",
                                "class": "gjs-feature-card feature",
                            },
                            "traits": [id_trait()],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.divider",
                component_entry(
                    "p_website.divider",
                    json!({
                        "defaults": {
                            "tagName": "hr",
                            "void": true,
                            "droppable": false,
                            "attributes": {
                                "data-gjs-type": "p_website.divider",
                                "class": "gjs-divider divider my-8",
                            },
                            "traits": [],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.dropdown",
                component_entry(
                    "p_website.dropdown",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "attributes": {
                                "data-gjs-type": "p_website.dropdown",
                                "class": "gjs-dropdown dropdown",
                            },
                            "script": DROPDOWN_SCRIPT,
                            "traits": [id_trait()],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.gallery",
                component_entry(
                    "p_website.gallery",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "droppable": true,
                            "attributes": {
                                "data-gjs-type": "p_website.gallery",
                                "class": "gjs-gallery grid grid-cols-2 md:grid-cols-3 gap-4",
                            },
                            "traits": [id_trait()],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.heading",
                GrapesJsComponent {
                    extend: "text".into(),
                    is_component: Some(is_component_type("p_website.heading")),
                    model: Some(json!({
                        "defaults": {
                            "tagName": "h2",
                            "editable": true,
                            "attributes": {
                                "data-gjs-type": "p_website.heading",
                                "class": "gjs-heading text-3xl font-bold tracking-tight text-base-content",
                                "data-level": "2",
                            },
                            "traits": [{
                                "type": "select",
                                "name": "data-level",
                                "label": "Level",
                                "options": [
                                    {"id": "1", "label": "H1"},
                                    {"id": "2", "label": "H2"},
                                    {"id": "3", "label": "H3"},
                                    {"id": "4", "label": "H4"},
                                    {"id": "5", "label": "H5"},
                                    {"id": "6", "label": "H6"},
                                ],
                            }],
                        },
                        "init": HEADING_INIT,
                    })),
                    view: None,
                },
            )
            .register_component(
                "p_website.hero",
                component_entry(
                    "p_website.hero",
                    json!({
                        "defaults": {
                            "tagName": "section",
                            "attributes": {
                                "data-gjs-type": "p_website.hero",
                                "class": "gjs-hero hero",
                                "data-show-media": "true",
                                "data-show-button": "false",
                            },
                            "traits": [id_trait(), show_media_trait(), show_button_trait()],
                        },
                        "init": HERO_MODEL_INIT,
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.hero-media",
                component_entry(
                    "p_website.hero-media",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "removable": false,
                            "draggable": false,
                            "droppable": false,
                            "attributes": {
                                "data-gjs-type": "p_website.hero-media",
                                "class": "gjs-hero-media hero-media",
                                "aria-hidden": "true",
                            },
                            "traits": [],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.hero-inner",
                component_entry(
                    "p_website.hero-inner",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "removable": false,
                            "draggable": false,
                            "attributes": {
                                "data-gjs-type": "p_website.hero-inner",
                                "class": "gjs-hero-inner hero-inner",
                            },
                            "traits": [],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.navbar-logo",
                component_entry(
                    "p_website.navbar-logo",
                    json!({
                        "defaults": {
                            "tagName": "img",
                            "void": true,
                            "droppable": false,
                            "draggable": false,
                            "selectable": false,
                            "hoverable": false,
                            "editable": false,
                            "attributes": {
                                "data-gjs-type": "p_website.navbar-logo",
                                "class": "gjs-navbar-logo h-12 w-auto max-w-[10rem] object-contain",
                                "alt": "Logo",
                            },
                            "traits": [],
                        },
                        "init": NAVBAR_LOGO_INIT,
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.navbar",
                component_entry(
                    "p_website.navbar",
                    json!({
                        "defaults": {
                            "tagName": "nav",
                            "droppable": false,
                            "attributes": {
                                "data-gjs-type": "p_website.navbar",
                                "class": "gjs-navbar site-header",
                                "data-nav-links": "[{\"title\":\"CNC\",\"path\":\"#cnc\"},{\"title\":\"Fabrication\",\"path\":\"#fabrication\"},{\"title\":\"Finishing\",\"path\":\"#finishing\"},{\"title\":\"Contact\",\"path\":\"#contact\"}]",
                                "data-logo-src": "https://kdstagore.com/static/logo.svg",
                                "data-logo-alt": "Logo",
                                "data-variant": "kds",
                            },
                            "traits": navbar_traits(),
                        },
                        "script": NAVBAR_SCRIPT,
                        "init": format!("{NAVBAR_MODEL_INIT}\n{NAVBAR_VARIANT_INIT}"),
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.icon",
                component_entry(
                    "p_website.icon",
                    json!({
                        "defaults": {
                            "tagName": "span",
                            "droppable": false,
                            "attributes": {
                                "data-gjs-type": "p_website.icon",
                                "class": "gjs-icon inline-flex items-center justify-center p-3 rounded-box bg-primary/10 text-primary",
                            },
                            "traits": [
                                {"type": "text", "name": "class", "label": "CSS class"},
                            ],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.icon-list",
                component_entry(
                    "p_website.icon-list",
                    json!({
                        "defaults": {
                            "tagName": "ul",
                            "droppable": true,
                            "attributes": {
                                "data-gjs-type": "p_website.icon-list",
                                "class": "gjs-icon-list menu bg-transparent p-0 gap-1",
                            },
                            "traits": [id_trait()],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.image",
                GrapesJsComponent {
                    extend: "image".into(),
                    is_component: Some(is_component_type("p_website.image")),
                    model: Some(json!({
                        "defaults": {
                            "attributes": {
                                "data-gjs-type": "p_website.image",
                                "class": "gjs-image rounded-box max-w-full h-auto shadow-sm",
                            },
                            "traits": [
                                src_url_trait("src", "Source"),
                                {"type": "text", "name": "alt", "label": "Alt"},
                                {"type": "text", "name": "title", "label": "Title"},
                            ],
                        },
                    })),
                    view: None,
                },
            )
            .register_component(
                "p_website.video",
                GrapesJsComponent {
                    extend: String::new(),
                    is_component: Some(Value::String(
                        "return !!(el && el.tagName && el.tagName.toLowerCase() === 'video');"
                            .into(),
                    )),
                    model: Some(json!({
                        "defaults": {
                            "tagName": "video",
                            "void": false,
                            "droppable": false,
                            "attributes": {
                                "data-gjs-type": "p_website.video",
                                "class": "gjs-video max-w-full h-auto",
                                "autoplay": "",
                                "muted": "",
                                "loop": "",
                                "playsinline": "",
                                "poster": "",
                                "src": "",
                                "data-src-landscape": "",
                                "data-src-portrait": "",
                            },
                            "traits": [
                                src_url_trait("data-src-landscape", "Landscape source"),
                                src_url_trait("data-src-portrait", "Portrait source"),
                                src_url_trait("poster", "Poster"),
                                {
                                    "type": "checkbox",
                                    "name": "autoplay",
                                    "label": "Autoplay",
                                    "valueTrue": "",
                                    "valueFalse": "false",
                                },
                                {
                                    "type": "checkbox",
                                    "name": "muted",
                                    "label": "Muted",
                                    "valueTrue": "",
                                    "valueFalse": "false",
                                },
                                {
                                    "type": "checkbox",
                                    "name": "loop",
                                    "label": "Loop",
                                    "valueTrue": "",
                                    "valueFalse": "false",
                                },
                                {
                                    "type": "checkbox",
                                    "name": "playsinline",
                                    "label": "Plays inline",
                                    "valueTrue": "",
                                    "valueFalse": "false",
                                },
                            ],
                            "script": VIDEO_SCRIPT,
                        },
                    })),
                    view: None,
                },
            )
            .register_component(
                "p_website.link",
                component_entry(
                    "p_website.link",
                    json!({
                        "defaults": {
                            "tagName": "a",
                            "editable": true,
                            "attributes": {
                                "data-gjs-type": "p_website.link",
                                "class": "gjs-link link link-primary font-medium",
                                "href": "#",
                            },
                            "traits": link_target_traits(),
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.dotlottie",
                GrapesJsComponent {
                    extend: String::new(),
                    is_component: Some(Value::String(
                        "return !!(el && el.tagName && el.tagName.toLowerCase() === 'dotlottie-wc');"
                            .into(),
                    )),
                    model: Some(json!({
                        "defaults": {
                            "tagName": "dotlottie-wc",
                            "void": false,
                            "droppable": false,
                            "attributes": {
                                "data-gjs-type": "p_website.dotlottie",
                                "class": "gjs-dotlottie mx-auto block",
                                "src": "https://lottie.host/4db68bbd-31f6-4cd8-84eb-189de081159a/IGmMCqhzpt.lottie",
                                "autoplay": "",
                                "loop": "",
                                "style": "width: 300px; height: 300px;",
                            },
                            "traits": [
                                src_url_trait("src", "Animation URL"),
                                {
                                    "type": "checkbox",
                                    "name": "autoplay",
                                    "label": "Autoplay",
                                    "valueTrue": "",
                                    "valueFalse": "false",
                                },
                                {
                                    "type": "checkbox",
                                    "name": "loop",
                                    "label": "Loop",
                                    "valueTrue": "",
                                    "valueFalse": "false",
                                },
                                {"type": "text", "name": "style", "label": "Style"},
                            ],
                        },
                    })),
                    view: Some(json!({
                        "onRender": DOTLOTTIE_ON_RENDER,
                    })),
                },
            )
            .register_component(
                "p_website.person",
                component_entry(
                    "p_website.person",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "attributes": {
                                "data-gjs-type": "p_website.person",
                                "class": "gjs-person card bg-base-100 shadow-md border border-base-200 max-w-xs text-center",
                            },
                            "traits": [id_trait()],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.pricing-tables",
                component_entry(
                    "p_website.pricing-tables",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "droppable": true,
                            "attributes": {
                                "data-gjs-type": "p_website.pricing-tables",
                                "class": "gjs-pricing grid grid-cols-1 md:grid-cols-2 gap-6",
                            },
                            "traits": [id_trait()],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.section-header",
                component_entry(
                    "p_website.section-header",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "attributes": {
                                "data-gjs-type": "p_website.section-header",
                                "class": "gjs-section-header section-header",
                                "data-align": "center",
                                "data-header-style": "default",
                            },
                            "traits": [id_trait(), align_trait(), header_style_trait()],
                        },
                        "init": SECTION_HEADER_INIT,
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.slider",
                component_entry(
                    "p_website.slider",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "attributes": {
                                "data-gjs-type": "p_website.slider",
                                "class": "gjs-slider w-full",
                            },
                            "script": SLIDER_SCRIPT,
                            "traits": [id_trait()],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.tabs",
                component_entry(
                    "p_website.tabs",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "attributes": {
                                "data-gjs-type": "p_website.tabs",
                                "class": "gjs-tabs w-full",
                            },
                            "script": TABS_SCRIPT,
                            "traits": [id_trait()],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.testimonial",
                component_entry(
                    "p_website.testimonial",
                    json!({
                        "defaults": {
                            "tagName": "blockquote",
                            "attributes": {
                                "data-gjs-type": "p_website.testimonial",
                                "class": "gjs-testimonial card bg-base-100 shadow-md border border-base-200 border-l-4 border-l-primary max-w-md",
                            },
                            "traits": [id_trait()],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.toggleable",
                component_entry(
                    "p_website.toggleable",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "attributes": {
                                "data-gjs-type": "p_website.toggleable",
                                "class": "gjs-toggleable w-full max-w-xl",
                            },
                            "script": TOGGLEABLE_SCRIPT,
                            "traits": [id_trait()],
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.text",
                GrapesJsComponent {
                    extend: "text".into(),
                    is_component: Some(is_component_type("p_website.text")),
                    model: Some(json!({
                        "defaults": {
                            "tagName": "p",
                            "editable": true,
                            "attributes": {
                                "data-gjs-type": "p_website.text",
                                "class": "gjs-text text-base-content/80 leading-relaxed max-w-prose",
                            },
                            "traits": [id_trait()],
                        },
                    })),
                    view: None,
                },
            )
            .register_component(
                "p_website.bar-counter",
                component_entry(
                    "p_website.bar-counter",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "attributes": {
                                "data-gjs-type": "p_website.bar-counter",
                                "class": "gjs-bar-counter w-full max-w-md",
                                "data-value": "75",
                                "data-duration": "1200",
                                "data-suffix": "%",
                            },
                            "traits": counter_attrs_traits(),
                            "script": BAR_COUNTER_SCRIPT,
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.circle-counter",
                component_entry(
                    "p_website.circle-counter",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "attributes": {
                                "data-gjs-type": "p_website.circle-counter",
                                "class": "gjs-circle-counter relative w-32 text-center mx-auto",
                                "data-value": "80",
                                "data-duration": "1200",
                                "data-suffix": "%",
                            },
                            "traits": counter_attrs_traits(),
                            "script": CIRCLE_COUNTER_SCRIPT,
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.countdown-counter",
                component_entry(
                    "p_website.countdown-counter",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "attributes": {
                                "data-gjs-type": "p_website.countdown-counter",
                                "class": "gjs-countdown stats stats-vertical sm:stats-horizontal shadow bg-base-100 border border-base-200 rounded-box",
                                "data-target": "",
                            },
                            "traits": [
                                {
                                    "type": "text",
                                    "name": "data-target",
                                    "label": "Target (ISO date)",
                                },
                            ],
                            "script": COUNTDOWN_SCRIPT,
                        },
                    }),
                    None,
                ),
            )
            .register_component(
                "p_website.number-counter",
                component_entry(
                    "p_website.number-counter",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "attributes": {
                                "data-gjs-type": "p_website.number-counter",
                                "class": "gjs-number-counter stat bg-base-100 border border-base-200 rounded-box shadow place-items-center max-w-xs mx-auto p-6",
                                "data-value": "1000",
                                "data-duration": "1500",
                                "data-suffix": "+",
                            },
                            "traits": counter_attrs_traits(),
                            "script": NUMBER_COUNTER_SCRIPT,
                        },
                    }),
                    None,
                ),
            )
            // Traits
            .register_trait(
                "p_website.src-url",
                GrapesJsTrait {
                    event_capture: vec!["input".into(), "change".into()],
                    create_input: Some(Value::String(SRC_URL_CREATE_INPUT.into())),
                    on_event: Some(Value::String(SRC_URL_ON_EVENT.into())),
                    on_update: Some(Value::String(SRC_URL_ON_UPDATE.into())),
                    ..Default::default()
                },
            )
            .register_trait(
                "p_website.nav-links",
                GrapesJsTrait {
                    no_label: true,
                    template_input: Some(Value::String("<div data-input class=\"w-full\"></div>".into())),
                    event_capture: vec!["input".into(), "change".into(), "click".into()],
                    create_input: Some(Value::String(NAV_LINKS_CREATE_INPUT.into())),
                    on_event: Some(Value::String(NAV_LINKS_ON_EVENT.into())),
                    on_update: Some(Value::String(NAV_LINKS_ON_UPDATE.into())),
                    ..Default::default()
                },
            )
            // Themes
            .register_theme(
                "p_website.default",
                GrapesJsTheme {
                    label: "Default".into(),
                    css: include_str!("assets/grapesjs_theme.css").trim().into(),
                    stylesheets: vec![DEFAULT_THEME_FONTS_CSS.into()],
                    ..Default::default()
                },
            )
            .register_theme(
                "p_website.mvp",
                GrapesJsTheme {
                    label: "MVP.css".into(),
                    stylesheets: vec!["https://andybrewer.github.io/mvp/mvp.css".into()],
                    ..Default::default()
                },
            )
            .register_theme(
                "p_website.tacit",
                GrapesJsTheme {
                    label: "Tacit".into(),
                    stylesheets: vec![
                        "https://cdn.jsdelivr.net/gh/yegor256/tacit@gh-pages/tacit-css-1.9.7.min.css"
                            .into(),
                    ],
                    ..Default::default()
                },
            )
            .register_theme(
                "p_website.pico",
                GrapesJsTheme {
                    label: "Pico".into(),
                    stylesheets: vec![
                        "https://cdn.jsdelivr.net/npm/@picocss/pico@2/css/pico.min.css".into(),
                    ],
                    ..Default::default()
                },
            )
            .register_theme(
                "p_website.water",
                GrapesJsTheme {
                    label: "Water.css".into(),
                    stylesheets: vec![
                        "https://cdn.jsdelivr.net/npm/water.css@2/out/water.css".into(),
                    ],
                    ..Default::default()
                },
            )
            .register_theme(
                "p_website.marx",
                GrapesJsTheme {
                    label: "Marx".into(),
                    stylesheets: vec![
                        "https://cdn.jsdelivr.net/npm/marx-css@5/css/marx.min.css".into(),
                    ],
                    ..Default::default()
                },
            )
            .register_theme(
                "p_website.tailwind",
                GrapesJsTheme {
                    label: "Tailwind CSS".into(),
                    scripts: vec![
                        "https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4".into(),
                    ],
                    css_type: Some("text/tailwindcss".into()),
                    css: include_str!("assets/grapesjs_tailwind_theme.css").trim().into(),
                    stylesheets: vec![
                        "https://api.fontshare.com/v2/css?f[]=satoshi@300,400,500,600,700&display=swap"
                            .into(),
                        "https://fonts.googleapis.com/css2?family=Roboto+Mono:wght@400;500;600;700&display=swap"
                            .into(),
                    ],
                    ..Default::default()
                },
            )
            .register_theme(
                "p_website.daisyui",
                GrapesJsTheme {
                    label: "DaisyUI".into(),
                    scripts: vec![
                        "https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4".into(),
                    ],
                    stylesheets: vec!["https://cdn.jsdelivr.net/npm/daisyui@5".into()],
                    css: include_str!("assets/grapesjs_daisyui_theme.css").trim().into(),
                    ..Default::default()
                },
            )
            .register_theme(
                "p_website.kds",
                GrapesJsTheme {
                    label: "KDS Tagore".into(),
                    stylesheets: vec![KDS_THEME_FONTS_CSS.into()],
                    css: include_str!("assets/grapesjs_kds_theme.css").trim().into(),
                    js: include_str!("assets/grapesjs_kds_theme.js").trim().into(),
                    ..Default::default()
                },
            )
            .register_theme(
                "p_website.custom",
                GrapesJsTheme {
                    label: "Custom".into(),
                    ..Default::default()
                },
            );
    }
}
