use octopus_sdk_contracts::{
    PermissionMode, PermissionOutcome, ToolCallId, ToolCallRequest, ToolCategory,
};
use octopus_sdk_permissions::{
    PermissionBehavior, PermissionContext, PermissionPolicy, PermissionRule, PermissionRuleSource,
};
use serde_json::json;

fn call(name: &str, input: serde_json::Value) -> ToolCallRequest {
    ToolCallRequest {
        id: ToolCallId(format!("call-{name}")),
        name: name.into(),
        input,
    }
}

fn context(call: ToolCallRequest) -> PermissionContext {
    PermissionContext::new(call, PermissionMode::Default, ToolCategory::Shell)
}

#[test]
fn test_source_priority_ordering() {
    let policy = PermissionPolicy::from_sources(vec![
        PermissionRule {
            source: PermissionRuleSource::UserSettings,
            behavior: PermissionBehavior::Allow,
            tool_name: "bash".into(),
            rule_content: Some("git status".into()),
        },
        PermissionRule {
            source: PermissionRuleSource::Session,
            behavior: PermissionBehavior::Allow,
            tool_name: "bash".into(),
            rule_content: Some("git status".into()),
        },
        PermissionRule {
            source: PermissionRuleSource::ProjectSettings,
            behavior: PermissionBehavior::Allow,
            tool_name: "bash".into(),
            rule_content: Some("git status".into()),
        },
    ]);

    let (allow_matches, deny_matches, ask_matches) =
        policy.match_rules(&call("bash", json!({ "command": "git status" })));

    assert!(deny_matches.is_empty());
    assert!(ask_matches.is_empty());
    assert_eq!(allow_matches.len(), 3);
    assert_eq!(allow_matches[0].source, PermissionRuleSource::Session);
    assert_eq!(
        allow_matches[1].source,
        PermissionRuleSource::ProjectSettings
    );
    assert_eq!(allow_matches[2].source, PermissionRuleSource::UserSettings);
}

#[test]
fn evaluate_returns_deny_when_deny_rule_matches() {
    let policy = PermissionPolicy::from_sources(vec![PermissionRule {
        source: PermissionRuleSource::ProjectSettings,
        behavior: PermissionBehavior::Deny,
        tool_name: "bash".into(),
        rule_content: Some("rm -rf /tmp/demo".into()),
    }]);

    let outcome = policy.evaluate(&context(call(
        "bash",
        json!({ "command": "rm -rf /tmp/demo" }),
    )));

    assert_eq!(
        outcome,
        Some(PermissionOutcome::Deny {
            reason: "tool 'bash' denied by ProjectSettings rule".into(),
        })
    );
}

#[test]
fn evaluate_returns_allow_when_allow_rule_matches() {
    let policy = PermissionPolicy::from_sources(vec![PermissionRule {
        source: PermissionRuleSource::LocalSettings,
        behavior: PermissionBehavior::Allow,
        tool_name: "bash".into(),
        rule_content: Some("git status".into()),
    }]);

    let outcome = policy.evaluate(&context(call("bash", json!({ "command": "git status" }))));

    assert_eq!(outcome, Some(PermissionOutcome::Allow));
}

#[test]
fn evaluate_returns_ask_prompt_when_ask_rule_matches() {
    let policy = PermissionPolicy::from_sources(vec![PermissionRule {
        source: PermissionRuleSource::FlagSettings,
        behavior: PermissionBehavior::Ask,
        tool_name: "bash".into(),
        rule_content: Some("npm install:*".into()),
    }]);

    let outcome = policy.evaluate(&context(call(
        "bash",
        json!({ "command": "npm install lodash" }),
    )));

    match outcome {
        Some(PermissionOutcome::AskApproval { prompt }) => {
            assert_eq!(prompt.kind, "permission-approval");
            assert_eq!(prompt.questions.len(), 1);
            assert!(prompt.questions[0].question.contains("FlagSettings"));
        }
        other => panic!("expected ask approval outcome, got {other:?}"),
    }
}

#[test]
fn evaluate_returns_none_when_no_rule_matches() {
    let policy = PermissionPolicy::new();

    let outcome = policy.evaluate(&context(call("bash", json!({ "command": "pwd" }))));

    assert_eq!(outcome, None);
}

#[test]
fn rules_by_source_bucket_only_the_selected_tool() {
    let policy = PermissionPolicy::from_sources(vec![
        PermissionRule {
            source: PermissionRuleSource::Session,
            behavior: PermissionBehavior::Allow,
            tool_name: "bash".into(),
            rule_content: Some("git status".into()),
        },
        PermissionRule {
            source: PermissionRuleSource::ProjectSettings,
            behavior: PermissionBehavior::Allow,
            tool_name: "write_file".into(),
            rule_content: Some("/tmp/demo.txt".into()),
        },
        PermissionRule {
            source: PermissionRuleSource::FlagSettings,
            behavior: PermissionBehavior::Allow,
            tool_name: "bash".into(),
            rule_content: None,
        },
    ]);

    let grouped = policy.rules_by_source("bash", PermissionBehavior::Allow);

    assert_eq!(grouped.len(), 2);
    assert_eq!(grouped[&PermissionRuleSource::Session], ["git status"]);
    assert_eq!(grouped[&PermissionRuleSource::FlagSettings], ["*"]);
    assert!(!grouped.contains_key(&PermissionRuleSource::ProjectSettings));
}
