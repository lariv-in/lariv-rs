//! Compile-time HTMX swap keys for the LLM assistant plugin.

use crate::swap_key;

swap_key!(SkillsTableKey, "skills-table");
swap_key!(SkillCreateModalKey, "skill-create-modal");
swap_key!(SkillEditModalKey, "skill-edit-modal");
swap_key!(SkillDeleteModalKey, "skill-delete-modal");
swap_key!(SkillImportModalKey, "skill-import-modal");
swap_key!(SkillFilesKey, "fk-llm-skill-files");
swap_key!(HistoryTableKey, "llm-assistant-history-table");
swap_key!(CronJobsTableKey, "llm-assistant-cron-jobs-table");
swap_key!(CronJobCreateModalKey, "cron-job-create-modal");
swap_key!(CronJobEditModalKey, "cron-job-edit-modal");
swap_key!(CronJobDeleteModalKey, "cron-job-delete-modal");
