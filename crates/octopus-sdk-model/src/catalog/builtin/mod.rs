use crate::{Model, ModelRole, Provider, Surface};

mod anthropic;
mod ark;
mod bigmodel;
mod deepseek;
mod google;
mod minimax;
mod moonshot;
mod openai;
mod qwen;

pub const BUILTIN_CATALOG_VERSION: &str = "builtin-2026-04-02";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinDefaultRoute {
    pub purpose: &'static str,
    pub role: Option<ModelRole>,
    pub candidates: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinCompatModel {
    pub provider: Provider,
    pub surface: Surface,
    pub model: Model,
}

const BUILTIN_DEFAULT_ROUTES: &[BuiltinDefaultRoute] = &[
    BuiltinDefaultRoute {
        purpose: "conversation",
        role: Some(ModelRole::Main),
        candidates: &["sonnet", "MiniMax-M2.7", "qwen3-coder-plus"],
    },
    BuiltinDefaultRoute {
        purpose: "fast",
        role: Some(ModelRole::Fast),
        candidates: &["haiku", "glm-5-turbo", "kimi-fast", "qwen-coder"],
    },
    BuiltinDefaultRoute {
        purpose: "best",
        role: Some(ModelRole::Best),
        candidates: &["opus", "MiniMax-M2.7", "sonnet"],
    },
    BuiltinDefaultRoute {
        purpose: "plan",
        role: Some(ModelRole::Plan),
        candidates: &["sonnet", "qwen-coder", "MiniMax-M2.7"],
    },
    BuiltinDefaultRoute {
        purpose: "compact",
        role: Some(ModelRole::Compact),
        candidates: &["haiku", "glm-turbo", "kimi-fast"],
    },
    BuiltinDefaultRoute {
        purpose: "vision",
        role: Some(ModelRole::Vision),
        candidates: &["qwen-vl", "qwen3-vl-plus"],
    },
    BuiltinDefaultRoute {
        purpose: "web_extract",
        role: Some(ModelRole::WebExtract),
        candidates: &["deepseek", "sonnet"],
    },
    BuiltinDefaultRoute {
        purpose: "eval",
        role: Some(ModelRole::Eval),
        candidates: &["opus", "sonnet"],
    },
    BuiltinDefaultRoute {
        purpose: "subagent_default",
        role: Some(ModelRole::SubagentDefault),
        candidates: &["sonnet", "qwen-coder", "haiku"],
    },
];

#[must_use]
pub(crate) fn all_providers() -> Vec<Provider> {
    vec![
        anthropic::provider(),
        deepseek::provider(),
        minimax::provider(),
        moonshot::provider(),
        bigmodel::provider(),
        qwen::provider(),
    ]
}

#[must_use]
pub(crate) fn all_surfaces() -> Vec<Surface> {
    [
        anthropic::surfaces(),
        deepseek::surfaces(),
        minimax::surfaces(),
        moonshot::surfaces(),
        bigmodel::surfaces(),
        qwen::surfaces(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

#[must_use]
pub(crate) fn all_models() -> Vec<Model> {
    [
        anthropic::models(),
        deepseek::models(),
        minimax::models(),
        moonshot::models(),
        bigmodel::models(),
        qwen::models(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

#[must_use]
pub(crate) fn all_aliases() -> Vec<(&'static str, &'static str)> {
    [
        anthropic::aliases(),
        deepseek::aliases(),
        minimax::aliases(),
        moonshot::aliases(),
        bigmodel::aliases(),
        qwen::aliases(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

#[must_use]
pub fn builtin_default_routes() -> &'static [BuiltinDefaultRoute] {
    BUILTIN_DEFAULT_ROUTES
}

#[must_use]
pub fn builtin_catalog_version() -> &'static str {
    BUILTIN_CATALOG_VERSION
}

#[must_use]
pub fn builtin_compat_models() -> Vec<BuiltinCompatModel> {
    compat_models()
}

#[must_use]
pub fn builtin_compat_model(model_id: &str) -> Option<BuiltinCompatModel> {
    compat_models()
        .into_iter()
        .find(|entry| entry.model.id.0 == model_id)
}

#[must_use]
pub fn builtin_canonical_model_id(model_id: &str) -> String {
    let normalized = normalize_lookup(model_id);
    all_builtin_aliases()
        .into_iter()
        .find(|(alias, _)| normalize_lookup(alias) == normalized)
        .map(|(_, canonical)| canonical.to_string())
        .unwrap_or_else(|| model_id.trim().to_string())
}

fn compat_models() -> Vec<BuiltinCompatModel> {
    [
        compat_from_provider(openai::provider(), openai::surfaces(), openai::models()),
        compat_from_provider(google::provider(), google::surfaces(), google::models()),
        compat_from_provider(ark::provider(), ark::surfaces(), ark::models()),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn compat_from_provider(
    provider: Provider,
    surfaces: Vec<Surface>,
    models: Vec<Model>,
) -> Vec<BuiltinCompatModel> {
    models
        .into_iter()
        .filter_map(|model| {
            let surface = surfaces
                .iter()
                .find(|surface| surface.id == model.surface)?
                .clone();
            Some(BuiltinCompatModel {
                provider: provider.clone(),
                surface,
                model,
            })
        })
        .collect()
}

fn all_builtin_aliases() -> Vec<(&'static str, &'static str)> {
    [
        all_aliases(),
        openai::aliases(),
        google::aliases(),
        ark::aliases(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn normalize_lookup(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}
