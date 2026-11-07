//! Compile-time HTMX swap keys for the LLM assistant plugin.

use lariv_core::components::SwapKey;

pub const SKILLS_TABLE: SwapKey = SwapKey("skills-table");
pub const SKILL_CREATE_MODAL: SwapKey = SwapKey("skill-create-modal");
pub const SKILL_EDIT_MODAL: SwapKey = SwapKey("skill-edit-modal");
pub const SKILL_DELETE_MODAL: SwapKey = SwapKey("skill-delete-modal");
pub const SKILL_IMPORT_MODAL: SwapKey = SwapKey("skill-import-modal");
pub const SKILL_FILES: SwapKey = SwapKey("fk-llm-skill-files");
pub const HISTORY_TABLE: SwapKey = SwapKey("llm-assistant-history-table");
pub const HISTORY_MODAL_TABLE: SwapKey = SwapKey("llm-assistant-history-modal-table");
pub const CRON_JOBS_TABLE: SwapKey = SwapKey("llm-assistant-cron-jobs-table");
pub const CRON_JOB_CREATE_MODAL: SwapKey = SwapKey("cron-job-create-modal");
pub const CRON_JOB_EDIT_MODAL: SwapKey = SwapKey("cron-job-edit-modal");
pub const CRON_JOB_DELETE_MODAL: SwapKey = SwapKey("cron-job-delete-modal");
