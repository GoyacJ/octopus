use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Value,
    pub required_permission: PermissionMode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeToolDefinition {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
    pub required_permission: PermissionMode,
}

#[allow(dead_code)]
pub(crate) fn permission_mode_from_plugin(value: &str) -> Result<PermissionMode, String> {
    match value {
        "read-only" => Ok(PermissionMode::ReadOnly),
        "workspace-write" => Ok(PermissionMode::WorkspaceWrite),
        "danger-full-access" => Ok(PermissionMode::DangerFullAccess),
        other => Err(format!("unsupported plugin permission: {other}")),
    }
}

#[must_use]
pub fn mvp_tool_specs() -> Vec<ToolSpec> {
    crate::builtin_catalog::builtin_capability_catalog().tool_specs()
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ToolSearchOutput {
    matches: Vec<String>,
    results: Vec<ToolSearchResult>,
    query: String,
    normalized_query: String,
    #[serde(rename = "total_deferred_tools")]
    total_deferred_tools: usize,
    #[serde(rename = "pending_mcp_servers")]
    pending_mcp_servers: Option<Vec<String>>,
    #[serde(rename = "mcp_degraded", skip_serializing_if = "Option::is_none")]
    mcp_degraded: Option<McpDegradedReport>,
}

impl ToolSearchOutput {
    #[must_use]
    pub(crate) fn new(
        matches: Vec<String>,
        results: Vec<ToolSearchResult>,
        query: String,
        normalized_query: String,
        total_deferred_tools: usize,
        pending_mcp_servers: Option<Vec<String>>,
        mcp_degraded: Option<McpDegradedReport>,
    ) -> Self {
        Self {
            matches,
            results,
            query,
            normalized_query,
            total_deferred_tools,
            pending_mcp_servers,
            mcp_degraded,
        }
    }

    #[must_use]
    pub fn matches(&self) -> &[String] {
        &self.matches
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ToolSearchResult {
    name: String,
    description: String,
    permission: String,
    deferred: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct SearchableToolSpec {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) permission: String,
    pub(crate) deferred: bool,
    pub(crate) search_hint: Option<String>,
}

impl SearchableToolSpec {
    pub(crate) fn from_builtin(entry: &crate::builtin_catalog::BuiltinCapability) -> Self {
        Self {
            name: entry.name.to_string(),
            description: entry.description.to_string(),
            permission: entry.required_permission.as_str().to_string(),
            deferred: matches!(
                entry.visibility,
                crate::builtin_catalog::BuiltinVisibility::Deferred
            ),
            search_hint: entry.search_hint.map(str::to_string),
        }
    }

    pub(crate) fn to_search_result(&self) -> ToolSearchResult {
        ToolSearchResult {
            name: self.name.clone(),
            description: self.description.clone(),
            permission: self.permission.clone(),
            deferred: self.deferred,
            search_hint: self.search_hint.clone(),
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn execute_tool_search(input: ToolSearchInput) -> ToolSearchOutput {
    let query = input.query;
    let normalized_query = normalize_tool_search_query(&query);
    let specs = crate::builtin_catalog::builtin_capability_catalog()
        .entries()
        .iter()
        .map(SearchableToolSpec::from_builtin)
        .collect::<Vec<_>>();
    let matches = search_tool_specs(&query, input.max_results.unwrap_or(5), &specs);
    let results = matches
        .iter()
        .filter_map(|name| specs.iter().find(|spec| spec.name == *name))
        .map(SearchableToolSpec::to_search_result)
        .collect::<Vec<_>>();
    let total_deferred_tools = specs.iter().filter(|spec| spec.deferred).count();
    ToolSearchOutput::new(
        matches,
        results,
        query,
        normalized_query,
        total_deferred_tools,
        None,
        None,
    )
}

#[allow(dead_code)]
pub(crate) fn deferred_tool_specs() -> Vec<ToolSpec> {
    crate::builtin_catalog::builtin_capability_catalog()
        .entries()
        .iter()
        .filter(|entry| {
            matches!(
                entry.visibility,
                crate::builtin_catalog::BuiltinVisibility::Deferred
            )
        })
        .map(crate::builtin_catalog::BuiltinCapability::to_tool_spec)
        .collect()
}

pub(crate) fn search_tool_specs(
    query: &str,
    max_results: usize,
    specs: &[SearchableToolSpec],
) -> Vec<String> {
    let lowered = query.to_lowercase();
    if let Some(selection) = lowered.strip_prefix("select:") {
        return selection
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .filter_map(|wanted| {
                let exact_wanted = wanted
                    .chars()
                    .filter(char::is_ascii_alphanumeric)
                    .flat_map(char::to_lowercase)
                    .collect::<String>();
                let wanted = canonical_tool_token(wanted);
                specs
                    .iter()
                    .find(|spec| {
                        spec.name
                            .chars()
                            .filter(char::is_ascii_alphanumeric)
                            .flat_map(char::to_lowercase)
                            .collect::<String>()
                            == exact_wanted
                    })
                    .or_else(|| {
                        specs
                            .iter()
                            .find(|spec| canonical_tool_token(&spec.name) == wanted)
                    })
                    .map(|spec| spec.name.clone())
            })
            .take(max_results)
            .collect();
    }

    let mut required = Vec::new();
    let mut optional = Vec::new();
    for term in lowered.split_whitespace() {
        if let Some(rest) = term.strip_prefix('+') {
            if !rest.is_empty() {
                required.push(rest);
            }
        } else {
            optional.push(term);
        }
    }
    let terms = if required.is_empty() {
        optional.clone()
    } else {
        required.iter().chain(optional.iter()).copied().collect()
    };

    let mut scored = specs
        .iter()
        .filter_map(|spec| {
            let name = spec.name.to_lowercase();
            let canonical_name = canonical_tool_token(&spec.name);
            let normalized_description = normalize_tool_search_query(&spec.description);
            let haystack = format!(
                "{name} {} {canonical_name}",
                spec.description.to_lowercase()
            );
            let normalized_haystack = format!("{canonical_name} {normalized_description}");
            if required.iter().any(|term| !haystack.contains(term)) {
                return None;
            }

            let mut score = 0_i32;
            for term in &terms {
                let canonical_term = canonical_tool_token(term);
                if haystack.contains(term) {
                    score += 2;
                }
                if name == *term {
                    score += 8;
                }
                if name.contains(term) {
                    score += 4;
                }
                if canonical_name == canonical_term {
                    score += 12;
                }
                if normalized_haystack.contains(&canonical_term) {
                    score += 3;
                }
            }

            if score == 0 && !lowered.is_empty() {
                return None;
            }
            Some((score, spec.name.clone()))
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    scored
        .into_iter()
        .map(|(_, name)| name)
        .take(max_results)
        .collect()
}

pub(crate) fn normalize_tool_search_query(query: &str) -> String {
    query
        .trim()
        .split(|ch: char| ch.is_whitespace() || ch == ',')
        .filter(|term| !term.is_empty())
        .map(canonical_tool_token)
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn canonical_tool_token(value: &str) -> String {
    let mut canonical = value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect::<String>();
    if let Some(stripped) = canonical.strip_suffix("tool") {
        canonical = stripped.to_string();
    }
    canonical
}
