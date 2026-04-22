pub(crate) fn load_builtin_catalog_sources() -> Result<BuiltinCatalogSources, AppError> {
    let ParsedBundle {
        agents,
        teams,
        skills,
        mcps,
        ..
    } = parse_builtin_bundle()?;
    let mcp_name_by_source = mcps
        .iter()
        .map(|mcp| (mcp.source_id.clone(), mcp.server_name.clone()))
        .collect::<HashMap<_, _>>();
    let agent_name_by_source = agents
        .iter()
        .map(|agent| (agent.source_id.clone(), agent.name.clone()))
        .collect::<HashMap<_, _>>();

    Ok(BuiltinCatalogSources {
        skill_sources: skills
            .into_iter()
            .map(|source| {
                let description = source
                    .files
                    .iter()
                    .find(|(path, _)| path == SKILL_FRONTMATTER_FILE)
                    .and_then(|(_, bytes)| String::from_utf8(bytes.clone()).ok())
                    .and_then(|content| parse_frontmatter(&content).ok())
                    .and_then(|(frontmatter, _)| yaml_string(&frontmatter, "description"))
                    .unwrap_or_default();
                BuiltinSkillCatalogSource {
                    source_id: source.source_id,
                    name: source.name,
                    canonical_slug: source.canonical_slug,
                    content_hash: source.content_hash,
                    description,
                    files: source.files,
                }
            })
            .collect(),
        agent_sources: {
            let mut records = agents
                .into_iter()
                .map(|agent| BuiltinAgentTemplateSource {
                    source_id: agent.source_id,
                    name: agent.name.clone(),
                    avatar_data_url: avatar_data_url(&agent.avatar),
                    personality: agent.personality.clone(),
                    tags: agent.tags.clone(),
                    prompt: agent.prompt.clone(),
                    builtin_tool_keys: agent.builtin_tool_keys.clone(),
                    skill_source_ids: agent.skill_source_ids,
                    mcp_server_names: agent
                        .mcp_source_ids
                        .iter()
                        .filter_map(|source_id| mcp_name_by_source.get(source_id))
                        .cloned()
                        .collect(),
                    model: agent.model,
                    description: agent.description,
                })
                .collect::<Vec<_>>();
            records.sort_by(|left, right| {
                left.name
                    .cmp(&right.name)
                    .then(left.source_id.cmp(&right.source_id))
            });
            records
        },
        team_sources: {
            let mut records = teams
                .into_iter()
                .map(|team| {
                    let member_name_set = team
                        .member_names
                        .iter()
                        .cloned()
                        .collect::<std::collections::HashSet<_>>();
                    let mut member_agent_source_ids = team
                        .agent_source_ids
                        .iter()
                        .filter(|source_id| {
                            agent_name_by_source
                                .get(*source_id)
                                .map(|name| {
                                    member_name_set.is_empty() || member_name_set.contains(name)
                                })
                                .unwrap_or(false)
                        })
                        .cloned()
                        .collect::<Vec<_>>();
                    if member_agent_source_ids.is_empty() {
                        member_agent_source_ids = team.agent_source_ids.clone();
                    }
                    let leader_agent_source_id = team
                        .leader_name
                        .as_ref()
                        .and_then(|leader_name| {
                            team.agent_source_ids
                                .iter()
                                .find(|source_id| {
                                    agent_name_by_source
                                        .get(*source_id)
                                        .map(|name| name == leader_name)
                                        .unwrap_or(false)
                                })
                                .cloned()
                        })
                        .or_else(|| member_agent_source_ids.first().cloned());

                    BuiltinTeamTemplateSource {
                        source_id: team.source_id,
                        name: team.name.clone(),
                        avatar_data_url: avatar_data_url(&team.avatar),
                        personality: team.personality.clone(),
                        tags: team.tags.clone(),
                        prompt: team.prompt.clone(),
                        builtin_tool_keys: team.builtin_tool_keys.clone(),
                        skill_source_ids: team.skill_source_ids,
                        mcp_server_names: team
                            .mcp_source_ids
                            .iter()
                            .filter_map(|source_id| mcp_name_by_source.get(source_id))
                            .cloned()
                            .collect(),
                        leader_agent_source_id,
                        member_agent_source_ids,
                        model: team.model,
                        description: team.description,
                    }
                })
                .collect::<Vec<_>>();
            records.sort_by(|left, right| {
                left.name
                    .cmp(&right.name)
                    .then(left.source_id.cmp(&right.source_id))
            });
            records
        },
    })
}

pub(crate) fn find_builtin_skill_asset_by_id(
    skill_id: &str,
) -> Result<Option<BuiltinSkillAsset>, AppError> {
    crate::agent_bundle::find_builtin_skill_asset_by_id(skill_id)
}

fn parse_builtin_bundle() -> Result<ParsedBundle, AppError> {
    let bundle_files =
        crate::agent_bundle::builtin::embedded_bundle_files(&BUILTIN_BUNDLE_ASSET_DIR)?;
    let mut issues = Vec::new();
    let parsed = parse_bundle_files(&bundle_files, &AssetTargetScope::Workspace, &mut issues)?;
    if let Some(issue) = issues.iter().find(|issue| issue.severity == ISSUE_ERROR) {
        return Err(AppError::invalid_input(format!(
            "invalid builtin asset bundle: {}",
            issue.message
        )));
    }
    Ok(parsed)
}

fn avatar_data_url(avatar: &ParsedAssetAvatar) -> Option<String> {
    Some(format!(
        "data:{};base64,{}",
        avatar.content_type,
        BASE64_STANDARD.encode(&avatar.bytes)
    ))
}

fn parse_agent_dir(
    source_id: &str,
    owner_dir: &str,
    team_name: Option<String>,
    agent_file: &BundleFile,
    root_files: &[BundleFile],
    target: &AssetTargetScope<'_>,
    builtin_tool_keys: &[String],
    issues: &mut Vec<ImportIssue>,
) -> Result<(ParsedAgent, Vec<ParsedSkillSource>, Vec<ParsedMcpSource>), AppError> {
    let markdown = String::from_utf8_lossy(&agent_file.bytes).to_string();
    let (frontmatter, body) = parse_frontmatter(&markdown)?;
    let dir_name = source_id.rsplit('/').next().unwrap_or(source_id);
    let name = yaml_string(&frontmatter, "name").unwrap_or_else(|| dir_name.to_string());
    let description = yaml_string(&frontmatter, "description")
        .or_else(|| first_non_empty_paragraph(&body))
        .unwrap_or_else(|| name.clone());
    let personality = yaml_string(&frontmatter, "character")
        .or_else(|| first_paragraph_after_heading(&body, "角色定义"))
        .or_else(|| first_paragraph_after_heading(&body, "Role Definition"))
        .unwrap_or_else(|| name.clone());
    let prompt = body.trim().to_string();
    let tags = split_tags(yaml_string(&frontmatter, "tag"));
    let builtin_tool_keys =
        resolve_builtin_tool_keys(yaml_string_list(&frontmatter, "tools"), builtin_tool_keys);
    let skills = parse_skill_sources(owner_dir, source_id, &name, root_files, issues)?;
    let mcps = parse_mcp_sources(
        owner_dir,
        source_id,
        &name,
        yaml_string_list(&frontmatter, "mcps"),
        root_files,
        issues,
    )?;
    let avatar = resolve_avatar(
        "agent",
        source_id,
        &name,
        owner_dir,
        yaml_string(&frontmatter, "avatar"),
        root_files,
        target,
        issues,
    )?;
    Ok((
        ParsedAgent {
            source_id: source_id.to_string(),
            team_name,
            name,
            description,
            personality,
            prompt,
            tags,
            builtin_tool_keys,
            skill_source_ids: skills.iter().map(|item| item.source_id.clone()).collect(),
            mcp_source_ids: mcps.iter().map(|item| item.source_id.clone()).collect(),
            avatar,
            model: yaml_string(&frontmatter, "model"),
        },
        skills,
        mcps,
    ))
}

fn parse_skill_sources(
    owner_dir: &str,
    owner_source_id: &str,
    owner_name: &str,
    files: &[BundleFile],
    issues: &mut Vec<ImportIssue>,
) -> Result<Vec<ParsedSkillSource>, AppError> {
    let prefix = join_bundle_path(owner_dir, "skills/");
    let mut grouped = BTreeMap::<String, Vec<(String, Vec<u8>)>>::new();
    for file in files {
        if !file.relative_path.starts_with(&prefix) {
            continue;
        }
        let suffix = &file.relative_path[prefix.len()..];
        let segments = suffix.split('/').collect::<Vec<_>>();
        if segments.len() < 2 {
            continue;
        }
        grouped
            .entry(segments[0].to_string())
            .or_default()
            .push((segments[1..].join("/"), file.bytes.clone()));
    }

    let mut parsed = Vec::new();
    for (skill_dir, mut skill_files) in grouped {
        skill_files.sort_by(|left, right| left.0.cmp(&right.0));
        let Some(skill_frontmatter_file) = skill_files
            .iter()
            .find(|(path, _)| path == SKILL_FRONTMATTER_FILE)
        else {
            issues.push(issue(
                ISSUE_WARNING,
                SOURCE_SCOPE_SKILL,
                Some(format!("{owner_source_id}/skills/{skill_dir}")),
                format!("skipped skill '{skill_dir}': missing required '{SKILL_FRONTMATTER_FILE}'"),
            ));
            continue;
        };
        let markdown = String::from_utf8_lossy(&skill_frontmatter_file.1).to_string();
        let (frontmatter, _) = parse_frontmatter(&markdown)?;
        let skill_name = yaml_string(&frontmatter, "name").unwrap_or_else(|| skill_dir.clone());
        let canonical_slug = validate_skill_slug(&slugify_skill_name(&skill_name, "skill"))?;
        let source_id = format!("{owner_source_id}/skills/{skill_dir}");
        parsed.push(ParsedSkillSource {
            source_id,
            owner_name: owner_name.to_string(),
            name: skill_name,
            canonical_slug,
            content_hash: hash_bundle_files(&skill_files),
            files: skill_files,
        });
    }
    Ok(parsed)
}

fn parse_mcp_sources(
    owner_dir: &str,
    owner_source_id: &str,
    owner_name: &str,
    frontmatter_mcps: Vec<String>,
    files: &[BundleFile],
    issues: &mut Vec<ImportIssue>,
) -> Result<Vec<ParsedMcpSource>, AppError> {
    let prefix = join_bundle_path(owner_dir, "mcps/");
    let mut parsed = Vec::new();
    let mut seen_names = BTreeSet::new();
    for file in files {
        if !file.relative_path.starts_with(&prefix) {
            continue;
        }
        let suffix = &file.relative_path[prefix.len()..];
        if suffix.contains('/') || !suffix.ends_with(".json") {
            continue;
        }
        let server_name = suffix.trim_end_matches(".json").to_string();
        let config = match serde_json::from_slice::<JsonValue>(&file.bytes) {
            Ok(config) if config.is_object() => config,
            Ok(_) => {
                issues.push(issue(
                    ISSUE_WARNING,
                    SOURCE_SCOPE_MCP,
                    Some(file.relative_path.clone()),
                    String::from("skipped MCP file because it is not a JSON object"),
                ));
                continue;
            }
            Err(error) => {
                issues.push(issue(
                    ISSUE_WARNING,
                    SOURCE_SCOPE_MCP,
                    Some(file.relative_path.clone()),
                    format!("skipped MCP file because JSON is invalid: {error}"),
                ));
                continue;
            }
        };
        let source_id = format!("{owner_source_id}/mcps/{server_name}");
        seen_names.insert(server_name.clone());
        parsed.push(ParsedMcpSource {
            source_id,
            owner_name: owner_name.to_string(),
            server_name,
            content_hash: Some(hash_json_value(&config)?),
            config: Some(config),
            referenced_only: false,
        });
    }

    for server_name in frontmatter_mcps {
        if seen_names.contains(&server_name) {
            continue;
        }
        parsed.push(ParsedMcpSource {
            source_id: format!("{owner_source_id}/mcps/{server_name}"),
            owner_name: owner_name.to_string(),
            server_name,
            content_hash: None,
            config: None,
            referenced_only: true,
        });
    }

    Ok(parsed)
}

fn resolve_avatar(
    owner_kind: &str,
    source_id: &str,
    owner_name: &str,
    owner_dir: &str,
    avatar_field: Option<String>,
    files: &[BundleFile],
    target: &AssetTargetScope<'_>,
    issues: &mut Vec<ImportIssue>,
) -> Result<ParsedAssetAvatar, AppError> {
    let mut candidates = Vec::new();
    if let Some(avatar_name) = avatar_field
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        let path = join_bundle_path(owner_dir, avatar_name.trim());
        if let Some(file) = files.iter().find(|file| file.relative_path == path) {
            candidates.push((avatar_name.trim().to_string(), file.bytes.clone()));
        }
    }

    if candidates.is_empty() {
        let prefix = if owner_dir.trim().is_empty() {
            String::new()
        } else {
            format!("{owner_dir}/")
        };
        for file in files {
            if !prefix.is_empty() && !file.relative_path.starts_with(&prefix) {
                continue;
            }
            let suffix = if prefix.is_empty() {
                file.relative_path.as_str()
            } else {
                &file.relative_path[prefix.len()..]
            };
            if suffix.contains('/') || !is_supported_avatar_file(suffix) {
                continue;
            }
            candidates.push((suffix.to_string(), file.bytes.clone()));
        }
    }

    for (file_name, bytes) in candidates {
        if let Some(content_type) = content_type_for_avatar(&file_name) {
            return Ok(ParsedAssetAvatar {
                source_id: source_id.to_string(),
                owner_kind: owner_kind.to_string(),
                owner_name: owner_name.to_string(),
                file_name,
                content_type: content_type.to_string(),
                bytes,
                generated: false,
            });
        }
    }

    issues.push(issue(
        ISSUE_WARNING,
        SOURCE_SCOPE_AVATAR,
        Some(source_id.to_string()),
        format!(
            "avatar for {} '{}' is missing or unsupported, generated a deterministic default avatar",
            owner_kind, owner_name
        ),
    ));
    let default_avatar = default_avatar_payload(owner_kind, &target.avatar_seed_key(source_id))?;
    Ok(ParsedAssetAvatar {
        source_id: source_id.to_string(),
        owner_kind: owner_kind.to_string(),
        owner_name: owner_name.to_string(),
        file_name: default_avatar.0,
        content_type: default_avatar.1,
        bytes: default_avatar.2,
        generated: true,
    })
}

fn default_avatar_payload(
    owner_kind: &str,
    seed_key: &str,
) -> Result<(String, String, Vec<u8>), AppError> {
    let dir = if owner_kind == "team" {
        &DEFAULT_LEADER_AVATARS
    } else {
        &DEFAULT_EMPLOYEE_AVATARS
    };
    let mut entries = dir
        .entries()
        .iter()
        .filter_map(|entry| match entry {
            DirEntry::File(file) => Some((
                file.path()
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
                    .to_string(),
                file.contents().to_vec(),
            )),
            DirEntry::Dir(_) => None,
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    let index = deterministic_index(seed_key, entries.len());
    let (file_name, bytes) = entries
        .get(index)
        .cloned()
        .ok_or_else(|| AppError::invalid_input("default avatar assets are not available"))?;
    let content_type = content_type_for_avatar(&file_name)
        .ok_or_else(|| AppError::invalid_input("default avatar has unsupported type"))?;
    Ok((file_name, content_type.to_string(), bytes))
}

fn deterministic_index(seed_key: &str, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let digest = Sha256::digest(seed_key.as_bytes());
    let mut value = 0_u64;
    for byte in digest.iter().take(8) {
        value = (value << 8) + u64::from(*byte);
    }
    (value as usize) % len
}
