//! Charts and the objects used to style them.

use std::cell::RefCell;

use rune::Any;
use rune::Value;
use rune::module::Module;
use rust_xlsxwriter::{
    Chart, ChartDataLabel, ChartFont, ChartFormat, ChartLine, ChartMarker, ChartSeries,
    ChartSolidFill, ChartTrendline,
};

use super::parse;
use super::{i64_u8, i64_u16, i64_u32, number};

fn cell<T>(value: T) -> RefCell<T> {
    RefCell::new(value)
}

fn replace<T>(slot: &RefCell<T>, update: impl FnOnce(&mut T)) {
    update(&mut slot.borrow_mut());
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

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = Chart)]
pub(crate) struct XlsxChart(pub RefCell<Chart>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ChartSeries)]
pub(crate) struct XlsxChartSeries(RefCell<ChartSeries>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ChartFormat)]
pub(crate) struct XlsxChartFormat(RefCell<ChartFormat>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ChartLine)]
pub(crate) struct XlsxChartLine(RefCell<ChartLine>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ChartSolidFill)]
pub(crate) struct XlsxChartFill(RefCell<ChartSolidFill>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ChartFont)]
pub(crate) struct XlsxChartFont(RefCell<ChartFont>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ChartMarker)]
pub(crate) struct XlsxChartMarker(RefCell<ChartMarker>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ChartDataLabel)]
pub(crate) struct XlsxChartLabel(RefCell<ChartDataLabel>);

#[derive(Any)]
#[rune(item = ::xlsx)]
#[rune(name = ChartTrendline)]
pub(crate) struct XlsxChartTrendline(RefCell<ChartTrendline>);

fn chart_new(kind: &str) -> Result<XlsxChart, String> {
    Ok(XlsxChart(cell(Chart::new(parse::chart_type(kind)?))))
}

fn push_series(this: &XlsxChart, series: &XlsxChartSeries) {
    let series = series.0.borrow().clone();
    this.0.borrow_mut().push_series(&series);
}

fn combine(this: &XlsxChart, other: &XlsxChart) {
    let other = other.0.borrow().clone();
    this.0.borrow_mut().combine(&other);
}

fn set_title(this: &XlsxChart, name: &str) {
    this.0.borrow_mut().title().set_name(name);
}

fn hide_title(this: &XlsxChart) {
    this.0.borrow_mut().title().set_hidden();
}

fn axis_name(this: &XlsxChart, axis: &str, name: &str) -> Result<(), String> {
    let mut chart = this.0.borrow_mut();
    match axis {
        "x" => chart.x_axis().set_name(name),
        "y" => chart.y_axis().set_name(name),
        "x2" => chart.x2_axis().set_name(name),
        "y2" => chart.y2_axis().set_name(name),
        other => return Err(format!("unknown axis {other}")),
    };
    Ok(())
}

fn axis_num_format(this: &XlsxChart, axis: &str, format: &str) -> Result<(), String> {
    let mut chart = this.0.borrow_mut();
    match axis {
        "x" => chart.x_axis().set_num_format(format),
        "y" => chart.y_axis().set_num_format(format),
        "x2" => chart.x2_axis().set_num_format(format),
        "y2" => chart.y2_axis().set_num_format(format),
        other => return Err(format!("unknown axis {other}")),
    };
    Ok(())
}

fn axis_limit(this: &XlsxChart, axis: &str, which: &str, value: Value) -> Result<(), String> {
    let value = number(value)?;
    let mut chart = this.0.borrow_mut();
    let axis = match axis {
        "x" => chart.x_axis(),
        "y" => chart.y_axis(),
        "x2" => chart.x2_axis(),
        "y2" => chart.y2_axis(),
        other => return Err(format!("unknown axis {other}")),
    };
    match which {
        "min" => {
            axis.set_min(value);
        }
        "max" => {
            axis.set_max(value);
        }
        other => return Err(format!("axis limit must be min or max, not {other}")),
    }
    Ok(())
}

fn hide_legend(this: &XlsxChart) {
    this.0.borrow_mut().legend().set_hidden();
}

fn legend_position(this: &XlsxChart, position: &str) -> Result<(), String> {
    let position = parse::legend_position(position)?;
    this.0.borrow_mut().legend().set_position(position);
    Ok(())
}

fn empty_cells(this: &XlsxChart, option: &str) -> Result<(), String> {
    let option = parse::empty_cells(option)?;
    this.0.borrow_mut().show_empty_cells_as(option);
    Ok(())
}

fn chart_size(this: &XlsxChart, width: i64, height: i64) -> Result<(), String> {
    let width = i64_u32(width, "width")?;
    let height = i64_u32(height, "height")?;
    let mut chart = this.0.borrow_mut();
    chart.set_width(width);
    chart.set_height(height);
    Ok(())
}

fn chart_style(this: &XlsxChart, style: i64) -> Result<(), String> {
    let style = i64_u8(style, "chart style")?;
    this.0.borrow_mut().set_style(style);
    Ok(())
}

fn chart_name(this: &XlsxChart, name: &str) {
    this.0.borrow_mut().set_name(name);
}

fn series_values(this: &XlsxChartSeries, range: &str) {
    this.0.borrow_mut().set_values(range);
}

fn series_categories(this: &XlsxChartSeries, range: &str) {
    this.0.borrow_mut().set_categories(range);
}

fn series_name(this: &XlsxChartSeries, name: &str) {
    this.0.borrow_mut().set_name(name);
}

fn series_format(this: &XlsxChartSeries, format: &XlsxChartFormat) {
    let mut format = format.0.borrow_mut();
    this.0.borrow_mut().set_format(&mut *format);
}

fn series_marker(this: &XlsxChartSeries, marker: &XlsxChartMarker) {
    let marker = marker.0.borrow().clone();
    this.0.borrow_mut().set_marker(&marker);
}

fn series_label(this: &XlsxChartSeries, label: &XlsxChartLabel) {
    let label = label.0.borrow().clone();
    this.0.borrow_mut().set_data_label(&label);
}

fn series_trendline(this: &XlsxChartSeries, trendline: &XlsxChartTrendline) {
    let trendline = trendline.0.borrow().clone();
    this.0.borrow_mut().set_trendline(&trendline);
}

fn series_overlap(this: &XlsxChartSeries, overlap: i64) -> Result<(), String> {
    let overlap = i8::try_from(overlap).map_err(|_| format!("overlap {overlap}"))?;
    this.0.borrow_mut().set_overlap(overlap);
    Ok(())
}

fn series_gap(this: &XlsxChartSeries, gap: i64) -> Result<(), String> {
    let gap = i64_u16(gap, "gap")?;
    this.0.borrow_mut().set_gap(gap);
    Ok(())
}

fn series_smooth(this: &XlsxChartSeries, enable: bool) {
    this.0.borrow_mut().set_smooth(enable);
}

fn series_secondary(this: &XlsxChartSeries, enable: bool) {
    this.0.borrow_mut().set_secondary_axis(enable);
}

fn line_color(this: &XlsxChartLine, color: &str) -> Result<(), String> {
    let color = parse::color(color)?;
    replace(&this.0, |line| {
        line.set_color(color);
    });
    Ok(())
}

fn line_width(this: &XlsxChartLine, width: Value) -> Result<(), String> {
    let width = number(width)?;
    replace(&this.0, |line| {
        line.set_width(width);
    });
    Ok(())
}

fn line_dash(this: &XlsxChartLine, dash: &str) -> Result<(), String> {
    let dash = parse::dash_type(dash)?;
    replace(&this.0, |line| {
        line.set_dash_type(dash);
    });
    Ok(())
}

fn fill_color(this: &XlsxChartFill, color: &str) -> Result<(), String> {
    let color = parse::color(color)?;
    replace(&this.0, |fill| {
        fill.set_color(color);
    });
    Ok(())
}

fn format_line(this: &XlsxChartFormat, line: &XlsxChartLine) {
    let line = line.0.borrow().clone();
    replace(&this.0, |format| {
        format.set_line(&line);
    });
}

fn format_fill(this: &XlsxChartFormat, fill: &XlsxChartFill) {
    let fill = fill.0.borrow().clone();
    replace(&this.0, |format| {
        format.set_solid_fill(&fill);
    });
}

fn format_no_line(this: &XlsxChartFormat) {
    replace(&this.0, |format| {
        format.set_no_line();
    });
}

fn format_no_fill(this: &XlsxChartFormat) {
    replace(&this.0, |format| {
        format.set_no_fill();
    });
}

fn font_name(this: &XlsxChartFont, name: &str) {
    replace(&this.0, |font| {
        font.set_name(name);
    });
}

fn font_size(this: &XlsxChartFont, size: Value) -> Result<(), String> {
    let size = number(size)?;
    replace(&this.0, |font| {
        font.set_size(size);
    });
    Ok(())
}

fn font_color(this: &XlsxChartFont, color: &str) -> Result<(), String> {
    let color = parse::color(color)?;
    replace(&this.0, |font| {
        font.set_color(color);
    });
    Ok(())
}

fn font_bold(this: &XlsxChartFont) {
    replace(&this.0, |font| {
        font.set_bold();
    });
}

fn font_italic(this: &XlsxChartFont) {
    replace(&this.0, |font| {
        font.set_italic();
    });
}

fn marker_type(this: &XlsxChartMarker, kind: &str) -> Result<(), String> {
    let kind = parse::marker_type(kind)?;
    replace(&this.0, |marker| {
        marker.set_type(kind);
    });
    Ok(())
}

fn marker_size(this: &XlsxChartMarker, size: i64) -> Result<(), String> {
    let size = i64_u8(size, "marker size")?;
    replace(&this.0, |marker| {
        marker.set_size(size);
    });
    Ok(())
}

fn marker_none(this: &XlsxChartMarker) {
    replace(&this.0, |marker| {
        marker.set_none();
    });
}

fn label_show(this: &XlsxChartLabel, which: &str) -> Result<(), String> {
    if !matches!(
        which,
        "value" | "category" | "series" | "percent" | "leader"
    ) {
        return Err(format!("unknown data label {which}"));
    }
    replace(&this.0, |label| match which {
        "value" => {
            label.show_value();
        }
        "category" => {
            label.show_category_name();
        }
        "series" => {
            label.show_series_name();
        }
        "percent" => {
            label.show_percentage();
        }
        "leader" => {
            label.show_leader_lines();
        }
        _ => {}
    });
    Ok(())
}

fn label_format(this: &XlsxChartLabel, format: &str) {
    replace(&this.0, |label| {
        label.set_num_format(format);
    });
}

fn trend_type(this: &XlsxChartTrendline, kind: &str, period: Option<i64>) -> Result<(), String> {
    let kind = parse::trendline(kind, period)?;
    replace(&this.0, |trend| {
        trend.set_type(kind);
    });
    Ok(())
}

fn trend_name(this: &XlsxChartTrendline, name: &str) {
    replace(&this.0, |trend| {
        trend.set_name(name);
    });
}

fn trend_equation(this: &XlsxChartTrendline, equation: bool, r_squared: bool) {
    replace(&this.0, |trend| {
        trend.display_equation(equation);
        trend.display_r_squared(r_squared);
    });
}

pub(crate) fn register(context: &mut rune::Context, module: &mut Module) -> Result<(), String> {
    module.ty::<XlsxChart>().map_err(|e| e.to_string())?;
    module.ty::<XlsxChartSeries>().map_err(|e| e.to_string())?;
    module.ty::<XlsxChartFormat>().map_err(|e| e.to_string())?;
    module.ty::<XlsxChartLine>().map_err(|e| e.to_string())?;
    module.ty::<XlsxChartFill>().map_err(|e| e.to_string())?;
    module.ty::<XlsxChartFont>().map_err(|e| e.to_string())?;
    module.ty::<XlsxChartMarker>().map_err(|e| e.to_string())?;
    module.ty::<XlsxChartLabel>().map_err(|e| e.to_string())?;
    module
        .ty::<XlsxChartTrendline>()
        .map_err(|e| e.to_string())?;
    {
        let mut ctors = Module::with_item(["xlsx", "Chart"]).map_err(|e| e.to_string())?;
        ctors
            .function("new", chart_new)
            .build()
            .map_err(|e| e.to_string())?;
        ctors
            .function("new_column", || XlsxChart(cell(Chart::new_column())))
            .build()
            .map_err(|e| e.to_string())?;
        ctors
            .function("new_line", || XlsxChart(cell(Chart::new_line())))
            .build()
            .map_err(|e| e.to_string())?;
        ctors
            .function("new_pie", || XlsxChart(cell(Chart::new_pie())))
            .build()
            .map_err(|e| e.to_string())?;
        ctors
            .function("new_bar", || XlsxChart(cell(Chart::new_bar())))
            .build()
            .map_err(|e| e.to_string())?;
        ctors
            .function("new_area", || XlsxChart(cell(Chart::new_area())))
            .build()
            .map_err(|e| e.to_string())?;
        ctors
            .function("new_scatter", || XlsxChart(cell(Chart::new_scatter())))
            .build()
            .map_err(|e| e.to_string())?;
        ctors
            .function("new_doughnut", || XlsxChart(cell(Chart::new_doughnut())))
            .build()
            .map_err(|e| e.to_string())?;
        ctors
            .function("new_radar", || XlsxChart(cell(Chart::new_radar())))
            .build()
            .map_err(|e| e.to_string())?;
        ctors
            .function("new_stock", || XlsxChart(cell(Chart::new_stock())))
            .build()
            .map_err(|e| e.to_string())?;
        context.install(ctors).map_err(|e| e.to_string())?;
    }
    assoc!(module, "push_series", push_series);
    assoc!(module, "combine", combine);
    assoc!(module, "set_title", set_title);
    assoc!(module, "hide_title", hide_title);
    assoc!(module, "set_axis_name", axis_name);
    assoc!(module, "set_axis_num_format", axis_num_format);
    assoc!(module, "set_axis_limit", axis_limit);
    assoc!(module, "hide_legend", hide_legend);
    assoc!(module, "set_legend_position", legend_position);
    assoc!(module, "show_empty_cells_as", empty_cells);
    assoc!(module, "set_size", chart_size);
    assoc!(module, "set_style", chart_style);
    assoc!(module, "set_name", chart_name);
    ctor!(context, "ChartSeries", "new", || {
        XlsxChartSeries(cell(ChartSeries::new()))
    });
    assoc!(module, "set_values", series_values);
    assoc!(module, "set_categories", series_categories);
    assoc!(module, "set_name", series_name);
    assoc!(module, "set_format", series_format);
    assoc!(module, "set_marker", series_marker);
    assoc!(module, "set_data_label", series_label);
    assoc!(module, "set_trendline", series_trendline);
    assoc!(module, "set_overlap", series_overlap);
    assoc!(module, "set_gap", series_gap);
    assoc!(module, "set_smooth", series_smooth);
    assoc!(module, "set_secondary_axis", series_secondary);
    ctor!(context, "ChartFormat", "new", || XlsxChartFormat(cell(
        ChartFormat::new()
    )));
    assoc!(module, "set_line", format_line);
    assoc!(module, "set_solid_fill", format_fill);
    assoc!(module, "set_no_line", format_no_line);
    assoc!(module, "set_no_fill", format_no_fill);
    ctor!(context, "ChartLine", "new", || XlsxChartLine(cell(
        ChartLine::new()
    )));
    assoc!(module, "set_color", line_color);
    assoc!(module, "set_width", line_width);
    assoc!(module, "set_dash_type", line_dash);
    ctor!(context, "ChartSolidFill", "new", || {
        XlsxChartFill(cell(ChartSolidFill::new()))
    });
    assoc!(module, "set_color", fill_color);
    ctor!(context, "ChartFont", "new", || XlsxChartFont(cell(
        ChartFont::new()
    )));
    assoc!(module, "set_name", font_name);
    assoc!(module, "set_size", font_size);
    assoc!(module, "set_color", font_color);
    assoc!(module, "set_bold", font_bold);
    assoc!(module, "set_italic", font_italic);
    ctor!(context, "ChartMarker", "new", || XlsxChartMarker(cell(
        ChartMarker::new()
    )));
    assoc!(module, "set_type", marker_type);
    assoc!(module, "set_size", marker_size);
    assoc!(module, "set_none", marker_none);
    ctor!(context, "ChartDataLabel", "new", || {
        XlsxChartLabel(cell(ChartDataLabel::new()))
    });
    assoc!(module, "show", label_show);
    assoc!(module, "set_num_format", label_format);
    ctor!(context, "ChartTrendline", "new", || {
        XlsxChartTrendline(cell(ChartTrendline::new()))
    });
    assoc!(module, "set_type", trend_type);
    assoc!(module, "set_name", trend_name);
    assoc!(module, "display", trend_equation);
    Ok(())
}
