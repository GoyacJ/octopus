//! Rule evaluation types for permission policy merging.

use octopus_sdk_contracts::{
    AskOption, AskPrompt, AskQuestion, PermissionMode, PermissionOutcome, ToolCallRequest,
    ToolCategory,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionBehavior {
    Allow,
    Deny,
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionRuleSource {
    UserSettings,
    ProjectSettings,
    LocalSettings,
    FlagSettings,
    PolicySettings,
    CliArg,
    Command,
    Session,
}

impl PermissionRuleSource {
    #[must_use]
    pub const fn priority(self) -> u8 {
        match self {
            Self::UserSettings => 0,
            Self::ProjectSettings => 1,
            Self::LocalSettings => 2,
            Self::FlagSettings => 3,
            Self::PolicySettings => 4,
            Self::CliArg => 5,
            Self::Command => 6,
            Self::Session => 7,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRule {
    pub source: PermissionRuleSource,
    pub behavior: PermissionBehavior,
    pub tool_name: String,
    pub rule_content: Option<String>,
}

impl PermissionRule {
    #[must_use]
    pub fn matches(&self, call: &ToolCallRequest) -> bool {
        if self.tool_name != "*" && self.tool_name != call.name {
            return false;
        }

        match normalize_rule_content(self.rule_content.as_deref()) {
            None | Some(RuleMatcher::Any) => true,
            Some(RuleMatcher::Exact(expected)) => extract_permission_subject(&call.input)
                .is_some_and(|candidate| candidate == expected),
            Some(RuleMatcher::Prefix(prefix)) => extract_permission_subject(&call.input)
                .is_some_and(|candidate| candidate.starts_with(prefix)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PermissionContext {
    pub call: ToolCallRequest,
    pub mode: PermissionMode,
    pub category: ToolCategory,
}

impl PermissionContext {
    #[must_use]
    pub fn new(call: ToolCallRequest, mode: PermissionMode, category: ToolCategory) -> Self {
        Self {
            call,
            mode,
            category,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PermissionPolicy {
    rules: Vec<PermissionRule>,
}

impl PermissionPolicy {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn from_sources(rules: Vec<PermissionRule>) -> Self {
        let mut indexed_rules = rules.into_iter().enumerate().collect::<Vec<_>>();
        indexed_rules.sort_by(|(left_idx, left_rule), (right_idx, right_rule)| {
            right_rule
                .source
                .priority()
                .cmp(&left_rule.source.priority())
                .then_with(|| left_idx.cmp(right_idx))
        });

        Self {
            rules: indexed_rules
                .into_iter()
                .map(|(_, rule)| rule)
                .collect::<Vec<_>>(),
        }
    }

    #[must_use]
    pub fn match_rules(
        &self,
        call: &ToolCallRequest,
    ) -> (
        Vec<&PermissionRule>,
        Vec<&PermissionRule>,
        Vec<&PermissionRule>,
    ) {
        let mut allow = Vec::new();
        let mut deny = Vec::new();
        let mut ask = Vec::new();

        for rule in &self.rules {
            if !rule.matches(call) {
                continue;
            }

            match rule.behavior {
                PermissionBehavior::Allow => allow.push(rule),
                PermissionBehavior::Deny => deny.push(rule),
                PermissionBehavior::Ask => ask.push(rule),
            }
        }

        (allow, deny, ask)
    }

    #[must_use]
    pub fn evaluate(&self, ctx: &PermissionContext) -> Option<PermissionOutcome> {
        let (allow_matches, deny_matches, ask_matches) = self.match_rules(&ctx.call);

        if let Some(rule) = deny_matches.first() {
            return Some(PermissionOutcome::Deny {
                reason: format!("tool '{}' denied by {:?} rule", ctx.call.name, rule.source),
            });
        }

        if !allow_matches.is_empty() {
            return Some(PermissionOutcome::Allow);
        }

        ask_matches
            .first()
            .map(|rule| PermissionOutcome::AskApproval {
                prompt: approval_prompt(rule, ctx),
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RuleMatcher<'a> {
    Any,
    Exact(&'a str),
    Prefix(&'a str),
}

fn normalize_rule_content(content: Option<&str>) -> Option<RuleMatcher<'_>> {
    match content.map(str::trim) {
        None => None,
        Some("") | Some("*") => Some(RuleMatcher::Any),
        Some(content) => content
            .strip_suffix(":*")
            .map_or(Some(RuleMatcher::Exact(content)), |prefix| {
                Some(RuleMatcher::Prefix(prefix))
            }),
    }
}

fn extract_permission_subject(input: &serde_json::Value) -> Option<&str> {
    match input {
        serde_json::Value::Object(object) => {
            for key in [
                "command",
                "path",
                "file_path",
                "filePath",
                "notebook_path",
                "notebookPath",
                "url",
                "pattern",
                "code",
                "message",
            ] {
                if let Some(value) = object.get(key).and_then(serde_json::Value::as_str) {
                    return Some(value);
                }
            }

            None
        }
        serde_json::Value::String(value) => (!value.trim().is_empty()).then_some(value.as_str()),
        _ => None,
    }
}

fn approval_prompt(rule: &PermissionRule, ctx: &PermissionContext) -> AskPrompt {
    AskPrompt {
        kind: "permission-approval".into(),
        questions: vec![AskQuestion {
            id: format!("permission-{}", ctx.call.id.0),
            header: "Permission approval".into(),
            question: format!(
                "Allow '{}' ({:?}) under {:?} mode? Rule source: {:?}.",
                ctx.call.name, ctx.category, ctx.mode, rule.source
            ),
            multi_select: false,
            options: vec![
                AskOption {
                    id: "approve".into(),
                    label: "Approve".into(),
                    description: "Allow this tool call.".into(),
                    preview: None,
                    preview_format: None,
                },
                AskOption {
                    id: "deny".into(),
                    label: "Deny".into(),
                    description: "Reject this tool call.".into(),
                    preview: None,
                    preview_format: None,
                },
            ],
        }],
    }
}
