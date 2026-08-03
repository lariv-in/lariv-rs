//! LLM tool registry capability — plugins register tools; assistant chat runs them.
//!
//! Mirrors [`crate::grapesjs::GrapesJsCapability`] and [`crate::apps::AppsCapability`]:
//! deferred registrar hooks at mount, mounted as [`Arc<LlmToolsCapability>`] for cheap
//! request-extension clones.
//!
//! # Lifecycle
//!
//! 1. Attach via [`with_llm_tools`].
//! 2. Plugins implement [`ToolsRegistrar`] to call [`LlmToolsCapability::register`].
//! 3. At mount, hooks fold over the capability → [`Arc<LlmToolsCapability>`] on the app HList.
//! 4. LLM Assistant reads [`LlmToolsCapability::declarations`] for Gemini function calling
//!    and dispatches tool calls via [`LlmTool::run`] with [`ToolCtx`].
//!
//! # Core types
//!
//! - [`LlmToolsTag`] — capability tag
//! - [`LlmTool`] — pluggable Gemini function-calling tool
//! - [`ToolCtx`] — request-time context (DB, filestore, CSE keys, Rune env)
//! - [`LlmToolsCapability`] — mounted tool registry
//! - [`LlmToolsCap`] — builder-phase [`CapStore`]
//! - [`ToolsRegistrar`] — plugin hook trait
//!
//! # Examples
//!
//! ```rust ignore
//! impl ToolsRegistrar for RegisterToolsHook {
//!     fn register_tools(self, tools: &mut LlmToolsCapability) {
//!         tools.register(MyTool);
//!     }
//! }
//!
//! let app = with_llm_tools(app);
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use frunk::{HCons, HNil, hlist::HList};
use sea_orm::DatabaseConnection;
use serde_json::Value;

use crate::{
    app::App,
    capability::{ApplyHooks, CapStore, Capability, mount_with_hooks},
    plugins::{
        filesystem::storage::DynFilestore,
        llm_assistant::genai::FunctionDeclaration,
    },
    rune_env::RuneEnvCapability,
    tag::Tagged,
    traits::add::{AddCapability, CapTagAbsent},
};

/// Capability tag for the LLM tool registry.
pub struct LlmToolsTag;

/// Request-time context passed into [`LlmTool::run`] (not stored on the capability).
///
/// Built per chat/tool invocation from mounted app capabilities and request extensions.
pub struct ToolCtx<'a> {
    pub db: &'a DatabaseConnection,
    pub store: Arc<DynFilestore>,
    pub cse_api_key: &'a str,
    pub cse_cx: &'a str,
    pub rune_env: &'a RuneEnvCapability,
}

/// Pluggable Gemini function-calling tool.
///
/// Register via [`LlmToolsCapability::register`]; the assistant uses [`Self::declaration`]
/// for the tools schema and [`Self::run`] when the model emits a function call.
#[async_trait]
pub trait LlmTool: Send + Sync {
    fn name(&self) -> &str;
    fn declaration(&self) -> FunctionDeclaration;
    async fn run(&self, ctx: &ToolCtx<'_>, args: Value) -> Result<Value, String>;
}

pub type DynLlmTool = Arc<dyn LlmTool>;

/// Plugin hook for appending tools onto a [`LlmToolsCapability`].
///
/// Must mutate in place via [`LlmToolsCapability::register`] (not chain by-value returns).
pub trait ToolsRegistrar {
    fn register_tools(self, tools: &mut LlmToolsCapability);
}

/// Builder-phase LLM tools capability.
#[derive(Clone, Default)]
pub struct LlmToolsCapability {
    tools: Vec<DynLlmTool>,
}

impl LlmToolsCapability {
    /// Empty tool registry (starting point for [`ToolsRegistrar`] hooks).
    pub fn new() -> Self {
        Self::default()
    }

    /// Register (or replace) a tool by name. Prefer `&mut self` over by-value chaining.
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

    /// Look up a registered tool by name.
    pub fn get(&self, name: &str) -> Option<DynLlmTool> {
        self.tools.iter().find(|t| t.name() == name).cloned()
    }

    /// All registered tools in registration order.
    pub fn all(&self) -> &[DynLlmTool] {
        &self.tools
    }

    /// Gemini function declarations for all registered tools.
    pub fn declarations(&self) -> Vec<FunctionDeclaration> {
        self.tools.iter().map(|t| t.declaration()).collect()
    }
}

/// Builder-phase LLM tools capability.
pub type LlmToolsCap<Hooks> = CapStore<LlmToolsTag, Hooks, LlmToolsCapability>;

impl<Hooks> LlmToolsCap<Hooks> {
    /// Eagerly fold registrar hooks into items (testing / pre-mount inspection).
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genai::FunctionDeclaration;

    struct DummyTool(&'static str);

    #[async_trait]
    impl LlmTool for DummyTool {
        fn name(&self) -> &str {
            self.0
        }
        fn declaration(&self) -> FunctionDeclaration {
            FunctionDeclaration {
                name: self.0.into(),
                description: "dummy".into(),
                parameters: None,
            }
        }
        async fn run(&self, _ctx: &ToolCtx<'_>, _args: Value) -> Result<Value, String> {
            Ok(Value::Null)
        }
    }

    #[test]
    fn register_get_upsert() {
        let mut cap = LlmToolsCapability::new();
        cap.register(DummyTool("a")).register(DummyTool("b"));
        assert_eq!(cap.all().len(), 2);
        assert!(cap.get("a").is_some());
        cap.register(DummyTool("a"));
        assert_eq!(cap.all().len(), 2);
        assert_eq!(cap.declarations().len(), 2);
    }
}
