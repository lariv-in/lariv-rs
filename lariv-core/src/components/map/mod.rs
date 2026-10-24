//! MapLibre location and range picker.
//!
//! The library is served from `/maplibre.js` and `/maplibre.css` (no CDN script).
//! Each map root registers `Alpine.data('larivMap')` and calls `map.remove()` from
//! Alpine `destroy()`, `htmx:beforeCleanupElement`, and a disconnect observer.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::escape_attr;
use crate::components::label::label_hint;
use crate::components::shell::{maplibre_css_href, maplibre_js_href};

/// Editor or review map for a point and a radius in meters.
pub struct LocationRangeInput<'a> {
    pub label: &'a str,
    pub hint: Option<&'a str>,
    /// Shown above a read-only map (the outside-zone modal).
    pub caption: Option<&'a str>,
    pub latitude_name: &'a str,
    pub longitude_name: &'a str,
    pub range_name: &'a str,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub range_meters: Option<f64>,
    pub readonly: bool,
    pub user_latitude: Option<f64>,
    pub user_longitude: Option<f64>,
    /// Tooltip on the center marker. Empty leaves the marker unlabeled.
    pub center_label: &'a str,
    /// Tooltip on the second marker. Empty leaves that marker unlabeled.
    pub user_label: &'a str,
    /// Move the second marker as the punch bar reports new geolocation fixes.
    pub live_user: bool,
}

impl Default for LocationRangeInput<'static> {
    fn default() -> Self {
        Self {
            label: "",
            hint: None,
            caption: None,
            latitude_name: "latitude",
            longitude_name: "longitude",
            range_name: "range_meters",
            latitude: None,
            longitude: None,
            range_meters: None,
            readonly: false,
            user_latitude: None,
            user_longitude: None,
            center_label: "",
            user_label: "",
            live_user: false,
        }
    }
}

fn data_text(name: &str, value: &str) -> String {
    if value.is_empty() {
        String::new()
    } else {
        format!(r#" {name}="{}""#, escape_attr(value))
    }
}

fn data_num(name: &str, value: Option<f64>) -> String {
    match value {
        Some(number) => format!(r#" {name}="{number}""#),
        None => String::new(),
    }
}

/// Classic script so it runs during HTML parse, before the deferred Alpine
/// bundle calls `Alpine.start()`. `alpine:init` then registers `larivMap`
/// before Alpine evaluates `x-data`. The MapLibre module loads afterwards.
fn map_alpine_data() -> &'static str {
    r#"<script>
(function () {
  function registerLarivMap() {
    if (!window.Alpine || window.__larivMapData) return;
    window.__larivMapData = true;
    window.Alpine.data("larivMap", function () {
      return {
        alive: true,
        init: function () { this.mountWhenReady(); },
        mountWhenReady: async function () {
          if (!window.LarivMap) {
            await new Promise(function (resolve) {
              var timer = setInterval(function () {
                if (!window.LarivMap) return;
                clearInterval(timer);
                resolve();
              }, 20);
              window.addEventListener("lariv-map-ready", function () {
                if (!window.LarivMap) return;
                clearInterval(timer);
                resolve();
              }, { once: true });
            });
          }
          if (!this.alive || !this.$el.isConnected) return;
          window.LarivMap.mount(this.$el);
        },
        destroy: function () {
          this.alive = false;
          if (this.$el._larivMap) this.$el._larivMap.teardown();
        }
      };
    });
  }
  document.addEventListener("alpine:init", registerLarivMap);
  registerLarivMap();
})();
</script>"#
}

fn map_bootstrap() -> String {
    let href = escape_attr(&maplibre_js_href());
    let mut script = String::from(map_alpine_data());
    script.push_str(r#"<script type="module">"#);
    script.push_str("\nif (!window.LarivMap) {\nconst maplibregl = await import(\"");
    script.push_str(&href);
    script.push_str("\");\n");
    script.push_str(include_str!("client.js"));
    script.push_str("\nwindow.dispatchEvent(new Event(\"lariv-map-ready\"));\n}\n</script>");
    script
}

/// Render the location map. Editor mode writes hidden latitude, longitude, and range inputs.
pub fn location_range_input(opts: LocationRangeInput<'_>) -> Markup {
    let css = maplibre_css_href();
    let readonly = if opts.readonly {
        r#" data-readonly="true""#
    } else {
        ""
    };
    let live_user = if opts.live_user {
        r#" data-live-user="true""#
    } else {
        ""
    };
    let height = if opts.readonly { "h-80" } else { "h-96" };
    let data = format!(
        "{}{}{}{}{}{}{}{}{}",
        data_num("data-lat", opts.latitude),
        data_num("data-lng", opts.longitude),
        data_num("data-range", opts.range_meters),
        data_num("data-user-lat", opts.user_latitude),
        data_num("data-user-lng", opts.user_longitude),
        data_text("data-center-label", opts.center_label),
        data_text("data-user-label", opts.user_label),
        readonly,
        live_user,
    );
    let editor = if opts.readonly {
        html! {}
    } else {
        let range_value = opts
            .range_meters
            .map(|meters| format!("{meters:.0}"))
            .unwrap_or_default();
        let lat_value = opts
            .latitude
            .map(|value| value.to_string())
            .unwrap_or_default();
        let lng_value = opts
            .longitude
            .map(|value| value.to_string())
            .unwrap_or_default();
        html! {
            div class="mt-2 flex flex-wrap items-end gap-3" {
                label class="form-control w-40" {
                    span class="label-text text-xs" { "Range (meters)" }
                    input type="number" min="1" max="100000" step="1" class="input input-bordered input-sm w-full"
                        data-range-visible="true" value=(range_value);
                }
                button type="button" class="btn btn-ghost btn-sm" data-map-clear="true" {
                    "Clear location"
                }
            }
            p class="mt-2 text-sm" data-map-status="" {}
            input type="hidden" data-lat-input="true" name=(opts.latitude_name) value=(lat_value);
            input type="hidden" data-lng-input="true" name=(opts.longitude_name) value=(lng_value);
            input type="hidden" data-range-input="true" name=(opts.range_name) value=(opts.range_meters.map(|meters| meters.to_string()).unwrap_or_default());
        }
    };
    let map = html! {
        (PreEscaped(format!(
            r#"<link rel="stylesheet" href="{}">"#,
            escape_attr(&css)
        )))
        (PreEscaped(map_bootstrap()))
        (PreEscaped(format!(
            r#"<div class="lariv-map w-full" data-lariv-map-root x-data="larivMap"{data}>"#
        )))
        @if let Some(caption) = opts.caption.filter(|text| !text.is_empty()) {
            p class="mb-2 text-sm" { (caption) }
        }
        div data-map-canvas class=(format!("{height} w-full overflow-hidden rounded-box border border-base-300")) {}
        (editor)
        (PreEscaped("</div>"))
    };
    if opts.label.is_empty() && opts.hint.is_none() {
        map
    } else {
        label_hint(opts.label, opts.hint, map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_imports_vendored_maplibre_and_tears_down() {
        let html = location_range_input(LocationRangeInput {
            label: "Attendance location",
            latitude_name: "attendance_latitude",
            longitude_name: "attendance_longitude",
            range_name: "attendance_range_meters",
            latitude: Some(12.97),
            longitude: Some(77.59),
            range_meters: Some(100.0),
            ..Default::default()
        })
        .into_string();
        assert!(html.contains(&maplibre_js_href()), "{html}");
        assert!(html.contains(&maplibre_css_href()), "{html}");
        assert!(html.contains("data-lariv-map-root"), "{html}");
        assert!(html.contains("x-data=\"larivMap\""), "{html}");
        assert!(html.contains("alpine:init"), "{html}");
        assert!(html.contains("Alpine.data(\"larivMap\""), "{html}");
        assert!(html.contains("htmx:beforeCleanupElement"), "{html}");
        assert!(html.contains("teardown"), "{html}");
        assert!(html.contains("map.remove"), "{html}");
        assert!(html.contains("enableHighAccuracy: true"), "{html}");
        assert!(html.contains("getCurrentPosition"), "{html}");
        assert!(html.contains("placeFix(pos, false)"), "{html}");
        assert!(html.contains("placeFix(pos, true)"), "{html}");
        assert!(html.contains("map.jumpTo"), "{html}");
        assert!(html.contains("enableHighAccuracy: false"), "{html}");
        assert!(html.contains("!fine && haveFine"), "{html}");
        assert!(html.contains("data-map-status"), "{html}");
        assert!(html.contains("name=\"attendance_latitude\""), "{html}");
        assert!(html.contains("name=\"attendance_longitude\""), "{html}");
        assert!(html.contains("name=\"attendance_range_meters\""), "{html}");
        assert!(html.contains("lariv:geolocation"), "{html}");
        assert!(html.contains("__larivGeoFix"), "{html}");
        assert!(!html.contains("unpkg.com"), "{html}");
        assert!(!html.contains("cdn.jsdelivr.net"), "{html}");
    }

    #[test]
    fn live_user_marks_a_readonly_map_as_following_the_watch() {
        let html = location_range_input(LocationRangeInput {
            latitude: Some(12.97),
            longitude: Some(77.59),
            range_meters: Some(100.0),
            readonly: true,
            user_latitude: Some(13.01),
            user_longitude: Some(77.61),
            live_user: true,
            ..Default::default()
        })
        .into_string();
        assert!(html.contains("data-readonly=\"true\""), "{html}");
        assert!(html.contains("data-live-user=\"true\""), "{html}");
        assert!(html.contains("data-user-lat=\"13.01\""), "{html}");
        assert!(html.contains("data-user-lng=\"77.61\""), "{html}");
    }

    #[test]
    fn readonly_maps_are_not_live_by_default() {
        let html = location_range_input(LocationRangeInput {
            latitude: Some(12.97),
            longitude: Some(77.59),
            readonly: true,
            ..Default::default()
        })
        .into_string();
        assert!(!html.contains("data-live-user"), "{html}");
    }
}
