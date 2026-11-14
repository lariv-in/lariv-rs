//! String arguments for `rust_xlsxwriter` enums and colors.

use rust_xlsxwriter::{
    ChartEmptyCells, ChartLegendPosition, ChartLineDashType, ChartMarkerType, ChartTrendlineType,
    ChartType, Color, DataValidationErrorStyle, DataValidationRule, FilterCriteria, FontScheme,
    FormatAlign, FormatBorder, FormatDiagonalBorder, FormatPattern, FormatScript, FormatUnderline,
    HeaderImagePosition, ObjectMovement, SparklineType, TableFunction, TableStyle,
};

pub(crate) fn norm(raw: &str) -> String {
    raw.chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}

pub(crate) fn color(raw: &str) -> Result<Color, String> {
    let key = norm(raw);
    if let Some(rest) = key.strip_prefix("theme") {
        let mut parts = rest.split(|c: char| !c.is_ascii_digit());
        let color = parts
            .next()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "theme color needs two indexes, as theme0:1".to_string())?;
        let shade = parts
            .next()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "theme color needs two indexes, as theme0:1".to_string())?;
        let color = color
            .parse::<u8>()
            .map_err(|_| format!("theme color index {color}"))?;
        let shade = shade
            .parse::<u8>()
            .map_err(|_| format!("theme shade index {shade}"))?;
        return Ok(Color::Theme(color, shade));
    }
    if let Some(hex) = raw.trim().strip_prefix('#') {
        let rgb = u32::from_str_radix(hex, 16).map_err(|_| format!("color {raw}"))?;
        return Ok(Color::RGB(rgb));
    }
    if raw.trim().chars().all(|c| c.is_ascii_hexdigit()) && raw.trim().len() == 6 {
        let rgb = u32::from_str_radix(raw.trim(), 16).map_err(|_| format!("color {raw}"))?;
        return Ok(Color::RGB(rgb));
    }
    Ok(match key.as_str() {
        "default" => Color::Default,
        "automatic" => Color::Automatic,
        "black" => Color::Black,
        "blue" => Color::Blue,
        "brown" => Color::Brown,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "green" => Color::Green,
        "lime" => Color::Lime,
        "magenta" => Color::Magenta,
        "navy" => Color::Navy,
        "orange" => Color::Orange,
        "pink" => Color::Pink,
        "purple" => Color::Purple,
        "red" => Color::Red,
        "silver" => Color::Silver,
        "white" => Color::White,
        "yellow" => Color::Yellow,
        other => return Err(format!("unknown color {other}")),
    })
}

pub(crate) fn border(raw: &str) -> Result<FormatBorder, String> {
    Ok(match norm(raw).as_str() {
        "none" => FormatBorder::None,
        "thin" => FormatBorder::Thin,
        "medium" => FormatBorder::Medium,
        "dashed" => FormatBorder::Dashed,
        "dotted" => FormatBorder::Dotted,
        "thick" => FormatBorder::Thick,
        "double" => FormatBorder::Double,
        "hair" => FormatBorder::Hair,
        "mediumdashed" => FormatBorder::MediumDashed,
        "dashdot" => FormatBorder::DashDot,
        "mediumdashdot" => FormatBorder::MediumDashDot,
        "dashdotdot" => FormatBorder::DashDotDot,
        "mediumdashdotdot" => FormatBorder::MediumDashDotDot,
        "slantdashdot" => FormatBorder::SlantDashDot,
        other => return Err(format!("unknown border {other}")),
    })
}

pub(crate) fn diagonal(raw: &str) -> Result<FormatDiagonalBorder, String> {
    Ok(match norm(raw).as_str() {
        "none" => FormatDiagonalBorder::None,
        "borderup" | "up" => FormatDiagonalBorder::BorderUp,
        "borderdown" | "down" => FormatDiagonalBorder::BorderDown,
        "borderupdown" | "both" => FormatDiagonalBorder::BorderUpDown,
        other => return Err(format!("unknown diagonal border {other}")),
    })
}

pub(crate) fn pattern(raw: &str) -> Result<FormatPattern, String> {
    Ok(match norm(raw).as_str() {
        "none" => FormatPattern::None,
        "solid" => FormatPattern::Solid,
        "mediumgray" => FormatPattern::MediumGray,
        "darkgray" => FormatPattern::DarkGray,
        "lightgray" => FormatPattern::LightGray,
        "darkhorizontal" => FormatPattern::DarkHorizontal,
        "darkvertical" => FormatPattern::DarkVertical,
        "darkdown" => FormatPattern::DarkDown,
        "darkup" => FormatPattern::DarkUp,
        "darkgrid" => FormatPattern::DarkGrid,
        "darktrellis" => FormatPattern::DarkTrellis,
        "lighthorizontal" => FormatPattern::LightHorizontal,
        "lightvertical" => FormatPattern::LightVertical,
        "lightdown" => FormatPattern::LightDown,
        "lightup" => FormatPattern::LightUp,
        "lightgrid" => FormatPattern::LightGrid,
        "lighttrellis" => FormatPattern::LightTrellis,
        "gray125" => FormatPattern::Gray125,
        "gray0625" => FormatPattern::Gray0625,
        other => return Err(format!("unknown pattern {other}")),
    })
}

pub(crate) fn underline(raw: &str) -> Result<FormatUnderline, String> {
    Ok(match norm(raw).as_str() {
        "none" => FormatUnderline::None,
        "single" => FormatUnderline::Single,
        "double" => FormatUnderline::Double,
        "singleaccounting" => FormatUnderline::SingleAccounting,
        "doubleaccounting" => FormatUnderline::DoubleAccounting,
        other => return Err(format!("unknown underline {other}")),
    })
}

pub(crate) fn script(raw: &str) -> Result<FormatScript, String> {
    Ok(match norm(raw).as_str() {
        "none" => FormatScript::None,
        "superscript" => FormatScript::Superscript,
        "subscript" => FormatScript::Subscript,
        other => return Err(format!("unknown font script {other}")),
    })
}

pub(crate) fn align(raw: &str) -> Result<FormatAlign, String> {
    Ok(match norm(raw).as_str() {
        "general" => FormatAlign::General,
        "left" => FormatAlign::Left,
        "center" => FormatAlign::Center,
        "right" => FormatAlign::Right,
        "fill" => FormatAlign::Fill,
        "justify" => FormatAlign::Justify,
        "centeracross" => FormatAlign::CenterAcross,
        "distributed" => FormatAlign::Distributed,
        "top" => FormatAlign::Top,
        "bottom" => FormatAlign::Bottom,
        "verticalcenter" => FormatAlign::VerticalCenter,
        "verticaljustify" => FormatAlign::VerticalJustify,
        "verticaldistributed" => FormatAlign::VerticalDistributed,
        other => return Err(format!("unknown align {other}")),
    })
}

pub(crate) fn font_scheme(raw: &str) -> Result<FontScheme, String> {
    Ok(match norm(raw).as_str() {
        "body" => FontScheme::Body,
        "headings" => FontScheme::Headings,
        "none" => FontScheme::None,
        other => return Err(format!("unknown font scheme {other}")),
    })
}

pub(crate) fn chart_type(raw: &str) -> Result<ChartType, String> {
    Ok(match norm(raw).as_str() {
        "area" => ChartType::Area,
        "areastacked" => ChartType::AreaStacked,
        "areapercentstacked" => ChartType::AreaPercentStacked,
        "bar" => ChartType::Bar,
        "barstacked" => ChartType::BarStacked,
        "barpercentstacked" => ChartType::BarPercentStacked,
        "column" => ChartType::Column,
        "columnstacked" => ChartType::ColumnStacked,
        "columnpercentstacked" => ChartType::ColumnPercentStacked,
        "doughnut" => ChartType::Doughnut,
        "line" => ChartType::Line,
        "linestacked" => ChartType::LineStacked,
        "linepercentstacked" => ChartType::LinePercentStacked,
        "pie" => ChartType::Pie,
        "radar" => ChartType::Radar,
        "radarwithmarkers" => ChartType::RadarWithMarkers,
        "radarfilled" => ChartType::RadarFilled,
        "scatter" => ChartType::Scatter,
        "scatterstraight" => ChartType::ScatterStraight,
        "scatterstraightwithmarkers" => ChartType::ScatterStraightWithMarkers,
        "scattersmooth" => ChartType::ScatterSmooth,
        "scattersmoothwithmarkers" => ChartType::ScatterSmoothWithMarkers,
        "stock" => ChartType::Stock,
        other => return Err(format!("unknown chart type {other}")),
    })
}

pub(crate) fn legend_position(raw: &str) -> Result<ChartLegendPosition, String> {
    Ok(match norm(raw).as_str() {
        "right" => ChartLegendPosition::Right,
        "left" => ChartLegendPosition::Left,
        "top" => ChartLegendPosition::Top,
        "bottom" => ChartLegendPosition::Bottom,
        "topright" => ChartLegendPosition::TopRight,
        other => return Err(format!("unknown legend position {other}")),
    })
}

pub(crate) fn empty_cells(raw: &str) -> Result<ChartEmptyCells, String> {
    Ok(match norm(raw).as_str() {
        "gaps" | "gap" => ChartEmptyCells::Gaps,
        "zero" => ChartEmptyCells::Zero,
        "connected" | "span" => ChartEmptyCells::Connected,
        other => return Err(format!("unknown empty-cell option {other}")),
    })
}

pub(crate) fn marker_type(raw: &str) -> Result<ChartMarkerType, String> {
    Ok(match norm(raw).as_str() {
        "square" => ChartMarkerType::Square,
        "diamond" => ChartMarkerType::Diamond,
        "triangle" => ChartMarkerType::Triangle,
        "x" => ChartMarkerType::X,
        "star" => ChartMarkerType::Star,
        "shortdash" | "dot" => ChartMarkerType::ShortDash,
        "longdash" | "dash" => ChartMarkerType::LongDash,
        "circle" => ChartMarkerType::Circle,
        "plussign" | "plus" => ChartMarkerType::PlusSign,
        other => return Err(format!("unknown marker type {other}")),
    })
}

pub(crate) fn dash_type(raw: &str) -> Result<ChartLineDashType, String> {
    Ok(match norm(raw).as_str() {
        "solid" => ChartLineDashType::Solid,
        "rounddot" => ChartLineDashType::RoundDot,
        "squaredot" => ChartLineDashType::SquareDot,
        "dash" => ChartLineDashType::Dash,
        "dashdot" => ChartLineDashType::DashDot,
        "longdash" => ChartLineDashType::LongDash,
        "longdashdot" => ChartLineDashType::LongDashDot,
        "longdashdotdot" => ChartLineDashType::LongDashDotDot,
        other => return Err(format!("unknown dash type {other}")),
    })
}

pub(crate) fn trendline(raw: &str, period: Option<i64>) -> Result<ChartTrendlineType, String> {
    Ok(match norm(raw).as_str() {
        "none" => ChartTrendlineType::None,
        "exponential" | "exp" => ChartTrendlineType::Exponential,
        "linear" => ChartTrendlineType::Linear,
        "logarithmic" | "log" => ChartTrendlineType::Logarithmic,
        "power" => ChartTrendlineType::Power,
        "polynomial" | "poly" => {
            let order = u8::try_from(period.unwrap_or(2)).map_err(|_| "polynomial order")?;
            ChartTrendlineType::Polynomial(order)
        }
        "movingaverage" | "movingavg" => {
            let period = u8::try_from(period.unwrap_or(2)).map_err(|_| "moving average period")?;
            ChartTrendlineType::MovingAverage(period)
        }
        other => return Err(format!("unknown trendline {other}")),
    })
}

pub(crate) fn object_movement(raw: &str) -> Result<ObjectMovement, String> {
    Ok(match norm(raw).as_str() {
        "moveandsizewithcells" => ObjectMovement::MoveAndSizeWithCells,
        "movebutdontsizewithcells" => ObjectMovement::MoveButDontSizeWithCells,
        "dontmoveorsizewithcells" => ObjectMovement::DontMoveOrSizeWithCells,
        "moveandsizewithcellsafter" => ObjectMovement::MoveAndSizeWithCellsAfter,
        other => return Err(format!("unknown object movement {other}")),
    })
}

pub(crate) fn header_position(raw: &str) -> Result<HeaderImagePosition, String> {
    Ok(match norm(raw).as_str() {
        "left" => HeaderImagePosition::Left,
        "center" => HeaderImagePosition::Center,
        "right" => HeaderImagePosition::Right,
        other => return Err(format!("unknown header image position {other}")),
    })
}

pub(crate) fn sparkline_type(raw: &str) -> Result<SparklineType, String> {
    Ok(match norm(raw).as_str() {
        "line" => SparklineType::Line,
        "column" => SparklineType::Column,
        "winlose" => SparklineType::WinLose,
        other => return Err(format!("unknown sparkline type {other}")),
    })
}

pub(crate) fn filter_criteria(raw: &str) -> Result<FilterCriteria, String> {
    Ok(match norm(raw).as_str() {
        "equalto" | "eq" => FilterCriteria::EqualTo,
        "notequalto" | "ne" => FilterCriteria::NotEqualTo,
        "greaterthan" | "gt" => FilterCriteria::GreaterThan,
        "greaterthanorequalto" | "ge" => FilterCriteria::GreaterThanOrEqualTo,
        "lessthan" | "lt" => FilterCriteria::LessThan,
        "lessthanorequalto" | "le" => FilterCriteria::LessThanOrEqualTo,
        "beginswith" => FilterCriteria::BeginsWith,
        "doesnotbeginwith" => FilterCriteria::DoesNotBeginWith,
        "endswith" => FilterCriteria::EndsWith,
        "doesnotendwith" => FilterCriteria::DoesNotEndWith,
        "contains" => FilterCriteria::Contains,
        "doesnotcontain" => FilterCriteria::DoesNotContain,
        other => return Err(format!("unknown filter criteria {other}")),
    })
}

pub(crate) fn error_style(raw: &str) -> Result<DataValidationErrorStyle, String> {
    Ok(match norm(raw).as_str() {
        "stop" => DataValidationErrorStyle::Stop,
        "warning" => DataValidationErrorStyle::Warning,
        "information" => DataValidationErrorStyle::Information,
        other => return Err(format!("unknown error style {other}")),
    })
}

pub(crate) fn number_rule<T: Copy + rust_xlsxwriter::IntoDataValidationValue>(
    raw: &str,
    first: T,
    second: Option<T>,
) -> Result<DataValidationRule<T>, String> {
    Ok(match norm(raw).as_str() {
        "equalto" | "eq" => DataValidationRule::EqualTo(first),
        "notequalto" | "ne" => DataValidationRule::NotEqualTo(first),
        "greaterthan" | "gt" => DataValidationRule::GreaterThan(first),
        "greaterthanorequalto" | "ge" => DataValidationRule::GreaterThanOrEqualTo(first),
        "lessthan" | "lt" => DataValidationRule::LessThan(first),
        "lessthanorequalto" | "le" => DataValidationRule::LessThanOrEqualTo(first),
        "between" => DataValidationRule::Between(
            first,
            second.ok_or_else(|| "between needs a second value".to_string())?,
        ),
        "notbetween" => DataValidationRule::NotBetween(
            first,
            second.ok_or_else(|| "not_between needs a second value".to_string())?,
        ),
        other => return Err(format!("unknown validation rule {other}")),
    })
}

pub(crate) fn table_function(raw: &str) -> Result<TableFunction, String> {
    Ok(match norm(raw).as_str() {
        "none" => TableFunction::None,
        "average" => TableFunction::Average,
        "count" => TableFunction::Count,
        "countnumbers" => TableFunction::CountNumbers,
        "max" => TableFunction::Max,
        "min" => TableFunction::Min,
        "sum" => TableFunction::Sum,
        "stddev" => TableFunction::StdDev,
        "var" => TableFunction::Var,
        other => return Err(format!("unknown table total {other}")),
    })
}

pub(crate) fn table_style(raw: &str) -> Result<TableStyle, String> {
    use TableStyle::{
        Dark1, Dark2, Dark3, Dark4, Dark5, Dark6, Dark7, Dark8, Dark9, Dark10, Dark11, Light1,
        Light2, Light3, Light4, Light5, Light6, Light7, Light8, Light9, Light10, Light11, Light12,
        Light13, Light14, Light15, Light16, Light17, Light18, Light19, Light20, Light21, Medium1,
        Medium2, Medium3, Medium4, Medium5, Medium6, Medium7, Medium8, Medium9, Medium10, Medium11,
        Medium12, Medium13, Medium14, Medium15, Medium16, Medium17, Medium18, Medium19, Medium20,
        Medium21, Medium22, Medium23, Medium24, Medium25, Medium26, Medium27, Medium28, None,
    };
    Ok(match norm(raw).as_str() {
        "none" => None,
        "light1" => Light1,
        "light2" => Light2,
        "light3" => Light3,
        "light4" => Light4,
        "light5" => Light5,
        "light6" => Light6,
        "light7" => Light7,
        "light8" => Light8,
        "light9" => Light9,
        "light10" => Light10,
        "light11" => Light11,
        "light12" => Light12,
        "light13" => Light13,
        "light14" => Light14,
        "light15" => Light15,
        "light16" => Light16,
        "light17" => Light17,
        "light18" => Light18,
        "light19" => Light19,
        "light20" => Light20,
        "light21" => Light21,
        "medium1" => Medium1,
        "medium2" => Medium2,
        "medium3" => Medium3,
        "medium4" => Medium4,
        "medium5" => Medium5,
        "medium6" => Medium6,
        "medium7" => Medium7,
        "medium8" => Medium8,
        "medium9" => Medium9,
        "medium10" => Medium10,
        "medium11" => Medium11,
        "medium12" => Medium12,
        "medium13" => Medium13,
        "medium14" => Medium14,
        "medium15" => Medium15,
        "medium16" => Medium16,
        "medium17" => Medium17,
        "medium18" => Medium18,
        "medium19" => Medium19,
        "medium20" => Medium20,
        "medium21" => Medium21,
        "medium22" => Medium22,
        "medium23" => Medium23,
        "medium24" => Medium24,
        "medium25" => Medium25,
        "medium26" => Medium26,
        "medium27" => Medium27,
        "medium28" => Medium28,
        "dark1" => Dark1,
        "dark2" => Dark2,
        "dark3" => Dark3,
        "dark4" => Dark4,
        "dark5" => Dark5,
        "dark6" => Dark6,
        "dark7" => Dark7,
        "dark8" => Dark8,
        "dark9" => Dark9,
        "dark10" => Dark10,
        "dark11" => Dark11,
        other => return Err(format!("unknown table style {other}")),
    })
}
