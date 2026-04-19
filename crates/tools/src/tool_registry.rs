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
    source_kind: crate::capability_runtime::CapabilitySourceKind,
    description: String,
    permission: String,
    state: crate::capability_runtime::CapabilityState,
    requires_auth: bool,
    requires_approval: bool,
    deferred: bool,
    discovered: bool,
    activated: bool,
    exposed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct SearchableToolSpec {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) source_kind: crate::capability_runtime::CapabilitySourceKind,
    pub(crate) permission: String,
    pub(crate) state: crate::capability_runtime::CapabilityState,
    pub(crate) requires_auth: bool,
    pub(crate) requires_approval: bool,
    pub(crate) deferred: bool,
    pub(crate) discovered: bool,
    pub(crate) activated: bool,
    pub(crate) exposed: bool,
    pub(crate) search_hint: Option<String>,
}

impl SearchableToolSpec {
    pub(crate) fn from_capability(
        capability: crate::capability_runtime::CapabilitySpec,
        session_state: &crate::SessionCapabilityState,
    ) -> Self {
        let tool_name = capability.display_name.clone();
        Self {
            name: tool_name.clone(),
            description: capability.description,
            source_kind: capability.source_kind,
            permission: capability
                .permission_profile
                .required_permission
                .as_str()
                .to_string(),
            state: capability.state,
            requires_auth: capability.invocation_policy.requires_auth,
            requires_approval: capability.invocation_policy.requires_approval,
            deferred: capability.visibility
                == crate::capability_runtime::CapabilityVisibility::Deferred,
            discovered: session_state.is_tool_discovered(&tool_name),
            activated: session_state.is_tool_activated(&tool_name),
            exposed: session_state.is_tool_exposed(&tool_name),
            search_hint: capability.search_hint,
        }
    }

    pub(crate) fn to_search_result(&self) -> ToolSearchResult {
        ToolSearchResult {
            name: self.name.clone(),
            source_kind: self.source_kind,
            description: self.description.clone(),
            permission: self.permission.clone(),
            state: self.state,
            requires_auth: self.requires_auth,
            requires_approval: self.requires_approval,
            deferred: self.deferred,
            discovered: self.discovered,
            activated: self.activated,
            exposed: self.exposed,
            search_hint: self.search_hint.clone(),
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn execute_tool_search(input: ToolSearchInput) -> ToolSearchOutput {
    crate::CapabilityRuntime::builtin().search(
        &input.query,
        input.max_results.unwrap_or(5),
        crate::CapabilityPlannerInput::default(),
        None,
        None,
    )
}

#[allow(dead_code)]
pub(crate) fn deferred_tool_specs() -> Vec<ToolSpec> {
    let surface = crate::capability_runtime::plan_effective_capability_surface(
        crate::capability_runtime::compile_capability_specs(
            crate::builtin_catalog::builtin_capability_catalog()
                .entries()
                .to_vec(),
            &[],
            &[],
            &[],
            None,
        ),
        None,
        None,
    );
    let deferred_names = surface
        .deferred_tools
        .into_iter()
        .map(|capability| capability.display_name)
        .collect::<BTreeSet<_>>();

    mvp_tool_specs()
        .into_iter()
        .filter(|spec| deferred_names.contains(spec.name))
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
