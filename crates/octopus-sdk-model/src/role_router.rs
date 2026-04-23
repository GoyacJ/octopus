use std::collections::HashMap;

use crate::{builtin_default_routes, ModelCatalog, ModelId, ModelRole};

#[derive(Debug, Clone, Default)]
pub struct RoleRouter {
    defaults: HashMap<ModelRole, ModelId>,
    overrides: HashMap<ModelRole, ModelId>,
}

impl RoleRouter {
    #[must_use]
    pub fn new_builtin(catalog: &ModelCatalog) -> Self {
        let mut defaults = HashMap::new();

        for route in builtin_default_routes() {
            let Some(role) = route.role.clone() else {
                continue;
            };
            let candidates = route.candidates;
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
