//! Builtin LLM tools registered onto [`crate::llm_tools::LlmToolsCapability`].

mod google_search;
mod list_rune_env;
mod read_file;
mod run_rune;
mod run_rune_file;
mod skills;

use crate::llm_tools::{LlmToolsCapability, ToolsRegistrar};

use google_search::GoogleSearchTool;
use list_rune_env::ListRuneEnvTool;
use read_file::ReadFileTool;
use run_rune::RunRuneTool;
use run_rune_file::RunRuneFileTool;
use skills::{GetSkillDetailTool, ListSkillsTool};

/// Register core assistant tools (CSE, skills, filesystem, Rune scripting).
pub fn register_builtins(cap: &mut LlmToolsCapability) {
    cap.register(GoogleSearchTool)
        .register(ListSkillsTool)
        .register(GetSkillDetailTool)
        .register(ReadFileTool)
        .register(RunRuneTool)
        .register(RunRuneFileTool)
        .register(ListRuneEnvTool);
}

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl ToolsRegistrar for Hook {
    fn register_tools(self, tools: &mut LlmToolsCapability) {
        register_builtins(tools);
    }
}
