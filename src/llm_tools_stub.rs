//! Stub LLM tools registry when `cap-llm` is disabled (empty capability only).

use std::sync::Arc;

use frunk::{HCons, HNil, hlist::HList};
use sea_orm::DatabaseConnection;
use serde_json::Value;

use crate::{
    app::App,
    capability::{ApplyHooks, CapStore, Capability, mount_with_hooks},
    genai::FunctionDeclaration,
    rune_env::RuneEnvCapability,
    tag::Tagged,
    traits::add::{AddCapability, CapTagAbsent},
};

/// Capability tag for the LLM tool registry.
pub struct LlmToolsTag;

/// Placeholder when the filesystem plugin is not enabled.
pub struct StubFilestore;

/// Request-time context passed into [`LlmTool::run`] (not stored on the capability).
pub struct ToolCtx<'a> {
    pub db: &'a DatabaseConnection,
    pub store: Arc<StubFilestore>,
    pub cse_api_key: &'a str,
    pub cse_cx: &'a str,
    pub rune_env: &'a RuneEnvCapability,
}

/// Pluggable Gemini function-calling tool (disabled without `cap-llm`).
pub trait LlmTool: Send + Sync {
    fn name(&self) -> &str;
    fn declaration(&self) -> FunctionDeclaration;
    fn run<'a>(
        &'a self,
        ctx: &'a ToolCtx<'_>,
        args: Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, String>> + Send + 'a>>
    {
        let _ = (ctx, args);
        Box::pin(async { Err("cap-llm feature disabled".into()) })
    }
}

pub type DynLlmTool = Arc<dyn LlmTool>;

/// Plugin hook for appending tools onto a [`LlmToolsCapability`].
pub trait ToolsRegistrar {
    fn register_tools(self, tools: &mut LlmToolsCapability);
}

/// Builder-phase LLM tools capability.
#[derive(Clone, Default)]
pub struct LlmToolsCapability {
    tools: Vec<DynLlmTool>,
}

impl LlmToolsCapability {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, tool: impl LlmTool + 'static) -> &mut Self {
        let name = tool.name().to_string();
        let arc: DynLlmTool = Arc::new(tool);
        if let Some(existing) = self.tools.iter_mut().find(|t| t.name() == name) {
            *existing = arc;
        } else {
            self.tools.push(arc);
        }
        self
    }

    pub fn get(&self, name: &str) -> Option<DynLlmTool> {
        self.tools.iter().find(|t| t.name() == name).cloned()
    }

    pub fn all(&self) -> &[DynLlmTool] {
        &self.tools
    }

    pub fn declarations(&self) -> Vec<FunctionDeclaration> {
        self.tools.iter().map(|t| t.declaration()).collect()
    }
}

pub type LlmToolsCap<Hooks> = CapStore<LlmToolsTag, Hooks, LlmToolsCapability>;

impl<Hooks> LlmToolsCap<Hooks> {
    pub fn resolve_hooks<Proof>(self) -> LlmToolsCap<HNil>
    where
        Hooks: ApplyHooks<LlmToolsCapability, Proof, Output = LlmToolsCapability>,
    {
        CapStore::with_items(self.hooks.apply_hooks(self.items))
    }
}

impl<Plugin, H, Tail, TailProof> ApplyHooks<LlmToolsCapability, (TailProof, ())>
    for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: ApplyHooks<LlmToolsCapability, TailProof, Output = LlmToolsCapability>,
    H: ToolsRegistrar,
{
    type Output = LlmToolsCapability;

    fn apply_hooks(self, items: LlmToolsCapability) -> Self::Output {
        let mut items = self.tail.apply_hooks(items);
        self.head.value.register_tools(&mut items);
        items
    }
}

impl<Hooks> Capability for LlmToolsCap<Hooks>
where
    Hooks: ApplyHooks<LlmToolsCapability, (), Output = LlmToolsCapability>,
{
    type Value = Arc<LlmToolsCapability>;
    type Output = Tagged<LlmToolsTag, Arc<LlmToolsCapability>>;
    type Hooks = Hooks;
    type Items = LlmToolsCapability;

    fn mount(self) -> Self::Output {
        mount_with_hooks(self, Arc::new)
    }
}

/// Attach an empty LLM tools capability to the app builder.
pub fn with_llm_tools<L, Proof>(app: App<L>) -> App<HCons<LlmToolsCap<HNil>, L>>
where
    L: HList + CapTagAbsent<LlmToolsTag, Proof>,
{
    app.add_capability(CapStore::with_items(LlmToolsCapability::new()))
}
