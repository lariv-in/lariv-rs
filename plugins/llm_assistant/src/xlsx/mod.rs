//! Rune `xlsx` module over [`rust_xlsxwriter`].
//!
//! Scripts build a workbook with the same objects as the Rust API, then
//! `Workbook.save` writes a new virtual-filesystem node. Host paths are not
//! accepted.

mod chart;
mod format;
mod io;
mod objects;
mod parse;
mod sheet;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use rune::module::Module;
use rune::{Any, Context, Value};
use rust_xlsxwriter::{Workbook, Worksheet, XlsxError};
use sea_orm::DatabaseConnection;

use lariv_core::rune_env::RuneEnvCtx;
use lariv_plugin_filesystem::storage::{DynFilestore, ErasedFilestore};
use objects::XlsxProperties;

pub(crate) const API: &str = r#"xlsx module wraps rust_xlsxwriter. Construct objects, then call methods. Enum and color arguments are strings: colors are names (red) or #RRGGBB; borders thin/medium/dashed/dotted/thick/double/hair; aligns left/center/right/top/bottom; chart types column/bar/line/pie/area/scatter/doughnut/radar/stock and stacked variants; table styles light9/medium2/dark1. Methods that can fail return Result. Host filesystem paths are not accepted.

Workbook.new() -> Workbook
Workbook.add_worksheet() -> Worksheet
Workbook.add_worksheet_with_constant_memory() -> Worksheet
Workbook.add_worksheet_with_low_memory() -> Worksheet
Workbook.add_chartsheet() -> Worksheet
Workbook.worksheet_from_index(index) -> Worksheet
Workbook.worksheet_from_name(name) -> Worksheet
Workbook.define_name(name, formula)
Workbook.set_properties(DocProperties)
Workbook.use_excel_2023_theme()
Workbook.read_only_recommended()
Workbook.set_vba_name(name)
Workbook.use_zip_large_file(bool)
Workbook.add_vba_project(vnode_path)
Workbook.add_vba_project_with_signature(project_path, signature_path)
Workbook.use_custom_theme(vnode_path)
Workbook.image(vnode_path) -> Image
Workbook.save(path) -> #{ id, name, path, bytes, sheets }  // creates a VNode; appends .xlsx unless the name ends in .xlsx or .xlsm; fails if the path exists

Worksheet.write(row, col, string|number|bool|null|Formula|Url|ExcelDateTime)
Worksheet.write_string / write_string_with_format / write_number / write_number_with_format / write_boolean
Worksheet.write_formula / write_formula_with_format / write_url / write_url_with_text
Worksheet.write_datetime(row, col, ExcelDateTime, Format) / write_blank(row, col, Format) / write_row(row, col, array)
Worksheet.merge_range((r1, c1, r2, c2), text, Format)
Worksheet.set_name(name) / name()
Worksheet.set_column_width(col, width) / set_column_range_width(c1, c2, width) / set_row_height(row, height)
Worksheet.set_column_format(col, Format) / set_row_format(row, Format)
Worksheet.autofilter(r1, c1, r2, c2) / filter_column(col, FilterCondition)
Worksheet.insert_image / insert_chart / insert_note / insert_shape
Worksheet.add_table((r1, c1, r2, c2), Table) / add_sparkline(row, col, Sparkline) / add_data_validation((r1, c1, r2, c2), DataValidation)
Worksheet.add_conditional_format_cell((r1, c1, r2, c2), format) and the same range tuple for _formula / _text / _blank / _duplicate / _error / _average / _top / _date / _2_color_scale / _3_color_scale / _data_bar
Worksheet.protect() / protect_with_password(password) / protect_with_options(ProtectionOptions)
Worksheet.set_freeze_panes(row, col) / set_tab_color(color) / set_header(text) / set_footer(text) / set_header_image(Image, left|center|right)
Worksheet.autofit() / set_active(bool) / set_hidden(bool) / set_zoom(percent) / set_landscape() / set_portrait() / set_print_area(r1, c1, r2, c2)
Worksheet.group_rows(r1, r2, collapsed) / group_columns(c1, c2, collapsed)

Format.new() then set_bold, set_italic, set_font_color, set_font_name, set_font_size, set_font_scheme, set_font_family, set_font_charset, set_underline, set_font_script, set_align, set_indent, set_rotation, set_reading_direction, set_text_wrap, set_shrink, set_num_format, set_num_format_index, set_pattern, set_background_color, set_foreground_color, set_border, set_border_color, set_border_top, set_border_top_color, set_border_bottom, set_border_bottom_color, set_border_left, set_border_left_color, set_border_right, set_border_right_color, set_border_diagonal, set_border_diagonal_color, set_border_diagonal_type, set_font_strikethrough, set_hyperlink, set_unlocked, set_hidden, set_locked, set_quote_prefix, set_checkbox, and the matching unset_* methods. merge(other) combines two formats. Each setter returns a new Format.

Formula.new(text)
Url.new(link).set_text(text).set_tip(tip)
ExcelDateTime.parse(text) / from_ymd(year, month, day) / from_hms(hour, min, sec)
Image.from_buffer(bytes) or Workbook.image(vnode_path). set_width, set_height, set_scale_to_size(w, h, keep_aspect), set_alt_text, set_decorative, set_url, set_object_movement
Note.new(text). set_author, set_size(w, h), set_visible, set_background_color, set_font(name, size), set_format
DocProperties.new(). set(field, text) for title/subject/manager/company/category/author/keywords/comment/status/hyperlink_base. set_creation_datetime(ExcelDateTime)
Table.new(). set_flag(header_row|total_row|banded_rows|banded_columns|first_column|last_column|autofilter, bool), set_name, set_style, set_alt_text, set_columns([TableColumn])
TableColumn.new(). set_header, set_total_function(sum|average|count|count_numbers|max|min|stddev|var|none), set_total_label, set_formula, set_format, set_header_format
DataValidation.new(). allow_list_strings([string]), allow_list_formula, allow_whole_number(op, n, second?), allow_decimal_number, allow_text_length, allow_custom, allow_any_value, set_flag(ignore_blank|show_dropdown|show_input_message|show_error_message, bool), set_input(title, message), set_error(title, message, stop|warning|information)
ConditionalFormatCell.new(). set_number_rule(op, n, second?) / set_text_rule(equal_to|not_equal_to, text) / set_format
ConditionalFormatFormula.new(). set_rule(formula, Format)
ConditionalFormatText.new(). set_rule(contains|does_not_contain|begins_with|ends_with, text, Format)
ConditionalFormatBlank/Duplicate/Error.new(). set_format
ConditionalFormatAverage.new(). set_rule(above_average|below_average|equal_or_above_average|equal_or_below_average|one_standard_deviation_above|..., Format)
ConditionalFormatTop.new(). set_rule(top|bottom|top_percent|bottom_percent, count, Format)
ConditionalFormatDate.new(). set_rule(yesterday|today|tomorrow|last_7_days|last_week|this_week|next_week|last_month|this_month|next_month, Format)
ConditionalFormat2ColorScale.new(). set_colors(min, max)
ConditionalFormat3ColorScale.new(). set_colors(min, mid, max)
ConditionalFormatDataBar.new(). set_colors(fill, border) / set_flags(solid, bar_only)
Sparkline.new(). set_range, set_type(line|column|win_lose), set_color(sparkline|high|low|first|last|negative|markers, color)
Shape.textbox(). set_text, set_size(w, h), set_object_movement
FilterCondition.new(). add_list_filter(value) / add_custom_filter(criteria, value)
ProtectionOptions.new(). set(select_locked_cells|select_unlocked_cells|format_cells|format_columns|format_rows|insert_columns|insert_rows|insert_links|delete_columns|delete_rows|sort|use_autofilter|use_pivot_tables|edit_scenarios|edit_objects|contents, bool)
Chart.new(type) or Chart.new_column/new_line/new_bar/new_pie/new_area/new_scatter/new_doughnut/new_radar/new_stock
Chart.push_series(ChartSeries) / combine(Chart) / set_title / hide_title / set_axis_name(x|y|x2|y2, name) / set_axis_num_format(axis, format) / set_axis_limit(axis, min|max, number) / hide_legend / set_legend_position / show_empty_cells_as(gaps|zero|connected) / set_size(w, h) / set_style / set_name
ChartSeries.new(). set_values(range) / set_categories(range) / set_name / set_format(ChartFormat) / set_marker / set_data_label / set_trendline / set_overlap / set_gap / set_smooth / set_secondary_axis
ChartFormat.new(). set_line(ChartLine) / set_solid_fill(ChartSolidFill) / set_no_line / set_no_fill
ChartLine.new(). set_color / set_width / set_dash_type(solid|round_dot|square_dot|dash|dash_dot|long_dash|long_dash_dot|long_dash_dot_dot)
ChartSolidFill.new(). set_color
ChartFont.new(). set_name / set_size / set_color / set_bold / set_italic
ChartMarker.new(). set_type(square|diamond|triangle|x|star|short_dash|long_dash|circle|plus) / set_size / set_none
ChartDataLabel.new(). show(value|category|series|percent|leader) / set_num_format
ChartTrendline.new(). set_type(none|linear|exponential|logarithmic|power|polynomial|moving_average, period?) / set_name / display(equation, r_squared)
"#;

struct Shared {
    book: RefCell<Workbook>,
    db: DatabaseConnection,
    store: Arc<DynFilestore>,
}

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = Workbook)]
struct XlsxWorkbook {
    shared: Rc<Shared>,
}

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = Worksheet)]
pub(crate) struct XlsxWorksheet {
    shared: Rc<Shared>,
    index: usize,
}

pub(crate) fn number(value: Value) -> Result<f64, String> {
    if let Ok(value) = value.as_integer::<i64>() {
        return Ok(value as f64);
    }
    value
        .as_float()
        .map_err(|_| "expected a number".to_string())
}

pub(crate) fn row(value: i64) -> Result<u32, String> {
    u32::try_from(value).map_err(|_| format!("row {value} is out of range"))
}

pub(crate) fn col(value: i64) -> Result<u16, String> {
    u16::try_from(value).map_err(|_| format!("column {value} is out of range"))
}

pub(crate) fn i64_u8(value: i64, what: &str) -> Result<u8, String> {
    u8::try_from(value).map_err(|_| format!("{what} {value} is out of range"))
}

pub(crate) fn i64_u16(value: i64, what: &str) -> Result<u16, String> {
    u16::try_from(value).map_err(|_| format!("{what} {value} is out of range"))
}

pub(crate) fn i64_u32(value: i64, what: &str) -> Result<u32, String> {
    u32::try_from(value).map_err(|_| format!("{what} {value} is out of range"))
}

pub(crate) fn with_sheet(
    sheet: &XlsxWorksheet,
    f: impl FnOnce(&mut Worksheet) -> Result<(), XlsxError>,
) -> Result<(), String> {
    let mut book = sheet.shared.book.borrow_mut();
    let worksheet = book
        .worksheet_from_index(sheet.index)
        .map_err(|e| e.to_string())?;
    f(worksheet).map_err(|e| e.to_string())
}

pub(crate) fn with_sheet_msg(
    sheet: &XlsxWorksheet,
    f: impl FnOnce(&mut Worksheet) -> Result<(), String>,
) -> Result<(), String> {
    let mut book = sheet.shared.book.borrow_mut();
    let worksheet = book
        .worksheet_from_index(sheet.index)
        .map_err(|e| e.to_string())?;
    f(worksheet)
}

pub(crate) fn with_sheet_mut(
    sheet: &XlsxWorksheet,
    f: impl FnOnce(&mut Worksheet),
) -> Result<(), String> {
    let mut book = sheet.shared.book.borrow_mut();
    let worksheet = book
        .worksheet_from_index(sheet.index)
        .map_err(|e| e.to_string())?;
    f(worksheet);
    Ok(())
}

fn worksheet_handle(shared: &Rc<Shared>, index: usize) -> XlsxWorksheet {
    XlsxWorksheet {
        shared: Rc::clone(shared),
        index,
    }
}

fn add_sheet(book: &XlsxWorkbook, kind: SheetKind) -> Result<XlsxWorksheet, String> {
    let mut workbook = book.shared.book.borrow_mut();
    match kind {
        SheetKind::Normal => {
            workbook.add_worksheet();
        }
        SheetKind::Constant => {
            workbook.add_worksheet_with_constant_memory();
        }
        SheetKind::Low => {
            workbook.add_worksheet_with_low_memory();
        }
        SheetKind::Chart => {
            workbook.add_chartsheet();
        }
    }
    let index = workbook
        .worksheets()
        .len()
        .checked_sub(1)
        .ok_or_else(|| "workbook has no worksheet".to_string())?;
    Ok(worksheet_handle(&book.shared, index))
}

enum SheetKind {
    Normal,
    Constant,
    Low,
    Chart,
}

fn worksheet_from_index(book: &XlsxWorkbook, index: i64) -> Result<XlsxWorksheet, String> {
    let index = usize::try_from(index).map_err(|_| format!("worksheet index {index}"))?;
    book.shared
        .book
        .borrow_mut()
        .worksheet_from_index(index)
        .map_err(|e| e.to_string())?;
    Ok(worksheet_handle(&book.shared, index))
}

fn worksheet_from_name(book: &XlsxWorkbook, name: &str) -> Result<XlsxWorksheet, String> {
    let mut workbook = book.shared.book.borrow_mut();
    workbook
        .worksheet_from_name(name)
        .map_err(|e| e.to_string())?;
    let index = workbook
        .worksheets()
        .iter()
        .position(|sheet| sheet.name() == name)
        .ok_or_else(|| format!("worksheet {name} was not found"))?;
    Ok(worksheet_handle(&book.shared, index))
}

fn define_name(book: &XlsxWorkbook, name: &str, formula: &str) -> Result<(), String> {
    book.shared
        .book
        .borrow_mut()
        .define_name(name, formula)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn set_properties(book: &XlsxWorkbook, properties: &XlsxProperties) {
    let properties = properties.0.borrow().clone();
    book.shared.book.borrow_mut().set_properties(&properties);
}

fn use_theme(book: &XlsxWorkbook) -> Result<(), String> {
    book.shared
        .book
        .borrow_mut()
        .use_excel_2023_theme()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn read_only(book: &XlsxWorkbook) {
    book.shared.book.borrow_mut().read_only_recommended();
}

fn vba_name(book: &XlsxWorkbook, name: &str) -> Result<(), String> {
    book.shared
        .book
        .borrow_mut()
        .set_vba_name(name)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn zip_large(book: &XlsxWorkbook, enable: bool) {
    book.shared.book.borrow_mut().use_zip_large_file(enable);
}

fn vba_project(book: &XlsxWorkbook, path: &str) -> Result<(), String> {
    let bytes = io::read_vnode(&book.shared.db, &book.shared.store, path)?;
    let mut workbook = book.shared.book.borrow_mut();
    io::with_temp_file(&bytes, ".bin", |file| workbook.add_vba_project(file))?;
    Ok(())
}

fn vba_signature(book: &XlsxWorkbook, project: &str, signature: &str) -> Result<(), String> {
    let project_bytes = io::read_vnode(&book.shared.db, &book.shared.store, project)?;
    let signature_bytes = io::read_vnode(&book.shared.db, &book.shared.store, signature)?;
    let mut workbook = book.shared.book.borrow_mut();
    io::with_temp_file(&project_bytes, ".bin", |project| {
        let signature_bytes = signature_bytes.clone();
        io::with_temp_file(&signature_bytes, ".bin", |signature| {
            workbook.add_vba_project_with_signature(project, signature)
        })
        .map_err(|e| rust_xlsxwriter::XlsxError::ParameterError(e))
    })?;
    Ok(())
}

fn custom_theme(book: &XlsxWorkbook, path: &str) -> Result<(), String> {
    let bytes = io::read_vnode(&book.shared.db, &book.shared.store, path)?;
    let mut workbook = book.shared.book.borrow_mut();
    io::with_temp_file(&bytes, ".bin", |file| workbook.use_custom_theme(file))?;
    Ok(())
}

fn image_from_vnode(book: &XlsxWorkbook, path: &str) -> Result<objects::XlsxImage, String> {
    let bytes = io::read_vnode(&book.shared.db, &book.shared.store, path)?;
    rust_xlsxwriter::Image::new_from_buffer(&bytes)
        .map(|image| objects::XlsxImage(RefCell::new(image)))
        .map_err(|e| e.to_string())
}

fn save(book: &XlsxWorkbook, path: &str) -> Result<Value, String> {
    let (bytes, sheets) = {
        let mut workbook = book.shared.book.borrow_mut();
        let sheets = workbook.worksheets().len();
        let bytes = workbook.save_to_buffer().map_err(|e| e.to_string())?;
        (bytes, sheets)
    };
    let db = book.shared.db.clone();
    let store = Arc::clone(&book.shared.store);
    let path = path.to_string();
    let saved = lariv_core::rune_env::block_on_async(async move {
        io::save_vnode(&db, store.as_ref(), &path, bytes, sheets).await
    })?;
    lariv_core::rune_env::json_to_rune(saved)
}

pub(crate) fn install(context: &mut Context, env_ctx: &RuneEnvCtx<'_>) -> Result<(), String> {
    let mut module = Module::with_item(["xlsx"]).map_err(|e| e.to_string())?;
    module.ty::<XlsxWorkbook>().map_err(|e| e.to_string())?;
    let db_for_new = env_ctx.db.clone();
    let store_for_new = env_ctx.store.filestore_arc();
    {
        let mut ctors = Module::with_item(["xlsx", "Workbook"]).map_err(|e| e.to_string())?;
        ctors
            .function("new", move || XlsxWorkbook {
                shared: Rc::new(Shared {
                    book: RefCell::new(Workbook::new()),
                    db: db_for_new.clone(),
                    store: Arc::clone(&store_for_new),
                }),
            })
            .build()
            .map_err(|e| e.to_string())?;
        context.install(ctors).map_err(|e| e.to_string())?;
    }
    module
        .associated_function("add_worksheet", |book: &XlsxWorkbook| {
            add_sheet(book, SheetKind::Normal)
        })
        .map_err(|e| e.to_string())?;
    module
        .associated_function(
            "add_worksheet_with_constant_memory",
            |book: &XlsxWorkbook| add_sheet(book, SheetKind::Constant),
        )
        .map_err(|e| e.to_string())?;
    module
        .associated_function("add_worksheet_with_low_memory", |book: &XlsxWorkbook| {
            add_sheet(book, SheetKind::Low)
        })
        .map_err(|e| e.to_string())?;
    module
        .associated_function("add_chartsheet", |book: &XlsxWorkbook| {
            add_sheet(book, SheetKind::Chart)
        })
        .map_err(|e| e.to_string())?;
    module
        .associated_function("worksheet_from_index", worksheet_from_index)
        .map_err(|e| e.to_string())?;
    module
        .associated_function("worksheet_from_name", worksheet_from_name)
        .map_err(|e| e.to_string())?;
    module
        .associated_function("define_name", define_name)
        .map_err(|e| e.to_string())?;
    module
        .associated_function("set_properties", set_properties)
        .map_err(|e| e.to_string())?;
    module
        .associated_function("use_excel_2023_theme", use_theme)
        .map_err(|e| e.to_string())?;
    module
        .associated_function("read_only_recommended", read_only)
        .map_err(|e| e.to_string())?;
    module
        .associated_function("set_vba_name", vba_name)
        .map_err(|e| e.to_string())?;
    module
        .associated_function("use_zip_large_file", zip_large)
        .map_err(|e| e.to_string())?;
    module
        .associated_function("add_vba_project", vba_project)
        .map_err(|e| e.to_string())?;
    module
        .associated_function("add_vba_project_with_signature", vba_signature)
        .map_err(|e| e.to_string())?;
    module
        .associated_function("use_custom_theme", custom_theme)
        .map_err(|e| e.to_string())?;
    module
        .associated_function("image", image_from_vnode)
        .map_err(|e| e.to_string())?;
    module
        .associated_function("save", save)
        .map_err(|e| e.to_string())?;
    format::register(context, &mut module)?;
    objects::register(context, &mut module)?;
    chart::register(context, &mut module)?;
    sheet::register(&mut module)?;
    context.install(module).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    use sea_orm::{ConnectionTrait, Database, Schema};

    use crate::rune_engine;
    use crate::rune_env::Hook;
    use lariv_core::rune_env::{RuneEnvCapability, RuneEnvCtx, RuneEnvRegistrar};
    use lariv_plugin_filesystem::entities::filesystem_node;
    use lariv_plugin_filesystem::storage::{DynFilestore, LocalFilestore};

    #[tokio::test(flavor = "multi_thread")]
    async fn workbook_save_writes_a_vnode() {
        let db = Database::connect("sqlite::memory:").await.expect("sqlite");
        let schema = Schema::new(db.get_database_backend());
        db.execute(&schema.create_table_from_entity(filesystem_node::Entity))
            .await
            .expect("table");
        let dir = std::env::temp_dir().join(format!("lariv-xlsx-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let store: Arc<DynFilestore> = Arc::new(LocalFilestore::new(dir.to_string_lossy()));
        let mut cap = RuneEnvCapability::new();
        Hook.register_rune_env(&mut cap);
        let env_ctx = RuneEnvCtx {
            db: &db,
            store: lariv_plugin_filesystem::storage::erase_store(Arc::clone(&store)),
            session_id: None,
        };
        let script = r##"
            fn must(value) {
                match value {
                    Ok(found) => found,
                    Err(error) => panic(error),
                }
            }
            let book = xlsx::Workbook::new();
            let sheet = must(book.add_worksheet());
            must(sheet.set_name("Parts"));
            let bold = must(xlsx::Format::new().set_bold().set_font_color("#112233"));
            must(sheet.write_string_with_format(0, 0, "name", bold));
            must(sheet.write_string(0, 1, "qty"));
            must(sheet.write_string(1, 0, "bolt"));
            must(sheet.write_number(1, 1, 3));
            let series = xlsx::ChartSeries::new();
            series.set_values("Parts!$B$2:$B$2");
            let chart = must(xlsx::Chart::new("column"));
            chart.set_title("Qty");
            chart.push_series(series);
            must(sheet.insert_chart(0, 3, chart));
            let saved = must(book.save("/reports/parts"));
            let again = book.save("/reports/parts");
            match again {
                Ok(_) => panic("overwrite should fail"),
                Err(error) => if !error.contains("already exists") { panic(error); },
            }
            saved
        "##;
        let out = rune_engine::compile_and_run(&cap, &env_ctx, script, &[]).await;
        assert!(
            out.get("error").is_none(),
            "{}",
            out.get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("save failed")
        );
        let result = out.get("result").expect("result");
        assert_eq!(
            result.get("path").and_then(|v| v.as_str()),
            Some("/reports/parts.xlsx")
        );
        assert_eq!(result.get("sheets").and_then(|v| v.as_u64()), Some(1));

        let read = rune_engine::compile_and_run(
            &cap,
            &env_ctx,
            r#"read_table(#{ path: "/reports/parts.xlsx", sheet: "Parts" })"#,
            &[],
        )
        .await;
        assert!(
            read.get("error").is_none(),
            "{}",
            read.get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("read failed")
        );
        let rows = read
            .get("result")
            .and_then(|v| v.get("rows"))
            .and_then(|v| v.as_array())
            .expect("rows");
        let name = rows
            .first()
            .and_then(|row| row.get("name"))
            .and_then(|v| v.as_str());
        assert_eq!(name, Some("bolt"));
    }
}
