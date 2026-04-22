pub(crate) fn content_type_for_export(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .as_deref()
    {
        Some("md") => "text/markdown",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

pub(crate) fn encode_file(
    relative_path: &str,
    content_type: &str,
    bytes: Vec<u8>,
) -> WorkspaceDirectoryUploadEntry {
    WorkspaceDirectoryUploadEntry {
        relative_path: relative_path.to_string(),
        file_name: Path::new(relative_path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string(),
        content_type: content_type.to_string(),
        data_base64: BASE64_STANDARD.encode(&bytes),
        byte_size: bytes.len() as u64,
    }
}

fn render_agent_markdown(
    agent: &AgentRecord,
    avatar_file_name: &str,
    exported_skill_slugs: &[String],
) -> String {
    let tools = if agent.builtin_tool_keys == builtin_tool_keys() {
        vec![String::from("ALL")]
    } else {
        agent.builtin_tool_keys.clone()
    };
    [
        "---".into(),
        format!("name: {}", yaml_inline_string(&agent.name)),
        format!("description: {}", yaml_inline_string(&agent.description)),
        format!("character: {}", yaml_inline_string(&agent.personality)),
        format!("avatar: {}", yaml_inline_string(avatar_file_name)),
        format!("tag: {}", yaml_inline_string(&agent.tags.join("、"))),
        format!(
            "tools: {}",
            serde_json::to_string(&tools).unwrap_or_else(|_| "[]".into())
        ),
        format!(
            "skills: {}",
            serde_json::to_string(exported_skill_slugs).unwrap_or_else(|_| "[]".into())
        ),
        format!(
            "mcps: {}",
            serde_json::to_string(&agent.mcp_server_names).unwrap_or_else(|_| "[]".into())
        ),
        format!(
            "model: {}",
            yaml_inline_string(&model_ref_for_export(&agent.default_model_strategy))
        ),
        "-----------".into(),
        String::new(),
        agent.prompt.clone(),
    ]
    .join("\n")
}

fn render_team_markdown(
    team: &TeamRecord,
    avatar_file_name: &str,
    leader_name: &str,
    member_names: &[String],
) -> String {
    [
        "---".into(),
        format!("name: {}", yaml_inline_string(&team.name)),
        format!("description: {}", yaml_inline_string(&team.description)),
        format!("avatar: {}", yaml_inline_string(avatar_file_name)),
        format!("tag: {}", yaml_inline_string(&team.tags.join("、"))),
        format!("leader: {}", yaml_inline_string(leader_name)),
        format!(
            "member: {}",
            serde_json::to_string(member_names).unwrap_or_else(|_| "[]".into())
        ),
        format!(
            "model: {}",
            yaml_inline_string(&model_ref_for_export(&team.default_model_strategy))
        ),
        "-----------".into(),
        String::new(),
        team.prompt.clone(),
    ]
    .join("\n")
}

fn team_summary_file_name(team_name: &str) -> String {
    if team_name.ends_with('部') {
        format!("{team_name}门说明.md")
    } else {
        format!("{team_name}说明.md")
    }
}

fn yaml_inline_string(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "\"\"".into()
    } else if trimmed.contains([':', '#', '"', '\'']) || trimmed.starts_with('[') {
        serde_json::to_string(trimmed).unwrap_or_else(|_| "\"\"".into())
    } else {
        trimmed.to_string()
    }
}

fn agent_matches_ref(agent: &AgentRecord, agent_ref: &str) -> bool {
    let trimmed = agent_ref.trim();
    !trimmed.is_empty() && (trimmed == agent.id || trimmed == crate::canonical_agent_ref(&agent.id))
}

fn team_leader_agent<'a>(agents: &'a [AgentRecord], team: &TeamRecord) -> Option<&'a AgentRecord> {
    agents
        .iter()
        .find(|agent| agent_matches_ref(agent, &team.leader_ref))
}

fn team_member_agents<'a>(agents: &'a [AgentRecord], team: &TeamRecord) -> Vec<&'a AgentRecord> {
    let mut seen = BTreeSet::new();
    team.member_refs
        .iter()
        .filter_map(|member_ref| {
            agents
                .iter()
                .find(|agent| agent_matches_ref(agent, member_ref))
                .filter(|agent| seen.insert(agent.id.clone()))
        })
        .collect()
}

pub(crate) fn export_team_files(
    paths: &WorkspacePaths,
    context: &ExportContext,
    team: &TeamRecord,
    root_dir_name: &str,
) -> Result<Vec<WorkspaceDirectoryUploadEntry>, AppError> {
    let team_dir_name = sanitize_export_dir_name(&team.name);
    let base_dir = if team_dir_name == root_dir_name {
        root_dir_name.to_string()
    } else {
        format!("{root_dir_name}/{team_dir_name}")
    };
    let mut files = Vec::new();
    let avatar_name = if let Some(Some((avatar_name, _content_type, avatar_bytes))) =
        context.avatar_payloads.get(&format!("team:{}", team.id))
    {
        files.push(encode_file(
            &format!("{base_dir}/{avatar_name}"),
            content_type_for_export(avatar_name),
            avatar_bytes.clone(),
        ));
        avatar_name.clone()
    } else {
        String::from("头像")
    };
    let member_agents = team_member_agents(&context.agents, team);
    let member_names = member_agents
        .iter()
        .map(|agent| agent.name.clone())
        .collect::<Vec<_>>();
    let leader_name = team_leader_agent(&context.agents, team)
        .map(|agent| agent.name.clone())
        .unwrap_or_default();
    files.push(encode_file(
        &format!("{base_dir}/{}", team_summary_file_name(&team_dir_name)),
        "text/markdown",
        render_team_markdown(team, &avatar_name, &leader_name, &member_names).into_bytes(),
    ));
    for agent in member_agents {
        files.extend(export_agent_files(
            paths,
            context,
            agent,
            Some(&base_dir),
            root_dir_name,
        )?);
    }
    export_owner_skill_and_mcp_files(context, team, &base_dir, &mut files)?;
    Ok(files)
}

pub(crate) fn export_agent_files(
    _paths: &WorkspacePaths,
    context: &ExportContext,
    agent: &AgentRecord,
    parent_dir: Option<&str>,
    root_dir_name: &str,
) -> Result<Vec<WorkspaceDirectoryUploadEntry>, AppError> {
    let agent_dir_name = sanitize_export_dir_name(&agent.name);
    let base_dir = match parent_dir {
        Some(parent_dir) => format!("{parent_dir}/{agent_dir_name}"),
        None if agent_dir_name == root_dir_name => root_dir_name.to_string(),
        None => format!("{root_dir_name}/{agent_dir_name}"),
    };
    let mut files = Vec::new();
    let avatar_name = if let Some(Some((avatar_name, _content_type, avatar_bytes))) =
        context.avatar_payloads.get(&format!("agent:{}", agent.id))
    {
        files.push(encode_file(
            &format!("{base_dir}/{avatar_name}"),
            content_type_for_export(avatar_name),
            avatar_bytes.clone(),
        ));
        avatar_name.clone()
    } else {
        String::from("头像")
    };
    let exported_skill_slugs = agent
        .skill_ids
        .iter()
        .filter_map(|skill_id| {
            context
                .skill_paths
                .get(skill_id)
                .and_then(|path| path.file_name().and_then(|value| value.to_str()))
                .map(ToOwned::to_owned)
                .or_else(|| {
                    context
                        .builtin_skill_assets
                        .get(skill_id)
                        .map(|asset| asset.slug.clone())
                })
        })
        .collect::<Vec<_>>();
    files.push(encode_file(
        &format!("{base_dir}/{agent_dir_name}.md"),
        "text/markdown",
        render_agent_markdown(agent, &avatar_name, &exported_skill_slugs).into_bytes(),
    ));
    export_owner_skill_and_mcp_files(context, agent, &base_dir, &mut files)?;
    Ok(files)
}

trait ExportOwner {
    fn skill_ids(&self) -> &[String];
    fn mcp_server_names(&self) -> &[String];
}

impl ExportOwner for AgentRecord {
    fn skill_ids(&self) -> &[String] {
        &self.skill_ids
    }

    fn mcp_server_names(&self) -> &[String] {
        &self.mcp_server_names
    }
}

impl ExportOwner for TeamRecord {
    fn skill_ids(&self) -> &[String] {
        &self.skill_ids
    }

    fn mcp_server_names(&self) -> &[String] {
        &self.mcp_server_names
    }
}

fn export_owner_skill_and_mcp_files<T: ExportOwner>(
    context: &ExportContext,
    owner: &T,
    owner_dir: &str,
    files: &mut Vec<WorkspaceDirectoryUploadEntry>,
) -> Result<(), AppError> {
    for skill_id in owner.skill_ids() {
        if let Some(skill_root) = context.skill_paths.get(skill_id) {
            let skill_dir_name = skill_root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("skill");
            for (relative_path, bytes) in read_directory_files(skill_root)? {
                files.push(encode_file(
                    &format!("{owner_dir}/skills/{skill_dir_name}/{relative_path}"),
                    content_type_for_export(&relative_path),
                    bytes,
                ));
            }
            continue;
        }

        if let Some(asset) = context.builtin_skill_assets.get(skill_id) {
            for (relative_path, bytes) in &asset.files {
                files.push(encode_file(
                    &format!("{owner_dir}/skills/{}/{relative_path}", asset.slug),
                    content_type_for_export(relative_path),
                    bytes.clone(),
                ));
            }
        }
    }

    for server_name in owner.mcp_server_names() {
        let Some(config) = context.mcp_configs.get(server_name) else {
            continue;
        };
        files.push(encode_file(
            &format!("{owner_dir}/mcps/{server_name}.json"),
            "application/json",
            serde_json::to_vec_pretty(config)?,
        ));
    }
    Ok(())
}

fn sanitize_export_dir_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return String::from("asset");
    }
    trimmed
        .chars()
        .map(|character| {
            if matches!(character, '/' | '\\' | ':') {
                '-'
            } else {
                character
            }
        })
        .collect()
}

