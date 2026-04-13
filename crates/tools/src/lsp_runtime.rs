use super::*;

fn global_lsp_registry() -> &'static LspRegistry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<LspRegistry> = OnceLock::new();
    REGISTRY.get_or_init(LspRegistry::new)
}

#[derive(Debug, Deserialize)]
pub(crate) struct LspInput {
    pub(crate) action: String,
    #[serde(default)]
    pub(crate) path: Option<String>,
    #[serde(default)]
    pub(crate) line: Option<u32>,
    #[serde(default)]
    pub(crate) character: Option<u32>,
    #[serde(default)]
    pub(crate) query: Option<String>,
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_lsp(input: LspInput) -> Result<String, String> {
    let registry = global_lsp_registry();
    let action = &input.action;
    let path = input.path.as_deref();
    let line = input.line;
    let character = input.character;
    let query = input.query.as_deref();

    match registry.dispatch(action, path, line, character, query) {
        Ok(result) => to_pretty_json(result),
        Err(error) => to_pretty_json(json!({
            "action": action,
            "error": error,
            "status": "error"
        })),
    }
}
