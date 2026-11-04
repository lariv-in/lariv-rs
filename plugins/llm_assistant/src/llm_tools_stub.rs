//! Stub LLM tools registry when `cap-llm` is disabled (empty capability only).

use std::sync::Arc;

use frunk::{HCons, HNil, hlist::HList};
use sea_orm::DatabaseConnection;
use serde_json::Value;

use lariv_core::app::App;
use lariv_core::capability::{CapHookExt, CapStore, Capability, HasCapTag};
use lariv_core::genai::{FunctionDeclaration, FunctionResponsePart};
use lariv_core::rune_env::RuneEnvCapability;
use lariv_core::tag::Tagged;
use lariv_core::traits::add::{AddCapability, CapTagAbsent};

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

/// JSON payload plus optional Gemini function-response parts.
#[derive(Debug, Clone, Default)]
pub struct ToolResult {
    pub response: Value,
    pub parts: Vec<FunctionResponsePart>,
}

impl From<Value> for ToolResult {
    fn from(response: Value) -> Self {
        Self {
            response,
            parts: Vec::new(),
        }
    }
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

    fn run_with_parts<'a>(
        &'a self,
        ctx: &'a ToolCtx<'_>,
        args: Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ToolResult, String>> + Send + 'a>>
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

pub struct LlmToolsCap<Hooks> {
    store: CapStore<LlmToolsTag, Hooks, LlmToolsCapability>,
}

impl LlmToolsCap<HNil> {
    pub fn with_items(items: LlmToolsCapability) -> Self {
        Self {
            store: CapStore::with_items(items),
        }
    }
}

impl<Hooks> HasCapTag for LlmToolsCap<Hooks> {
    type Tag = LlmToolsTag;
}

impl<Hooks, Plugin, Hook> CapHookExt<Plugin, Hook> for LlmToolsCap<Hooks> {
    type Hooked = LlmToolsCap<HCons<Tagged<Plugin, Hook>, Hooks>>;

    fn prepend_cap_hook(self, hook: Hook) -> Self::Hooked {
        LlmToolsCap {
            store: self.store.add_hook::<Plugin, Hook>(hook),
        }
    }
}

/// Fold [`ToolsRegistrar`] hooks (tail first = install order).
pub trait FoldLlmToolsHooks {
    fn fold(self, items: LlmToolsCapability) -> LlmToolsCapability;
}

impl FoldLlmToolsHooks for HNil {
    fn fold(self, items: LlmToolsCapability) -> LlmToolsCapability {
        items
    }
}

impl<Plugin, H, Tail> FoldLlmToolsHooks for HCons<Tagged<Plugin, H>, Tail>
where
    Tail: FoldLlmToolsHooks,
    H: ToolsRegistrar,
{
    fn fold(self, items: LlmToolsCapability) -> LlmToolsCapability {
        let mut items = self.tail.fold(items);
        self.head.value.register_tools(&mut items);
        items
    }
}

impl<Hooks> LlmToolsCap<Hooks> {
    pub fn resolve_hooks(self) -> LlmToolsCap<HNil>
    where
        Hooks: FoldLlmToolsHooks,
    {
        LlmToolsCap::with_items(self.store.hooks.fold(self.store.items))
    }
}

impl<Hooks> Capability for LlmToolsCap<Hooks>
where
    Hooks: FoldLlmToolsHooks,
{
    type Value = Arc<LlmToolsCapability>;
    type Output = Tagged<LlmToolsTag, Arc<LlmToolsCapability>>;
    type Hooks = Hooks;
    type Items = LlmToolsCapability;

    fn mount(self) -> Self::Output {
        let items = self.store.hooks.fold(self.store.items);
        Tagged::new(Arc::new(items))
    }
}

/// Attach an empty LLM tools capability to the app builder.
pub fn with_llm_tools<L, Proof>(app: App<L>) -> App<HCons<LlmToolsCap<HNil>, L>>
where
    L: HList + CapTagAbsent<LlmToolsTag, Proof>,
{
    app.add_capability(LlmToolsCap::with_items(LlmToolsCapability::new()))
}
