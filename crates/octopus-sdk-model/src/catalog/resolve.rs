use crate::{ModelId, ModelTrack, ProviderStatus};

use super::{ModelCatalog, ResolvedModel};

pub(crate) fn normalize_lookup(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub(crate) fn canonicalize_reference(catalog: &ModelCatalog, id: &str) -> Option<ModelId> {
    let normalized = normalize_lookup(id);

    if let Some(model_id) = catalog.aliases().get(&normalized) {
        return Some(model_id.clone());
    }

    let stripped = strip_vendor_prefix(&normalized);
    if let Some(model_id) = catalog.aliases().get(&stripped) {
        return Some(model_id.clone());
    }

    let without_revision = strip_bedrock_revision(&stripped);
    catalog.aliases().get(&without_revision).cloned()
}

pub(crate) fn resolve_reference(catalog: &ModelCatalog, reference: &str) -> Option<ResolvedModel> {
    if let Some(model_id) = canonicalize_reference(catalog, reference) {
        return resolved_from_id(catalog, &model_id);
    }

    let normalized = normalize_lookup(reference);
    let model = catalog.list_models().iter().find(|model| {
        normalize_lookup(&model.family) == normalized
            && model.track == ModelTrack::Stable
            && catalog
                .surface_by_id(&model.surface)
                .and_then(|surface| catalog.provider_by_id(&surface.provider_id))
                .is_some_and(|provider| provider.status == ProviderStatus::Active)
    })?;

    resolved_from_id(catalog, &model.id)
}

fn resolved_from_id(catalog: &ModelCatalog, model_id: &ModelId) -> Option<ResolvedModel> {
    let model = catalog.model_by_id(model_id)?.clone();
    let surface = catalog.surface_by_id(&model.surface)?.clone();
    let provider = catalog.provider_by_id(&surface.provider_id)?.clone();

    Some(ResolvedModel {
        provider,
        surface,
        model,
    })
}

fn strip_vendor_prefix(value: &str) -> String {
    value.rsplit('.').next().unwrap_or(value).to_string()
}

fn strip_bedrock_revision(value: &str) -> String {
    let trimmed = value.split(':').next().unwrap_or(value);

    if let Some(index) = trimmed.rfind("-v") {
        let suffix = &trimmed[index + 2..];
        if !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()) {
            return trimmed[..index].to_string();
        }
    }

    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use crate::{ModelCatalog, ProtocolFamily};

    #[test]
    fn resolve_alias_and_canonical_model_to_same_target() {
        let catalog = ModelCatalog::new_builtin();
        let alias = catalog.resolve("opus").unwrap();
        let canonical = catalog.resolve("claude-opus-4-6").unwrap();

        assert_eq!(alias.model.id, canonical.model.id);
        assert_eq!(alias.surface.protocol, ProtocolFamily::AnthropicMessages);
    }

    #[test]
    fn canonicalize_bedrock_revision_suffix() {
        let catalog = ModelCatalog::new_builtin();

        assert_eq!(
            catalog
                .canonicalize("anthropic.claude-opus-4-6-v1:0")
                .unwrap()
                .0,
            "claude-opus-4-6"
        );
        assert!(catalog.resolve("unknown/xxx").is_none());
    }
}
