use std::collections::BTreeMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{AskPrompt, ToolCallId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub id: ToolCallId,
    pub name: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionMode {
    Default,
    AcceptEdits,
    BypassPermissions,
    DontAsk,
    Auto,
    Bubble,
    Plan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
pub struct AdditionalWorkingDirectory {
    pub writable: bool,
}

pub type ToolPermissionRulesBySource = BTreeMap<PermissionRuleSource, Vec<String>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolPermissionContext {
    pub mode: PermissionMode,
    #[serde(default)]
    pub additional_working_directories: BTreeMap<String, AdditionalWorkingDirectory>,
    #[serde(default)]
    pub always_allow_rules: ToolPermissionRulesBySource,
    #[serde(default)]
    pub always_deny_rules: ToolPermissionRulesBySource,
    #[serde(default)]
    pub always_ask_rules: ToolPermissionRulesBySource,
    #[serde(default = "default_true")]
    pub is_bypass_permissions_mode_available: bool,
    #[serde(default)]
    pub is_auto_mode_available: Option<bool>,
    #[serde(default)]
    pub stripped_dangerous_rules: Option<ToolPermissionRulesBySource>,
    #[serde(default)]
    pub should_avoid_permission_prompts: Option<bool>,
    #[serde(default)]
    pub await_automated_checks_before_dialog: Option<bool>,
    #[serde(default)]
    pub pre_plan_mode: Option<PermissionMode>,
}

impl ToolPermissionContext {
    #[must_use]
    pub fn for_mode(mode: PermissionMode) -> Self {
        Self {
            mode,
            additional_working_directories: BTreeMap::new(),
            always_allow_rules: BTreeMap::new(),
            always_deny_rules: BTreeMap::new(),
            always_ask_rules: BTreeMap::new(),
            is_bypass_permissions_mode_available: true,
            is_auto_mode_available: Some(true),
            stripped_dangerous_rules: None,
            should_avoid_permission_prompts: Some(matches!(
                mode,
                PermissionMode::DontAsk | PermissionMode::Bubble
            )),
            await_automated_checks_before_dialog: Some(matches!(mode, PermissionMode::Auto)),
            pre_plan_mode: None,
        }
    }
}

impl Default for ToolPermissionContext {
    fn default() -> Self {
        Self::for_mode(PermissionMode::Default)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionOutcome {
    Allow,
    Deny { reason: String },
    AskApproval { prompt: AskPrompt },
    RequireAuth { prompt: AskPrompt },
}

#[async_trait]
pub trait PermissionGate: Send + Sync {
    async fn check(&self, call: &ToolCallRequest) -> PermissionOutcome;

    fn tool_permission_context(
        &self,
        mode: PermissionMode,
        tool_name: &str,
    ) -> ToolPermissionContext {
        let _ = tool_name;
        ToolPermissionContext::for_mode(mode)
    }
}

const fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use std::collections::BTreeMap;

    use super::{
        AdditionalWorkingDirectory, PermissionMode, PermissionOutcome, PermissionRuleSource,
        ToolCallRequest, ToolPermissionContext,
    };
    use crate::{AskOption, AskPrompt, AskQuestion, ToolCallId};

    #[test]
    fn permission_mode_variants_round_trip() {
        for (mode, expected) in [
            (PermissionMode::BypassPermissions, "bypass_permissions"),
            (PermissionMode::DontAsk, "dont_ask"),
            (PermissionMode::Auto, "auto"),
            (PermissionMode::Bubble, "bubble"),
        ] {
            let value = serde_json::to_value(mode).expect("permission mode should serialize");
            assert_eq!(value, Value::String(expected.into()));

            let roundtrip: PermissionMode =
                serde_json::from_value(value).expect("permission mode should deserialize");
            assert_eq!(roundtrip, mode);
        }
    }

    #[test]
    fn permission_outcome_ask_approval_keeps_prompt_payload() {
        let outcome = PermissionOutcome::AskApproval {
            prompt: AskPrompt {
                kind: "ask-user".into(),
                questions: vec![AskQuestion {
                    id: "question-1".into(),
                    question: "Proceed?".into(),
                    header: "Approval".into(),
                    multi_select: false,
                    options: vec![AskOption {
                        id: "approve".into(),
                        label: "Approve".into(),
                        description: "Allow the call".into(),
                        preview: None,
                        preview_format: None,
                    }],
                }],
            },
        };

        let value = serde_json::to_value(&outcome).expect("outcome should serialize");

        assert_eq!(
            value["ask_approval"]["prompt"]["questions"][0]["id"],
            "question-1"
        );
    }

    #[test]
    fn permission_outcome_require_auth_keeps_prompt_payload() {
        let outcome = PermissionOutcome::RequireAuth {
            prompt: AskPrompt {
                kind: "require-auth".into(),
                questions: vec![AskQuestion {
                    id: "question-1".into(),
                    question: "Sign in?".into(),
                    header: "Auth".into(),
                    multi_select: false,
                    options: vec![AskOption {
                        id: "approve".into(),
                        label: "Open OAuth".into(),
                        description: "Allow the tool to request authentication.".into(),
                        preview: None,
                        preview_format: None,
                    }],
                }],
            },
        };

        let value = serde_json::to_value(&outcome).expect("outcome should serialize");

        assert_eq!(
            value["require_auth"]["prompt"]["questions"][0]["header"],
            "Auth"
        );
    }

    #[test]
    fn tool_call_request_round_trips_minimal_shape() {
        let call = ToolCallRequest {
            id: ToolCallId("call-1".into()),
            name: "bash".into(),
            input: json!({ "command": "pwd" }),
        };

        let value = serde_json::to_value(&call).expect("tool call request should serialize");

        assert_eq!(value["id"], "call-1");
        assert_eq!(value["name"], "bash");
        assert_eq!(value["input"]["command"], "pwd");
    }

    #[test]
    fn tool_permission_context_round_trips_rules_by_source() {
        let mut allow_rules = BTreeMap::new();
        allow_rules.insert(
            PermissionRuleSource::Session,
            vec!["/tmp/workspace:*".into()],
        );
        let mut directories = BTreeMap::new();
        directories.insert(
            "/tmp/workspace".into(),
            AdditionalWorkingDirectory { writable: true },
        );
        let context = ToolPermissionContext {
            mode: PermissionMode::DontAsk,
            additional_working_directories: directories,
            always_allow_rules: allow_rules,
            always_deny_rules: BTreeMap::new(),
            always_ask_rules: BTreeMap::new(),
            is_bypass_permissions_mode_available: true,
            is_auto_mode_available: Some(true),
            stripped_dangerous_rules: None,
            should_avoid_permission_prompts: Some(true),
            await_automated_checks_before_dialog: Some(false),
            pre_plan_mode: Some(PermissionMode::Default),
        };

        let value =
            serde_json::to_value(&context).expect("tool permission context should serialize");

        assert_eq!(value["mode"], "dont_ask");
        assert_eq!(
            value["additionalWorkingDirectories"]["/tmp/workspace"]["writable"],
            true
        );
        assert_eq!(value["alwaysAllowRules"]["session"][0], "/tmp/workspace:*");
        assert_eq!(value["prePlanMode"], "default");
    }
}
