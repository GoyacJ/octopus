use super::*;

fn infer_resource_preview_kind(
    kind: &str,
    name: &str,
    location: Option<&str>,
    content_type: Option<&str>,
) -> String {
    if kind == "folder" {
        return "folder".into();
    }
    if kind == "url" {
        return "url".into();
    }

    let content_type = content_type.unwrap_or_default().to_ascii_lowercase();
    if content_type.starts_with("image/") {
        return "image".into();
    }
    if content_type == "application/pdf" {
        return "pdf".into();
    }
    if content_type.starts_with("audio/") {
        return "audio".into();
    }
    if content_type.starts_with("video/") {
        return "video".into();
    }
    if content_type == "text/markdown" {
        return "markdown".into();
    }
    if content_type.starts_with("text/") || content_type == "application/json" {
        let extension = Path::new(name)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .or_else(|| {
                location.and_then(|value| {
                    Path::new(value)
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .map(|extension| extension.to_ascii_lowercase())
                })
            });
        if matches!(
            extension.as_deref(),
            Some(
                "rs" | "ts"
                    | "tsx"
                    | "js"
                    | "jsx"
                    | "vue"
                    | "py"
                    | "go"
                    | "java"
                    | "kt"
                    | "swift"
                    | "c"
                    | "cc"
                    | "cpp"
                    | "h"
                    | "hpp"
                    | "html"
                    | "css"
                    | "json"
                    | "yaml"
                    | "yml"
                    | "toml"
                    | "md"
                    | "sql"
                    | "sh"
            )
        ) {
            return if extension.as_deref() == Some("md") {
                "markdown".into()
            } else {
                "code".into()
            };
        }
        return if content_type == "text/markdown" {
            "markdown".into()
        } else {
            "text".into()
        };
    }

    let lower_name = name.to_ascii_lowercase();
    if lower_name.ends_with(".md") {
        return "markdown".into();
    }
    if lower_name.ends_with(".pdf") {
        return "pdf".into();
    }
    if matches!(
        lower_name.rsplit('.').next(),
        Some("png" | "jpg" | "jpeg" | "webp" | "gif" | "svg")
    ) {
        return "image".into();
    }
    if matches!(
        lower_name.rsplit('.').next(),
        Some("mp3" | "wav" | "ogg" | "m4a")
    ) {
        return "audio".into();
    }
    if matches!(
        lower_name.rsplit('.').next(),
        Some("mp4" | "mov" | "webm" | "avi" | "mkv")
    ) {
        return "video".into();
    }
    if matches!(
        lower_name.rsplit('.').next(),
        Some(
            "rs" | "ts"
                | "tsx"
                | "js"
                | "jsx"
                | "vue"
                | "py"
                | "go"
                | "java"
                | "kt"
                | "swift"
                | "c"
                | "cc"
                | "cpp"
                | "h"
                | "hpp"
                | "html"
                | "css"
                | "json"
                | "yaml"
                | "yml"
                | "toml"
                | "sql"
                | "sh"
        )
    ) {
        return "code".into();
    }

    "binary".into()
}

fn infer_resource_content_type(name: &str, location: Option<&str>) -> Option<String> {
    let extension = Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .or_else(|| {
            location.and_then(|value| {
                Path::new(value)
                    .extension()
                    .and_then(|extension| extension.to_str())
            })
        })?
        .to_ascii_lowercase();

    let content_type = match extension.as_str() {
        "md" => "text/markdown",
        "txt" | "csv" | "rs" | "ts" | "tsx" | "js" | "jsx" | "vue" | "py" | "go" | "java"
        | "kt" | "swift" | "c" | "cc" | "cpp" | "h" | "hpp" | "html" | "css" | "yaml" | "yml"
        | "toml" | "sql" | "sh" => "text/plain",
        "json" => "application/json",
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        _ => "application/octet-stream",
    };

    Some(content_type.into())
}

pub(crate) fn load_resources(
    connection: &Connection,
) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, kind, name, location, origin, scope, visibility, owner_user_id, storage_path, content_type, byte_size, preview_kind, status, updated_at, tags, source_artifact_id FROM resources",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let kind: String = row.get(3)?;
            let name: String = row.get(4)?;
            let location: Option<String> = row.get(5)?;
            let content_type = row
                .get::<_, Option<String>>(11)?
                .or_else(|| infer_resource_content_type(&name, location.as_deref()));
            let preview_kind = row.get::<_, Option<String>>(13)?.unwrap_or_else(|| {
                infer_resource_preview_kind(
                    &kind,
                    &name,
                    location.as_deref(),
                    content_type.as_deref(),
                )
            });
            let tags_raw: String = row.get(16)?;
            Ok(WorkspaceResourceRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                kind: kind.clone(),
                name: name.clone(),
                location,
                origin: row.get(6)?,
                scope: row
                    .get::<_, Option<String>>(7)?
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| {
                        if row.get::<_, Option<String>>(2).ok().flatten().is_some() {
                            "project".into()
                        } else {
                            "workspace".into()
                        }
                    }),
                visibility: row
                    .get::<_, Option<String>>(8)?
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| "public".into()),
                owner_user_id: row
                    .get::<_, Option<String>>(9)?
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| "user-owner".into()),
                storage_path: row.get(10)?,
                content_type,
                byte_size: row.get::<_, Option<i64>>(12)?.map(|value| value as u64),
                preview_kind,
                status: row.get(14)?,
                updated_at: row.get::<_, i64>(15)? as u64,
                tags: serde_json::from_str(&tags_raw).unwrap_or_default(),
                source_artifact_id: row.get(17)?,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_knowledge_records(
    connection: &Connection,
) -> Result<Vec<KnowledgeRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT
                id,
                workspace_id,
                project_id,
                title,
                summary,
                kind,
                COALESCE(scope, CASE WHEN project_id IS NULL THEN 'workspace' ELSE 'project' END) AS scope,
                status,
                COALESCE(
                    visibility,
                    CASE
                        WHEN COALESCE(scope, CASE WHEN project_id IS NULL THEN 'workspace' ELSE 'project' END) = 'personal'
                            THEN 'private'
                        ELSE 'public'
                    END
                ) AS visibility,
                owner_user_id,
                source_type,
                source_ref,
                updated_at
             FROM knowledge_records",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(KnowledgeRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                title: row.get(3)?,
                summary: row.get(4)?,
                kind: row.get(5)?,
                scope: row.get(6)?,
                status: row.get(7)?,
                visibility: row.get(8)?,
                owner_user_id: row.get(9)?,
                source_type: row.get(10)?,
                source_ref: row.get(11)?,
                updated_at: row.get::<_, i64>(12)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_artifact_records(
    connection: &Connection,
) -> Result<Vec<ArtifactRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, conversation_id, title, status, preview_kind,
                    latest_version, promotion_state, updated_at, content_type
             FROM artifact_records
             ORDER BY updated_at DESC, id ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let id = row.get::<_, String>(0)?;
            let title = row.get::<_, String>(4)?;
            let preview_kind = row.get::<_, String>(6)?;
            let latest_version = row.get::<_, i64>(7)?.max(0) as u32;
            let updated_at = row.get::<_, i64>(9)?.max(0) as u64;
            let content_type = row.get::<_, Option<String>>(10)?;
            Ok(ArtifactRecord {
                id: id.clone(),
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                conversation_id: row.get(3)?,
                title: title.clone(),
                status: row.get(5)?,
                preview_kind: preview_kind.clone(),
                latest_version,
                latest_version_ref: ArtifactVersionReference {
                    artifact_id: id,
                    version: latest_version,
                    title,
                    preview_kind,
                    updated_at,
                    content_type: content_type.clone(),
                },
                promotion_state: row.get(8)?,
                updated_at,
                content_type,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_project_artifact_records(
    connection: &Connection,
    project_id: &str,
) -> Result<Vec<ArtifactRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT id, workspace_id, project_id, conversation_id, title, status, preview_kind,
                    latest_version, promotion_state, updated_at, content_type
             FROM artifact_records
             WHERE project_id = ?1
             ORDER BY updated_at DESC, id ASC",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([project_id], |row| {
            let id = row.get::<_, String>(0)?;
            let title = row.get::<_, String>(4)?;
            let preview_kind = row.get::<_, String>(6)?;
            let latest_version = row.get::<_, i64>(7)?.max(0) as u32;
            let updated_at = row.get::<_, i64>(9)?.max(0) as u64;
            let content_type = row.get::<_, Option<String>>(10)?;
            Ok(ArtifactRecord {
                id: id.clone(),
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                conversation_id: row.get(3)?,
                title: title.clone(),
                status: row.get(5)?,
                preview_kind: preview_kind.clone(),
                latest_version,
                latest_version_ref: ArtifactVersionReference {
                    artifact_id: id,
                    version: latest_version,
                    title,
                    preview_kind,
                    updated_at,
                    content_type: content_type.clone(),
                },
                promotion_state: row.get(8)?,
                updated_at,
                content_type,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn agent_avatar(paths: &WorkspacePaths, avatar_path: Option<&str>) -> Option<String> {
    let avatar_path = avatar_path?;
    let absolute_path = paths.root.join(avatar_path);
    let bytes = fs::read(&absolute_path).ok()?;
    let content_type = match absolute_path
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        _ => return Some(avatar_path.to_string()),
    };
    Some(format!(
        "data:{content_type};base64,{}",
        BASE64_STANDARD.encode(bytes)
    ))
}

pub(crate) fn load_agents(connection: &Connection) -> Result<Vec<AgentRecord>, AppError> {
    let workspace_root = connection
        .path()
        .map(Path::new)
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .map(Path::to_path_buf)
        .ok_or_else(|| AppError::database("could not resolve workspace root"))?;
    let paths = WorkspacePaths::new(workspace_root);
    let mut stmt = connection
        .prepare(
            "SELECT
                id, workspace_id, project_id, scope, owner_user_id, asset_role, name, avatar_path, personality, tags, prompt,
                builtin_tool_keys, skill_ids, mcp_server_names, task_domains, manifest_revision,
                default_model_strategy_json, capability_policy_json, permission_envelope_json,
                memory_policy_json, delegation_policy_json, approval_preference_json,
                output_contract_json, shared_capability_policy_json, integration_source_json,
                trust_metadata_json, dependency_resolution_json, import_metadata_json,
                description, status, updated_at
             FROM agents",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let avatar_path: Option<String> = row.get(7)?;
            let avatar = agent_avatar(&paths, avatar_path.as_deref());
            let tags_raw: String = row.get(9)?;
            let builtin_tool_keys_raw: String = row.get(11)?;
            let skill_ids_raw: String = row.get(12)?;
            let mcp_server_names_raw: String = row.get(13)?;
            let task_domains_raw: String = row.get(14)?;
            let builtin_tool_keys: Vec<String> =
                serde_json::from_str(&builtin_tool_keys_raw).unwrap_or_default();
            let skill_ids: Vec<String> = serde_json::from_str(&skill_ids_raw).unwrap_or_default();
            let mcp_server_names: Vec<String> =
                serde_json::from_str(&mcp_server_names_raw).unwrap_or_default();
            let default_model_strategy_raw: String = row.get(16)?;
            let capability_policy_raw: String = row.get(17)?;
            let permission_envelope_raw: String = row.get(18)?;
            let memory_policy_raw: String = row.get(19)?;
            let delegation_policy_raw: String = row.get(20)?;
            let approval_preference_raw: String = row.get(21)?;
            let output_contract_raw: String = row.get(22)?;
            let shared_capability_policy_raw: String = row.get(23)?;
            let integration_source_raw: Option<String> = row.get(24)?;
            let trust_metadata_raw: String = row.get(25)?;
            let dependency_resolution_raw: String = row.get(26)?;
            let import_metadata_raw: String = row.get(27)?;
            Ok(AgentRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                scope: row.get(3)?,
                owner_user_id: row.get(4)?,
                asset_role: row
                    .get::<_, Option<String>>(5)?
                    .unwrap_or_else(octopus_core::default_agent_asset_role),
                name: row.get(6)?,
                avatar_path,
                avatar,
                personality: row.get(8)?,
                tags: serde_json::from_str(&tags_raw).unwrap_or_default(),
                prompt: row.get(10)?,
                builtin_tool_keys: builtin_tool_keys.clone(),
                skill_ids: skill_ids.clone(),
                mcp_server_names: mcp_server_names.clone(),
                task_domains: parse_json_or_default(&task_domains_raw, || {
                    normalize_task_domains(Vec::new())
                }),
                manifest_revision: row.get(15)?,
                default_model_strategy: parse_json_or_default(
                    &default_model_strategy_raw,
                    default_model_strategy,
                ),
                capability_policy: parse_json_or_default(&capability_policy_raw, || {
                    capability_policy_from_sources(
                        &builtin_tool_keys,
                        &skill_ids,
                        &mcp_server_names,
                    )
                }),
                permission_envelope: parse_json_or_default(
                    &permission_envelope_raw,
                    default_permission_envelope,
                ),
                memory_policy: parse_json_or_default(
                    &memory_policy_raw,
                    default_agent_memory_policy,
                ),
                delegation_policy: parse_json_or_default(
                    &delegation_policy_raw,
                    default_agent_delegation_policy,
                ),
                approval_preference: parse_json_or_default(
                    &approval_preference_raw,
                    default_approval_preference,
                ),
                output_contract: parse_json_or_default(
                    &output_contract_raw,
                    default_output_contract,
                ),
                shared_capability_policy: parse_json_or_default(
                    &shared_capability_policy_raw,
                    default_agent_shared_capability_policy,
                ),
                integration_source: integration_source_raw
                    .as_deref()
                    .and_then(|value| serde_json::from_str(value).ok()),
                trust_metadata: parse_json_or_default(
                    &trust_metadata_raw,
                    default_asset_trust_metadata,
                ),
                dependency_resolution: parse_json_or_default(&dependency_resolution_raw, Vec::new),
                import_metadata: parse_json_or_default(
                    &import_metadata_raw,
                    default_asset_import_metadata,
                ),
                description: row.get(28)?,
                status: row.get(29)?,
                updated_at: row.get::<_, i64>(30)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_project_agent_links(
    connection: &Connection,
) -> Result<Vec<ProjectAgentLinkRecord>, AppError> {
    let mut stmt = connection
        .prepare("SELECT workspace_id, project_id, agent_id, linked_at FROM project_agent_links")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectAgentLinkRecord {
                workspace_id: row.get(0)?,
                project_id: row.get(1)?,
                agent_id: row.get(2)?,
                linked_at: row.get::<_, i64>(3)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_bundle_asset_descriptor_records(
    connection: &Connection,
) -> Result<Vec<BundleAssetDescriptorRecord>, AppError> {
    let mut stmt = connection
        .prepare(
            "SELECT
                id, workspace_id, project_id, scope, asset_kind, source_id, display_name,
                source_path, storage_path, content_hash, byte_size, manifest_revision,
                task_domains_json, translation_mode, trust_metadata_json,
                dependency_resolution_json, import_metadata_json, updated_at
             FROM bundle_asset_descriptors",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let task_domains_raw: String = row.get(12)?;
            let trust_metadata_raw: String = row.get(14)?;
            let dependency_resolution_raw: String = row.get(15)?;
            let import_metadata_raw: String = row.get(16)?;
            Ok(BundleAssetDescriptorRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                scope: row.get(3)?,
                asset_kind: row.get(4)?,
                source_id: row.get(5)?,
                display_name: row.get(6)?,
                source_path: row.get(7)?,
                storage_path: row.get(8)?,
                content_hash: row.get(9)?,
                byte_size: row.get::<_, i64>(10)? as u64,
                manifest_revision: row.get(11)?,
                task_domains: parse_json_or_default(&task_domains_raw, Vec::new),
                translation_mode: row.get(13)?,
                trust_metadata: parse_json_or_default(
                    &trust_metadata_raw,
                    default_asset_trust_metadata,
                ),
                dependency_resolution: parse_json_or_default(&dependency_resolution_raw, Vec::new),
                import_metadata: parse_json_or_default(
                    &import_metadata_raw,
                    default_asset_import_metadata,
                ),
                updated_at: row.get::<_, i64>(17)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_teams(connection: &Connection) -> Result<Vec<TeamRecord>, AppError> {
    let workspace_root = connection
        .path()
        .map(Path::new)
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .map(Path::to_path_buf)
        .ok_or_else(|| AppError::database("could not resolve workspace root"))?;
    let paths = WorkspacePaths::new(workspace_root);
    let mut stmt = connection
        .prepare(
            "SELECT
                id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt,
                builtin_tool_keys, skill_ids, mcp_server_names, task_domains, manifest_revision,
                default_model_strategy_json, capability_policy_json, permission_envelope_json,
                memory_policy_json, delegation_policy_json, approval_preference_json,
                output_contract_json, shared_capability_policy_json, leader_ref, member_refs,
                team_topology_json,
                shared_memory_policy_json, mailbox_policy_json, artifact_handoff_policy_json,
                workflow_affordance_json, worker_concurrency_limit, integration_source_json,
                trust_metadata_json, dependency_resolution_json, import_metadata_json,
                description, status, updated_at
             FROM teams",
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            let avatar_path: Option<String> = row.get(5)?;
            let avatar = agent_avatar(&paths, avatar_path.as_deref());
            let tags_raw: String = row.get(7)?;
            let builtin_tool_keys_raw: String = row.get(9)?;
            let skill_ids_raw: String = row.get(10)?;
            let mcp_server_names_raw: String = row.get(11)?;
            let task_domains_raw: String = row.get(12)?;
            let builtin_tool_keys: Vec<String> =
                serde_json::from_str(&builtin_tool_keys_raw).unwrap_or_default();
            let skill_ids: Vec<String> = serde_json::from_str(&skill_ids_raw).unwrap_or_default();
            let mcp_server_names: Vec<String> =
                serde_json::from_str(&mcp_server_names_raw).unwrap_or_default();
            let default_model_strategy_raw: String = row.get(14)?;
            let capability_policy_raw: String = row.get(15)?;
            let permission_envelope_raw: String = row.get(16)?;
            let memory_policy_raw: String = row.get(17)?;
            let delegation_policy_raw: String = row.get(18)?;
            let approval_preference_raw: String = row.get(19)?;
            let output_contract_raw: String = row.get(20)?;
            let shared_capability_policy_raw: String = row.get(21)?;
            let leader_ref: String = row.get(22)?;
            let member_refs_raw: String = row.get(23)?;
            let team_topology_raw: String = row.get(24)?;
            let shared_memory_policy_raw: String = row.get(25)?;
            let mailbox_policy_raw: String = row.get(26)?;
            let artifact_handoff_policy_raw: String = row.get(27)?;
            let workflow_affordance_raw: String = row.get(28)?;
            let integration_source_raw: Option<String> = row.get(30)?;
            let trust_metadata_raw: String = row.get(31)?;
            let dependency_resolution_raw: String = row.get(32)?;
            let import_metadata_raw: String = row.get(33)?;
            let member_refs = parse_json_or_default(&member_refs_raw, Vec::new);
            Ok(TeamRecord {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                project_id: row.get(2)?,
                scope: row.get(3)?,
                name: row.get(4)?,
                avatar_path,
                avatar,
                personality: row.get(6)?,
                tags: serde_json::from_str(&tags_raw).unwrap_or_default(),
                prompt: row.get(8)?,
                builtin_tool_keys: builtin_tool_keys.clone(),
                skill_ids: skill_ids.clone(),
                mcp_server_names: mcp_server_names.clone(),
                task_domains: parse_json_or_default(&task_domains_raw, || {
                    normalize_task_domains(Vec::new())
                }),
                manifest_revision: row.get(13)?,
                default_model_strategy: parse_json_or_default(
                    &default_model_strategy_raw,
                    default_model_strategy,
                ),
                capability_policy: parse_json_or_default(&capability_policy_raw, || {
                    capability_policy_from_sources(
                        &builtin_tool_keys,
                        &skill_ids,
                        &mcp_server_names,
                    )
                }),
                permission_envelope: parse_json_or_default(
                    &permission_envelope_raw,
                    default_permission_envelope,
                ),
                memory_policy: parse_json_or_default(
                    &memory_policy_raw,
                    default_team_memory_policy,
                ),
                delegation_policy: parse_json_or_default(
                    &delegation_policy_raw,
                    default_team_delegation_policy,
                ),
                approval_preference: parse_json_or_default(
                    &approval_preference_raw,
                    default_approval_preference,
                ),
                output_contract: parse_json_or_default(
                    &output_contract_raw,
                    default_output_contract,
                ),
                shared_capability_policy: parse_json_or_default(
                    &shared_capability_policy_raw,
                    default_team_shared_capability_policy,
                ),
                leader_ref: leader_ref.clone(),
                member_refs: member_refs.clone(),
                team_topology: parse_json_or_default(&team_topology_raw, || {
                    team_topology_from_refs(Some(leader_ref.clone()), member_refs.clone())
                }),
                shared_memory_policy: parse_json_or_default(
                    &shared_memory_policy_raw,
                    default_shared_memory_policy,
                ),
                mailbox_policy: parse_json_or_default(&mailbox_policy_raw, default_mailbox_policy),
                artifact_handoff_policy: parse_json_or_default(
                    &artifact_handoff_policy_raw,
                    default_artifact_handoff_policy,
                ),
                workflow_affordance: parse_json_or_default(&workflow_affordance_raw, || {
                    workflow_affordance_from_task_domains(&Vec::new(), true, true)
                }),
                worker_concurrency_limit: row.get::<_, i64>(29)? as u64,
                integration_source: integration_source_raw
                    .as_deref()
                    .and_then(|value| serde_json::from_str(value).ok()),
                trust_metadata: parse_json_or_default(
                    &trust_metadata_raw,
                    default_asset_trust_metadata,
                ),
                dependency_resolution: parse_json_or_default(&dependency_resolution_raw, Vec::new),
                import_metadata: parse_json_or_default(
                    &import_metadata_raw,
                    default_asset_import_metadata,
                ),
                description: row.get(34)?,
                status: row.get(35)?,
                updated_at: row.get::<_, i64>(36)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}

pub(crate) fn load_project_team_links(
    connection: &Connection,
) -> Result<Vec<ProjectTeamLinkRecord>, AppError> {
    let mut stmt = connection
        .prepare("SELECT workspace_id, project_id, team_id, linked_at FROM project_team_links")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectTeamLinkRecord {
                workspace_id: row.get(0)?,
                project_id: row.get(1)?,
                team_id: row.get(2)?,
                linked_at: row.get::<_, i64>(3)? as u64,
            })
        })
        .map_err(|error| AppError::database(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| AppError::database(error.to_string()))
}
