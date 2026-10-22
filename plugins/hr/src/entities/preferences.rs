use chrono::{DateTime, Utc};
use lariv_core::db::location::{GeoPoint, LatLng};

/// HR preferences row, including the attendance zone when one is configured.
#[derive(Clone, Debug, PartialEq)]
pub struct HrPreferences {
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub employee_id_format: Option<String>,
    pub id_card_format: Option<String>,
    pub id_card_signing_key: Option<String>,
    pub attendance: Option<LatLng>,
    pub attendance_range_meters: Option<f64>,
}

macro_rules! define_preferences_entity {
    (pub mod $name:ident { $($extra:tt)* }) => {
        pub mod $name {
            use chrono::{DateTime, Utc};
            use sea_orm::entity::prelude::*;
            #[allow(unused_imports)]
            use lariv_core::db::location::GeoPoint;
            use serde::{Deserialize, Serialize};

            /// Singleton HR preferences (`id = 1`).
            #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
            #[sea_orm(table_name = "hr_preferences")]
            pub struct Model {
                #[sea_orm(primary_key)]
                pub id: i64,
                pub created_at: Option<DateTime<Utc>>,
                pub updated_at: Option<DateTime<Utc>>,
                /// Minijinja template rendered with the employee and user when an employee
                /// is created. Empty uses the default year-of-joining plus padded user id.
                pub employee_id_format: Option<String>,
                /// Minijinja template that expands to Typst for an employee ID card.
                /// Empty uses the shipped example.
                pub id_card_format: Option<String>,
                /// PKCS#8 PEM P-256 private key. Empty is replaced with a generated key before use.
                pub id_card_signing_key: Option<String>,
                pub attendance_range_meters: Option<f64>,
                $($extra)*
            }

            #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
            pub enum Relation {}

            impl ActiveModelBehavior for ActiveModel {}
        }
    };
}

define_preferences_entity! {
    pub mod sqlite {
        pub attendance_latitude: Option<f64>,
        pub attendance_longitude: Option<f64>,
    }
}

define_preferences_entity! {
    pub mod postgres {
        #[sea_orm(
            column_type = "custom(\"geography(Point,4326)\")",
            select_as = "text",
            save_as = "geography(Point,4326)"
        )]
        pub attendance_position: Option<GeoPoint>,
    }
}

impl From<sqlite::Model> for HrPreferences {
    fn from(row: sqlite::Model) -> Self {
        let attendance = match (row.attendance_latitude, row.attendance_longitude) {
            (Some(latitude), Some(longitude)) => Some(LatLng {
                latitude,
                longitude,
            }),
            _ => None,
        };
        Self {
            id: row.id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            employee_id_format: row.employee_id_format,
            id_card_format: row.id_card_format,
            id_card_signing_key: row.id_card_signing_key,
            attendance,
            attendance_range_meters: row.attendance_range_meters,
        }
    }
}

impl From<postgres::Model> for HrPreferences {
    fn from(row: postgres::Model) -> Self {
        Self {
            id: row.id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            employee_id_format: row.employee_id_format,
            id_card_format: row.id_card_format,
            id_card_signing_key: row.id_card_signing_key,
            attendance: row.attendance_position.map(GeoPoint::lat_lng),
            attendance_range_meters: row.attendance_range_meters,
        }
    }
}
