use crate::grapesjs::{
    GrapesJsBlock, GrapesJsCapability, GrapesJsComponent, GrapesJsRegistrar, GrapesJsTheme,
    GrapesJsTrait,
};
use serde_json::{json, Value};

const DEFAULT_THEME_FONTS_CSS: &str = "https://fonts.googleapis.com/css2?family=Fraunces:opsz,wght@9..144,500;600;700&family=Manrope:wght@400;500;600;700&display=swap";

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
                "p_website.2-columns",
                block_html(
                    "2 Columns",
                    "Layout",
                    include_str!("assets/grapesjs_blocks/2-columns.html"),
                ),
            )
            .register_block(
                "p_website.3-columns",
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
                "p_website.cta",
                block_html("CTA", "Basic", include_str!("assets/grapesjs_components/cta.html")),
            )
            .register_block(
                "p_website.code",
                block_html("Code", "Basic", include_str!("assets/grapesjs_components/code.html")),
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
                "p_website.accordion",
                component_entry(
                    "p_website.accordion",
                    json!({
                        "defaults": {
                            "tagName": "div",
                            "droppable": false,
                            "attributes": {
                                "data-gjs-type": "p_website.accordion",
                                "class": "gjs-accordion",
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
                                "class": "gjs-blurb",
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
                                "class": "gjs-button",
                                "href": "#",
                            },
                            "traits": link_target_traits(),
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
                                "class": "gjs-cta",
                            },
                            "traits": [id_trait()],
                        },
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
                                "class": "gjs-code",
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
                                "class": "gjs-divider",
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
                                "class": "gjs-dropdown",
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
                                "class": "gjs-gallery",
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
                                "class": "gjs-heading",
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
                                "class": "gjs-hero",
                            },
                            "traits": [id_trait()],
                        },
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
                                "class": "gjs-icon",
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
                                "class": "gjs-icon-list",
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
                                "class": "gjs-image",
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
                "p_website.link",
                component_entry(
                    "p_website.link",
                    json!({
                        "defaults": {
                            "tagName": "a",
                            "editable": true,
                            "attributes": {
                                "data-gjs-type": "p_website.link",
                                "class": "gjs-link",
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
                                "class": "gjs-dotlottie",
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
                                "class": "gjs-person",
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
                                "class": "gjs-pricing",
                            },
                            "traits": [id_trait()],
                        },
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
                                "class": "gjs-slider",
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
                                "class": "gjs-tabs",
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
                                "class": "gjs-testimonial",
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
                                "class": "gjs-toggleable",
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
                                "class": "gjs-text",
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
                                "class": "gjs-bar-counter",
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
                                "class": "gjs-circle-counter",
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
                                "class": "gjs-countdown",
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
                                "class": "gjs-number-counter",
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
                    ..Default::default()
                },
            );
    }
}
