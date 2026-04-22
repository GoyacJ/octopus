pub(crate) fn parse_bundle_files(
    files: &[BundleFile],
    target: &AssetTargetScope<'_>,
    issues: &mut Vec<ImportIssue>,
) -> Result<ParsedBundle, AppError> {
    let asset_state = parse_bundle_asset_state(files, issues);
    let bundle_manifest = parse_bundle_manifest(files, issues);
    let content_files = files
        .iter()
        .filter(|file| !file.relative_path.starts_with(".octopus/"))
        .cloned()
        .collect::<Vec<_>>();
    let descriptor_assets =
        parse_bundle_descriptors(bundle_manifest.as_ref(), &content_files, issues);
    let builtin_tool_keys = builtin_tool_keys();
    let mut collected = ParsedPackage::default();

    if let Some(rootless_package) =
        parse_rootless_package(&content_files, target, &builtin_tool_keys, issues)?
    {
        collected.agents.extend(rootless_package.agents);
        collected.teams.extend(rootless_package.teams);
        collected.skills.extend(rootless_package.skills);
        collected.mcps.extend(rootless_package.mcps);
        collected.avatars.extend(rootless_package.avatars);
    } else {
        for (root_name, root_files) in group_top_level(&content_files) {
            if RESERVED_DIRS.iter().any(|reserved| reserved == &root_name)
                || IGNORED_TEMPLATE_ROOTS
                    .iter()
                    .any(|candidate| candidate == &root_name)
            {
                continue;
            }
            let Some(package) =
                parse_named_package(&root_name, &root_files, target, &builtin_tool_keys, issues)?
            else {
                continue;
            };
            collected.agents.extend(package.agents);
            collected.teams.extend(package.teams);
            collected.skills.extend(package.skills);
            collected.mcps.extend(package.mcps);
            collected.avatars.extend(package.avatars);
        }
    }

    if collected.agents.is_empty() && collected.teams.is_empty() {
        issues.push(issue(
            ISSUE_ERROR,
            SOURCE_SCOPE_BUNDLE,
            None,
            String::from("no compatible agent assets were found in the selected bundle"),
        ));
    }

    Ok(ParsedBundle {
        bundle_manifest,
        descriptor_assets,
        agents: collected.agents,
        teams: collected.teams,
        skills: collected.skills,
        mcps: collected.mcps,
        avatars: collected.avatars,
        asset_state,
    })
}

fn parse_rootless_package(
    files: &[BundleFile],
    target: &AssetTargetScope<'_>,
    builtin_tool_keys: &[String],
    issues: &mut Vec<ImportIssue>,
) -> Result<Option<ParsedPackage>, AppError> {
    let root_level_markdown = files
        .iter()
        .filter(|file| !file.relative_path.contains('/') && file.relative_path.ends_with(".md"))
        .collect::<Vec<_>>();
    if root_level_markdown.is_empty() {
        return Ok(None);
    }

    if let Some(team_file) = root_level_markdown
        .iter()
        .find(|file| file.relative_path.ends_with("说明.md"))
    {
        let source_id = team_source_id_from_file(team_file)?;
        return Ok(Some(parse_team_package(
            &source_id,
            "",
            team_file,
            files,
            target,
            builtin_tool_keys,
            issues,
        )?));
    }

    let agent_markdown = root_level_markdown
        .into_iter()
        .filter(|file| !file.relative_path.ends_with("说明.md"))
        .collect::<Vec<_>>();
    if agent_markdown.len() != 1 {
        return Ok(None);
    }
    let agent_file = agent_markdown[0];
    let source_id = markdown_stem(&agent_file.relative_path).to_string();
    Ok(Some(parse_agent_package(
        &source_id,
        "",
        None,
        agent_file,
        files,
        target,
        builtin_tool_keys,
        issues,
    )?))
}

fn parse_named_package(
    root_name: &str,
    root_files: &[BundleFile],
    target: &AssetTargetScope<'_>,
    builtin_tool_keys: &[String],
    issues: &mut Vec<ImportIssue>,
) -> Result<Option<ParsedPackage>, AppError> {
    let root_prefix = format!("{root_name}/");
    let root_level_markdown = root_files
        .iter()
        .filter(|file| {
            file.relative_path
                .strip_prefix(&root_prefix)
                .is_some_and(|suffix| !suffix.contains('/') && suffix.ends_with(".md"))
        })
        .collect::<Vec<_>>();

    if let Some(team_file) = root_level_markdown
        .iter()
        .find(|file| file.relative_path.ends_with("说明.md"))
    {
        return Ok(Some(parse_team_package(
            root_name,
            root_name,
            team_file,
            root_files,
            target,
            builtin_tool_keys,
            issues,
        )?));
    }

    let preferred_agent_markdown = format!("{root_name}/{root_name}.md");
    let agent_file = root_level_markdown
        .iter()
        .find(|file| file.relative_path == preferred_agent_markdown)
        .copied()
        .or_else(|| {
            let candidates = root_level_markdown
                .iter()
                .filter(|file| !file.relative_path.ends_with("说明.md"))
                .copied()
                .collect::<Vec<_>>();
            if candidates.len() == 1 {
                Some(candidates[0])
            } else {
                None
            }
        });
    let Some(agent_file) = agent_file else {
        issues.push(issue(
            ISSUE_WARNING,
            SOURCE_SCOPE_BUNDLE,
            Some(root_name.to_string()),
            format!(
                "skipped '{root_name}': missing root markdown entry like '{root_name}/{root_name}.md' or '*说明.md'"
            ),
        ));
        return Ok(None);
    };

    Ok(Some(parse_agent_package(
        root_name,
        root_name,
        None,
        agent_file,
        root_files,
        target,
        builtin_tool_keys,
        issues,
    )?))
}

fn parse_team_package(
    source_id: &str,
    owner_dir: &str,
    team_file: &BundleFile,
    files: &[BundleFile],
    target: &AssetTargetScope<'_>,
    builtin_tool_keys: &[String],
    issues: &mut Vec<ImportIssue>,
) -> Result<ParsedPackage, AppError> {
    let team_source = String::from_utf8_lossy(&team_file.bytes).to_string();
    let (frontmatter, body) = parse_frontmatter(&team_source)?;
    let team_name = yaml_string(&frontmatter, "name").unwrap_or_else(|| source_id.to_string());
    let team_description = yaml_string(&frontmatter, "description")
        .or_else(|| first_non_empty_paragraph(&body))
        .unwrap_or_else(|| team_name.clone());
    let team_prompt = body.trim().to_string();
    let team_personality = first_non_empty_paragraph(&body).unwrap_or_else(|| team_name.clone());
    let team_tags = split_tags(yaml_string(&frontmatter, "tag"));
    let team_builtin_tools =
        resolve_builtin_tool_keys(yaml_string_list(&frontmatter, "tools"), builtin_tool_keys);
    let team_avatar = resolve_avatar(
        "team",
        source_id,
        &team_name,
        owner_dir,
        yaml_string(&frontmatter, "avatar"),
        files,
        target,
        issues,
    )?;
    let team_skills = parse_skill_sources(owner_dir, source_id, &team_name, files, issues)?;
    let team_mcps = parse_mcp_sources(
        owner_dir,
        source_id,
        &team_name,
        yaml_string_list(&frontmatter, "mcps"),
        files,
        issues,
    )?;

    let mut package = ParsedPackage::default();
    let mut team_agent_source_ids = Vec::new();
    let mut parsed_member_names = Vec::new();

    for member_dir in member_dirs_for_owner(files, owner_dir) {
        let member_source_id = join_bundle_path(source_id, &member_dir);
        let member_owner_dir = join_bundle_path(owner_dir, &member_dir);
        let member_md = join_bundle_path(&member_owner_dir, &format!("{member_dir}.md"));
        let Some(member_file) = files.iter().find(|file| file.relative_path == member_md) else {
            continue;
        };
        let member_package = parse_agent_package(
            &member_source_id,
            &member_owner_dir,
            Some(team_name.clone()),
            member_file,
            files,
            target,
            builtin_tool_keys,
            issues,
        )?;
        let agent =
            member_package.agents.first().cloned().ok_or_else(|| {
                AppError::invalid_input("team member package missing parsed agent")
            })?;
        parsed_member_names.push(agent.name.clone());
        team_agent_source_ids.push(agent.source_id.clone());
        package.agents.extend(member_package.agents);
        package.skills.extend(member_package.skills);
        package.mcps.extend(member_package.mcps);
        package.avatars.extend(member_package.avatars);
    }

    package.skills.extend(team_skills.clone());
    package.mcps.extend(team_mcps.clone());
    package.avatars.push(team_avatar.clone());
    package.teams.push(ParsedTeam {
        source_id: source_id.to_string(),
        name: team_name.clone(),
        description: team_description,
        personality: team_personality,
        prompt: team_prompt,
        tags: team_tags,
        builtin_tool_keys: team_builtin_tools,
        skill_source_ids: team_skills.into_iter().map(|item| item.source_id).collect(),
        mcp_source_ids: team_mcps.into_iter().map(|item| item.source_id).collect(),
        leader_name: yaml_string(&frontmatter, "leader"),
        member_names: {
            let members = yaml_string_list(&frontmatter, "member");
            if members.is_empty() {
                parsed_member_names
            } else {
                members
            }
        },
        agent_source_ids: team_agent_source_ids,
        avatar: team_avatar,
        model: yaml_string(&frontmatter, "model"),
    });
    Ok(package)
}

fn parse_agent_package(
    source_id: &str,
    owner_dir: &str,
    team_name: Option<String>,
    agent_file: &BundleFile,
    files: &[BundleFile],
    target: &AssetTargetScope<'_>,
    builtin_tool_keys: &[String],
    issues: &mut Vec<ImportIssue>,
) -> Result<ParsedPackage, AppError> {
    let (agent, skills, mcps) = parse_agent_dir(
        source_id,
        owner_dir,
        team_name,
        agent_file,
        files,
        target,
        builtin_tool_keys,
        issues,
    )?;
    Ok(ParsedPackage {
        agents: vec![agent.clone()],
        teams: Vec::new(),
        skills,
        mcps,
        avatars: vec![agent.avatar],
    })
}

fn parse_bundle_asset_state(
    files: &[BundleFile],
    issues: &mut Vec<ImportIssue>,
) -> BundleAssetStateDocument {
    let Some(file) = files.iter().find(|file| {
        file.relative_path == BUNDLE_ASSET_STATE_PATH
            || file
                .relative_path
                .ends_with(&format!("/{BUNDLE_ASSET_STATE_PATH}"))
    }) else {
        return BundleAssetStateDocument::default();
    };

    match serde_json::from_slice::<BundleAssetStateDocument>(&file.bytes) {
        Ok(document) => document,
        Err(error) => {
            issues.push(issue(
                ISSUE_WARNING,
                SOURCE_SCOPE_BUNDLE,
                None,
                format!("ignored invalid '{BUNDLE_ASSET_STATE_PATH}': {error}"),
            ));
            BundleAssetStateDocument::default()
        }
    }
}

fn parse_bundle_manifest(
    files: &[BundleFile],
    issues: &mut Vec<ImportIssue>,
) -> Option<AssetBundleManifestV2> {
    let file = files.iter().find(|file| {
        file.relative_path == ".octopus/manifest.json"
            || file.relative_path.ends_with("/.octopus/manifest.json")
    })?;

    match serde_json::from_slice::<AssetBundleManifestV2>(&file.bytes) {
        Ok(manifest) => Some(manifest),
        Err(error) => {
            issues.push(issue(
                ISSUE_WARNING,
                SOURCE_SCOPE_BUNDLE,
                Some(file.relative_path.clone()),
                format!("ignored invalid '.octopus/manifest.json': {error}"),
            ));
            None
        }
    }
}

fn parse_bundle_descriptors(
    bundle_manifest: Option<&AssetBundleManifestV2>,
    content_files: &[BundleFile],
    issues: &mut Vec<ImportIssue>,
) -> Vec<ParsedBundleDescriptor> {
    let Some(bundle_manifest) = bundle_manifest else {
        return Vec::new();
    };

    let mut descriptors = Vec::new();
    for entry in &bundle_manifest.assets {
        if !matches!(entry.asset_kind.as_str(), "plugin" | "workflow-template") {
            continue;
        }
        let Some(file) = content_files
            .iter()
            .find(|file| file.relative_path == entry.source_path)
        else {
            issues.push(issue(
                ISSUE_ERROR,
                SOURCE_SCOPE_BUNDLE,
                Some(entry.source_id.clone()),
                format!(
                    "bundle manifest entry '{}' is missing source file '{}'",
                    entry.display_name, entry.source_path
                ),
            ));
            continue;
        };
        descriptors.push(ParsedBundleDescriptor {
            asset_kind: entry.asset_kind.clone(),
            source_id: entry.source_id.clone(),
            display_name: entry.display_name.clone(),
            source_path: entry.source_path.clone(),
            manifest_revision: entry.manifest_revision.clone(),
            task_domains: entry.task_domains.clone(),
            translation_mode: entry.translation_mode.clone(),
            bytes: file.bytes.clone(),
        });
    }

    descriptors
}

