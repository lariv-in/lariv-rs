//! [`rust_xlsxwriter::Format`] exposed as `xlsx::Format`.

use rune::module::Module;
use rune::{Any, Value};
use rust_xlsxwriter::Format;

use super::parse;

#[derive(Any, Clone)]
#[rune(item = ::xlsx)]
#[rune(name = Format)]
pub(crate) struct XlsxFormat(pub Format);

impl XlsxFormat {
    pub(crate) fn new() -> Self {
        Self(Format::new())
    }
}

macro_rules! flag {
    ($($name:ident),* $(,)?) => {
        $(
            fn $name(this: &XlsxFormat) -> XlsxFormat {
                XlsxFormat(this.0.clone().$name())
            }
        )*
    };
}

flag!(
    set_bold,
    set_italic,
    set_font_strikethrough,
    set_text_wrap,
    set_shrink,
    set_hyperlink,
    set_unlocked,
    set_hidden,
    set_quote_prefix,
    set_checkbox,
    unset_bold,
    unset_italic,
    unset_font_strikethrough,
    unset_text_wrap,
    unset_shrink,
    set_locked,
    unset_hidden,
    unset_hyperlink_style,
    unset_quote_prefix,
    unset_checkbox,
);

fn set_num_format(this: &XlsxFormat, format: &str) -> XlsxFormat {
    XlsxFormat(this.0.clone().set_num_format(format))
}

fn set_num_format_index(this: &XlsxFormat, index: i64) -> Result<XlsxFormat, String> {
    let index = u8::try_from(index).map_err(|_| format!("num format index {index}"))?;
    Ok(XlsxFormat(this.0.clone().set_num_format_index(index)))
}

fn set_font_color(this: &XlsxFormat, color: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_font_color(parse::color(color)?),
    ))
}

fn set_font_name(this: &XlsxFormat, name: &str) -> XlsxFormat {
    XlsxFormat(this.0.clone().set_font_name(name))
}

fn set_font_size(this: &XlsxFormat, size: Value) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_font_size(super::number(size)?),
    ))
}

fn set_font_scheme(this: &XlsxFormat, scheme: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_font_scheme(parse::font_scheme(scheme)?),
    ))
}

fn set_font_family(this: &XlsxFormat, family: i64) -> Result<XlsxFormat, String> {
    let family = u8::try_from(family).map_err(|_| format!("font family {family}"))?;
    Ok(XlsxFormat(this.0.clone().set_font_family(family)))
}

fn set_font_charset(this: &XlsxFormat, charset: i64) -> Result<XlsxFormat, String> {
    let charset = u8::try_from(charset).map_err(|_| format!("font charset {charset}"))?;
    Ok(XlsxFormat(this.0.clone().set_font_charset(charset)))
}

fn set_underline(this: &XlsxFormat, underline: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_underline(parse::underline(underline)?),
    ))
}

fn set_font_script(this: &XlsxFormat, script: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_font_script(parse::script(script)?),
    ))
}

fn set_align(this: &XlsxFormat, align: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(this.0.clone().set_align(parse::align(align)?)))
}

fn set_indent(this: &XlsxFormat, indent: i64) -> Result<XlsxFormat, String> {
    let indent = u8::try_from(indent).map_err(|_| format!("indent {indent}"))?;
    Ok(XlsxFormat(this.0.clone().set_indent(indent)))
}

fn set_rotation(this: &XlsxFormat, rotation: i64) -> Result<XlsxFormat, String> {
    let rotation = i16::try_from(rotation).map_err(|_| format!("rotation {rotation}"))?;
    Ok(XlsxFormat(this.0.clone().set_rotation(rotation)))
}

fn set_reading_direction(this: &XlsxFormat, direction: i64) -> Result<XlsxFormat, String> {
    let direction =
        u8::try_from(direction).map_err(|_| format!("reading direction {direction}"))?;
    Ok(XlsxFormat(this.0.clone().set_reading_direction(direction)))
}

fn set_pattern(this: &XlsxFormat, pattern: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_pattern(parse::pattern(pattern)?),
    ))
}

fn set_background_color(this: &XlsxFormat, color: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_background_color(parse::color(color)?),
    ))
}

fn set_foreground_color(this: &XlsxFormat, color: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_foreground_color(parse::color(color)?),
    ))
}

fn set_border(this: &XlsxFormat, border: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_border(parse::border(border)?),
    ))
}

fn set_border_color(this: &XlsxFormat, color: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_border_color(parse::color(color)?),
    ))
}

fn set_border_top(this: &XlsxFormat, border: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_border_top(parse::border(border)?),
    ))
}

fn set_border_top_color(this: &XlsxFormat, color: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_border_top_color(parse::color(color)?),
    ))
}

fn set_border_bottom(this: &XlsxFormat, border: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_border_bottom(parse::border(border)?),
    ))
}

fn set_border_bottom_color(this: &XlsxFormat, color: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_border_bottom_color(parse::color(color)?),
    ))
}

fn set_border_left(this: &XlsxFormat, border: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_border_left(parse::border(border)?),
    ))
}

fn set_border_left_color(this: &XlsxFormat, color: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_border_left_color(parse::color(color)?),
    ))
}

fn set_border_right(this: &XlsxFormat, border: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_border_right(parse::border(border)?),
    ))
}

fn set_border_right_color(this: &XlsxFormat, color: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_border_right_color(parse::color(color)?),
    ))
}

fn set_border_diagonal(this: &XlsxFormat, border: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0.clone().set_border_diagonal(parse::border(border)?),
    ))
}

fn set_border_diagonal_color(this: &XlsxFormat, color: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0
            .clone()
            .set_border_diagonal_color(parse::color(color)?),
    ))
}

fn set_border_diagonal_type(this: &XlsxFormat, kind: &str) -> Result<XlsxFormat, String> {
    Ok(XlsxFormat(
        this.0
            .clone()
            .set_border_diagonal_type(parse::diagonal(kind)?),
    ))
}

fn merge(this: &XlsxFormat, other: &XlsxFormat) -> XlsxFormat {
    XlsxFormat(this.0.merge(&other.0))
}

macro_rules! assoc {
    ($module:expr, $name:literal, $func:expr) => {
        $module
            .associated_function($name, $func)
            .map_err(|err| err.to_string())?;
    };
}

pub(crate) fn register(context: &mut rune::Context, module: &mut Module) -> Result<(), String> {
    module.ty::<XlsxFormat>().map_err(|e| e.to_string())?;
    {
        let mut ctors = Module::with_item(["xlsx", "Format"]).map_err(|e| e.to_string())?;
        ctors
            .function("new", XlsxFormat::new)
            .build()
            .map_err(|e| e.to_string())?;
        context.install(ctors).map_err(|e| e.to_string())?;
    }
    assoc!(module, "merge", merge);
    assoc!(module, "set_bold", set_bold);
    assoc!(module, "set_italic", set_italic);
    assoc!(module, "set_font_strikethrough", set_font_strikethrough);
    assoc!(module, "set_text_wrap", set_text_wrap);
    assoc!(module, "set_shrink", set_shrink);
    assoc!(module, "set_hyperlink", set_hyperlink);
    assoc!(module, "set_unlocked", set_unlocked);
    assoc!(module, "set_hidden", set_hidden);
    assoc!(module, "set_quote_prefix", set_quote_prefix);
    assoc!(module, "set_checkbox", set_checkbox);
    assoc!(module, "unset_bold", unset_bold);
    assoc!(module, "unset_italic", unset_italic);
    assoc!(module, "unset_font_strikethrough", unset_font_strikethrough);
    assoc!(module, "unset_text_wrap", unset_text_wrap);
    assoc!(module, "unset_shrink", unset_shrink);
    assoc!(module, "set_locked", set_locked);
    assoc!(module, "unset_hidden", unset_hidden);
    assoc!(module, "unset_hyperlink_style", unset_hyperlink_style);
    assoc!(module, "unset_quote_prefix", unset_quote_prefix);
    assoc!(module, "unset_checkbox", unset_checkbox);
    assoc!(module, "set_num_format", set_num_format);
    assoc!(module, "set_num_format_index", set_num_format_index);
    assoc!(module, "set_font_color", set_font_color);
    assoc!(module, "set_font_name", set_font_name);
    assoc!(module, "set_font_size", set_font_size);
    assoc!(module, "set_font_scheme", set_font_scheme);
    assoc!(module, "set_font_family", set_font_family);
    assoc!(module, "set_font_charset", set_font_charset);
    assoc!(module, "set_underline", set_underline);
    assoc!(module, "set_font_script", set_font_script);
    assoc!(module, "set_align", set_align);
    assoc!(module, "set_indent", set_indent);
    assoc!(module, "set_rotation", set_rotation);
    assoc!(module, "set_reading_direction", set_reading_direction);
    assoc!(module, "set_pattern", set_pattern);
    assoc!(module, "set_background_color", set_background_color);
    assoc!(module, "set_foreground_color", set_foreground_color);
    assoc!(module, "set_border", set_border);
    assoc!(module, "set_border_color", set_border_color);
    assoc!(module, "set_border_top", set_border_top);
    assoc!(module, "set_border_top_color", set_border_top_color);
    assoc!(module, "set_border_bottom", set_border_bottom);
    assoc!(module, "set_border_bottom_color", set_border_bottom_color);
    assoc!(module, "set_border_left", set_border_left);
    assoc!(module, "set_border_left_color", set_border_left_color);
    assoc!(module, "set_border_right", set_border_right);
    assoc!(module, "set_border_right_color", set_border_right_color);
    assoc!(module, "set_border_diagonal", set_border_diagonal);
    assoc!(
        module,
        "set_border_diagonal_color",
        set_border_diagonal_color
    );
    assoc!(module, "set_border_diagonal_type", set_border_diagonal_type);
    Ok(())
}
