//! Builtin LLM tools registered onto [`crate::llm_tools::LlmToolsCapability`].

mod get_rune_env;
mod google_search;
mod list_rune_env;
mod read_webpage;
mod run_rune;
mod run_rune_file;
mod skills;

use crate::llm_tools::{LlmToolsCapability, ToolsRegistrar};

use get_rune_env::GetRuneEnvTool;
use google_search::GoogleSearchTool;
use list_rune_env::ListRuneEnvTool;
use read_webpage::ReadWebpageTool;
use run_rune::RunRuneTool;
use run_rune_file::RunRuneFileTool;
use skills::{CreateSkillTool, EditSkillTool, GetSkillDetailTool, ListSkillsTool};

/// Register core assistant tools (CSE, skills, Rune scripting).
pub fn register_builtins(cap: &mut LlmToolsCapability) {
    cap.register(GoogleSearchTool)
        .register(ReadWebpageTool)
        .register(ListSkillsTool)
        .register(GetSkillDetailTool)
        .register(CreateSkillTool)
        .register(EditSkillTool)
        .register(RunRuneTool)
        .register(RunRuneFileTool)
        .register(ListRuneEnvTool)
        .register(GetRuneEnvTool);
}

#[derive(Clone, Copy, Default)]
pub struct Hook;

impl ToolsRegistrar for Hook {
    fn register_tools(self, tools: &mut LlmToolsCapability) {
        register_builtins(tools);
    }
}
