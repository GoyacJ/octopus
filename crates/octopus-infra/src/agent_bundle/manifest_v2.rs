use octopus_core::{
    default_agent_delegation_policy, default_agent_memory_policy, default_approval_preference,
    default_asset_trust_metadata, default_model_strategy, default_permission_envelope,
    normalize_task_domains, AgentRecord, AssetBundleAssetEntry, AssetBundleCompatibilityMapping,
    AssetBundleManifestV2, AssetBundlePolicyDefaults, AssetTranslationReport,
    BundleAssetDescriptorRecord, TeamRecord, ASSET_IMPORT_MANIFEST_VERSION,
};

use crate::agent_assets::{self, BundlePlan};

pub(crate) fn bundle_manifest_from_plan(plan: &BundlePlan) -> AssetBundleManifestV2 {
    let manifest_template = plan.bundle_manifest_template.as_ref();
    AssetBundleManifestV2 {
        version: manifest_template
            .map(|manifest| manifest.version)
            .unwrap_or(2),
        bundle_kind: manifest_template
            .map(|manifest| manifest.bundle_kind.clone())
            .unwrap_or_else(|| "octopus-asset-bundle".into()),
        bundle_root: manifest_template
            .map(|manifest| manifest.bundle_root.clone())
            .unwrap_or_else(|| ".".into()),
        assets: plan_asset_entries(plan),
        dependencies: agent_assets::dependencies_from_resolution(&plan.dependency_resolution),
        trust_metadata: manifest_template
            .map(|manifest| manifest.trust_metadata.clone())
            .unwrap_or_else(default_asset_trust_metadata),
        compatibility_mapping: manifest_template
            .map(|manifest| manifest.compatibility_mapping.clone())
            .unwrap_or_else(|| AssetBundleCompatibilityMapping {
                supported_targets: vec!["octopus".into()],
                downgraded_features: Vec::new(),
                rejected_features: Vec::new(),
                translator_version: "phase-1".into(),
            }),
        policy_defaults: manifest_template
            .map(|manifest| manifest.policy_defaults.clone())
            .unwrap_or_else(|| AssetBundlePolicyDefaults {
                default_model_strategy: default_model_strategy(),
                permission_envelope: default_permission_envelope(),
                memory_policy: default_agent_memory_policy(),
                delegation_policy: default_agent_delegation_policy(),
                approval_preference: default_approval_preference(),
            }),
        registry_metadata: manifest_template
            .and_then(|manifest| manifest.registry_metadata.clone()),
    }
}

pub(crate) fn build_export_bundle_manifest(
    agents: &[AgentRecord],
    teams: &[TeamRecord],
    descriptors: &[BundleAssetDescriptorRecord],
) -> AssetBundleManifestV2 {
    let mut assets = agents
        .iter()
        .map(|agent| AssetBundleAssetEntry {
            asset_kind: "agent".into(),
            source_id: agent
                .import_metadata
                .source_id
                .clone()
                .unwrap_or_else(|| agent.id.clone()),
            display_name: agent.name.clone(),
            source_path: format!(
                "{}/{}.md",
                agent
                    .import_metadata
                    .source_id
                    .clone()
                    .unwrap_or_else(|| agent.id.clone()),
                agent_assets::basename_from_source_id(
                    agent
                        .import_metadata
                        .source_id
                        .as_deref()
                        .unwrap_or(agent.id.as_str())
                )
            ),
            manifest_revision: agent.manifest_revision.clone(),
            task_domains: agent.task_domains.clone(),
            translation_mode: agent.import_metadata.translation_status.clone(),
        })
        .chain(teams.iter().map(|team| {
            AssetBundleAssetEntry {
                asset_kind: "team".into(),
                source_id: team
                    .import_metadata
                    .source_id
                    .clone()
                    .unwrap_or_else(|| team.id.clone()),
                display_name: team.name.clone(),
                source_path: format!(
                    "{}/{}.md",
                    team.import_metadata
                        .source_id
                        .clone()
                        .unwrap_or_else(|| team.id.clone()),
                    agent_assets::basename_from_source_id(
                        team.import_metadata
                            .source_id
                            .as_deref()
                            .unwrap_or(team.id.as_str())
                    )
                ),
                manifest_revision: team.manifest_revision.clone(),
                task_domains: team.task_domains.clone(),
                translation_mode: team.import_metadata.translation_status.clone(),
            }
        }))
        .collect::<Vec<_>>();
    assets.extend(descriptors.iter().map(|descriptor| AssetBundleAssetEntry {
        asset_kind: descriptor.asset_kind.clone(),
        source_id: descriptor.source_id.clone(),
        display_name: descriptor.display_name.clone(),
        source_path: descriptor.source_path.clone(),
        manifest_revision: descriptor.manifest_revision.clone(),
        task_domains: descriptor.task_domains.clone(),
        translation_mode: descriptor.translation_mode.clone(),
    }));

    let trust_metadata = agents
        .first()
        .map(|agent| agent.trust_metadata.clone())
        .or_else(|| teams.first().map(|team| team.trust_metadata.clone()))
        .or_else(|| {
            descriptors
                .first()
                .map(|descriptor| descriptor.trust_metadata.clone())
        })
        .unwrap_or_else(default_asset_trust_metadata);

    let dependency_resolution = agents
        .first()
        .map(|agent| agent.dependency_resolution.clone())
        .or_else(|| teams.first().map(|team| team.dependency_resolution.clone()))
        .or_else(|| {
            descriptors
                .first()
                .map(|descriptor| descriptor.dependency_resolution.clone())
        })
        .unwrap_or_default();

    AssetBundleManifestV2 {
        version: ASSET_IMPORT_MANIFEST_VERSION,
        bundle_kind: "octopus-asset-bundle".into(),
        bundle_root: ".".into(),
        assets,
        dependencies: agent_assets::dependencies_from_resolution(&dependency_resolution),
        trust_metadata,
        compatibility_mapping: AssetBundleCompatibilityMapping {
            supported_targets: vec!["octopus".into()],
            downgraded_features: Vec::new(),
            rejected_features: Vec::new(),
            translator_version: "phase-1".into(),
        },
        policy_defaults: AssetBundlePolicyDefaults {
            default_model_strategy: default_model_strategy(),
            permission_envelope: default_permission_envelope(),
            memory_policy: default_agent_memory_policy(),
            delegation_policy: default_agent_delegation_policy(),
            approval_preference: default_approval_preference(),
        },
        registry_metadata: None,
    }
}

pub(crate) fn build_export_translation_report(
    agents: &[AgentRecord],
    teams: &[TeamRecord],
    descriptors: &[BundleAssetDescriptorRecord],
    bundle_manifest: &AssetBundleManifestV2,
) -> AssetTranslationReport {
    let mut translated_count = 0;
    let mut downgraded_count = 0;
    let mut rejected_count = 0;
    for asset in &bundle_manifest.assets {
        increment_translation_mode_counts(
            &asset.translation_mode,
            &mut translated_count,
            &mut downgraded_count,
            &mut rejected_count,
        );
    }
    let dependency_resolution = agents
        .first()
        .map(|agent| agent.dependency_resolution.clone())
        .or_else(|| teams.first().map(|team| team.dependency_resolution.clone()))
        .or_else(|| {
            descriptors
                .first()
                .map(|descriptor| descriptor.dependency_resolution.clone())
        })
        .unwrap_or_default();
    let status = if rejected_count > 0 {
        "rejected"
    } else if downgraded_count > 0 {
        "downgraded"
    } else if translated_count > 0 {
        "translated"
    } else {
        "native"
    };
    AssetTranslationReport {
        status: status.into(),
        translated_count,
        downgraded_count,
        rejected_count,
        unsupported_features: Vec::new(),
        trust_warnings: bundle_manifest.trust_metadata.trust_warnings.clone(),
        dependency_resolution,
        diagnostics: Vec::new(),
    }
}

fn increment_translation_mode_counts(
    translation_mode: &str,
    translated_count: &mut u64,
    downgraded_count: &mut u64,
    rejected_count: &mut u64,
) {
    match translation_mode {
        "translated" => *translated_count += 1,
        "downgraded" => *downgraded_count += 1,
        "reject" | "rejected" => *rejected_count += 1,
        _ => {}
    }
}

fn plan_asset_entries(plan: &BundlePlan) -> Vec<AssetBundleAssetEntry> {
    let mut entries = Vec::new();
    for agent in &plan.agents {
        entries.push(AssetBundleAssetEntry {
            asset_kind: "agent".into(),
            source_id: agent.source_id.clone(),
            display_name: agent.name.clone(),
            source_path: format!(
                "{}/{}.md",
                agent.source_id,
                agent_assets::basename_from_source_id(&agent.source_id)
            ),
            manifest_revision: octopus_core::ASSET_MANIFEST_REVISION_V2.into(),
            task_domains: normalize_task_domains(agent.tags.clone()),
            translation_mode: super::translation::import_action_translation_mode(agent.action)
                .into(),
        });
    }
    for team in &plan.teams {
        entries.push(AssetBundleAssetEntry {
            asset_kind: "team".into(),
            source_id: team.source_id.clone(),
            display_name: team.name.clone(),
            source_path: format!(
                "{}/{}.md",
                team.source_id,
                agent_assets::basename_from_source_id(&team.source_id)
            ),
            manifest_revision: octopus_core::ASSET_MANIFEST_REVISION_V2.into(),
            task_domains: normalize_task_domains(team.tags.clone()),
            translation_mode: super::translation::import_action_translation_mode(team.action)
                .into(),
        });
    }
    for skill in &plan.skills {
        entries.push(AssetBundleAssetEntry {
            asset_kind: "skill".into(),
            source_id: skill
                .source_ids
                .first()
                .cloned()
                .unwrap_or_else(|| skill.slug.clone()),
            display_name: skill.name.clone(),
            source_path: format!(
                "{}/{}",
                skill
                    .source_ids
                    .first()
                    .cloned()
                    .unwrap_or_else(|| format!("skills/{}", skill.slug)),
                agent_assets::SKILL_FRONTMATTER_FILE
            ),
            manifest_revision: octopus_core::ASSET_MANIFEST_REVISION_V2.into(),
            task_domains: Vec::new(),
            translation_mode: super::translation::import_action_translation_mode(skill.action)
                .into(),
        });
    }
    for mcp in &plan.mcps {
        entries.push(AssetBundleAssetEntry {
            asset_kind: "mcp-server".into(),
            source_id: mcp
                .source_ids
                .first()
                .cloned()
                .unwrap_or_else(|| mcp.server_name.clone()),
            display_name: mcp.server_name.clone(),
            source_path: format!(
                "{}.json",
                mcp.source_ids
                    .first()
                    .cloned()
                    .unwrap_or_else(|| format!("mcps/{}", mcp.server_name))
            ),
            manifest_revision: octopus_core::ASSET_MANIFEST_REVISION_V2.into(),
            task_domains: Vec::new(),
            translation_mode: super::translation::import_action_translation_mode(mcp.action).into(),
        });
    }
    for descriptor in &plan.descriptor_assets {
        entries.push(AssetBundleAssetEntry {
            asset_kind: descriptor.record.asset_kind.clone(),
            source_id: descriptor.record.source_id.clone(),
            display_name: descriptor.record.display_name.clone(),
            source_path: descriptor.record.source_path.clone(),
            manifest_revision: descriptor.record.manifest_revision.clone(),
            task_domains: descriptor.record.task_domains.clone(),
            translation_mode: match descriptor.action {
                agent_assets::ImportAction::Create
                | agent_assets::ImportAction::Update
                | agent_assets::ImportAction::Skip => descriptor.record.translation_mode.clone(),
                agent_assets::ImportAction::Failed => "reject".into(),
            },
        });
    }
    entries
}

#[cfg(test)]
mod tests {
    use octopus_core::{
        default_agent_delegation_policy, default_agent_memory_policy,
        default_approval_preference, default_asset_trust_metadata, default_model_strategy,
        default_permission_envelope, AssetBundleAssetEntry, AssetBundleCompatibilityMapping,
        AssetBundleManifestV2, AssetBundlePolicyDefaults,
    };

    use super::build_export_translation_report;

    #[test]
    fn export_translation_report_counts_asset_translation_modes() {
        let mut trust_metadata = default_asset_trust_metadata();
        trust_metadata.trust_warnings = vec!["unsigned bundle".into()];
        let bundle_manifest = AssetBundleManifestV2 {
            version: 2,
            bundle_kind: "octopus-asset-bundle".into(),
            bundle_root: ".".into(),
            assets: vec![
                asset_entry("native-agent", "native"),
                asset_entry("translated-agent", "translated"),
                asset_entry("downgraded-agent", "downgraded"),
                asset_entry("rejected-agent", "rejected"),
            ],
            dependencies: Vec::new(),
            trust_metadata,
            compatibility_mapping: AssetBundleCompatibilityMapping {
                supported_targets: vec!["octopus".into()],
                downgraded_features: Vec::new(),
                rejected_features: Vec::new(),
                translator_version: "phase-1".into(),
            },
            policy_defaults: AssetBundlePolicyDefaults {
                default_model_strategy: default_model_strategy(),
                permission_envelope: default_permission_envelope(),
                memory_policy: default_agent_memory_policy(),
                delegation_policy: default_agent_delegation_policy(),
                approval_preference: default_approval_preference(),
            },
            registry_metadata: None,
        };

        let report = build_export_translation_report(&[], &[], &[], &bundle_manifest);

        assert_eq!(report.translated_count, 1);
        assert_eq!(report.downgraded_count, 1);
        assert_eq!(report.rejected_count, 1);
        assert_eq!(report.status, "rejected");
        assert_eq!(report.trust_warnings, vec!["unsigned bundle"]);
    }

    fn asset_entry(source_id: &str, translation_mode: &str) -> AssetBundleAssetEntry {
        AssetBundleAssetEntry {
            asset_kind: "agent".into(),
            source_id: source_id.into(),
            display_name: source_id.into(),
            source_path: format!("{source_id}.md"),
            manifest_revision: octopus_core::ASSET_MANIFEST_REVISION_V2.into(),
            task_domains: Vec::new(),
            translation_mode: translation_mode.into(),
        }
    }
}
