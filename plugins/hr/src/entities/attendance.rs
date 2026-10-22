use chrono::{DateTime, Utc};
use lariv_core::db::location::{GeoPoint, LatLng};
use sea_orm::ColumnTrait;

/// Attendance row with punch locations, independent of the database shape.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub user_id: i64,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub started: Option<LatLng>,
    pub ended: Option<LatLng>,
}

/// Columns shared by the SQLite and PostgreSQL attendance entities.
pub trait AttendanceColumns: ColumnTrait + Copy + Send + Sync + 'static {
    fn col_id() -> Self;
    fn col_user_id() -> Self;
    fn col_started_at() -> Self;
    fn col_ended_at() -> Self;
}

fn pair(latitude: Option<f64>, longitude: Option<f64>) -> Option<LatLng> {
    match (latitude, longitude) {
        (Some(latitude), Some(longitude)) => Some(LatLng {
            latitude,
            longitude,
        }),
        _ => None,
    }
}

macro_rules! define_attendance_entity {
    (pub mod $name:ident { $($extra:tt)* }) => {
        pub mod $name {
            use chrono::{DateTime, Utc};
            use sea_orm::entity::prelude::*;
            #[allow(unused_imports)]
            use lariv_core::db::location::GeoPoint;
            use serde::{Deserialize, Serialize};

            #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
            #[sea_orm(table_name = "hr_attendances")]
            pub struct Model {
                #[sea_orm(primary_key)]
                pub id: i64,
                pub created_at: Option<DateTime<Utc>>,
                pub updated_at: Option<DateTime<Utc>>,
                #[sea_orm(indexed)]
                pub user_id: i64,
                pub started_at: DateTime<Utc>,
                pub ended_at: Option<DateTime<Utc>>,
                $($extra)*
            }

            #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
            pub enum Relation {
                #[sea_orm(
                    belongs_to = "lariv_plugin_users::entities::user::Entity",
                    from = "Column::UserId",
                    to = "lariv_plugin_users::entities::user::Column::Id",
                    on_delete = "Cascade"
                )]
                User,
            }

            impl Related<lariv_plugin_users::entities::user::Entity> for Entity {
                fn to() -> RelationDef {
                    Relation::User.def()
                }
            }

            impl ActiveModelBehavior for ActiveModel {}
        }
    };
}

define_attendance_entity! {
    pub mod sqlite {
        pub started_latitude: Option<f64>,
        pub started_longitude: Option<f64>,
        pub ended_latitude: Option<f64>,
        pub ended_longitude: Option<f64>,
    }
}

define_attendance_entity! {
    pub mod postgres {
        #[sea_orm(
            column_type = "custom(\"geography(Point,4326)\")",
            select_as = "text",
            save_as = "geography(Point,4326)"
        )]
        pub started_position: Option<GeoPoint>,
        #[sea_orm(
            column_type = "custom(\"geography(Point,4326)\")",
            select_as = "text",
            save_as = "geography(Point,4326)"
        )]
        pub ended_position: Option<GeoPoint>,
    }
}

impl AttendanceColumns for sqlite::Column {
    fn col_id() -> Self {
        Self::Id
    }
    fn col_user_id() -> Self {
        Self::UserId
    }
    fn col_started_at() -> Self {
        Self::StartedAt
    }
    fn col_ended_at() -> Self {
        Self::EndedAt
    }
}

impl AttendanceColumns for postgres::Column {
    fn col_id() -> Self {
        Self::Id
    }
    fn col_user_id() -> Self {
        Self::UserId
    }
    fn col_started_at() -> Self {
        Self::StartedAt
    }
    fn col_ended_at() -> Self {
        Self::EndedAt
    }
}

impl From<sqlite::Model> for Record {
    fn from(row: sqlite::Model) -> Self {
        Self {
            id: row.id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            user_id: row.user_id,
            started_at: row.started_at,
            ended_at: row.ended_at,
            started: pair(row.started_latitude, row.started_longitude),
            ended: pair(row.ended_latitude, row.ended_longitude),
        }
    }
}

impl From<postgres::Model> for Record {
    fn from(row: postgres::Model) -> Self {
        Self {
            id: row.id,
            created_at: row.created_at,
            updated_at: row.updated_at,
            user_id: row.user_id,
            started_at: row.started_at,
            ended_at: row.ended_at,
            started: row.started_position.map(GeoPoint::lat_lng),
            ended: row.ended_position.map(GeoPoint::lat_lng),
        }
    }
}
