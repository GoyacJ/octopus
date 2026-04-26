mod bash;
mod clarify;
mod grep;
mod list_dir;
mod read;
mod read_blob;
mod send_message;
mod web_search;
mod write;

pub use bash::BashTool;
pub use clarify::ClarifyTool;
pub use grep::GrepTool;
pub use list_dir::ListDirTool;
pub use read::FileReadTool;
pub use read_blob::ReadBlobTool;
pub use send_message::SendMessageTool;
pub use web_search::{WebSearchBackend, WebSearchRequest, WebSearchResult, WebSearchTool};
pub use write::FileWriteTool;

use harness_contracts::{
    BudgetMetric, DeferPolicy, OverflowAction, ProviderRestriction, ResultBudget, ToolCapability,
    ToolDescriptor, ToolGroup, ToolOrigin, ToolProperties, TrustLevel,
};
use serde_json::{json, Value};

fn descriptor(
    name: &str,
    display_name: &str,
    description: &str,
    group: ToolGroup,
    is_concurrency_safe: bool,
    is_read_only: bool,
    is_destructive: bool,
    budget_limit: u64,
    required_capabilities: Vec<ToolCapability>,
    input_schema: Value,
) -> ToolDescriptor {
    ToolDescriptor {
        name: name.to_owned(),
        display_name: display_name.to_owned(),
        description: description.to_owned(),
        category: "builtin".to_owned(),
        group,
        version: "0.1.0".to_owned(),
        input_schema,
        output_schema: None,
        dynamic_schema: false,
        properties: ToolProperties {
            is_concurrency_safe,
            is_read_only,
            is_destructive,
            long_running: None,
            defer_policy: DeferPolicy::AlwaysLoad,
        },
        trust_level: TrustLevel::AdminTrusted,
        required_capabilities,
        budget: ResultBudget {
            metric: BudgetMetric::Chars,
            limit: budget_limit,
            on_overflow: OverflowAction::Offload,
            preview_head_chars: 2_000,
            preview_tail_chars: 2_000,
        },
        provider_restriction: ProviderRestriction::All,
        origin: ToolOrigin::Builtin,
        search_hint: None,
    }
}

fn object_schema(required: &[&str], properties: Value) -> Value {
    json!({
        "type": "object",
        "required": required,
        "properties": properties
    })
}
