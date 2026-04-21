use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use octopus_sdk_contracts::{
    AskAnswer, AskError, AskPrompt, AskResolver, EventSink, PermissionGate, PermissionMode,
    PermissionOutcome, SessionEvent, ToolCallId, ToolCallRequest, ToolCategory,
};
use octopus_sdk_permissions::{
    ApprovalBroker, DefaultPermissionGate, PermissionBehavior, PermissionPolicy, PermissionRule,
    PermissionRuleSource,
};
use serde_json::json;

struct RecordingEventSink {
    events: Arc<Mutex<Vec<SessionEvent>>>,
}

impl EventSink for RecordingEventSink {
    fn emit(&self, event: SessionEvent) {
        self.events
            .lock()
            .expect("events mutex poisoned")
            .push(event);
    }
}

struct FixedAskResolver {
    option_id: &'static str,
}

#[async_trait]
impl AskResolver for FixedAskResolver {
    async fn resolve(&self, prompt_id: &str, _prompt: &AskPrompt) -> Result<AskAnswer, AskError> {
        Ok(AskAnswer {
            prompt_id: prompt_id.into(),
            option_id: self.option_id.into(),
            text: self.option_id.into(),
        })
    }
}

fn tool_call(name: &str, input: serde_json::Value) -> ToolCallRequest {
    ToolCallRequest {
        id: ToolCallId(format!("call-{name}")),
        name: name.into(),
        input,
    }
}

fn gate(
    mode: PermissionMode,
    rules: Vec<PermissionRule>,
    resolver_option: &'static str,
) -> DefaultPermissionGate {
    let events = Arc::new(Mutex::new(Vec::new()));
    let broker = Arc::new(ApprovalBroker::new(
        Arc::new(RecordingEventSink { events }),
        Arc::new(FixedAskResolver {
            option_id: resolver_option,
        }),
    ));

    DefaultPermissionGate::new(
        PermissionPolicy::from_sources(rules),
        mode,
        broker,
        Arc::new(|name| match name {
            "read_file" => ToolCategory::Read,
            "write_file" => ToolCategory::Write,
            "bash" => ToolCategory::Shell,
            "subagent" => ToolCategory::Subagent,
            _ => ToolCategory::Meta,
        }),
    )
}

#[tokio::test]
async fn deny_rule_wins_before_bypass_mode() {
    let gate = gate(
        PermissionMode::BypassPermissions,
        vec![PermissionRule {
            source: PermissionRuleSource::Session,
            behavior: PermissionBehavior::Deny,
            tool_name: "bash".into(),
            rule_content: Some("rm -rf /tmp/demo".into()),
        }],
        "approve",
    );

    let outcome = gate
        .check(&tool_call("bash", json!({ "command": "rm -rf /tmp/demo" })))
        .await;

    assert_eq!(
        outcome,
        PermissionOutcome::Deny {
            reason: "tool 'bash' denied by Session rule".into(),
        }
    );
}

#[tokio::test]
async fn allow_rule_wins_before_default_mode_prompt() {
    let gate = gate(
        PermissionMode::Default,
        vec![PermissionRule {
            source: PermissionRuleSource::ProjectSettings,
            behavior: PermissionBehavior::Allow,
            tool_name: "write_file".into(),
            rule_content: Some("/tmp/demo.txt".into()),
        }],
        "deny",
    );

    let outcome = gate
        .check(&tool_call("write_file", json!({ "path": "/tmp/demo.txt" })))
        .await;

    assert_eq!(outcome, PermissionOutcome::Allow);
}

#[tokio::test]
async fn ask_rule_runs_through_broker() {
    let gate = gate(
        PermissionMode::Default,
        vec![PermissionRule {
            source: PermissionRuleSource::FlagSettings,
            behavior: PermissionBehavior::Ask,
            tool_name: "bash".into(),
            rule_content: Some("npm install:*".into()),
        }],
        "approve",
    );

    let outcome = gate
        .check(&tool_call(
            "bash",
            json!({ "command": "npm install lodash" }),
        ))
        .await;

    assert_eq!(outcome, PermissionOutcome::Allow);
}

#[tokio::test]
async fn bypass_mode_allows_without_matching_rules() {
    let gate = gate(PermissionMode::BypassPermissions, Vec::new(), "deny");

    let outcome = gate
        .check(&tool_call("bash", json!({ "command": "rm demo.txt" })))
        .await;

    assert_eq!(outcome, PermissionOutcome::Allow);
}

#[tokio::test]
async fn plan_mode_denies_write_tools() {
    let gate = gate(PermissionMode::Plan, Vec::new(), "approve");

    let outcome = gate
        .check(&tool_call("write_file", json!({ "path": "/tmp/demo.txt" })))
        .await;

    assert_eq!(
        outcome,
        PermissionOutcome::Deny {
            reason: "tool 'write_file' is not allowed in plan mode".into(),
        }
    );
}

#[tokio::test]
async fn plan_mode_allows_read_tools() {
    let gate = gate(PermissionMode::Plan, Vec::new(), "deny");

    let outcome = gate
        .check(&tool_call("read_file", json!({ "path": "/tmp/demo.txt" })))
        .await;

    assert_eq!(outcome, PermissionOutcome::Allow);
}

#[tokio::test]
async fn default_mode_allows_read_tools() {
    let gate = gate(PermissionMode::Default, Vec::new(), "deny");

    let outcome = gate
        .check(&tool_call("read_file", json!({ "path": "/tmp/demo.txt" })))
        .await;

    assert_eq!(outcome, PermissionOutcome::Allow);
}

#[tokio::test]
async fn default_mode_prompts_for_write_tools() {
    let gate = gate(PermissionMode::Default, Vec::new(), "deny");

    let outcome = gate
        .check(&tool_call("write_file", json!({ "path": "/tmp/demo.txt" })))
        .await;

    assert_eq!(
        outcome,
        PermissionOutcome::Deny {
            reason: "tool 'write_file' denied by approval option 'deny'".into(),
        }
    );
}

#[tokio::test]
async fn accept_edits_allows_write_tools() {
    let gate = gate(PermissionMode::AcceptEdits, Vec::new(), "deny");

    let outcome = gate
        .check(&tool_call("write_file", json!({ "path": "/tmp/demo.txt" })))
        .await;

    assert_eq!(outcome, PermissionOutcome::Allow);
}

#[tokio::test]
async fn accept_edits_prompts_for_shell_tools() {
    let gate = gate(PermissionMode::AcceptEdits, Vec::new(), "approve");

    let outcome = gate
        .check(&tool_call("bash", json!({ "command": "git status" })))
        .await;

    assert_eq!(outcome, PermissionOutcome::Allow);
}

#[tokio::test]
async fn accept_edits_allows_subagent_tools() {
    let gate = gate(PermissionMode::AcceptEdits, Vec::new(), "deny");

    let outcome = gate
        .check(&tool_call("subagent", json!({ "task": "index repo" })))
        .await;

    assert_eq!(outcome, PermissionOutcome::Allow);
}

#[tokio::test]
async fn plan_mode_denies_subagent_tools() {
    let gate = gate(PermissionMode::Plan, Vec::new(), "approve");

    let outcome = gate
        .check(&tool_call("subagent", json!({ "task": "index repo" })))
        .await;

    assert_eq!(
        outcome,
        PermissionOutcome::Deny {
            reason: "tool 'subagent' is not allowed in plan mode".into(),
        }
    );
}
