//! GrapesJS builder HTML/JS assembly.

use crate::grapesjs::GrapesJsCapability;

use super::dotlottie::{DOTLOTTIE_CDN_URL, DOTLOTTIE_SCRIPT_ATTR};

pub const GRAPESJS_CDN_VERSION: &str = "0.22.6";

pub fn grapesjs_head_html() -> String {
    let base = format!("https://unpkg.com/grapesjs@{GRAPESJS_CDN_VERSION}/dist/");
    format!(
        r#"<link rel="stylesheet" href="{base}css/grapes.min.css">
<script src="{base}grapes.min.js"></script>
<style>
  html, body {{ height: 100%; margin: 0; background: var(--gjs-primary-color); }}
  .gjs-builder-wrap {{ display: flex; flex-direction: column; height: 100vh; background: var(--gjs-primary-color); }}
  .gjs-builder-bar {{ display: flex; align-items: center; gap: 0.75rem; padding: 0.5rem 1rem; border-bottom: 1px solid rgba(255,255,255,0.12); background: var(--gjs-primary-color); flex-shrink: 0; color: #fff; }}
  .gjs-builder-bar-actions {{ margin-left: auto; display: flex; align-items: center; gap: 0.75rem; }}
  .gjs-builder-path {{ font-size: 0.875rem; opacity: 0.7; }}
  .gjs-builder-label {{ font-size: 0.875rem; opacity: 0.8; }}
  .gjs-builder-save-status {{ font-size: 0.875rem; opacity: 0.7; min-width: 4rem; text-align: right; }}
  .gjs-builder-theme-group {{ display: flex; align-items: center; gap: 0.5rem; }}
  .gjs-builder-btn {{
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.35rem;
    padding: 0.35rem 0.85rem;
    font-family: var(--gjs-main-font, system-ui, sans-serif);
    font-size: 0.8125rem;
    font-weight: 500;
    line-height: 1.25;
    border-radius: 4px;
    border: 1px solid transparent;
    cursor: pointer;
    text-decoration: none;
    white-space: nowrap;
    transition: background-color 0.15s ease, border-color 0.15s ease, filter 0.15s ease, opacity 0.15s ease;
  }}
  .gjs-builder-btn:disabled {{ opacity: 0.55; cursor: not-allowed; }}
  .gjs-builder-btn-outline {{
    color: var(--gjs-font-color, #ddd);
    background: transparent;
    border-color: var(--gjs-light-border, rgba(255,255,255,0.25));
  }}
  .gjs-builder-btn-outline:hover {{
    background: var(--gjs-main-light-color, rgba(255,255,255,0.1));
    color: var(--gjs-font-color-active, #fff);
  }}
  .gjs-builder-btn-primary {{
    color: #fff;
    background: var(--gjs-color-highlight, #3b97e3);
    border-color: var(--gjs-color-highlight, #3b97e3);
  }}
  .gjs-builder-btn-primary:hover:not(:disabled) {{ filter: brightness(1.08); }}
  .gjs-builder-select {{
    appearance: none;
    -webkit-appearance: none;
    padding: 0.35rem 1.75rem 0.35rem 0.65rem;
    font-family: var(--gjs-main-font, system-ui, sans-serif);
    font-size: 0.8125rem;
    color: #fff;
    background-color: var(--gjs-main-dark-color, rgba(0,0,0,0.25));
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 12 12'%3E%3Cpath fill='%23fff' d='M2.5 4.5 6 8l3.5-3.5z'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 0.5rem center;
    background-size: 0.65rem;
    border: 1px solid var(--gjs-light-border, rgba(255,255,255,0.25));
    border-radius: 4px;
    cursor: pointer;
    min-width: 8rem;
  }}
  .gjs-builder-select:hover:not(:disabled) {{ border-color: rgba(255,255,255,0.4); }}
  .gjs-builder-select:focus {{ outline: 2px solid var(--gjs-color-highlight, #3b97e3); outline-offset: 1px; }}
  .gjs-builder-select:disabled {{ opacity: 0.55; cursor: wait; }}
  .gjs-builder-select option {{ background: var(--gjs-primary-color, #444); color: #fff; }}
  #gjs {{ flex: 1; min-height: 0; }}
  .lariv-region-locked {{ opacity: 0.92; pointer-events: none; user-select: none; }}
</style>
"#
    )
}

pub fn grapesjs_body_html(
    route_id: i64,
    path: &str,
    theme: &str,
    grapes: &GrapesJsCapability,
) -> String {
    let detail_url = format!("/website/{route_id}/");
    let load_url = format!("/website/{route_id}/builder/project/");
    let store_url = load_url.clone();
    let upload_url = "/website/builder/assets/";
    let theme_url = format!("/website/{route_id}/builder/theme/");

    let blocks = grapes.blocks_json().to_string();
    let components = grapes.components_json().to_string();
    let traits = grapes.traits_json().to_string();
    let themes = grapes.themes_json().to_string();
    let current_theme = serde_json::to_string(theme).unwrap_or_else(|_| "\"\"".into());
    let path_esc = html_escape(path);
    let detail_esc = html_escape(&detail_url);

    let script = format!(
        r#"(function() {{
  const loadURL = {load_url:?};
  const storeURL = {store_url:?};
  const uploadURL = {upload_url:?};
  const themeURL = {theme_url:?};
  const registeredBlocks = {blocks};
  const registeredComponents = {components};
  const registeredTraits = {traits};
  const registeredThemes = {themes};
  let currentThemeId = {current_theme};
  const dotLottieCDN = {DOTLOTTIE_CDN_URL:?};
  const dotLottieAttr = {DOTLOTTIE_SCRIPT_ATTR:?};

  function themeById(id) {{
    if (!id) return null;
    for (var i = 0; i < (registeredThemes || []).length; i++) {{
      if (registeredThemes[i].id === id) return registeredThemes[i];
    }}
    return null;
  }}

  function applyHeaderHeadHtml(editor, headHtml) {{
    if (!headHtml) return;
    var frame = editor && editor.Canvas && editor.Canvas.getFrameEl && editor.Canvas.getFrameEl();
    var doc = frame && frame.contentDocument;
    if (!doc || !doc.head) return;
    Array.prototype.slice.call(doc.querySelectorAll('[data-lariv-header-head]')).forEach(function (n) {{
      n.parentNode && n.parentNode.removeChild(n);
    }});
    var tpl = doc.createElement('template');
    tpl.innerHTML = headHtml;
    Array.prototype.slice.call(tpl.content.childNodes).forEach(function (node) {{
      var clone = node.cloneNode(true);
      if (clone.nodeType === 1) clone.setAttribute('data-lariv-header-head', '');
      doc.head.appendChild(clone);
    }});
  }}

  function hasRouteRefs(result) {{
    return !!(result && (result.header_html || result.footer_html));
  }}

  function buildThreeRegionComponent(result) {{
    var header = (result && result.header_html) || '';
    var content = (result && result.content_html) || '';
    var footer = (result && result.footer_html) || '';
    return (
      '<div data-gjs-type="p_website.lariv-region" data-lariv-region="header" class="lariv-region-locked">' + header + '</div>' +
      '<div data-gjs-type="p_website.lariv-content" data-lariv-region="content">' + content + '</div>' +
      '<div data-gjs-type="p_website.lariv-region" data-lariv-region="footer" class="lariv-region-locked">' + footer + '</div>'
    );
  }}

  function refreshRefRegions(editor, refs) {{
    if (!refs || !editor) return;
    var wrapper = editor.getWrapper && editor.getWrapper();
    if (!wrapper) return;
    var header = wrapper.find('[data-lariv-region="header"]')[0];
    var footer = wrapper.find('[data-lariv-region="footer"]')[0];
    if (header && refs.header_html !== undefined) {{
      header.components(refs.header_html);
    }}
    if (footer && refs.footer_html !== undefined) {{
      footer.components(refs.footer_html);
    }}
  }}

  function getContentHtml(ed) {{
    var wrapper = ed.getWrapper && ed.getWrapper();
    if (!wrapper) return ed.getHtml();
    var contentComp = wrapper.find('[data-lariv-region="content"]')[0];
    if (!contentComp) return ed.getHtml();
    return contentComp.components().map(function (c) {{ return c.toHTML(); }}).join('');
  }}

  function syncNavbarLogos(ed) {{
    var wrapper = ed.getWrapper && ed.getWrapper();
    if (!wrapper) return;
    var navs = wrapper.find('[data-gjs-type="p_website.navbar"]');
    if (!navs || !navs.length) return;
    for (var i = 0; i < navs.length; i++) {{
      var nav = navs[i];
      if (typeof nav.renderNavbar === 'function') nav.renderNavbar();
    }}
  }}

  var pendingRefRefresh = null;
  var serverContentHtml = '';

  function applyThemeToCanvas(editor, themeId) {{
    var frame = editor && editor.Canvas && editor.Canvas.getFrameEl && editor.Canvas.getFrameEl();
    var doc = frame && frame.contentDocument;
    if (!doc || !doc.head) return;
    Array.prototype.slice.call(doc.querySelectorAll('[data-lariv-theme]')).forEach(function (n) {{
      n.parentNode && n.parentNode.removeChild(n);
    }});
    var theme = themeById(themeId);
    if (!theme) return;
    (theme.scripts || []).forEach(function (src) {{
      if (!src) return;
      var script = doc.createElement('script');
      script.src = src;
      script.setAttribute('data-lariv-theme', themeId);
      doc.head.appendChild(script);
    }});
    (theme.stylesheets || []).forEach(function (href) {{
      if (!href) return;
      var link = doc.createElement('link');
      link.rel = 'stylesheet';
      link.href = href;
      link.setAttribute('data-lariv-theme', themeId);
      doc.head.appendChild(link);
    }});
    if (theme.css) {{
      var style = doc.createElement('style');
      if (theme.css_type) style.type = theme.css_type;
      style.setAttribute('data-lariv-theme', themeId);
      style.textContent = theme.css;
      doc.head.appendChild(style);
    }}
  }}

  function normalizeBlockProps(props) {{
    if (typeof props.onClick === 'string') {{
      const body = props.onClick;
      props.onClick = function (block, editor) {{
        return new Function('block', 'editor', body)(block, editor);
      }};
    }}
    return props;
  }}

  function reviveTraitProps(props) {{
    ['createInput', 'createLabel', 'onEvent', 'onUpdate'].forEach(function (key) {{
      if (typeof props[key] === 'string') {{
        const body = props[key];
        props[key] = function (p) {{
          p = p || {{}};
          return new Function('trait', 'elInput', 'component', 'event', 'label', body)(
            p.trait, p.elInput, p.component, p.event, p.label
          );
        }};
      }}
    }});
    if (typeof props.templateInput === 'string' && props.templateInput.indexOf('<') !== 0) {{
      const body = props.templateInput;
      props.templateInput = function (p) {{
        p = p || {{}};
        return new Function('trait', body)(p.trait);
      }};
    }}
    return props;
  }}

  function reviveObjectMethods(obj, keys) {{
    if (!obj || typeof obj !== 'object') return;
    keys.forEach(function (key) {{
      if (typeof obj[key] === 'string') {{
        const body = obj[key];
        obj[key] = function () {{
          return new Function(body).call(this);
        }};
      }}
    }});
  }}

  function normalizeComponentProps(props) {{
    if (typeof props.isComponent === 'string') {{
      const body = props.isComponent;
      props.isComponent = function (el) {{
        return new Function('el', body)(el);
      }};
    }}
    if (props.model && typeof props.model === 'object') {{
      reviveObjectMethods(props.model, ['init', 'updated', 'removed']);
    }}
    if (props.view && typeof props.view === 'object') {{
      reviveObjectMethods(props.view, ['init', 'onRender', 'onRemove']);
    }}
    return props;
  }}

  window.__larivEnsureDotLottie = function (doc) {{
    doc = doc || document;
    if (!doc || !doc.head) return;
    if (doc.querySelector('script[' + dotLottieAttr + ']')) return;
    if (doc.defaultView && doc.defaultView.customElements && doc.defaultView.customElements.get('dotlottie-wc')) return;
    const s = doc.createElement('script');
    s.type = 'module';
    s.src = dotLottieCDN;
    s.setAttribute(dotLottieAttr, '');
    doc.head.appendChild(s);
  }};

  const editor = grapesjs.init({{
    container: '#gjs',
    height: '100%',
    width: 'auto',
    fromElement: false,
    assetManager: {{
      upload: uploadURL,
      uploadName: 'files',
      multiUpload: true,
      autoAdd: true,
      embedAsBase64: false,
      credentials: 'include'
    }},
    storageManager: {{
      type: 'remote',
      autosave: true,
      autoload: true,
      stepsBeforeSave: 3,
      options: {{
        remote: {{
          urlLoad: loadURL,
          urlStore: storeURL,
          onStore: (data, ed) => {{
            syncNavbarLogos(ed);
            return {{
              data: data,
              html: getContentHtml(ed),
              css: ed.getCss()
            }};
          }},
          onLoad: (result) => {{
            serverContentHtml = (result && result.content_html) || '';
            if (result && hasRouteRefs(result)) {{
              pendingRefRefresh = {{
                header_html: result.header_html || '',
                footer_html: result.footer_html || '',
                header_head_html: result.header_head_html || '',
                content_html: serverContentHtml,
              }};
              if (result.data) return result.data;
              return {{ pages: [{{ name: 'Page', component: buildThreeRegionComponent(result) }}] }};
            }}
            pendingRefRefresh = null;
            if (result && result.data) return result.data;
            if (result && result.content_html) {{
              return {{ pages: [{{ name: 'Page', component: result.content_html }}] }};
            }}
            if (result && result.html) {{
              return {{ pages: [{{ name: 'Page', component: result.html }}] }};
            }}
            return {{ pages: [{{ name: 'Page', component: '<h1>New page</h1>' }}] }};
          }}
        }}
      }}
    }}
  }});

  const tm = editor.TraitManager;
  (registeredTraits || []).forEach(function (trait) {{
    const id = trait.id;
    const props = Object.assign({{}}, trait);
    delete props.id;
    tm.addType(id, reviveTraitProps(props));
  }});

  const dc = editor.DomComponents;
  (registeredComponents || []).forEach(function (comp) {{
    const id = comp.id;
    const props = Object.assign({{}}, comp);
    delete props.id;
    dc.addType(id, normalizeComponentProps(props));
  }});

  const bm = editor.BlockManager;
  (registeredBlocks || []).forEach(function (block) {{
    const id = block.id;
    const props = Object.assign({{}}, block);
    delete props.id;
    bm.add(id, normalizeBlockProps(props));
  }});

  function syncThemeSelect() {{
    var sel = document.getElementById('gjs-theme-select');
    if (!sel) return;
    sel.innerHTML = '';
    var none = document.createElement('option');
    none.value = '';
    none.textContent = 'None';
    sel.appendChild(none);
    (registeredThemes || []).forEach(function (theme) {{
      var opt = document.createElement('option');
      opt.value = theme.id;
      opt.textContent = theme.label || theme.id;
      sel.appendChild(opt);
    }});
    sel.value = currentThemeId || '';
  }}

  syncThemeSelect();
  editor.on('load', function () {{
    applyThemeToCanvas(editor, currentThemeId);
    syncNavbarLogos(editor);
    if (pendingRefRefresh) {{
      applyHeaderHeadHtml(editor, pendingRefRefresh.header_head_html);
      var wrapper = editor.getWrapper && editor.getWrapper();
      var hasContentRegion = wrapper && wrapper.find('[data-lariv-region="content"]').length;
      if (hasContentRegion) {{
        refreshRefRegions(editor, pendingRefRefresh);
      }} else if (pendingRefRefresh.content_html !== undefined) {{
        editor.setComponents(buildThreeRegionComponent(pendingRefRefresh));
      }}
      pendingRefRefresh = null;
    }}
  }});
  applyThemeToCanvas(editor, currentThemeId);

  var themeSelect = document.getElementById('gjs-theme-select');
  if (themeSelect) {{
    themeSelect.addEventListener('change', function () {{
      var next = themeSelect.value || '';
      themeSelect.disabled = true;
      fetch(themeURL, {{
        method: 'POST',
        credentials: 'include',
        headers: {{ 'Content-Type': 'application/json' }},
        body: JSON.stringify({{ theme: next }})
      }}).then(function (res) {{
        if (!res.ok) throw new Error('theme save failed');
        currentThemeId = next;
        applyThemeToCanvas(editor, currentThemeId);
      }}).catch(function (err) {{
        console.error('Failed to save theme', err);
        themeSelect.value = currentThemeId || '';
      }}).finally(function () {{ themeSelect.disabled = false; }});
    }});
  }}

  const saveBtn = document.getElementById('gjs-save-btn');
  const saveStatus = document.getElementById('gjs-save-status');
  function setSaveStatus(text) {{ if (saveStatus) saveStatus.textContent = text || ''; }}
  if (saveBtn) {{
    saveBtn.addEventListener('click', function () {{
      saveBtn.disabled = true;
      setSaveStatus('Saving…');
      Promise.resolve(editor.store())
        .then(function () {{ setSaveStatus('Saved'); }})
        .catch(function (err) {{
          console.error('GrapesJS store failed', err);
          setSaveStatus('Save failed');
        }})
        .finally(function () {{ saveBtn.disabled = false; }});
    }});
  }}
  window.__gjsEditor = editor;
}})();"#
    );

    format!(
        r#"
<div class="gjs-builder-wrap">
  <div class="gjs-builder-bar">
    <a class="gjs-builder-btn gjs-builder-btn-outline" href="{detail_esc}" hx-boost="false">← Back to route</a>
    <span class="gjs-builder-path">Editing {path_esc}</span>
    <div class="gjs-builder-bar-actions">
      <div class="gjs-builder-theme-group">
        <label class="gjs-builder-label" for="gjs-theme-select">Theme</label>
        <select id="gjs-theme-select" class="gjs-builder-select"></select>
      </div>
      <span id="gjs-save-status" class="gjs-builder-save-status" aria-live="polite"></span>
      <button type="button" id="gjs-save-btn" class="gjs-builder-btn gjs-builder-btn-primary">Save</button>
    </div>
  </div>
  <div id="gjs"></div>
</div>
<script>{script}</script>
"#
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
