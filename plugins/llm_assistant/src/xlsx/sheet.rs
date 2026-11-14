//! Worksheet methods. Names match `rust_xlsxwriter::Worksheet`.

use rune::Value;
use rune::module::Module;
use rust_xlsxwriter::Worksheet;

use super::chart::XlsxChart;
use super::format::XlsxFormat;
use super::objects::{
    XlsxCf2Color, XlsxCf3Color, XlsxCfAverage, XlsxCfBlank, XlsxCfCell, XlsxCfDataBar, XlsxCfDate,
    XlsxCfDuplicate, XlsxCfError, XlsxCfFormula, XlsxCfText, XlsxCfTop, XlsxDateTime, XlsxFilter,
    XlsxFormula, XlsxImage, XlsxNote, XlsxProtection, XlsxShape, XlsxSparkline, XlsxTable, XlsxUrl,
    XlsxValidation,
};
use super::parse;
use super::{XlsxWorksheet, col, number, row, with_sheet, with_sheet_msg, with_sheet_mut};

macro_rules! assoc {
    ($module:expr, $name:literal, $func:expr) => {
        $module
            .associated_function($name, $func)
            .map_err(|err| err.to_string())?;
    };
}

fn cell_value(sheet: &mut Worksheet, row: u32, col: u16, value: &Value) -> Result<(), String> {
    if value.clone().into_unit().is_ok() {
        sheet
            .write_blank(row, col, &rust_xlsxwriter::Format::new())
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if let Ok(value) = value.as_bool() {
        sheet
            .write_boolean(row, col, value)
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if let Ok(value) = value.as_integer::<i64>() {
        sheet.write(row, col, value).map_err(|e| e.to_string())?;
        return Ok(());
    }
    if let Ok(value) = value.as_float() {
        sheet
            .write_number(row, col, value)
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if let Ok(value) = value.borrow_string_ref() {
        sheet
            .write_string(row, col, &*value)
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if let Ok(value) = rune::from_value::<XlsxFormula>(value.clone()) {
        sheet
            .write_formula(row, col, value.0)
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if let Ok(value) = rune::from_value::<XlsxUrl>(value.clone()) {
        sheet
            .write_url(row, col, value.0)
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if let Ok(value) = rune::from_value::<XlsxDateTime>(value.clone()) {
        sheet
            .write_datetime(row, col, &value.0)
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    Err("cell value must be a string, number, bool, null, Formula, Url, or ExcelDateTime".into())
}

fn write(this: &XlsxWorksheet, at_row: i64, at_col: i64, value: Value) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    with_sheet_msg(this, |sheet| cell_value(sheet, at_row, at_col, &value))
}

fn write_string(this: &XlsxWorksheet, at_row: i64, at_col: i64, value: &str) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    with_sheet(this, |sheet| {
        sheet.write_string(at_row, at_col, value).map(|_| ())
    })
}

fn write_string_with_format(
    this: &XlsxWorksheet,
    at_row: i64,
    at_col: i64,
    value: &str,
    format: &XlsxFormat,
) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    let format = format.0.clone();
    with_sheet(this, |sheet| {
        sheet
            .write_string_with_format(at_row, at_col, value, &format)
            .map(|_| ())
    })
}

fn write_number(
    this: &XlsxWorksheet,
    at_row: i64,
    at_col: i64,
    value: Value,
) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    let value = number(value)?;
    with_sheet(this, |sheet| {
        sheet.write_number(at_row, at_col, value).map(|_| ())
    })
}

fn write_number_with_format(
    this: &XlsxWorksheet,
    at_row: i64,
    at_col: i64,
    value: Value,
    format: &XlsxFormat,
) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    let value = number(value)?;
    let format = format.0.clone();
    with_sheet(this, |sheet| {
        sheet
            .write_number_with_format(at_row, at_col, value, &format)
            .map(|_| ())
    })
}

fn write_boolean(
    this: &XlsxWorksheet,
    at_row: i64,
    at_col: i64,
    value: bool,
) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    with_sheet(this, |sheet| {
        sheet.write_boolean(at_row, at_col, value).map(|_| ())
    })
}

fn write_formula(
    this: &XlsxWorksheet,
    at_row: i64,
    at_col: i64,
    formula: &str,
) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    with_sheet(this, |sheet| {
        sheet.write_formula(at_row, at_col, formula).map(|_| ())
    })
}

fn write_formula_with_format(
    this: &XlsxWorksheet,
    at_row: i64,
    at_col: i64,
    formula: &str,
    format: &XlsxFormat,
) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    let format = format.0.clone();
    with_sheet(this, |sheet| {
        sheet
            .write_formula_with_format(at_row, at_col, formula, &format)
            .map(|_| ())
    })
}

fn write_url(this: &XlsxWorksheet, at_row: i64, at_col: i64, url: &str) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    with_sheet(this, |sheet| {
        sheet.write_url(at_row, at_col, url).map(|_| ())
    })
}

fn write_url_with_text(
    this: &XlsxWorksheet,
    at_row: i64,
    at_col: i64,
    url: &str,
    text: &str,
) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    with_sheet(this, |sheet| {
        sheet
            .write_url_with_text(at_row, at_col, url, text)
            .map(|_| ())
    })
}

fn write_datetime(
    this: &XlsxWorksheet,
    at_row: i64,
    at_col: i64,
    when: &XlsxDateTime,
    format: &XlsxFormat,
) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    let when = when.0.clone();
    let format = format.0.clone();
    with_sheet(this, |sheet| {
        sheet
            .write_datetime_with_format(at_row, at_col, &when, &format)
            .map(|_| ())
    })
}

fn write_blank(
    this: &XlsxWorksheet,
    at_row: i64,
    at_col: i64,
    format: &XlsxFormat,
) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    let format = format.0.clone();
    with_sheet(this, |sheet| {
        sheet.write_blank(at_row, at_col, &format).map(|_| ())
    })
}

fn write_row(
    this: &XlsxWorksheet,
    at_row: i64,
    at_col: i64,
    values: Vec<Value>,
) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    with_sheet_msg(this, |sheet| {
        for (offset, value) in values.iter().enumerate() {
            let offset = u16::try_from(offset).map_err(|_| "column is out of range".to_string())?;
            let at_col = at_col
                .checked_add(offset)
                .ok_or_else(|| "column is out of range".to_string())?;
            cell_value(sheet, at_row, at_col, value)?;
        }
        Ok(())
    })
}

fn rect(range: (i64, i64, i64, i64)) -> Result<(u32, u16, u32, u16), String> {
    Ok((row(range.0)?, col(range.1)?, row(range.2)?, col(range.3)?))
}

fn merge_range(
    this: &XlsxWorksheet,
    range: (i64, i64, i64, i64),
    text: &str,
    format: &XlsxFormat,
) -> Result<(), String> {
    let (first_row, first_col, last_row, last_col) = rect(range)?;
    let format = format.0.clone();
    with_sheet(this, |sheet| {
        sheet
            .merge_range(first_row, first_col, last_row, last_col, text, &format)
            .map(|_| ())
    })
}

fn set_name(this: &XlsxWorksheet, name: &str) -> Result<(), String> {
    with_sheet(this, |sheet| sheet.set_name(name).map(|_| ()))
}

fn name(this: &XlsxWorksheet) -> Result<String, String> {
    let mut book = this.shared.book.borrow_mut();
    let sheet = book
        .worksheet_from_index(this.index)
        .map_err(|e| e.to_string())?;
    Ok(sheet.name())
}

fn column_width(this: &XlsxWorksheet, at_col: i64, width: Value) -> Result<(), String> {
    let at_col = col(at_col)?;
    let width = number(width)?;
    with_sheet(this, |sheet| {
        sheet.set_column_width(at_col, width).map(|_| ())
    })
}

fn column_range_width(
    this: &XlsxWorksheet,
    first: i64,
    last: i64,
    width: Value,
) -> Result<(), String> {
    let first = col(first)?;
    let last = col(last)?;
    let width = number(width)?;
    with_sheet(this, |sheet| {
        sheet.set_column_range_width(first, last, width).map(|_| ())
    })
}

fn row_height(this: &XlsxWorksheet, at_row: i64, height: Value) -> Result<(), String> {
    let at_row = row(at_row)?;
    let height = number(height)?;
    with_sheet(this, |sheet| {
        sheet.set_row_height(at_row, height).map(|_| ())
    })
}

fn column_format(this: &XlsxWorksheet, at_col: i64, format: &XlsxFormat) -> Result<(), String> {
    let at_col = col(at_col)?;
    let format = format.0.clone();
    with_sheet(this, |sheet| {
        sheet.set_column_format(at_col, &format).map(|_| ())
    })
}

fn row_format(this: &XlsxWorksheet, at_row: i64, format: &XlsxFormat) -> Result<(), String> {
    let at_row = row(at_row)?;
    let format = format.0.clone();
    with_sheet(this, |sheet| {
        sheet.set_row_format(at_row, &format).map(|_| ())
    })
}

fn autofilter(
    this: &XlsxWorksheet,
    first_row: i64,
    first_col: i64,
    last_row: i64,
    last_col: i64,
) -> Result<(), String> {
    let first_row = row(first_row)?;
    let first_col = col(first_col)?;
    let last_row = row(last_row)?;
    let last_col = col(last_col)?;
    with_sheet(this, |sheet| {
        sheet
            .autofilter(first_row, first_col, last_row, last_col)
            .map(|_| ())
    })
}

fn filter_column(this: &XlsxWorksheet, at_col: i64, filter: &XlsxFilter) -> Result<(), String> {
    let at_col = col(at_col)?;
    let filter = filter.0.borrow().clone();
    with_sheet(this, |sheet| {
        sheet.filter_column(at_col, &filter).map(|_| ())
    })
}

fn insert_image(
    this: &XlsxWorksheet,
    at_row: i64,
    at_col: i64,
    image: &XlsxImage,
) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    let image = image.0.borrow().clone();
    with_sheet(this, |sheet| {
        sheet.insert_image(at_row, at_col, &image).map(|_| ())
    })
}

fn insert_chart(
    this: &XlsxWorksheet,
    at_row: i64,
    at_col: i64,
    chart: &XlsxChart,
) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    let chart = chart.0.borrow().clone();
    with_sheet(this, |sheet| {
        sheet.insert_chart(at_row, at_col, &chart).map(|_| ())
    })
}

fn insert_note(
    this: &XlsxWorksheet,
    at_row: i64,
    at_col: i64,
    note: &XlsxNote,
) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    let note = note.0.borrow().clone();
    with_sheet(this, |sheet| {
        sheet.insert_note(at_row, at_col, &note).map(|_| ())
    })
}

fn insert_shape(
    this: &XlsxWorksheet,
    at_row: i64,
    at_col: i64,
    shape: &XlsxShape,
) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    let shape = shape.0.borrow().clone();
    with_sheet(this, |sheet| {
        sheet.insert_shape(at_row, at_col, &shape).map(|_| ())
    })
}

fn add_table(
    this: &XlsxWorksheet,
    range: (i64, i64, i64, i64),
    table: &XlsxTable,
) -> Result<(), String> {
    let (first_row, first_col, last_row, last_col) = rect(range)?;
    let table = table.0.borrow().clone();
    with_sheet(this, |sheet| {
        sheet
            .add_table(first_row, first_col, last_row, last_col, &table)
            .map(|_| ())
    })
}

fn add_sparkline(
    this: &XlsxWorksheet,
    at_row: i64,
    at_col: i64,
    spark: &XlsxSparkline,
) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    let spark = spark.0.borrow().clone();
    with_sheet(this, |sheet| {
        sheet.add_sparkline(at_row, at_col, &spark).map(|_| ())
    })
}

fn add_validation(
    this: &XlsxWorksheet,
    range: (i64, i64, i64, i64),
    rule: &XlsxValidation,
) -> Result<(), String> {
    let (first_row, first_col, last_row, last_col) = rect(range)?;
    let rule = rule.0.borrow().clone();
    with_sheet(this, |sheet| {
        sheet
            .add_data_validation(first_row, first_col, last_row, last_col, &rule)
            .map(|_| ())
    })
}

fn protect(this: &XlsxWorksheet) -> Result<(), String> {
    with_sheet_mut(this, |sheet| {
        sheet.protect();
    })
}

fn protect_with_password(this: &XlsxWorksheet, password: &str) -> Result<(), String> {
    with_sheet_mut(this, |sheet| {
        sheet.protect_with_password(password);
    })
}

fn protect_with_options(this: &XlsxWorksheet, options: &XlsxProtection) -> Result<(), String> {
    let options = options.0.borrow().clone();
    with_sheet_mut(this, |sheet| {
        sheet.protect_with_options(&options);
    })
}

fn freeze(this: &XlsxWorksheet, at_row: i64, at_col: i64) -> Result<(), String> {
    let at_row = row(at_row)?;
    let at_col = col(at_col)?;
    with_sheet(this, |sheet| {
        sheet.set_freeze_panes(at_row, at_col).map(|_| ())
    })
}

fn tab_color(this: &XlsxWorksheet, color: &str) -> Result<(), String> {
    let color = parse::color(color)?;
    with_sheet_mut(this, |sheet| {
        sheet.set_tab_color(color);
    })
}

fn header(this: &XlsxWorksheet, header: &str) -> Result<(), String> {
    with_sheet_mut(this, |sheet| {
        sheet.set_header(header);
    })
}

fn footer(this: &XlsxWorksheet, footer: &str) -> Result<(), String> {
    with_sheet_mut(this, |sheet| {
        sheet.set_footer(footer);
    })
}

fn header_image(this: &XlsxWorksheet, image: &XlsxImage, position: &str) -> Result<(), String> {
    let position = parse::header_position(position)?;
    let image = image.0.borrow().clone();
    with_sheet(this, |sheet| {
        sheet.set_header_image(&image, position).map(|_| ())
    })
}

fn autofit(this: &XlsxWorksheet) -> Result<(), String> {
    with_sheet_mut(this, |sheet| {
        sheet.autofit();
    })
}

fn active(this: &XlsxWorksheet, enable: bool) -> Result<(), String> {
    with_sheet_mut(this, |sheet| {
        sheet.set_active(enable);
    })
}

fn hidden(this: &XlsxWorksheet, enable: bool) -> Result<(), String> {
    with_sheet_mut(this, |sheet| {
        sheet.set_hidden(enable);
    })
}

fn zoom(this: &XlsxWorksheet, zoom: i64) -> Result<(), String> {
    let zoom = u16::try_from(zoom).map_err(|_| format!("zoom {zoom}"))?;
    with_sheet_mut(this, |sheet| {
        sheet.set_zoom(zoom);
    })
}

fn landscape(this: &XlsxWorksheet) -> Result<(), String> {
    with_sheet_mut(this, |sheet| {
        sheet.set_landscape();
    })
}

fn portrait(this: &XlsxWorksheet) -> Result<(), String> {
    with_sheet_mut(this, |sheet| {
        sheet.set_portrait();
    })
}

fn print_area(
    this: &XlsxWorksheet,
    first_row: i64,
    first_col: i64,
    last_row: i64,
    last_col: i64,
) -> Result<(), String> {
    let first_row = row(first_row)?;
    let first_col = col(first_col)?;
    let last_row = row(last_row)?;
    let last_col = col(last_col)?;
    with_sheet(this, |sheet| {
        sheet
            .set_print_area(first_row, first_col, last_row, last_col)
            .map(|_| ())
    })
}

fn group_rows(this: &XlsxWorksheet, first: i64, last: i64, collapsed: bool) -> Result<(), String> {
    let first = row(first)?;
    let last = row(last)?;
    with_sheet(this, |sheet| {
        if collapsed {
            sheet.group_rows_collapsed(first, last).map(|_| ())
        } else {
            sheet.group_rows(first, last).map(|_| ())
        }
    })
}

fn group_columns(
    this: &XlsxWorksheet,
    first: i64,
    last: i64,
    collapsed: bool,
) -> Result<(), String> {
    let first = col(first)?;
    let last = col(last)?;
    with_sheet(this, |sheet| {
        if collapsed {
            sheet.group_columns_collapsed(first, last).map(|_| ())
        } else {
            sheet.group_columns(first, last).map(|_| ())
        }
    })
}

fn add_cf_cell(
    this: &XlsxWorksheet,
    range: (i64, i64, i64, i64),
    format: &XlsxCfCell,
) -> Result<(), String> {
    add_cf(this, range, format.0.borrow().clone())
}

fn add_cf<T>(this: &XlsxWorksheet, range: (i64, i64, i64, i64), format: T) -> Result<(), String>
where
    T: rust_xlsxwriter::ConditionalFormat + Send + Sync,
{
    let (first_row, first_col, last_row, last_col) = rect(range)?;
    with_sheet(this, |sheet| {
        sheet
            .add_conditional_format(first_row, first_col, last_row, last_col, &format)
            .map(|_| ())
    })
}

pub(crate) fn register(module: &mut Module) -> Result<(), String> {
    module.ty::<XlsxWorksheet>().map_err(|e| e.to_string())?;
    assoc!(module, "write", write);
    assoc!(module, "write_string", write_string);
    assoc!(module, "write_string_with_format", write_string_with_format);
    assoc!(module, "write_number", write_number);
    assoc!(module, "write_number_with_format", write_number_with_format);
    assoc!(module, "write_boolean", write_boolean);
    assoc!(module, "write_formula", write_formula);
    assoc!(
        module,
        "write_formula_with_format",
        write_formula_with_format
    );
    assoc!(module, "write_url", write_url);
    assoc!(module, "write_url_with_text", write_url_with_text);
    assoc!(module, "write_datetime", write_datetime);
    assoc!(module, "write_blank", write_blank);
    assoc!(module, "write_row", write_row);
    assoc!(module, "merge_range", merge_range);
    assoc!(module, "set_name", set_name);
    assoc!(module, "name", name);
    assoc!(module, "set_column_width", column_width);
    assoc!(module, "set_column_range_width", column_range_width);
    assoc!(module, "set_row_height", row_height);
    assoc!(module, "set_column_format", column_format);
    assoc!(module, "set_row_format", row_format);
    assoc!(module, "autofilter", autofilter);
    assoc!(module, "filter_column", filter_column);
    assoc!(module, "insert_image", insert_image);
    assoc!(module, "insert_chart", insert_chart);
    assoc!(module, "insert_note", insert_note);
    assoc!(module, "insert_shape", insert_shape);
    assoc!(module, "add_table", add_table);
    assoc!(module, "add_sparkline", add_sparkline);
    assoc!(module, "add_data_validation", add_validation);
    assoc!(module, "protect", protect);
    assoc!(module, "protect_with_password", protect_with_password);
    assoc!(module, "protect_with_options", protect_with_options);
    assoc!(module, "set_freeze_panes", freeze);
    assoc!(module, "set_tab_color", tab_color);
    assoc!(module, "set_header", header);
    assoc!(module, "set_footer", footer);
    assoc!(module, "set_header_image", header_image);
    assoc!(module, "autofit", autofit);
    assoc!(module, "set_active", active);
    assoc!(module, "set_hidden", hidden);
    assoc!(module, "set_zoom", zoom);
    assoc!(module, "set_landscape", landscape);
    assoc!(module, "set_portrait", portrait);
    assoc!(module, "set_print_area", print_area);
    assoc!(module, "group_rows", group_rows);
    assoc!(module, "group_columns", group_columns);
    assoc!(module, "add_conditional_format_cell", add_cf_cell);
    module
        .associated_function(
            "add_conditional_format_formula",
            |this: &XlsxWorksheet, range: (i64, i64, i64, i64), format: &XlsxCfFormula| {
                add_cf(this, range, format.0.borrow().clone())
            },
        )
        .map_err(|e| e.to_string())?;
    module
        .associated_function(
            "add_conditional_format_text",
            |this: &XlsxWorksheet, range: (i64, i64, i64, i64), format: &XlsxCfText| {
                add_cf(this, range, format.0.borrow().clone())
            },
        )
        .map_err(|e| e.to_string())?;
    module
        .associated_function(
            "add_conditional_format_blank",
            |this: &XlsxWorksheet, range: (i64, i64, i64, i64), format: &XlsxCfBlank| {
                add_cf(this, range, format.0.borrow().clone())
            },
        )
        .map_err(|e| e.to_string())?;
    module
        .associated_function(
            "add_conditional_format_duplicate",
            |this: &XlsxWorksheet, range: (i64, i64, i64, i64), format: &XlsxCfDuplicate| {
                add_cf(this, range, format.0.borrow().clone())
            },
        )
        .map_err(|e| e.to_string())?;
    module
        .associated_function(
            "add_conditional_format_error",
            |this: &XlsxWorksheet, range: (i64, i64, i64, i64), format: &XlsxCfError| {
                add_cf(this, range, format.0.borrow().clone())
            },
        )
        .map_err(|e| e.to_string())?;
    module
        .associated_function(
            "add_conditional_format_average",
            |this: &XlsxWorksheet, range: (i64, i64, i64, i64), format: &XlsxCfAverage| {
                add_cf(this, range, format.0.borrow().clone())
            },
        )
        .map_err(|e| e.to_string())?;
    module
        .associated_function(
            "add_conditional_format_top",
            |this: &XlsxWorksheet, range: (i64, i64, i64, i64), format: &XlsxCfTop| {
                add_cf(this, range, format.0.borrow().clone())
            },
        )
        .map_err(|e| e.to_string())?;
    module
        .associated_function(
            "add_conditional_format_date",
            |this: &XlsxWorksheet, range: (i64, i64, i64, i64), format: &XlsxCfDate| {
                add_cf(this, range, format.0.borrow().clone())
            },
        )
        .map_err(|e| e.to_string())?;
    module
        .associated_function(
            "add_conditional_format_2_color_scale",
            |this: &XlsxWorksheet, range: (i64, i64, i64, i64), format: &XlsxCf2Color| {
                add_cf(this, range, format.0.borrow().clone())
            },
        )
        .map_err(|e| e.to_string())?;
    module
        .associated_function(
            "add_conditional_format_3_color_scale",
            |this: &XlsxWorksheet, range: (i64, i64, i64, i64), format: &XlsxCf3Color| {
                add_cf(this, range, format.0.borrow().clone())
            },
        )
        .map_err(|e| e.to_string())?;
    module
        .associated_function(
            "add_conditional_format_data_bar",
            |this: &XlsxWorksheet, range: (i64, i64, i64, i64), format: &XlsxCfDataBar| {
                add_cf(this, range, format.0.borrow().clone())
            },
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}
