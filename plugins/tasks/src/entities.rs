//! SeaORM entities for task statuses, tasks, and logs.

pub mod task;
pub mod task_log;
pub mod task_status;

pub use task::Entity as TaskEntity;
pub use task::Model as Task;
pub use task_log::Entity as TaskLogEntity;
pub use task_log::Model as TaskLog;
pub use task_status::Entity as TaskStatusEntity;
pub use task_status::Model as TaskStatus;
