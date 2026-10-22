//! Geographic points stored as latitude/longitude or a PostGIS point.
//!
//! SQLite columns are `{prefix}_latitude` and `{prefix}_longitude`. PostgreSQL
//! stores the same point as `{prefix}_position geography(Point, 4326)`.
//! [`GeoPoint`] is the SeaORM value for that column: reads decode hex EWKB from
//! `CAST(column AS text)`, and writes bind `SRID=4326;POINT(longitude latitude)`.

use sea_orm::entity::prelude::*;
use sea_orm::sea_query::{ArrayType, ColumnType, Nullable, ValueType, ValueTypeErr};
use sea_orm::{ColIdx, TryGetable};
use serde::{Deserialize, Serialize};

/// WGS84 coordinate. Latitude is north, longitude is east.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LatLng {
    pub latitude: f64,
    pub longitude: f64,
}

impl LatLng {
    /// Rejects values outside the WGS84 ranges.
    pub fn new(latitude: f64, longitude: f64) -> Result<Self, String> {
        if !(-90.0..=90.0).contains(&latitude) {
            return Err("latitude must be between -90 and 90".to_string());
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err("longitude must be between -180 and 180".to_string());
        }
        if !latitude.is_finite() || !longitude.is_finite() {
            return Err("location must be a finite coordinate".to_string());
        }
        Ok(Self {
            latitude,
            longitude,
        })
    }
}

/// PostGIS `geography(Point, 4326)` column value.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeoPoint(pub LatLng);

impl GeoPoint {
    pub fn lat_lng(self) -> LatLng {
        self.0
    }

    /// EWKT bound on insert and cast to `geography(Point, 4326)`.
    pub fn ewkt(self) -> String {
        format!(
            "SRID=4326;POINT({} {})",
            format_ord(self.0.longitude),
            format_ord(self.0.latitude)
        )
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("empty geography".to_string());
        }
        if text.chars().all(|ch| ch.is_ascii_hexdigit()) && text.len() >= 42 && text.len() % 2 == 0
        {
            return parse_ewkb_hex(text).map(Self);
        }
        parse_wkt(text).map(Self)
    }
}

impl From<LatLng> for GeoPoint {
    fn from(value: LatLng) -> Self {
        Self(value)
    }
}

fn format_ord(number: f64) -> String {
    let text = format!("{number:.8}");
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() || trimmed == "-" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_wkt(text: &str) -> Result<LatLng, String> {
    let rest = text
        .strip_prefix("SRID=")
        .and_then(|body| body.split_once(';').map(|(_, point)| point))
        .unwrap_or(text)
        .trim();
    let inner = rest
        .strip_prefix("POINT")
        .ok_or_else(|| format!("unsupported geography: {text}"))?
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim();
    let mut parts = inner.split_whitespace();
    let longitude: f64 = parts
        .next()
        .ok_or_else(|| "point is missing longitude".to_string())?
        .parse()
        .map_err(|_| "invalid longitude".to_string())?;
    let latitude: f64 = parts
        .next()
        .ok_or_else(|| "point is missing latitude".to_string())?
        .parse()
        .map_err(|_| "invalid latitude".to_string())?;
    LatLng::new(latitude, longitude)
}

/// Little-endian or big-endian EWKB hex for a 2D point, with or without an SRID.
pub fn parse_ewkb_hex(hex: &str) -> Result<LatLng, String> {
    let bytes = decode_hex(hex.trim())?;
    if bytes.len() < 21 {
        return Err("ewkb point is too short".to_string());
    }
    let little = match bytes[0] {
        1 => true,
        0 => false,
        _ => return Err("ewkb byte order is invalid".to_string()),
    };
    let typ = read_u32(&bytes[1..5], little);
    let base = typ & 0xff;
    if base != 1 {
        return Err("ewkb value is not a point".to_string());
    }
    let mut index = 5;
    if typ & 0x2000_0000 != 0 {
        index += 4;
    }
    if bytes.len() < index + 16 {
        return Err("ewkb point is truncated".to_string());
    }
    let longitude = read_f64(&bytes[index..index + 8], little);
    let latitude = read_f64(&bytes[index + 8..index + 16], little);
    LatLng::new(latitude, longitude)
}

fn decode_hex(hex: &str) -> Result<Vec<u8>, String> {
    if !hex.len().is_multiple_of(2) || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err("ewkb hex is invalid".to_string());
    }
    (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).map_err(|err| err.to_string()))
        .collect()
}

fn read_u32(bytes: &[u8], little: bool) -> u32 {
    let raw = [bytes[0], bytes[1], bytes[2], bytes[3]];
    if little {
        u32::from_le_bytes(raw)
    } else {
        u32::from_be_bytes(raw)
    }
}

fn read_f64(bytes: &[u8], little: bool) -> f64 {
    let mut raw = [0u8; 8];
    raw.copy_from_slice(bytes);
    if little {
        f64::from_le_bytes(raw)
    } else {
        f64::from_be_bytes(raw)
    }
}

impl From<GeoPoint> for Value {
    fn from(source: GeoPoint) -> Self {
        source.ewkt().into()
    }
}

impl TryGetable for GeoPoint {
    fn try_get_by<I: ColIdx>(res: &QueryResult, index: I) -> Result<Self, TryGetError> {
        let text = <String as TryGetable>::try_get_by(res, index)?;
        GeoPoint::parse(&text)
            .map_err(|err| TryGetError::DbErr(DbErr::Custom(format!("geography point: {err}"))))
    }
}

impl ValueType for GeoPoint {
    fn try_from(value: Value) -> Result<Self, ValueTypeErr> {
        let text = <String as ValueType>::try_from(value)?;
        GeoPoint::parse(&text).map_err(|_| ValueTypeErr)
    }

    fn type_name() -> String {
        "geography(Point,4326)".to_owned()
    }

    fn array_type() -> ArrayType {
        ArrayType::String
    }

    fn column_type() -> ColumnType {
        ColumnType::custom("geography(Point,4326)")
    }
}

impl Nullable for GeoPoint {
    fn null() -> Value {
        <String as Nullable>::null()
    }
}

/// Great-circle distance in meters. SQLite uses this for the attendance zone.
pub fn distance_meters(from: LatLng, to: LatLng) -> f64 {
    const EARTH: f64 = 6_378_137.0;
    let lat1 = from.latitude.to_radians();
    let lat2 = to.latitude.to_radians();
    let d_lat = (to.latitude - from.latitude).to_radians();
    let d_lng = (to.longitude - from.longitude).to_radians();
    let h = (d_lat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (d_lng / 2.0).sin().powi(2);
    2.0 * EARTH * h.sqrt().min(1.0).asin()
}

pub fn within_range(point: LatLng, center: LatLng, range_meters: f64) -> bool {
    distance_meters(point, center) <= range_meters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geopoint_decodes_known_ewkb_and_saves_ewkt() {
        let origin =
            GeoPoint::parse("0101000020E610000000000000000000000000000000000000").expect("origin");
        assert_eq!(origin.lat_lng().latitude, 0.0);
        assert_eq!(origin.lat_lng().longitude, 0.0);
        assert_eq!(origin.ewkt(), "SRID=4326;POINT(0 0)");
        let saved = Value::from(origin);
        let restored = <GeoPoint as ValueType>::try_from(saved).expect("ewkt value");
        assert_eq!(restored.ewkt(), "SRID=4326;POINT(0 0)");
    }

    #[test]
    fn haversine_inside_outside_and_on_the_radius() {
        let center = LatLng::new(0.0, 0.0).unwrap();
        let north = LatLng::new(1.0, 0.0).unwrap();
        let distance = distance_meters(center, north);
        assert!(distance > 110_000.0 && distance < 112_000.0, "{distance}");
        assert!(within_range(north, center, distance));
        assert!(within_range(north, center, distance + 1.0));
        assert!(!within_range(north, center, distance - 1.0));
        assert!(within_range(center, center, 0.0));
    }
}
