use std::collections::HashMap;

use crate::{ModelCatalog, ModelId, ModelRole};

#[derive(Debug, Clone, Default)]
pub struct RoleRouter {
    defaults: HashMap<ModelRole, ModelId>,
    overrides: HashMap<ModelRole, ModelId>,
}

impl RoleRouter {
    #[must_use]
    pub fn new_builtin(catalog: &ModelCatalog) -> Self {
        let mut defaults = HashMap::new();

        for (role, candidates) in [
            (ModelRole::Main, &["opus"][..]),
            (ModelRole::Fast, &["haiku", "gpt-5.4-mini", "gemini-2.5-flash"][..]),
            (ModelRole::Best, &["opus-1m", "opus", "gpt-5.4"][..]),
            (ModelRole::Plan, &["opus", "gpt-5.4"][..]),
            (
                ModelRole::Compact,
                &["gemini-2.5-flash", "haiku", "gpt-5.4-mini"][..],
            ),
        ] {
            if let Some(model_id) = resolve_first(catalog, candidates) {
                defaults.insert(role, model_id);
            }
        }

        Self {
            defaults,
            overrides: HashMap::new(),
        }
    }

    #[must_use]
    pub fn with_override(mut self, role: ModelRole, model: ModelId) -> Self {
        self.overrides.insert(role, model);
        self
    }

    #[must_use]
    pub fn resolve(&self, role: ModelRole) -> Option<ModelId> {
        self.overrides
            .get(&role)
            .cloned()
            .or_else(|| self.defaults.get(&role).cloned())
    }
}

fn resolve_first(catalog: &ModelCatalog, candidates: &[&str]) -> Option<ModelId> {
    candidates
        .iter()
        .find_map(|candidate| catalog.resolve(candidate).map(|resolved| resolved.model.id))
}
