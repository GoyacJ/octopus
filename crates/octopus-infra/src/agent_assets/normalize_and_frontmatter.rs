pub(crate) fn normalize_bundle_files(
    files: &[WorkspaceDirectoryUploadEntry],
) -> Result<(Vec<BundleFile>, u64, Vec<ImportIssue>), AppError> {
    if files.is_empty() {
        return Err(AppError::invalid_input("agent bundle files are required"));
    }

    let mut normalized = Vec::new();
    let mut filtered = 0_u64;
    let mut issues = Vec::new();
    for file in files {
        let relative_path = validate_skill_file_relative_path(&file.relative_path)?;
        if path_contains_filtered_directory(&relative_path) {
            filtered += 1;
            continue;
        }
        let bytes = match BASE64_STANDARD.decode(&file.data_base64) {
            Ok(bytes) => bytes,
            Err(error) => {
                issues.push(issue(
                    ISSUE_WARNING,
                    SOURCE_SCOPE_BUNDLE,
                    Some(relative_path),
                    format!("skipped file with invalid base64 payload: {error}"),
                ));
                continue;
            }
        };
        normalized.push(BundleFile {
            relative_path,
            bytes,
        });
    }
    Ok((normalized, filtered, issues))
}

pub(crate) fn strip_optional_bundle_root(files: Vec<BundleFile>) -> Vec<BundleFile> {
    let mut first_segments = files
        .iter()
        .filter_map(|file| file.relative_path.split('/').next().map(ToOwned::to_owned))
        .collect::<BTreeSet<_>>();
    if first_segments.len() != 1 {
        return files;
    }
    let root = first_segments.pop_first().unwrap_or_default();
    let root_md = format!("{root}/{root}.md");
    if files.iter().any(|file| file.relative_path == root_md) {
        return files;
    }

    files
        .into_iter()
        .filter_map(|file| {
            file.relative_path
                .strip_prefix(&format!("{root}/"))
                .map(|relative_path| BundleFile {
                    relative_path: relative_path.to_string(),
                    bytes: file.bytes,
                })
        })
        .collect()
}

fn group_top_level(files: &[BundleFile]) -> BTreeMap<String, Vec<BundleFile>> {
    let mut grouped = BTreeMap::<String, Vec<BundleFile>>::new();
    for file in files {
        if let Some(segment) = file.relative_path.split('/').next() {
            grouped
                .entry(segment.to_string())
                .or_default()
                .push(file.clone());
        }
    }
    grouped
}

fn join_bundle_path(base: &str, child: &str) -> String {
    if base.trim().is_empty() {
        child.to_string()
    } else {
        format!("{base}/{child}")
    }
}

fn markdown_stem(path: &str) -> &str {
    Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(path)
}

fn team_source_id_from_file(file: &BundleFile) -> Result<String, AppError> {
    let markdown = String::from_utf8_lossy(&file.bytes).to_string();
    let (frontmatter, _) = parse_frontmatter(&markdown)?;
    if let Some(name) = yaml_string(&frontmatter, "name") {
        return Ok(name);
    }
    Ok(markdown_stem(&file.relative_path)
        .trim_end_matches("说明")
        .to_string())
}

fn member_dirs_for_owner(files: &[BundleFile], owner_dir: &str) -> Vec<String> {
    immediate_child_dirs(files, owner_dir)
        .into_iter()
        .filter(|item| {
            !RESERVED_DIRS.iter().any(|reserved| reserved == item)
                && !IGNORED_TEMPLATE_ROOTS
                    .iter()
                    .any(|candidate| candidate == item)
        })
        .filter(|child| {
            let child_dir = join_bundle_path(owner_dir, child);
            let expected = join_bundle_path(&child_dir, &format!("{child}.md"));
            files.iter().any(|file| file.relative_path == expected)
        })
        .collect()
}

fn immediate_child_dirs(files: &[BundleFile], root: &str) -> BTreeSet<String> {
    let prefix = if root.trim().is_empty() {
        String::new()
    } else {
        format!("{root}/")
    };
    files
        .iter()
        .filter_map(|file| {
            if prefix.is_empty() {
                Some(file.relative_path.as_str())
            } else {
                file.relative_path.strip_prefix(&prefix)
            }
        })
        .filter(|suffix| suffix.contains('/'))
        .filter_map(|suffix| suffix.split('/').next())
        .map(ToOwned::to_owned)
        .collect()
}

fn path_contains_filtered_directory(relative_path: &str) -> bool {
    relative_path.split('/').any(|segment| {
        FILTERED_DIR_NAMES
            .iter()
            .any(|candidate| candidate == &segment)
    })
}

fn parse_frontmatter(
    contents: &str,
) -> Result<(BTreeMap<String, serde_yaml::Value>, String), AppError> {
    let normalized = contents.replace("\r\n", "\n");
    let lines = normalized.lines().collect::<Vec<_>>();
    let Some(first) = lines.first() else {
        return Ok((BTreeMap::new(), String::new()));
    };
    if !is_frontmatter_delimiter(first) {
        return Ok((BTreeMap::new(), normalized));
    }

    let mut frontmatter_lines = Vec::new();
    let mut body_index = 1_usize;
    while body_index < lines.len() {
        let line = lines[body_index];
        if is_frontmatter_delimiter(line) {
            body_index += 1;
            break;
        }
        frontmatter_lines.push(line);
        body_index += 1;
    }

    let normalized_frontmatter_lines = frontmatter_lines
        .into_iter()
        .map(sanitize_frontmatter_line)
        .collect::<Vec<_>>();

    let frontmatter = if normalized_frontmatter_lines.is_empty() {
        BTreeMap::new()
    } else {
        let normalized_frontmatter = normalized_frontmatter_lines.join("\n");
        match serde_yaml::from_str::<BTreeMap<String, serde_yaml::Value>>(&normalized_frontmatter) {
            Ok(frontmatter) => frontmatter,
            Err(_) => parse_frontmatter_fallback(&normalized_frontmatter_lines)?,
        }
    };
    Ok((frontmatter, lines[body_index..].join("\n")))
}

fn sanitize_frontmatter_line(line: &str) -> String {
    let trimmed = line.trim_end();
    if trimmed != "---" && trimmed.ends_with("---") && trimmed.contains(':') {
        trimmed.trim_end_matches('-').trim_end().to_string()
    } else {
        line.to_string()
    }
}

fn parse_frontmatter_fallback(
    lines: &[String],
) -> Result<BTreeMap<String, serde_yaml::Value>, AppError> {
    let mut frontmatter = BTreeMap::new();
    let mut current_key: Option<String> = None;
    let mut current_value_lines: Vec<String> = Vec::new();

    for line in lines {
        if let Some((key, value)) = parse_frontmatter_entry_line(line) {
            flush_frontmatter_entry(&mut frontmatter, &mut current_key, &mut current_value_lines)?;
            current_key = Some(key);
            current_value_lines.push(value);
            continue;
        }

        if current_key.is_some() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                current_value_lines.push(trimmed.to_string());
            }
        }
    }

    flush_frontmatter_entry(&mut frontmatter, &mut current_key, &mut current_value_lines)?;
    Ok(frontmatter)
}

fn flush_frontmatter_entry(
    frontmatter: &mut BTreeMap<String, serde_yaml::Value>,
    current_key: &mut Option<String>,
    current_value_lines: &mut Vec<String>,
) -> Result<(), AppError> {
    let Some(key) = current_key.take() else {
        current_value_lines.clear();
        return Ok(());
    };

    let normalized_value_lines = current_value_lines
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(strip_wrapping_quotes)
        .collect::<Vec<_>>()
        .join(" ");
    current_value_lines.clear();

    let value = if normalized_value_lines.is_empty() {
        serde_yaml::Value::Null
    } else {
        serde_yaml::from_str::<serde_yaml::Value>(&normalized_value_lines).unwrap_or_else(|_| {
            serde_yaml::Value::String(strip_wrapping_quotes(&normalized_value_lines))
        })
    };
    frontmatter.insert(key, value);
    Ok(())
}

fn parse_frontmatter_entry_line(line: &str) -> Option<(String, String)> {
    if line.trim().is_empty() || line.starts_with(char::is_whitespace) {
        return None;
    }

    let colon_index = line.find(':')?;
    let key = line[..colon_index].trim();
    if key.is_empty() {
        return None;
    }

    Some((key.to_string(), line[colon_index + 1..].trim().to_string()))
}

fn strip_wrapping_quotes(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let first = bytes[0];
        let last = bytes[value.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}

fn is_frontmatter_delimiter(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && trimmed.len() >= 3 && trimmed.chars().all(|value| value == '-')
}

fn yaml_string(frontmatter: &BTreeMap<String, serde_yaml::Value>, key: &str) -> Option<String> {
    frontmatter
        .get(key)
        .and_then(|value| match value {
            serde_yaml::Value::String(value) => Some(value.trim().to_string()),
            serde_yaml::Value::Number(value) => Some(value.to_string()),
            serde_yaml::Value::Bool(value) => Some(value.to_string()),
            _ => None,
        })
        .filter(|value| !value.is_empty())
}

fn yaml_string_list(frontmatter: &BTreeMap<String, serde_yaml::Value>, key: &str) -> Vec<String> {
    match frontmatter.get(key) {
        Some(serde_yaml::Value::Sequence(items)) => items
            .iter()
            .filter_map(|item| match item {
                serde_yaml::Value::String(value) => Some(value.trim().to_string()),
                serde_yaml::Value::Number(value) => Some(value.to_string()),
                _ => None,
            })
            .filter(|value| !value.is_empty())
            .collect(),
        Some(serde_yaml::Value::String(value)) => split_csv(Some(value.clone())),
        _ => Vec::new(),
    }
}

fn split_csv(value: Option<String>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split(['、', '，', ',', ';'])
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

fn split_tags(value: Option<String>) -> Vec<String> {
    split_csv(value)
}

fn first_non_empty_paragraph(body: &str) -> Option<String> {
    let mut paragraph = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        if trimmed.starts_with('#') {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        paragraph.push(trimmed.to_string());
    }
    (!paragraph.is_empty()).then(|| paragraph.join(" "))
}

fn first_paragraph_after_heading(body: &str, heading: &str) -> Option<String> {
    let mut heading_found = false;
    let mut paragraph = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if !heading_found {
            let candidate = trimmed.trim_start_matches('#').trim();
            if trimmed.starts_with('#') && candidate == heading {
                heading_found = true;
            }
            continue;
        }
        if trimmed.is_empty() {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        if trimmed.starts_with('#') {
            break;
        }
        paragraph.push(trimmed.to_string());
    }
    (!paragraph.is_empty()).then(|| paragraph.join(" "))
}

fn resolve_builtin_tool_keys(values: Vec<String>, builtin_tool_keys: &[String]) -> Vec<String> {
    let catalog = builtin_tool_catalog();
    if values.iter().any(|value| value.eq_ignore_ascii_case("ALL")) {
        return builtin_tool_keys.to_vec();
    }
    let builtin_set = builtin_tool_keys.iter().cloned().collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .filter_map(|value| catalog.resolve(&value).map(|entry| entry.name.to_string()))
        .filter(|value| builtin_set.contains(value))
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn builtin_tool_keys() -> Vec<String> {
    builtin_tool_catalog().names()
}

fn hash_text(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("sha256-{:x}", hasher.finalize())
}

fn hash_bundle_files(files: &[(String, Vec<u8>)]) -> String {
    let mut hasher = Sha256::new();
    for (relative_path, bytes) in files {
        hasher.update(relative_path.as_bytes());
        hasher.update(b"\n");
        hasher.update(bytes);
        hasher.update(b"\n");
    }
    format!("sha256-{:x}", hasher.finalize())
}

fn hash_json_value(value: &JsonValue) -> Result<String, AppError> {
    Ok(hash_text(&serde_json::to_string(value)?))
}

pub(crate) fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256-{:x}", hasher.finalize())
}

fn short_hash(value: &str) -> String {
    value
        .rsplit('-')
        .next()
        .unwrap_or(value)
        .chars()
        .take(8)
        .collect()
}

fn slugify_skill_name(value: &str, fallback_prefix: &str) -> String {
    let mut slug = String::new();
    let mut last_was_separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            last_was_separator = false;
            continue;
        }
        if matches!(character, '-' | '_' | '.' | ' ') && !last_was_separator && !slug.is_empty() {
            slug.push('-');
            last_was_separator = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        format!("{fallback_prefix}-{}", short_hash(&hash_text(value)))
    } else {
        slug
    }
}

