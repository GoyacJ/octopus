use std::collections::HashMap;

use crate::{Model, ModelId, Provider, Surface};

mod builtin;
mod resolve;

pub use builtin::{
    builtin_canonical_model_id, builtin_catalog_version, builtin_compat_model,
    builtin_compat_models, builtin_default_routes, BuiltinCompatModel, BuiltinDefaultRoute,
};

#[derive(Debug, Clone)]
pub struct ModelCatalog {
    providers: Vec<Provider>,
    surfaces: Vec<Surface>,
    models: Vec<Model>,
    aliases: HashMap<String, ModelId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedModel {
    pub provider: Provider,
    pub surface: Surface,
    pub model: Model,
}

impl ModelCatalog {
    #[must_use]
    pub fn new_builtin() -> Self {
        let providers = builtin::all_providers();
        let surfaces = builtin::all_surfaces();
        let models = builtin::all_models();
        let aliases = builtin::all_aliases()
            .into_iter()
            .map(|(alias, model_id)| {
                (
                    resolve::normalize_lookup(alias),
                    ModelId(model_id.to_string()),
                )
            })
            .chain(
                models
                    .iter()
                    .map(|model| (resolve::normalize_lookup(&model.id.0), model.id.clone())),
            )
            .collect();

        Self {
            providers,
            surfaces,
            models,
            aliases,
        }
    }

    #[must_use]
    pub fn list_providers(&self) -> &[Provider] {
        &self.providers
    }

    #[must_use]
    pub fn list_models(&self) -> &[Model] {
        &self.models
    }

    #[must_use]
    pub fn list_surfaces(&self) -> &[Surface] {
        &self.surfaces
    }

    #[must_use]
    pub fn resolve(&self, reference: &str) -> Option<ResolvedModel> {
        resolve::resolve_reference(self, reference)
    }

    #[must_use]
    pub fn canonicalize(&self, id: &str) -> Option<ModelId> {
        resolve::canonicalize_reference(self, id)
    }

    pub(crate) fn aliases(&self) -> &HashMap<String, ModelId> {
        &self.aliases
    }

    pub(crate) fn model_by_id(&self, id: &ModelId) -> Option<&Model> {
        self.models.iter().find(|model| model.id == *id)
    }

    pub(crate) fn surface_by_id(&self, id: &crate::SurfaceId) -> Option<&Surface> {
        self.surfaces.iter().find(|surface| surface.id == *id)
    }

    pub(crate) fn provider_by_id(&self, id: &crate::ProviderId) -> Option<&Provider> {
        self.providers.iter().find(|provider| provider.id == *id)
    }

    #[cfg(test)]
    pub(crate) fn from_parts(
        providers: Vec<Provider>,
        surfaces: Vec<Surface>,
        models: Vec<Model>,
        aliases: HashMap<String, ModelId>,
    ) -> Self {
        Self {
            providers,
            surfaces,
            models,
            aliases,
        }
    }
}
