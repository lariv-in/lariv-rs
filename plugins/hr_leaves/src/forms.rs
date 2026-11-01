use lariv_core::html_form::{
    html_form,
    widgets::{Date, Number, Section, Select, Text, Textarea},
};
use lariv_plugin_users::routes::UsersSelectRouteTag;

use lariv_plugin_hr::entities::leaves::LeaveType;

#[html_form]
pub struct LeaveApplicationForm {
    /// Superuser picks who the leave is for. Hidden for every other role.
    #[form(
        label = "User",
        required,
        widget = ForeignKey,
        route = UsersSelectRouteTag,
        swap_key = "hr-leave-user",
        display = "user",
        placeholder = "Select user…",
        when = "pick_user"
    )]
    pub user_id: i64,

    #[form(label = "Date", required, widget = Date)]
    pub date: String,

    #[form(label = "Type", required, widget = Select, choices = "leave_type")]
    pub leave_type: String,

    #[form(label = "Reason", required, widget = Textarea, rows = 4)]
    pub reason: String,
}

impl LeaveApplicationForm {
    pub fn leave_type_choices() -> &'static [(&'static str, &'static str)] {
        LeaveType::choices()
    }
}

/// Confirm-only. The approver is the signed-in user and the time is now.
#[html_form]
pub struct ApproveLeaveForm {}

#[html_form]
pub struct RejectLeaveForm {
    #[form(label = "Reason", widget = Textarea, rows = 3)]
    pub reason: String,
}

/// Confirm-only. Removes the approval and returns the leave to pending.
#[html_form]
pub struct RevokeApprovalForm {}

/// Confirm-only. Removes the rejection and returns the leave to pending.
#[html_form]
pub struct RevokeRejectionForm {}

/// Credits an employee's leave journal. Amount is a positive number of days.
#[html_form]
pub struct GiveLeaveForm {
    #[form(label = "Type", required, widget = Select, choices = "leave_type")]
    pub leave_type: String,

    #[form(label = "Days", required, widget = Number)]
    pub amount: String,
}

impl GiveLeaveForm {
    pub fn leave_type_choices() -> &'static [(&'static str, &'static str)] {
        LeaveType::choices()
    }
}

const ALLOCATED_HINT: &str =
    "Whole days credited for each qualifying streak. 0 skips this leave type.";
const DAY_HINT: &str =
    "Last is the last day of the month, or 31 December when the schedule is yearly.";
const MONTH_HINT: &str = "Month of a yearly schedule. Ignored when the day is Last.";

#[html_form]
pub struct LeaveCalcPreferencesForm {
    #[form(
        label = "Timezone",
        required,
        widget = Text,
        hint = "IANA name used for the schedule and for attendance dates. Example: Asia/Kolkata"
    )]
    pub timezone: String,

    #[form(widget = Section, label = "Casual")]
    _section_casual: (),

    #[form(label = "Leave allocated", required, widget = Number, hint = ALLOCATED_HINT)]
    pub casual_leave_allocated: String,

    #[form(
        label = "Schedule",
        required,
        widget = Select,
        choices = "schedule_kind",
        model = "casualSchedule"
    )]
    pub casual_schedule_kind: String,

    #[form(
        label = "Month",
        required,
        widget = Select,
        choices = "month",
        hint = MONTH_HINT,
        show = "casualSchedule === 'yearly'"
    )]
    pub casual_month: String,

    #[form(label = "Day", required, widget = Select, choices = "day", hint = DAY_HINT)]
    pub casual_day: String,

    #[form(widget = Section, label = "Sick")]
    _section_sick: (),

    #[form(label = "Leave allocated", required, widget = Number, hint = ALLOCATED_HINT)]
    pub sick_leave_allocated: String,

    #[form(
        label = "Schedule",
        required,
        widget = Select,
        choices = "schedule_kind",
        model = "sickSchedule"
    )]
    pub sick_schedule_kind: String,

    #[form(
        label = "Month",
        required,
        widget = Select,
        choices = "month",
        hint = MONTH_HINT,
        show = "sickSchedule === 'yearly'"
    )]
    pub sick_month: String,

    #[form(label = "Day", required, widget = Select, choices = "day", hint = DAY_HINT)]
    pub sick_day: String,

    #[form(widget = Section, label = "Privilege Leave")]
    _section_privilege: (),

    #[form(
        label = "Consecutive perfect days",
        required,
        widget = Number,
        hint = "Perfect weekdays in a row. The allocated days are added when this many are reached. 0 skips privilege leave."
    )]
    pub privilege_consecutive_required: String,

    #[form(
        label = "Leave allocated",
        required,
        widget = Number,
        hint = "Whole days added to the leave journal each time the consecutive days are reached. 0 skips privilege leave."
    )]
    pub privilege_leave_allocated: String,
}

impl LeaveCalcPreferencesForm {
    pub fn schedule_kind_choices() -> &'static [(&'static str, &'static str)] {
        &[
            (crate::logic::leave_calc::SCHEDULE_MONTHLY, "Monthly"),
            (crate::logic::leave_calc::SCHEDULE_YEARLY, "Yearly"),
        ]
    }

    pub fn month_choices() -> &'static [(&'static str, &'static str)] {
        &[
            ("1", "January"),
            ("2", "February"),
            ("3", "March"),
            ("4", "April"),
            ("5", "May"),
            ("6", "June"),
            ("7", "July"),
            ("8", "August"),
            ("9", "September"),
            ("10", "October"),
            ("11", "November"),
            ("12", "December"),
        ]
    }

    pub fn day_choices() -> &'static [(&'static str, &'static str)] {
        &[
            ("1", "1"),
            ("2", "2"),
            ("3", "3"),
            ("4", "4"),
            ("5", "5"),
            ("6", "6"),
            ("7", "7"),
            ("8", "8"),
            ("9", "9"),
            ("10", "10"),
            ("11", "11"),
            ("12", "12"),
            ("13", "13"),
            ("14", "14"),
            ("15", "15"),
            ("16", "16"),
            ("17", "17"),
            ("18", "18"),
            ("19", "19"),
            ("20", "20"),
            ("21", "21"),
            ("22", "22"),
            ("23", "23"),
            ("24", "24"),
            ("25", "25"),
            ("26", "26"),
            ("27", "27"),
            ("28", "28"),
            ("29", "29"),
            ("30", "30"),
            ("31", "31"),
            (crate::logic::leave_calc::DAY_LAST, "Last"),
        ]
    }
}
