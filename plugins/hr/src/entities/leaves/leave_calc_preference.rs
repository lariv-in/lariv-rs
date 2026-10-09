use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use super::leave_type::LeaveType;

/// How one leave type earns days from consecutive perfect attendance.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "hr_leave_calc_preferences")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    #[sea_orm(unique)]
    pub leave_type: LeaveType,
    /// Perfect weekdays in a row before a streak can earn leave. Zero disables the type.
    pub consecutive_required: i64,
    /// Whole days credited for each qualifying streak. Zero disables the type.
    pub leave_allocated: i64,
    /// `yearly` or `monthly`.
    pub schedule_kind: String,
    /// Month 1–12 when the schedule is yearly. Empty for a monthly schedule.
    pub month: Option<i32>,
    /// `last`, or a day number `1`–`31`.
    pub day_spec: String,
    /// IANA name used for the schedule clock and for attendance dates.
    pub timezone: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
