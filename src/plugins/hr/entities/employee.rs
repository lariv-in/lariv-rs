use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use crate::plugins::hr::blood_group::BloodGroup;
use crate::plugins::hr::gender::ApplicantGender;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "hr_employees")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    /// User account this employment record belongs to.
    #[sea_orm(indexed)]
    pub user_id: i64,
    pub name: Option<String>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    /// When the employment record was opened.
    pub hired_at: DateTime<Utc>,
    pub is_probationary: bool,
    pub fathers_name: Option<String>,
    pub date_of_birth: Option<NaiveDate>,
    pub gender: Option<ApplicantGender>,
    pub marital_status: Option<String>,
    pub nationality: Option<String>,
    pub is_disabled: Option<bool>,
    pub disability_type: Option<String>,
    #[sea_orm(indexed)]
    pub photograph_vnode_id: Option<i64>,
    pub blood_group: Option<BloodGroup>,
    pub identification_mark: Option<String>,
    pub present_address: Option<String>,
    pub present_pin_code: Option<String>,
    pub permanent_address: Option<String>,
    pub permanent_pin_code: Option<String>,
    pub emergency_contact_name: Option<String>,
    pub emergency_contact_relation: Option<String>,
    pub emergency_contact_mobile: Option<String>,
    /// `filesystem_nodes.id` for Aadhar file.
    #[sea_orm(indexed)]
    pub aadhar_vnode_id: Option<i64>,
    /// `filesystem_nodes.id` for PAN file.
    #[sea_orm(indexed)]
    pub pan_vnode_id: Option<i64>,
    /// `filesystem_nodes.id` for Passport file.
    #[sea_orm(indexed)]
    pub passport_vnode_id: Option<i64>,
    pub account_holder_name: Option<String>,
    pub account_number: Option<String>,
    pub account_ifsc_code: Option<String>,
    pub account_type: Option<String>,
    pub qualifications: Option<String>,
    pub date_of_joining: Option<NaiveDate>,
    pub probation_end_date: Option<NaiveDate>,
    /// Shift start, clock time only.
    pub work_start: Option<NaiveTime>,
    /// Shift end, clock time only.
    pub work_end: Option<NaiveTime>,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub base_salary: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((19, 6)))")]
    pub hourly_wage: Option<Decimal>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "crate::plugins::users::entities::user::Entity",
        from = "Column::UserId",
        to = "crate::plugins::users::entities::user::Column::Id",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<crate::plugins::users::entities::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
