pub mod applicant;
pub mod attendance;
pub mod employee;
pub mod ex_employee;
pub mod holiday;
pub mod job_form;
pub mod leaves;

pub use applicant::Entity as ApplicantEntity;
pub use attendance::Entity as AttendanceEntity;
pub use employee::Entity as EmployeeEntity;
pub use ex_employee::Entity as ExEmployeeEntity;
pub use holiday::Entity as HolidayEntity;
pub use job_form::Entity as JobFormEntity;
pub use leaves::ApprovedLeaveEntity;
pub use leaves::LeaveApplicationEntity;
pub use leaves::LeaveJournalEntity;
pub use leaves::LeaveType;
pub use leaves::RejectedLeaveEntity;
