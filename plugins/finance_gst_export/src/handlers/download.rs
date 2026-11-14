use axum::{
    body::Body,
    extract::Form,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use rust_decimal::Decimal;
use serde::Deserialize;

use lariv_core::http::Cap;
use lariv_plugin_users::middleware::RequireAuth;

use crate::gstr1;
use crate::state::GstExportState;

#[derive(Debug, Deserialize)]
pub struct Gstr1Form {
    /// `YYYY-MM` from the month combobox.
    pub period: String,
    #[serde(default)]
    pub gt: String,
    #[serde(default)]
    pub cur_gt: String,
}

pub async fn download(
    Cap(state): Cap<GstExportState>,
    RequireAuth(_ctx): RequireAuth,
    Form(form): Form<Gstr1Form>,
) -> Response {
    let (year, month) = match parse_period(&form.period) {
        Ok(period) => period,
        Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
    };
    let gt = match optional_decimal(&form.gt) {
        Ok(value) => value,
        Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
    };
    let cur_gt = match optional_decimal(&form.cur_gt) {
        Ok(value) => value,
        Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
    };
    let bytes = match gstr1::build(&state.db, year, month, gt, cur_gt).await {
        Ok(bytes) => bytes,
        Err(error) => return (StatusCode::BAD_REQUEST, error).into_response(),
    };
    let filename = format!("gstr1_{:02}{year}.json", month);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(Body::from(bytes))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn parse_period(raw: &str) -> Result<(i32, u32), String> {
    let (year, month) = raw
        .trim()
        .split_once('-')
        .ok_or_else(|| "choose a return month".to_string())?;
    let year = year
        .parse::<i32>()
        .map_err(|_| "choose a return month".to_string())?;
    let month = month
        .parse::<u32>()
        .map_err(|_| "choose a return month".to_string())?;
    if !(1..=12).contains(&month) {
        return Err("choose a return month".into());
    }
    Ok((year, month))
}

fn optional_decimal(raw: &str) -> Result<Option<Decimal>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    raw.parse::<Decimal>()
        .map(Some)
        .map_err(|error| format!("{raw} is not a number: {error}"))
}
