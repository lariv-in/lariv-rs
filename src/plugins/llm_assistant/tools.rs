//! Builtin LLM tools registered onto [`crate::llm_tools::LlmToolsCapability`].

mod attach_vnode_to_context;
mod generate_pdf;
mod get_rune_env;
mod google_search;
mod list_rune_env;
mod read_webpage;
mod run_rune;
mod run_rune_file;
mod skills;

use crate::llm_tools::{LlmToolsCapability, ToolsRegistrar};

use attach_vnode_to_context::AttachVnodeToContextTool;
use generate_pdf::GeneratePdfTool;
use get_rune_env::GetRuneEnvTool;
use google_search::GoogleSearchTool;
use list_rune_env::ListRuneEnvTool;
use read_webpage::ReadWebpageTool;
use run_rune::RunRuneTool;
use run_rune_file::RunRuneFileTool;
use skills::{CreateSkillTool, EditSkillTool, GetSkillDetailTool, ListSkillsTool};

/// Register core assistant tools (CSE, skills, Rune scripting, Typst PDF, VNode attach).
pub fn register_builtins(cap: &mut LlmToolsCapability) {
    cap.register(GoogleSearchTool)
        .register(ReadWebpageTool)
        .register(GeneratePdfTool)
        .register(AttachVnodeToContextTool)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_attach_vnode_to_context() {
        let mut cap = LlmToolsCapability::new();
        register_builtins(&mut cap);
        assert!(cap.get("attach_vnode_to_context").is_some());
        assert!(cap.get("generate_pdf").is_some());
    }
}
