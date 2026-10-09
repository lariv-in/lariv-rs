//! Overtime applications, and the approval or rejection recorded against one.

pub mod approved_overtime;
pub mod overtime_application;
pub mod rejected_overtime;

pub use approved_overtime::Entity as ApprovedOvertimeEntity;
pub use overtime_application::Entity as OvertimeApplicationEntity;
pub use rejected_overtime::Entity as RejectedOvertimeEntity;
