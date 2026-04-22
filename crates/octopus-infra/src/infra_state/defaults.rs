use super::*;

pub(crate) fn default_project_default_permissions() -> ProjectDefaultPermissions {
    ProjectDefaultPermissions {
        agents: "allow".into(),
        resources: "allow".into(),
        tools: "allow".into(),
        knowledge: "allow".into(),
        tasks: "allow".into(),
    }
}

pub(crate) fn default_project_permission_overrides() -> ProjectPermissionOverrides {
    ProjectPermissionOverrides {
        agents: "inherit".into(),
        resources: "inherit".into(),
        tools: "inherit".into(),
        knowledge: "inherit".into(),
        tasks: "inherit".into(),
    }
}

pub(crate) fn empty_project_linked_workspace_assets() -> ProjectLinkedWorkspaceAssets {
    ProjectLinkedWorkspaceAssets {
        agent_ids: Vec::new(),
        resource_ids: Vec::new(),
        tool_source_keys: Vec::new(),
        knowledge_ids: Vec::new(),
    }
}

pub(crate) fn default_project_model_assignments() -> ProjectModelAssignments {
    ProjectModelAssignments {
        configured_model_ids: vec!["claude-sonnet-4-5".into()],
        default_configured_model_id: "claude-sonnet-4-5".into(),
    }
}

pub(crate) fn default_project_assignments() -> ProjectWorkspaceAssignments {
    ProjectWorkspaceAssignments {
        models: Some(default_project_model_assignments()),
        tools: None,
        agents: None,
    }
}

pub(crate) fn normalized_project_member_user_ids(
    owner_user_id: &str,
    member_user_ids: Vec<String>,
) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut normalized = Vec::new();

    if !owner_user_id.trim().is_empty() && seen.insert(owner_user_id.to_string()) {
        normalized.push(owner_user_id.to_string());
    }

    for user_id in member_user_ids
        .into_iter()
        .map(|value| value.trim().to_string())
    {
        if user_id.is_empty() || !seen.insert(user_id.clone()) {
            continue;
        }
        normalized.push(user_id);
    }

    normalized
}

pub(crate) fn default_workspace_resources() -> Vec<WorkspaceResourceRecord> {
    let now = timestamp_now();
    vec![
        WorkspaceResourceRecord {
            id: "res-workspace-handbook".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: None,
            kind: "file".into(),
            name: "Workspace Handbook".into(),
            location: Some("/docs/workspace-handbook.md".into()),
            origin: "source".into(),
            scope: "workspace".into(),
            visibility: "public".into(),
            owner_user_id: "user-owner".into(),
            storage_path: Some("data/resources/workspace/workspace-handbook.md".into()),
            content_type: Some("text/markdown".into()),
            byte_size: Some(63),
            preview_kind: "markdown".into(),
            status: "healthy".into(),
            updated_at: now,
            tags: vec!["workspace".into(), "handbook".into()],
            source_artifact_id: None,
        },
        WorkspaceResourceRecord {
            id: "res-project-board".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: Some(DEFAULT_PROJECT_ID.into()),
            kind: "folder".into(),
            name: "Project Delivery Board".into(),
            location: Some("/projects/default".into()),
            origin: "generated".into(),
            scope: "project".into(),
            visibility: "public".into(),
            owner_user_id: "user-owner".into(),
            storage_path: Some(format!(
                "data/projects/{DEFAULT_PROJECT_ID}/resources/delivery-board"
            )),
            content_type: None,
            byte_size: None,
            preview_kind: "folder".into(),
            status: "configured".into(),
            updated_at: now,
            tags: vec!["project".into(), "delivery".into()],
            source_artifact_id: None,
        },
    ]
}

pub(crate) fn default_knowledge_records() -> Vec<KnowledgeRecord> {
    let now = timestamp_now();
    vec![
        KnowledgeRecord {
            id: "kn-workspace-onboarding".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: None,
            title: "Workspace onboarding".into(),
            summary: "Shared operating rules, review expectations, and release cadence for this workspace.".into(),
            kind: "shared".into(),
            scope: "workspace".into(),
            status: "shared".into(),
            visibility: "public".into(),
            owner_user_id: None,
            source_type: "artifact".into(),
            source_ref: "workspace-handbook".into(),
            updated_at: now,
        },
        KnowledgeRecord {
            id: "kn-project-brief".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            project_id: Some(DEFAULT_PROJECT_ID.into()),
            title: "Default project brief".into(),
            summary: "Project goals, runtime expectations, and delivery checkpoints.".into(),
            kind: "private".into(),
            scope: "project".into(),
            status: "reviewed".into(),
            visibility: "public".into(),
            owner_user_id: None,
            source_type: "run".into(),
            source_ref: "default-project".into(),
            updated_at: now,
        },
    ]
}

pub(crate) fn default_model_catalog() -> Vec<ModelCatalogRecord> {
    vec![
        ModelCatalogRecord {
            id: "claude-sonnet-4-5".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            label: "Claude Sonnet 4.5".into(),
            provider: "Anthropic".into(),
            description: "Balanced reasoning model for daily runtime turns.".into(),
            recommended_for: "Planning, coding, and reviews".into(),
            availability: "healthy".into(),
            default_permission: "auto".into(),
        },
        ModelCatalogRecord {
            id: "gpt-4o".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            label: "GPT-4o".into(),
            provider: "OpenAI".into(),
            description: "Fast multimodal model for general assistant work.".into(),
            recommended_for: "Conversation and lightweight execution".into(),
            availability: "configured".into(),
            default_permission: "auto".into(),
        },
    ]
}

pub(crate) fn default_provider_credentials() -> Vec<ProviderCredentialRecord> {
    vec![
        ProviderCredentialRecord {
            id: "cred-anthropic".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            provider: "Anthropic".into(),
            name: "Anthropic Primary".into(),
            base_url: None,
            status: "healthy".into(),
        },
        ProviderCredentialRecord {
            id: "cred-openai".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            provider: "OpenAI".into(),
            name: "OpenAI Backup".into(),
            base_url: None,
            status: "unconfigured".into(),
        },
    ]
}

pub(crate) fn default_tool_records() -> Vec<ToolRecord> {
    let now = timestamp_now();
    vec![
        ToolRecord {
            id: "tool-filesystem".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            kind: "builtin".into(),
            name: "Filesystem".into(),
            description: "Read and write files inside the workspace boundary.".into(),
            status: "active".into(),
            permission_mode: "ask".into(),
            updated_at: now,
        },
        ToolRecord {
            id: "tool-shell".into(),
            workspace_id: DEFAULT_WORKSPACE_ID.into(),
            kind: "builtin".into(),
            name: "Shell".into(),
            description: "Execute workspace commands with approval.".into(),
            status: "active".into(),
            permission_mode: "ask".into(),
            updated_at: now,
        },
    ]
}

pub(crate) fn avatar_data_url(paths: &WorkspacePaths, user: &StoredUser) -> Option<String> {
    stored_avatar_data_url(
        paths,
        user.record.avatar_path.as_deref(),
        user.record.avatar_content_type.as_deref(),
    )
}

pub(crate) fn stored_avatar_data_url(
    paths: &WorkspacePaths,
    avatar_path: Option<&str>,
    content_type: Option<&str>,
) -> Option<String> {
    let avatar_path = avatar_path?;
    let Some(content_type) = content_type else {
        return Some(avatar_path.to_string());
    };
    let Ok(bytes) = fs::read(paths.root.join(avatar_path)) else {
        return Some(avatar_path.to_string());
    };
    Some(format!(
        "data:{content_type};base64,{}",
        BASE64_STANDARD.encode(bytes)
    ))
}

pub(crate) fn content_hash(bytes: &[u8]) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("hash-{:x}", hasher.finish())
}
