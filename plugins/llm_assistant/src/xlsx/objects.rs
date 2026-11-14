//! Owned `rust_xlsxwriter` values other than formats, charts, and worksheets.

use std::cell::RefCell;

use rune::module::Module;
use rune::runtime::Bytes;
use rune::{Any, Value};
use rust_xlsxwriter::{
    ConditionalFormat2ColorScale, ConditionalFormat3ColorScale, ConditionalFormatAverage,
    ConditionalFormatAverageRule, ConditionalFormatBlank, ConditionalFormatCell,
    ConditionalFormatCellRule, ConditionalFormatDataBar, ConditionalFormatDate,
    ConditionalFormatDateRule, ConditionalFormatDuplicate, ConditionalFormatError,
    ConditionalFormatFormula, ConditionalFormatText, ConditionalFormatTextRule,
    ConditionalFormatTop, ConditionalFormatTopRule, DataValidation, DocProperties, ExcelDateTime,
    FilterCondition, Formula, Image, Note, ProtectionOptions, Shape, Sparkline, Table, TableColumn,
    Url,
};

use super::format::XlsxFormat;
use super::parse;
use super::{i64_u8, i64_u32, number};

fn cell<T>(value: T) -> RefCell<T> {
    RefCell::new(value)
}

macro_rules! ctor {
    ($context:expr, $ty:literal, $name:literal, $f:expr) => {{
        let mut nested = Module::with_item(["xlsx", $ty]).map_err(|e| e.to_string())?;
        nested
            .function($name, $f)
            .build()
            .map_err(|e| e.to_string())?;
        $context.install(nested).map_err(|e| e.to_string())?;
    }};
}

macro_rules! assoc {
    ($module:expr, $name:literal, $func:expr) => {
        $module
            .associated_function($name, $func)
            .map_err(|err| err.to_string())?;
    };
}

#[derive(Any, Clone)]
#[rune(item = ::xlsx)]
#[rune(name = Formula)]
pub(crate) struct XlsxFormula(pub Formula);

#[derive(Any, Clone)]
#[rune(item = ::xlsx)]
#[rune(name = Url)]
pub(crate) struct XlsxUrl(pub Url);

#[derive(Any, Clone)]
#[rune(item = ::xlsx)]
#[rune(name = ExcelDateTime)]
pub(crate) struct XlsxDateTime(pub ExcelDateTime);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = Image)]
pub(crate) struct XlsxImage(pub RefCell<Image>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = Note)]
pub(crate) struct XlsxNote(pub(crate) RefCell<Note>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = DocProperties)]
pub(crate) struct XlsxProperties(pub(crate) RefCell<DocProperties>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = Table)]
pub(crate) struct XlsxTable(pub RefCell<Table>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = TableColumn)]
pub(crate) struct XlsxTableColumn(RefCell<TableColumn>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = DataValidation)]
pub(crate) struct XlsxValidation(pub RefCell<DataValidation>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ConditionalFormatCell)]
pub(crate) struct XlsxCfCell(pub RefCell<ConditionalFormatCell>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ConditionalFormatFormula)]
pub(crate) struct XlsxCfFormula(pub RefCell<ConditionalFormatFormula>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ConditionalFormatText)]
pub(crate) struct XlsxCfText(pub RefCell<ConditionalFormatText>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ConditionalFormatBlank)]
pub(crate) struct XlsxCfBlank(pub RefCell<ConditionalFormatBlank>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ConditionalFormatDuplicate)]
pub(crate) struct XlsxCfDuplicate(pub RefCell<ConditionalFormatDuplicate>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ConditionalFormatError)]
pub(crate) struct XlsxCfError(pub RefCell<ConditionalFormatError>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ConditionalFormatAverage)]
pub(crate) struct XlsxCfAverage(pub RefCell<ConditionalFormatAverage>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ConditionalFormatTop)]
pub(crate) struct XlsxCfTop(pub RefCell<ConditionalFormatTop>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ConditionalFormatDate)]
pub(crate) struct XlsxCfDate(pub RefCell<ConditionalFormatDate>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ConditionalFormat2ColorScale)]
pub(crate) struct XlsxCf2Color(pub RefCell<ConditionalFormat2ColorScale>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ConditionalFormat3ColorScale)]
pub(crate) struct XlsxCf3Color(pub RefCell<ConditionalFormat3ColorScale>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ConditionalFormatDataBar)]
pub(crate) struct XlsxCfDataBar(pub RefCell<ConditionalFormatDataBar>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = Sparkline)]
pub(crate) struct XlsxSparkline(pub RefCell<Sparkline>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = Shape)]
pub(crate) struct XlsxShape(pub RefCell<Shape>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = FilterCondition)]
pub(crate) struct XlsxFilter(pub RefCell<FilterCondition>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ProtectionOptions)]
pub(crate) struct XlsxProtection(pub RefCell<ProtectionOptions>);

fn replace<T: Clone>(slot: &RefCell<T>, update: impl FnOnce(T) -> T) {
    let next = update(slot.borrow().clone());
    *slot.borrow_mut() = next;
}

fn url_text(this: &XlsxUrl, text: &str) -> XlsxUrl {
    XlsxUrl(this.0.clone().set_text(text))
}

fn url_tip(this: &XlsxUrl, tip: &str) -> XlsxUrl {
    XlsxUrl(this.0.clone().set_tip(tip))
}

fn image_buffer(bytes: Bytes) -> Result<XlsxImage, String> {
    Image::new_from_buffer(bytes.as_slice())
        .map(|image| XlsxImage(cell(image)))
        .map_err(|e| e.to_string())
}

fn image_width(this: &XlsxImage, width: i64) -> Result<(), String> {
    let width = i64_u32(width, "width")?;
    replace(&this.0, |image| image.set_width(width));
    Ok(())
}

fn image_height(this: &XlsxImage, height: i64) -> Result<(), String> {
    let height = i64_u32(height, "height")?;
    replace(&this.0, |image| image.set_height(height));
    Ok(())
}

fn image_scale(this: &XlsxImage, width: Value, height: Value, keep: bool) -> Result<(), String> {
    let width = number(width)?;
    let height = number(height)?;
    replace(&this.0, |image| {
        image.set_scale_to_size(width, height, keep)
    });
    Ok(())
}

fn image_alt(this: &XlsxImage, text: &str) {
    replace(&this.0, |image| image.set_alt_text(text));
}

fn image_decorative(this: &XlsxImage, enable: bool) {
    replace(&this.0, |image| image.set_decorative(enable));
}

fn image_url(this: &XlsxImage, url: &str) -> Result<(), String> {
    let updated = this
        .0
        .borrow()
        .clone()
        .set_url(url)
        .map_err(|e| e.to_string())?;
    *this.0.borrow_mut() = updated;
    Ok(())
}

fn image_movement(this: &XlsxImage, movement: &str) -> Result<(), String> {
    let movement = parse::object_movement(movement)?;
    replace(&this.0, |image| image.set_object_movement(movement));
    Ok(())
}

fn note_author(this: &XlsxNote, author: &str) {
    replace(&this.0, |note| note.set_author(author));
}

fn note_size(this: &XlsxNote, width: i64, height: i64) -> Result<(), String> {
    let width = i64_u32(width, "width")?;
    let height = i64_u32(height, "height")?;
    replace(&this.0, |note| note.set_width(width).set_height(height));
    Ok(())
}

fn note_visible(this: &XlsxNote, enable: bool) {
    replace(&this.0, |note| note.set_visible(enable));
}

fn note_color(this: &XlsxNote, color: &str) -> Result<(), String> {
    let color = parse::color(color)?;
    replace(&this.0, |note| note.set_background_color(color));
    Ok(())
}

fn note_font(this: &XlsxNote, name: &str, size: Value) -> Result<(), String> {
    let size = number(size)?;
    replace(&this.0, |note| note.set_font_name(name).set_font_size(size));
    Ok(())
}

fn note_format(this: &XlsxNote, format: &XlsxFormat) {
    let format = format.0.clone();
    replace(&this.0, |note| note.set_format(format));
}

fn props_string(this: &XlsxProperties, field: &str, value: &str) -> Result<(), String> {
    let value = value.to_string();
    replace(&this.0, |props| match field {
        "title" => props.set_title(value),
        "subject" => props.set_subject(value),
        "manager" => props.set_manager(value),
        "company" => props.set_company(value),
        "category" => props.set_category(value),
        "author" => props.set_author(value),
        "keywords" => props.set_keywords(value),
        "comment" => props.set_comment(value),
        "status" => props.set_status(value),
        "hyperlink_base" => props.set_hyperlink_base(value),
        _ => props,
    });
    if !matches!(
        field,
        "title"
            | "subject"
            | "manager"
            | "company"
            | "category"
            | "author"
            | "keywords"
            | "comment"
            | "status"
            | "hyperlink_base"
    ) {
        return Err(format!("unknown property {field}"));
    }
    Ok(())
}

fn props_created(this: &XlsxProperties, when: &XlsxDateTime) {
    let when = when.0.clone();
    replace(&this.0, |props| props.set_creation_datetime(&when));
}

fn table_flag(this: &XlsxTable, field: &str, enable: bool) -> Result<(), String> {
    replace(&this.0, |table| match field {
        "header_row" => table.set_header_row(enable),
        "total_row" => table.set_total_row(enable),
        "banded_rows" => table.set_banded_rows(enable),
        "banded_columns" => table.set_banded_columns(enable),
        "first_column" => table.set_first_column(enable),
        "last_column" => table.set_last_column(enable),
        "autofilter" => table.set_autofilter(enable),
        _ => table,
    });
    if !matches!(
        field,
        "header_row"
            | "total_row"
            | "banded_rows"
            | "banded_columns"
            | "first_column"
            | "last_column"
            | "autofilter"
    ) {
        return Err(format!("unknown table flag {field}"));
    }
    Ok(())
}

fn table_name(this: &XlsxTable, name: &str) {
    replace(&this.0, |table| table.set_name(name));
}

fn table_style(this: &XlsxTable, style: &str) -> Result<(), String> {
    let style = parse::table_style(style)?;
    replace(&this.0, |table| table.set_style(style));
    Ok(())
}

fn table_alt(this: &XlsxTable, text: &str) {
    replace(&this.0, |table| table.set_alt_text(text));
}

fn table_columns(this: &XlsxTable, columns: Vec<Value>) -> Result<(), String> {
    let mut built = Vec::with_capacity(columns.len());
    for column in columns {
        let column = rune::from_value::<XlsxTableColumn>(column).map_err(|e| e.to_string())?;
        built.push(column.0.borrow().clone());
    }
    replace(&this.0, |table| table.set_columns(&built));
    Ok(())
}

fn column_header(this: &XlsxTableColumn, header: &str) {
    replace(&this.0, |column| column.set_header(header));
}

fn column_total(this: &XlsxTableColumn, function: &str) -> Result<(), String> {
    let function = parse::table_function(function)?;
    replace(&this.0, |column| column.set_total_function(function));
    Ok(())
}

fn column_label(this: &XlsxTableColumn, label: &str) {
    replace(&this.0, |column| column.set_total_label(label));
}

fn column_formula(this: &XlsxTableColumn, formula: &str) {
    replace(&this.0, |column| column.set_formula(formula));
}

fn column_format(this: &XlsxTableColumn, format: &XlsxFormat) {
    let format = format.0.clone();
    replace(&this.0, |column| column.set_format(format));
}

fn column_header_format(this: &XlsxTableColumn, format: &XlsxFormat) {
    let format = format.0.clone();
    replace(&this.0, |column| column.set_header_format(format));
}

fn validation_list(this: &XlsxValidation, values: Vec<String>) -> Result<(), String> {
    let refs: Vec<&str> = values.iter().map(String::as_str).collect();
    let updated = this
        .0
        .borrow()
        .clone()
        .allow_list_strings(&refs)
        .map_err(|e| e.to_string())?;
    *this.0.borrow_mut() = updated;
    Ok(())
}

fn validation_list_formula(this: &XlsxValidation, formula: &str) {
    replace(&this.0, |rule| {
        rule.allow_list_formula(Formula::new(formula))
    });
}

fn validation_whole(
    this: &XlsxValidation,
    op: &str,
    first: i64,
    second: Option<i64>,
) -> Result<(), String> {
    let first = i32::try_from(first).map_err(|_| format!("whole number {first}"))?;
    let second = match second {
        Some(value) => Some(i32::try_from(value).map_err(|_| format!("whole number {value}"))?),
        None => None,
    };
    let rule = parse::number_rule(op, first, second)?;
    replace(&this.0, |validation| validation.allow_whole_number(rule));
    Ok(())
}

fn validation_decimal(
    this: &XlsxValidation,
    op: &str,
    first: Value,
    second: Option<Value>,
) -> Result<(), String> {
    let first = number(first)?;
    let second = match second {
        Some(value) => Some(number(value)?),
        None => None,
    };
    let rule = parse::number_rule(op, first, second)?;
    replace(&this.0, |validation| validation.allow_decimal_number(rule));
    Ok(())
}

fn validation_text_length(
    this: &XlsxValidation,
    op: &str,
    first: i64,
    second: Option<i64>,
) -> Result<(), String> {
    let first = i64_u32(first, "text length")?;
    let second = match second {
        Some(value) => Some(i64_u32(value, "text length")?),
        None => None,
    };
    let rule = parse::number_rule(op, first, second)?;
    replace(&this.0, |validation| validation.allow_text_length(rule));
    Ok(())
}

fn validation_custom(this: &XlsxValidation, formula: &str) {
    replace(&this.0, |rule| rule.allow_custom(Formula::new(formula)));
}

fn validation_any(this: &XlsxValidation) {
    replace(&this.0, DataValidation::allow_any_value);
}

fn validation_flag(this: &XlsxValidation, field: &str, enable: bool) -> Result<(), String> {
    replace(&this.0, |rule| match field {
        "ignore_blank" => rule.ignore_blank(enable),
        "show_dropdown" => rule.show_dropdown(enable),
        "show_input_message" => rule.show_input_message(enable),
        "show_error_message" => rule.show_error_message(enable),
        _ => rule,
    });
    if !matches!(
        field,
        "ignore_blank" | "show_dropdown" | "show_input_message" | "show_error_message"
    ) {
        return Err(format!("unknown validation flag {field}"));
    }
    Ok(())
}

fn validation_input(this: &XlsxValidation, title: &str, message: &str) -> Result<(), String> {
    let updated = this
        .0
        .borrow()
        .clone()
        .set_input_title(title)
        .map_err(|e| e.to_string())?
        .set_input_message(message)
        .map_err(|e| e.to_string())?;
    *this.0.borrow_mut() = updated;
    Ok(())
}

fn validation_error(
    this: &XlsxValidation,
    title: &str,
    message: &str,
    style: &str,
) -> Result<(), String> {
    let style = parse::error_style(style)?;
    let updated = this
        .0
        .borrow()
        .clone()
        .set_error_title(title)
        .map_err(|e| e.to_string())?
        .set_error_message(message)
        .map_err(|e| e.to_string())?
        .set_error_style(style);
    *this.0.borrow_mut() = updated;
    Ok(())
}

fn cf_cell_number(
    this: &XlsxCfCell,
    op: &str,
    first: f64,
    second: Option<f64>,
) -> Result<(), String> {
    let rule = match parse::norm(op).as_str() {
        "equalto" | "eq" => ConditionalFormatCellRule::EqualTo(first),
        "notequalto" | "ne" => ConditionalFormatCellRule::NotEqualTo(first),
        "greaterthan" | "gt" => ConditionalFormatCellRule::GreaterThan(first),
        "greaterthanorequalto" | "ge" => ConditionalFormatCellRule::GreaterThanOrEqualTo(first),
        "lessthan" | "lt" => ConditionalFormatCellRule::LessThan(first),
        "lessthanorequalto" | "le" => ConditionalFormatCellRule::LessThanOrEqualTo(first),
        "between" => ConditionalFormatCellRule::Between(
            first,
            second.ok_or_else(|| "between needs a second value".to_string())?,
        ),
        "notbetween" => ConditionalFormatCellRule::NotBetween(
            first,
            second.ok_or_else(|| "not_between needs a second value".to_string())?,
        ),
        other => return Err(format!("unknown conditional rule {other}")),
    };
    replace(&this.0, |format| format.set_rule(rule));
    Ok(())
}

fn cf_cell_text(this: &XlsxCfCell, op: &str, text: &str) -> Result<(), String> {
    let text = text.to_string();
    let rule = match parse::norm(op).as_str() {
        "equalto" | "eq" => ConditionalFormatCellRule::EqualTo(text),
        "notequalto" | "ne" => ConditionalFormatCellRule::NotEqualTo(text),
        other => {
            return Err(format!(
                "text conditional rules support equal_to and not_equal_to, not {other}"
            ));
        }
    };
    replace(&this.0, |format| format.set_rule(rule));
    Ok(())
}

fn cf_cell_format(this: &XlsxCfCell, format: &XlsxFormat) {
    let format = format.0.clone();
    replace(&this.0, |rule| rule.set_format(format));
}

fn cf_formula(this: &XlsxCfFormula, formula: &str, format: &XlsxFormat) {
    let format = format.0.clone();
    replace(&this.0, |rule| {
        rule.set_rule(Formula::new(formula)).set_format(format)
    });
}

fn cf_text(this: &XlsxCfText, op: &str, text: &str, format: &XlsxFormat) -> Result<(), String> {
    let text = text.to_string();
    let rule = match parse::norm(op).as_str() {
        "contains" => ConditionalFormatTextRule::Contains(text),
        "doesnotcontain" => ConditionalFormatTextRule::DoesNotContain(text),
        "beginswith" => ConditionalFormatTextRule::BeginsWith(text),
        "endswith" => ConditionalFormatTextRule::EndsWith(text),
        other => return Err(format!("unknown text rule {other}")),
    };
    let format = format.0.clone();
    replace(&this.0, |item| item.set_rule(rule).set_format(format));
    Ok(())
}

fn cf_simple_format<T: Clone>(
    slot: &RefCell<T>,
    format: &XlsxFormat,
    apply: fn(T, rust_xlsxwriter::Format) -> T,
) {
    let format = format.0.clone();
    replace(slot, |item| apply(item, format.clone()));
}

fn cf_average(this: &XlsxCfAverage, rule: &str, format: &XlsxFormat) -> Result<(), String> {
    let rule = match parse::norm(rule).as_str() {
        "aboveaverage" => ConditionalFormatAverageRule::AboveAverage,
        "belowaverage" => ConditionalFormatAverageRule::BelowAverage,
        "equaloraboveaverage" => ConditionalFormatAverageRule::EqualOrAboveAverage,
        "equalorbelowaverage" => ConditionalFormatAverageRule::EqualOrBelowAverage,
        "onestandarddeviationabove" => ConditionalFormatAverageRule::OneStandardDeviationAbove,
        "onestandarddeviationbelow" => ConditionalFormatAverageRule::OneStandardDeviationBelow,
        "twostandarddeviationsabove" => ConditionalFormatAverageRule::TwoStandardDeviationsAbove,
        "twostandarddeviationsbelow" => ConditionalFormatAverageRule::TwoStandardDeviationsBelow,
        "threestandarddeviationsabove" => {
            ConditionalFormatAverageRule::ThreeStandardDeviationsAbove
        }
        "threestandarddeviationsbelow" => {
            ConditionalFormatAverageRule::ThreeStandardDeviationsBelow
        }
        other => return Err(format!("unknown average rule {other}")),
    };
    let format = format.0.clone();
    replace(&this.0, |item| item.set_rule(rule).set_format(format));
    Ok(())
}

fn cf_top(this: &XlsxCfTop, rule: &str, count: i64, format: &XlsxFormat) -> Result<(), String> {
    let count = u16::try_from(count).map_err(|_| format!("top count {count}"))?;
    let rule = match parse::norm(rule).as_str() {
        "top" => ConditionalFormatTopRule::Top(count),
        "bottom" => ConditionalFormatTopRule::Bottom(count),
        "toppercent" => ConditionalFormatTopRule::TopPercent(count),
        "bottompercent" => ConditionalFormatTopRule::BottomPercent(count),
        other => return Err(format!("unknown top rule {other}")),
    };
    let format = format.0.clone();
    replace(&this.0, |item| item.set_rule(rule).set_format(format));
    Ok(())
}

fn cf_date(this: &XlsxCfDate, rule: &str, format: &XlsxFormat) -> Result<(), String> {
    let rule = match parse::norm(rule).as_str() {
        "yesterday" => ConditionalFormatDateRule::Yesterday,
        "today" => ConditionalFormatDateRule::Today,
        "tomorrow" => ConditionalFormatDateRule::Tomorrow,
        "last7days" => ConditionalFormatDateRule::Last7Days,
        "lastweek" => ConditionalFormatDateRule::LastWeek,
        "thisweek" => ConditionalFormatDateRule::ThisWeek,
        "nextweek" => ConditionalFormatDateRule::NextWeek,
        "lastmonth" => ConditionalFormatDateRule::LastMonth,
        "thismonth" => ConditionalFormatDateRule::ThisMonth,
        "nextmonth" => ConditionalFormatDateRule::NextMonth,
        other => return Err(format!("unknown date rule {other}")),
    };
    let format = format.0.clone();
    replace(&this.0, |item| item.set_rule(rule).set_format(format));
    Ok(())
}

fn spark_range(this: &XlsxSparkline, range: &str) {
    replace(&this.0, |spark| spark.set_range(range));
}

fn spark_type(this: &XlsxSparkline, kind: &str) -> Result<(), String> {
    let kind = parse::sparkline_type(kind)?;
    replace(&this.0, |spark| spark.set_type(kind));
    Ok(())
}

fn spark_color(this: &XlsxSparkline, which: &str, color: &str) -> Result<(), String> {
    let color = parse::color(color)?;
    replace(&this.0, |spark| match which {
        "sparkline" => spark.set_sparkline_color(color),
        "high" => spark.set_high_point_color(color),
        "low" => spark.set_low_point_color(color),
        "first" => spark.set_first_point_color(color),
        "last" => spark.set_last_point_color(color),
        "negative" => spark.set_negative_points_color(color),
        "markers" => spark.set_markers_color(color),
        _ => spark,
    });
    if !matches!(
        which,
        "sparkline" | "high" | "low" | "first" | "last" | "negative" | "markers"
    ) {
        return Err(format!("unknown sparkline color {which}"));
    }
    Ok(())
}

fn shape_text(this: &XlsxShape, text: &str) {
    replace(&this.0, |shape| shape.set_text(text));
}

fn shape_size(this: &XlsxShape, width: i64, height: i64) -> Result<(), String> {
    let width = i64_u32(width, "width")?;
    let height = i64_u32(height, "height")?;
    replace(&this.0, |shape| shape.set_width(width).set_height(height));
    Ok(())
}

fn shape_movement(this: &XlsxShape, movement: &str) -> Result<(), String> {
    let movement = parse::object_movement(movement)?;
    replace(&this.0, |shape| shape.set_object_movement(movement));
    Ok(())
}

fn filter_list(this: &XlsxFilter, value: &str) {
    replace(&this.0, |filter| filter.add_list_filter(value));
}

fn filter_custom(this: &XlsxFilter, criteria: &str, value: &str) -> Result<(), String> {
    let criteria = parse::filter_criteria(criteria)?;
    replace(&this.0, |filter| filter.add_custom_filter(criteria, value));
    Ok(())
}

fn protect_flag(this: &XlsxProtection, field: &str, enable: bool) -> Result<(), String> {
    let mut options = this.0.borrow().clone();
    match field {
        "select_locked_cells" => options.select_locked_cells = enable,
        "select_unlocked_cells" => options.select_unlocked_cells = enable,
        "format_cells" => options.format_cells = enable,
        "format_columns" => options.format_columns = enable,
        "format_rows" => options.format_rows = enable,
        "insert_columns" => options.insert_columns = enable,
        "insert_rows" => options.insert_rows = enable,
        "insert_links" => options.insert_links = enable,
        "delete_columns" => options.delete_columns = enable,
        "delete_rows" => options.delete_rows = enable,
        "sort" => options.sort = enable,
        "use_autofilter" => options.use_autofilter = enable,
        "use_pivot_tables" => options.use_pivot_tables = enable,
        "edit_scenarios" => options.edit_scenarios = enable,
        "edit_objects" => options.edit_objects = enable,
        "contents" => options.contents = enable,
        other => return Err(format!("unknown protection option {other}")),
    }
    *this.0.borrow_mut() = options;
    Ok(())
}

fn color_scale2(this: &XlsxCf2Color, minimum: &str, maximum: &str) -> Result<(), String> {
    let minimum = parse::color(minimum)?;
    let maximum = parse::color(maximum)?;
    replace(&this.0, |scale| {
        scale.set_minimum_color(minimum).set_maximum_color(maximum)
    });
    Ok(())
}

fn color_scale3(
    this: &XlsxCf3Color,
    minimum: &str,
    mid: &str,
    maximum: &str,
) -> Result<(), String> {
    let minimum = parse::color(minimum)?;
    let mid = parse::color(mid)?;
    let maximum = parse::color(maximum)?;
    replace(&this.0, |scale| {
        scale
            .set_minimum_color(minimum)
            .set_midpoint_color(mid)
            .set_maximum_color(maximum)
    });
    Ok(())
}

fn data_bar_color(this: &XlsxCfDataBar, fill: &str, border: &str) -> Result<(), String> {
    let fill = parse::color(fill)?;
    let border = parse::color(border)?;
    replace(&this.0, |bar| {
        bar.set_fill_color(fill).set_border_color(border)
    });
    Ok(())
}

fn data_bar_flags(this: &XlsxCfDataBar, solid: bool, bar_only: bool) {
    replace(&this.0, |bar| {
        bar.set_solid_fill(solid).set_bar_only(bar_only)
    });
}

pub(crate) fn register(context: &mut rune::Context, module: &mut Module) -> Result<(), String> {
    module.ty::<XlsxFormula>().map_err(|e| e.to_string())?;
    module.ty::<XlsxUrl>().map_err(|e| e.to_string())?;
    module.ty::<XlsxDateTime>().map_err(|e| e.to_string())?;
    module.ty::<XlsxImage>().map_err(|e| e.to_string())?;
    module.ty::<XlsxNote>().map_err(|e| e.to_string())?;
    module.ty::<XlsxProperties>().map_err(|e| e.to_string())?;
    module.ty::<XlsxTable>().map_err(|e| e.to_string())?;
    module.ty::<XlsxTableColumn>().map_err(|e| e.to_string())?;
    module.ty::<XlsxValidation>().map_err(|e| e.to_string())?;
    module.ty::<XlsxCfCell>().map_err(|e| e.to_string())?;
    module.ty::<XlsxCfFormula>().map_err(|e| e.to_string())?;
    module.ty::<XlsxCfText>().map_err(|e| e.to_string())?;
    module.ty::<XlsxCfBlank>().map_err(|e| e.to_string())?;
    module.ty::<XlsxCfDuplicate>().map_err(|e| e.to_string())?;
    module.ty::<XlsxCfError>().map_err(|e| e.to_string())?;
    module.ty::<XlsxCfAverage>().map_err(|e| e.to_string())?;
    module.ty::<XlsxCfTop>().map_err(|e| e.to_string())?;
    module.ty::<XlsxCfDate>().map_err(|e| e.to_string())?;
    module.ty::<XlsxCf2Color>().map_err(|e| e.to_string())?;
    module.ty::<XlsxCf3Color>().map_err(|e| e.to_string())?;
    module.ty::<XlsxCfDataBar>().map_err(|e| e.to_string())?;
    module.ty::<XlsxSparkline>().map_err(|e| e.to_string())?;
    module.ty::<XlsxShape>().map_err(|e| e.to_string())?;
    module.ty::<XlsxFilter>().map_err(|e| e.to_string())?;
    module.ty::<XlsxProtection>().map_err(|e| e.to_string())?;

    ctor!(context, "Formula", "new", |formula: &str| XlsxFormula(
        Formula::new(formula)
    ));
    ctor!(context, "Url", "new", |link: &str| XlsxUrl(Url::new(link)));
    assoc!(module, "set_text", url_text);
    assoc!(module, "set_tip", url_tip);
    {
        let mut ctors = Module::with_item(["xlsx", "ExcelDateTime"]).map_err(|e| e.to_string())?;
        ctors
            .function("parse", |text: &str| {
                ExcelDateTime::parse_from_str(text)
                    .map(XlsxDateTime)
                    .map_err(|e| e.to_string())
            })
            .build()
            .map_err(|e| e.to_string())?;
        ctors
            .function("from_ymd", |year: i64, month: i64, day: i64| {
                let year = u16::try_from(year).map_err(|_| format!("year {year}"))?;
                let month = i64_u8(month, "month")?;
                let day = i64_u8(day, "day")?;
                ExcelDateTime::from_ymd(year, month, day)
                    .map(XlsxDateTime)
                    .map_err(|e| e.to_string())
            })
            .build()
            .map_err(|e| e.to_string())?;
        ctors
            .function("from_hms", |hour: i64, min: i64, sec: Value| {
                let hour = u16::try_from(hour).map_err(|_| format!("hour {hour}"))?;
                let min = i64_u8(min, "minute")?;
                ExcelDateTime::from_hms(hour, min, number(sec)?)
                    .map(XlsxDateTime)
                    .map_err(|e| e.to_string())
            })
            .build()
            .map_err(|e| e.to_string())?;
        context.install(ctors).map_err(|e| e.to_string())?;
    }
    ctor!(context, "Image", "from_buffer", image_buffer);
    assoc!(module, "set_width", image_width);
    assoc!(module, "set_height", image_height);
    assoc!(module, "set_scale_to_size", image_scale);
    assoc!(module, "set_alt_text", image_alt);
    assoc!(module, "set_decorative", image_decorative);
    assoc!(module, "set_url", image_url);
    assoc!(module, "set_object_movement", image_movement);
    ctor!(context, "Note", "new", |text: &str| XlsxNote(cell(
        Note::new(text)
    )));
    assoc!(module, "set_author", note_author);
    assoc!(module, "set_size", note_size);
    assoc!(module, "set_visible", note_visible);
    assoc!(module, "set_background_color", note_color);
    assoc!(module, "set_font", note_font);
    assoc!(module, "set_format", note_format);
    ctor!(context, "DocProperties", "new", || XlsxProperties(cell(
        DocProperties::new()
    )));
    assoc!(module, "set", props_string);
    assoc!(module, "set_creation_datetime", props_created);
    ctor!(context, "Table", "new", || XlsxTable(cell(Table::new())));
    assoc!(module, "set_flag", table_flag);
    assoc!(module, "set_name", table_name);
    assoc!(module, "set_style", table_style);
    assoc!(module, "set_alt_text", table_alt);
    assoc!(module, "set_columns", table_columns);
    ctor!(context, "TableColumn", "new", || {
        XlsxTableColumn(cell(TableColumn::new()))
    });
    assoc!(module, "set_header", column_header);
    assoc!(module, "set_total_function", column_total);
    assoc!(module, "set_total_label", column_label);
    assoc!(module, "set_formula", column_formula);
    assoc!(module, "set_format", column_format);
    assoc!(module, "set_header_format", column_header_format);
    ctor!(context, "DataValidation", "new", || {
        XlsxValidation(cell(DataValidation::new()))
    });
    assoc!(module, "allow_list_strings", validation_list);
    assoc!(module, "allow_list_formula", validation_list_formula);
    assoc!(module, "allow_whole_number", validation_whole);
    assoc!(module, "allow_decimal_number", validation_decimal);
    assoc!(module, "allow_text_length", validation_text_length);
    assoc!(module, "allow_custom", validation_custom);
    assoc!(module, "allow_any_value", validation_any);
    assoc!(module, "set_flag", validation_flag);
    assoc!(module, "set_input", validation_input);
    assoc!(module, "set_error", validation_error);
    ctor!(context, "ConditionalFormatCell", "new", || {
        XlsxCfCell(cell(ConditionalFormatCell::new()))
    });
    assoc!(module, "set_number_rule", cf_cell_number);
    assoc!(module, "set_text_rule", cf_cell_text);
    assoc!(module, "set_format", cf_cell_format);
    ctor!(context, "ConditionalFormatFormula", "new", || {
        XlsxCfFormula(cell(ConditionalFormatFormula::new()))
    });
    assoc!(module, "set_rule", cf_formula);
    ctor!(context, "ConditionalFormatText", "new", || {
        XlsxCfText(cell(ConditionalFormatText::new()))
    });
    assoc!(module, "set_rule", cf_text);
    ctor!(context, "ConditionalFormatBlank", "new", || {
        XlsxCfBlank(cell(ConditionalFormatBlank::new()))
    });
    ctor!(context, "ConditionalFormatDuplicate", "new", || {
        XlsxCfDuplicate(cell(ConditionalFormatDuplicate::new()))
    });
    ctor!(context, "ConditionalFormatError", "new", || {
        XlsxCfError(cell(ConditionalFormatError::new()))
    });
    module
        .associated_function("set_format", |this: &XlsxCfBlank, format: &XlsxFormat| {
            cf_simple_format(&this.0, format, |item, format| item.set_format(format));
        })
        .map_err(|e| e.to_string())?;
    ctor!(context, "ConditionalFormatAverage", "new", || {
        XlsxCfAverage(cell(ConditionalFormatAverage::new()))
    });
    assoc!(module, "set_rule", cf_average);
    ctor!(context, "ConditionalFormatTop", "new", || {
        XlsxCfTop(cell(ConditionalFormatTop::new()))
    });
    assoc!(module, "set_rule", cf_top);
    ctor!(context, "ConditionalFormatDate", "new", || {
        XlsxCfDate(cell(ConditionalFormatDate::new()))
    });
    assoc!(module, "set_rule", cf_date);
    ctor!(context, "ConditionalFormat2ColorScale", "new", || {
        XlsxCf2Color(cell(ConditionalFormat2ColorScale::new()))
    });
    assoc!(module, "set_colors", color_scale2);
    ctor!(context, "ConditionalFormat3ColorScale", "new", || {
        XlsxCf3Color(cell(ConditionalFormat3ColorScale::new()))
    });
    assoc!(module, "set_colors", color_scale3);
    ctor!(context, "ConditionalFormatDataBar", "new", || {
        XlsxCfDataBar(cell(ConditionalFormatDataBar::new()))
    });
    assoc!(module, "set_colors", data_bar_color);
    assoc!(module, "set_flags", data_bar_flags);
    ctor!(context, "Sparkline", "new", || XlsxSparkline(cell(
        Sparkline::new()
    )));
    assoc!(module, "set_range", spark_range);
    assoc!(module, "set_type", spark_type);
    assoc!(module, "set_color", spark_color);
    ctor!(context, "Shape", "textbox", || XlsxShape(cell(
        Shape::textbox()
    )));
    assoc!(module, "set_text", shape_text);
    assoc!(module, "set_size", shape_size);
    assoc!(module, "set_object_movement", shape_movement);
    ctor!(context, "FilterCondition", "new", || {
        XlsxFilter(cell(FilterCondition::new()))
    });
    assoc!(module, "add_list_filter", filter_list);
    assoc!(module, "add_custom_filter", filter_custom);
    ctor!(context, "ProtectionOptions", "new", || {
        XlsxProtection(cell(ProtectionOptions::new()))
    });
    assoc!(module, "set", protect_flag);
    module
        .associated_function(
            "set_format",
            |this: &XlsxCfDuplicate, format: &XlsxFormat| {
                cf_simple_format(&this.0, format, |item, format| item.set_format(format));
            },
        )
        .map_err(|e| e.to_string())?;
    module
        .associated_function("set_format", |this: &XlsxCfError, format: &XlsxFormat| {
            cf_simple_format(&this.0, format, |item, format| item.set_format(format));
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}
