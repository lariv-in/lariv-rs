use chrono::{DateTime, NaiveDate, Utc};
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
    #[sea_orm(indexed)]
    pub user_id: i64,
    pub name: String,
    pub mobile: String,
    pub email: String,
    pub hired_at: DateTime<Utc>,
    pub is_probationary: bool,
    pub fathers_name: String,
    pub date_of_birth: Option<NaiveDate>,
    pub gender: Option<ApplicantGender>,
    pub marital_status: String,
    pub nationality: String,
    pub is_disabled: bool,
    pub disability_type: String,
    #[sea_orm(indexed)]
    pub photograph_vnode_id: Option<i64>,
    pub blood_group: Option<BloodGroup>,
    pub identification_mark: String,
    pub present_address: String,
    pub present_pin_code: String,
    pub permanent_address: String,
    pub permanent_pin_code: String,
    pub emergency_contact_name: String,
    pub emergency_contact_relation: String,
    pub emergency_contact_mobile: String,
    /// `documents.id` where `document_type` is `aadhar_card`.
    #[sea_orm(indexed)]
    pub aadhar_document_id: Option<i64>,
    /// `documents.id` where `document_type` is `pan`.
    #[sea_orm(indexed)]
    pub pan_document_id: Option<i64>,
    /// `documents.id` where `document_type` is `passport`.
    #[sea_orm(indexed)]
    pub passport_document_id: Option<i64>,
    pub account_holder_name: String,
    pub account_number: String,
    pub account_ifsc_code: String,
    pub account_type: String,
    pub qualifications: String,
    pub date_of_joining: Option<NaiveDate>,
    pub probation_end_date: Option<NaiveDate>,
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
