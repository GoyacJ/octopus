use crate::{Model, Provider, Surface};

mod anthropic;
mod ark;
mod bigmodel;
mod deepseek;
mod google;
mod minimax;
mod moonshot;
mod openai;
mod qwen;

#[must_use]
pub(crate) fn all_providers() -> Vec<Provider> {
    vec![
        anthropic::provider(),
        openai::provider(),
        google::provider(),
        deepseek::provider(),
        minimax::provider(),
        moonshot::provider(),
        bigmodel::provider(),
        qwen::provider(),
        ark::provider(),
    ]
}

#[must_use]
pub(crate) fn all_surfaces() -> Vec<Surface> {
    [
        anthropic::surfaces(),
        openai::surfaces(),
        google::surfaces(),
        deepseek::surfaces(),
        minimax::surfaces(),
        moonshot::surfaces(),
        bigmodel::surfaces(),
        qwen::surfaces(),
        ark::surfaces(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

#[must_use]
pub(crate) fn all_models() -> Vec<Model> {
    [
        anthropic::models(),
        openai::models(),
        google::models(),
        deepseek::models(),
        minimax::models(),
        moonshot::models(),
        bigmodel::models(),
        qwen::models(),
        ark::models(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

#[must_use]
pub(crate) fn all_aliases() -> Vec<(&'static str, &'static str)> {
    [
        anthropic::aliases(),
        openai::aliases(),
        google::aliases(),
        deepseek::aliases(),
        minimax::aliases(),
        moonshot::aliases(),
        bigmodel::aliases(),
        qwen::aliases(),
        ark::aliases(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
