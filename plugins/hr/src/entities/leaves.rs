//! Leave applications, and the approval or rejection recorded against one.

pub mod approved_leave;
pub mod leave_application;
pub mod leave_attendance_use;
pub mod leave_calc_preference;
pub mod leave_evaluation_run;
pub mod leave_journal;
pub mod leave_type;
pub mod rejected_leave;

pub use approved_leave::Entity as ApprovedLeaveEntity;
pub use leave_application::Entity as LeaveApplicationEntity;
pub use leave_journal::Entity as LeaveJournalEntity;
pub use leave_type::LeaveType;
pub use rejected_leave::Entity as RejectedLeaveEntity;
