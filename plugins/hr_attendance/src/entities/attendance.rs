//! `hr_attendances` is created by the hub's attendance migrations and read by the hub's leave
//! calculation, so the entity definition stays in the hub and is re-exported here.

pub use lariv_plugin_hr::entities::attendance::*;
