//! Length input: millimetres stored as `Decimal(15, 6)`, with a unit selector.
//!
//! The submitted value is always millimetres. The visible control lets the user
//! type and switch between mm, cm, m, km, in, and ft without changing storage.

use maud::{Markup, PreEscaped, html};

use crate::components::attrs::{HtmlAttrs, escape_attr};
use crate::length::{self, LengthUnit, format_nm, normalize_mm, parse_mm};

/// Length input with a unit selector. `value` is millimetres (`Decimal(15, 6)`).
pub struct InputLength<'a> {
    pub label: &'a str,
    pub name: &'a str,
    /// Canonical millimetre string.
    pub value: &'a str,
    /// Initial display unit (`mm`, `cm`, `m`, `km`, `in`, `ft`). Defaults to mm.
    pub unit: &'a str,
    pub required: bool,
    pub classes: &'a str,
    pub attrs: HtmlAttrs,
}

impl Default for InputLength<'_> {
    fn default() -> Self {
        Self {
            label: "Length",
            name: "",
            value: "",
            unit: "mm",
            required: false,
            classes: "",
            attrs: HtmlAttrs::new(),
        }
    }
}

/// Render a length input. Submits millimetres; the selector is display-only.
pub fn input_length(opts: InputLength<'_>) -> Markup {
    let unit = length::parse_length_unit(opts.unit).unwrap_or(LengthUnit::Millimetre);
    let (mm, display) = match normalize_mm(opts.value) {
        Ok(mm) if mm.is_empty() => (String::new(), String::new()),
        Ok(mm) => {
            let nm = parse_mm(&mm).unwrap_or(0);
            (mm, format_nm(nm, unit))
        }
        Err(_) => (opts.value.trim().to_string(), opts.value.trim().to_string()),
    };
    let alpine = length_alpine_data(&mm, unit);
    let required_attr = if opts.required { " required" } else { "" };
    let wrap = format!("my-1 {}", opts.classes);
    html! {
        (PreEscaped(format!(
            r#"<div class="{}" x-data="{}" x-init="bindForm()">"#,
            escape_attr(&wrap),
            escape_attr(&alpine),
        )))
        label class="label text-sm font-bold flex flex-col items-start gap-1 w-full" {
            @if !opts.label.is_empty() {
                span { (opts.label) }
            }
            div class="join w-full" {
                (PreEscaped(format!(
                    r#"<input type="text" inputmode="decimal" x-model="display" @input="syncMm(false)" value="{}" placeholder="0" class="input input-bordered join-item min-w-0 flex-1" autocomplete="off" spellcheck="false"{}{}>"#,
                    escape_attr(&display),
                    required_attr,
                    opts.attrs.as_string(),
                )))
                (PreEscaped(
                    r#"<select class="select select-bordered join-item w-20 shrink-0" x-model="unit" @change="mmToDisplay()" aria-label="Length unit">"#,
                ))
                @for u in LengthUnit::ALL {
                    option value=(u.as_str()) selected[u == unit] { (u.as_str()) }
                }
                (PreEscaped("</select>"))
            }
            @if !opts.name.is_empty() {
                (PreEscaped(format!(
                    r#"<input type="hidden" name="{}" x-model="mm" value="{}">"#,
                    escape_attr(opts.name),
                    escape_attr(&mm),
                )))
            }
            p class="text-error text-sm" x-bind:hidden="!error" x-text="error" role="alert" {}
        }
        (PreEscaped("</div>"))
    }
}

fn json_str(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into())
}

fn length_alpine_data(mm: &str, unit: LengthUnit) -> String {
    let mut units_js = String::from("{");
    for (i, u) in LengthUnit::ALL.iter().enumerate() {
        if i > 0 {
            units_js.push(',');
        }
        units_js.push_str(u.as_str());
        units_js.push(':');
        units_js.push_str(&u.nm_per_unit().to_string());
    }
    units_js.push('}');
    format!(
        r#"{{
            mm: {mm},
            display: {display},
            unit: {unit},
            error: '',
            units: {units_js},
            maxNm: 999999999999999n,
            bindForm() {{
                const form = this.$el.closest('form');
                if (!form) {{
                    return;
                }}
                form.addEventListener('submit', (event) => {{
                    if (!this.syncMm(true)) {{
                        event.preventDefault();
                        event.stopImmediatePropagation();
                    }}
                }});
            }},
            parseToNm(str, unitNm) {{
                str = String(str || '').trim().replace(/,/g, '');
                if (!str) {{
                    return {{ empty: true }};
                }}
                let neg = false;
                if (str[0] === '-' || str[0] === '+') {{
                    neg = str[0] === '-';
                    str = str.slice(1).trim();
                }}
                if (!str || str === '.') {{
                    return {{ incomplete: true }};
                }}
                const parts = str.split('.');
                if (parts.length > 2) {{
                    return {{ invalid: true }};
                }}
                const intRaw = parts[0] || '0';
                let frac = parts[1] || '';
                if (!/^\d+$/.test(intRaw) || (frac && !/^\d+$/.test(frac))) {{
                    return {{ invalid: true }};
                }}
                if (frac.length > 18) {{
                    frac = frac.slice(0, 18);
                }}
                try {{
                    const fracDen = 10n ** BigInt(frac.length);
                    const intN = BigInt(intRaw.replace(/^0+(?=\d)/, '') || '0');
                    const fracN = frac ? BigInt(frac) : 0n;
                    const numer = (intN * fracDen + fracN) * BigInt(unitNm);
                    let nm = numer / fracDen;
                    if ((numer % fracDen) * 2n >= fracDen) {{
                        nm += 1n;
                    }}
                    if (neg) {{
                        nm = -nm;
                    }}
                    return {{ nm }};
                }} catch (e) {{
                    return {{ invalid: true }};
                }}
            }},
            formatUnit(nm, unitNm) {{
                const neg = nm < 0n;
                if (neg) {{
                    nm = -nm;
                }}
                const denom = BigInt(unitNm);
                const whole = nm / denom;
                let rem = nm % denom;
                const sign = neg ? '-' : '';
                if (rem === 0n) {{
                    return sign + String(whole);
                }}
                let frac = '';
                for (let i = 0; i < 18 && rem !== 0n; i++) {{
                    rem *= 10n;
                    frac += String(rem / denom);
                    rem %= denom;
                }}
                frac = frac.replace(/0+$/, '');
                return sign + String(whole) + '.' + frac;
            }},
            formatMm(nm) {{
                return this.formatUnit(nm, this.units.mm);
            }},
            syncMm(fromSubmit) {{
                this.error = '';
                const parsed = this.parseToNm(this.display, this.units[this.unit]);
                if (parsed.empty) {{
                    this.mm = '';
                    return true;
                }}
                if (parsed.incomplete) {{
                    if (fromSubmit) {{
                        this.error = 'Enter a valid length';
                        return false;
                    }}
                    return true;
                }}
                if (parsed.invalid || typeof parsed.nm !== 'bigint') {{
                    this.error = 'Enter a valid length';
                    return false;
                }}
                const abs = parsed.nm < 0n ? -parsed.nm : parsed.nm;
                if (abs > this.maxNm) {{
                    this.error = 'Length is too large';
                    return false;
                }}
                this.mm = this.formatMm(parsed.nm);
                return true;
            }},
            mmToDisplay() {{
                this.error = '';
                const parsed = this.parseToNm(this.mm, this.units.mm);
                if (parsed.empty) {{
                    this.display = '';
                    return;
                }}
                if (typeof parsed.nm !== 'bigint') {{
                    return;
                }}
                this.display = this.formatUnit(parsed.nm, this.units[this.unit]);
            }}
        }}"#,
        mm = json_str(mm),
        display = json_str(&{
            match parse_mm(mm) {
                Ok(nm) => format_nm(nm, unit),
                Err(_) => mm.to_string(),
            }
        }),
        unit = json_str(unit.as_str()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::attrs::alpine_js_leaked_as_text;
    use crate::components::field::{FieldLength, field_length};

    #[test]
    fn input_length_submits_mm_and_lists_units() {
        let html = input_length(InputLength {
            label: "Width",
            name: "WidthMm",
            value: "25.4",
            unit: "in",
            required: true,
            ..Default::default()
        })
        .into_string();
        assert!(html.contains("Width"));
        assert!(html.contains(r#"name="WidthMm""#));
        assert!(html.contains(r#"type="hidden""#));
        assert!(html.contains(r#"value="25.4""#));
        assert!(html.contains(r#"value="1""#));
        for unit in ["mm", "cm", "m", "km", "in", "ft"] {
            assert!(
                html.contains(&format!(r#"value="{unit}""#)),
                "missing unit {unit} in {html}"
            );
        }
        assert!(html.contains("required"));
        assert!(html.contains("join"));
        assert!(html.contains("999999999999999n"));
        assert!(html.contains("in:25400000"));
        assert!(html.contains("ft:304800000"));
        assert!(
            !alpine_js_leaked_as_text(&html),
            "Alpine JS rendered as text: {html}"
        );
    }

    #[test]
    fn input_length_empty_name_omits_hidden() {
        let html = input_length(InputLength {
            label: "",
            name: "",
            value: "10",
            ..Default::default()
        })
        .into_string();
        assert!(!html.contains(r#"type="hidden""#));
        assert!(html.contains(r#"type="text""#));
    }

    #[test]
    fn input_length_empty_omits_value() {
        let html = input_length(InputLength {
            name: "Len",
            value: "",
            ..Default::default()
        })
        .into_string();
        assert!(html.contains(r#"name="Len""#));
        assert!(html.contains("mm: &quot;&quot;"));
    }

    #[test]
    fn field_length_converts_mm_to_unit() {
        let html = field_length(FieldLength {
            value: "25.4",
            unit: "in",
            classes: "",
        })
        .into_string();
        assert!(html.contains("1 in"));
    }
}
