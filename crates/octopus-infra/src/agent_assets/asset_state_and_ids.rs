#[cfg(test)]
fn managed_skill_id(target: &AssetTargetScope<'_>, slug: &str) -> String {
    let display_path = match target {
        AssetTargetScope::Workspace => format!("data/skills/{slug}/{SKILL_FRONTMATTER_FILE}"),
        AssetTargetScope::Project(project_id) => {
            format!("data/projects/{project_id}/skills/{slug}/{SKILL_FRONTMATTER_FILE}")
        }
    };
    crate::catalog_hash_id("skill", &display_path)
}

pub(crate) fn apply_imported_asset_state(
    paths: &WorkspacePaths,
    target: &AssetTargetScope<'_>,
    plan: &BundlePlan,
) -> Result<(), AppError> {
    if plan.asset_state.is_empty() {
        return Ok(());
    }

    let mut document = load_workspace_asset_state_document(paths)?;
    let mut changed = false;

    for skill in &plan.skills {
        if skill.action == ImportAction::Failed {
            continue;
        }
        let Some(metadata) = plan.asset_state.skills.get(&skill.slug) else {
            continue;
        };
        let source_key = skill_source_key(
            &target
                .skill_root(paths)
                .join(&skill.slug)
                .join(SKILL_FRONTMATTER_FILE),
            &paths.root,
        );
        apply_asset_metadata(&mut document, &source_key, metadata);
        changed = true;
    }

    for mcp in &plan.mcps {
        if mcp.action == ImportAction::Failed || (!mcp.resolved && mcp.action != ImportAction::Skip)
        {
            continue;
        }
        let Some(metadata) = plan.asset_state.mcps.get(&mcp.server_name) else {
            continue;
        };
        let source_key = bundle_mcp_source_key(target, &mcp.server_name);
        apply_asset_metadata(&mut document, &source_key, metadata);
        changed = true;
    }

    if changed {
        save_workspace_asset_state_document(paths, &document)?;
    }

    Ok(())
}

fn apply_asset_metadata(
    document: &mut crate::resources_skills::WorkspaceCapabilityAssetStateDocument,
    source_key: &str,
    metadata: &WorkspaceCapabilityAssetMetadata,
) {
    if let Some(enabled) = metadata.enabled {
        set_workspace_asset_enabled(document, source_key, enabled);
    }
    if let Some(trusted) = metadata.trusted {
        set_workspace_asset_trusted(document, source_key, trusted);
    }
}

pub(crate) fn bundle_mcp_source_key(target: &AssetTargetScope<'_>, server_name: &str) -> String {
    match target {
        AssetTargetScope::Project(project_id) => format!("mcp:project:{project_id}:{server_name}"),
        AssetTargetScope::Workspace => format!("mcp:{server_name}"),
    }
}

pub(crate) fn deterministic_asset_id(
    prefix: &str,
    target: &AssetTargetScope<'_>,
    source_id: &str,
) -> String {
    format!(
        "{prefix}-{}",
        short_hash(&hash_text(&format!("{}:{source_id}", target.scope_label())))
    )
}

pub(crate) fn deterministic_descriptor_id(
    target: &AssetTargetScope<'_>,
    asset_kind: &str,
    source_id: &str,
) -> String {
    let prefix = match asset_kind {
        "plugin" => "plugin-asset",
        "workflow-template" => "workflow-template-asset",
        other => other,
    };
    deterministic_asset_id(prefix, target, source_id)
}

pub(crate) fn descriptor_storage_path(
    target: &AssetTargetScope<'_>,
    descriptor: &ParsedBundleDescriptor,
) -> String {
    let scope_segment = match target {
        AssetTargetScope::Workspace => String::from("workspace"),
        AssetTargetScope::Project(project_id) => format!("project/{project_id}"),
    };
    format!(
        "data/artifacts/bundle-assets/{scope_segment}/{}/{}/{}",
        descriptor.asset_kind,
        short_hash(&hash_text(&descriptor.source_id)),
        descriptor.source_path
    )
}

pub(crate) fn basename_from_source_id(source_id: &str) -> &str {
    source_id.rsplit('/').next().unwrap_or(source_id)
}

pub(crate) fn dependency_resolution_from_manifest(
    manifest: Option<&AssetBundleManifestV2>,
    issues: &[ImportIssue],
) -> Vec<AssetDependencyResolution> {
    let Some(manifest) = manifest else {
        return Vec::new();
    };
    let unresolved_dependency_refs = issues
        .iter()
        .filter_map(|issue| issue.dependency_ref.clone())
        .collect::<BTreeSet<_>>();
    manifest
        .dependencies
        .iter()
        .map(|dependency| AssetDependencyResolution {
            kind: dependency.kind.clone(),
            r#ref: dependency.r#ref.clone(),
            required: dependency.required,
            resolution_state: if unresolved_dependency_refs.contains(&dependency.r#ref) {
                "missing".into()
            } else {
                "resolved".into()
            },
            resolved_ref: if unresolved_dependency_refs.contains(&dependency.r#ref) {
                None
            } else {
                Some(dependency.r#ref.clone())
            },
        })
        .collect()
}

pub(crate) fn dependencies_from_resolution(
    dependency_resolution: &[AssetDependencyResolution],
) -> Vec<AssetDependency> {
    dependency_resolution
        .iter()
        .map(|dependency| AssetDependency {
            kind: dependency.kind.clone(),
            r#ref: dependency.r#ref.clone(),
            version_range: "*".into(),
            required: dependency.required,
        })
        .collect()
}

pub(crate) fn descriptor_record_matches(
    existing: &BundleAssetDescriptorRecord,
    candidate: &BundleAssetDescriptorRecord,
) -> bool {
    existing.workspace_id == candidate.workspace_id
        && existing.project_id == candidate.project_id
        && existing.scope == candidate.scope
        && existing.asset_kind == candidate.asset_kind
        && existing.source_id == candidate.source_id
        && existing.display_name == candidate.display_name
        && existing.source_path == candidate.source_path
        && existing.storage_path == candidate.storage_path
        && existing.content_hash == candidate.content_hash
        && existing.byte_size == candidate.byte_size
        && existing.manifest_revision == candidate.manifest_revision
        && existing.task_domains == candidate.task_domains
        && existing.translation_mode == candidate.translation_mode
        && existing.trust_metadata == candidate.trust_metadata
        && existing.dependency_resolution == candidate.dependency_resolution
        && existing.import_metadata.origin_kind == candidate.import_metadata.origin_kind
        && existing.import_metadata.source_id == candidate.import_metadata.source_id
        && existing.import_metadata.manifest_version == candidate.import_metadata.manifest_version
        && existing.import_metadata.translation_status
            == candidate.import_metadata.translation_status
}

pub(crate) fn issue(
    severity: &str,
    scope: &str,
    source_id: Option<String>,
    message: String,
) -> ImportIssue {
    ImportIssue {
        severity: severity.into(),
        scope: scope.into(),
        code: format!("{scope}-diagnostic"),
        stage: "translate".into(),
        source_id,
        source_path: None,
        dependency_ref: None,
        asset_kind: None,
        message,
        suggestion: None,
        details: None,
    }
}

pub(crate) fn model_strategy_from_template(model: Option<&str>) -> DefaultModelStrategy {
    let trimmed = model.map(str::trim).filter(|value| !value.is_empty());
    match trimmed {
        Some(model_ref) => DefaultModelStrategy {
            selection_mode: "actor-default".into(),
            preferred_model_ref: Some(model_ref.to_string()),
            fallback_model_refs: Vec::new(),
            allow_turn_override: true,
        },
        None => default_model_strategy(),
    }
}

fn model_ref_for_export(strategy: &DefaultModelStrategy) -> String {
    strategy
        .preferred_model_ref
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_default()
}

