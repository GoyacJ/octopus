#![cfg(feature = "rule-engine")]

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use futures::StreamExt;
use harness_contracts::{
    Decision, DecisionScope, FallbackPolicy, InteractivityLevel, PermissionMode, PermissionSubject,
    RequestId, RuleSource, SessionId, Severity, TenantId, ToolUseId,
};
use harness_permission::{
    AdminRuleProvider, FileRuleProvider, InMemoryRuleProvider, InlineRuleProvider,
    PermissionBroker, PermissionContext, PermissionRequest, PermissionRule, RuleAction,
    RuleEngineBroker, RuleProvider, RuleSnapshot,
};

#[tokio::test]
async fn rule_engine_orders_by_source_then_priority() {
    let broker = RuleEngineBroker::builder()
        .with_rule_provider(Arc::new(InlineRuleProvider::new(
            "workspace",
            RuleSource::Workspace,
            vec![
                rule("workspace-low", RuleSource::Workspace, 10, RuleAction::Deny),
                rule(
                    "workspace-high",
                    RuleSource::Workspace,
                    20,
                    RuleAction::Allow,
                ),
            ],
        )))
        .with_rule_provider(Arc::new(InlineRuleProvider::new(
            "session",
            RuleSource::Session,
            vec![rule("session", RuleSource::Session, 1, RuleAction::Deny)],
        )))
        .build()
        .await
        .unwrap();

    let snapshot = broker.current_snapshot();
    let rules: Vec<_> = snapshot.rules.iter().map(|rule| rule.id.as_str()).collect();

    assert_eq!(rules, vec!["session", "workspace-high", "workspace-low"]);
}

#[tokio::test]
async fn policy_deny_overrides_session_allow() {
    let broker = RuleEngineBroker::builder()
        .with_rules(vec![
            rule("policy-deny", RuleSource::Policy, 1, RuleAction::Deny),
            rule("session-allow", RuleSource::Session, 100, RuleAction::Allow),
        ])
        .build()
        .await
        .unwrap();

    assert_eq!(
        broker
            .decide(
                permission_request(),
                permission_context(InteractivityLevel::FullyInteractive)
            )
            .await,
        Decision::DenyOnce
    );
}

#[tokio::test]
async fn rule_hit_returns_decision() {
    let broker = RuleEngineBroker::builder()
        .with_rules(vec![rule(
            "workspace-allow",
            RuleSource::Workspace,
            10,
            RuleAction::Allow,
        )])
        .build()
        .await
        .unwrap();

    assert_eq!(
        broker
            .decide(
                permission_request(),
                permission_context(InteractivityLevel::FullyInteractive)
            )
            .await,
        Decision::AllowOnce
    );
}

#[tokio::test]
async fn no_match_uses_fallback_policy() {
    let ask_broker = RuleEngineBroker::builder().build().await.unwrap();
    assert_eq!(
        ask_broker
            .decide(
                permission_request(),
                permission_context(InteractivityLevel::FullyInteractive)
            )
            .await,
        Decision::Escalate
    );

    let deny_broker = RuleEngineBroker::builder()
        .with_fallback(FallbackPolicy::DenyAll)
        .build()
        .await
        .unwrap();
    assert_eq!(
        deny_broker
            .decide(
                permission_request(),
                permission_context(InteractivityLevel::FullyInteractive)
            )
            .await,
        Decision::DenyOnce
    );
}

#[tokio::test]
async fn ask_with_default_depends_on_interactivity() {
    let broker = RuleEngineBroker::builder()
        .with_rules(vec![rule(
            "ask",
            RuleSource::Workspace,
            10,
            RuleAction::AskWithDefault(Decision::DenyPermanent),
        )])
        .build()
        .await
        .unwrap();

    assert_eq!(
        broker
            .decide(
                permission_request(),
                permission_context(InteractivityLevel::NoInteractive)
            )
            .await,
        Decision::DenyPermanent
    );
    assert_eq!(
        broker
            .decide(
                permission_request(),
                permission_context(InteractivityLevel::FullyInteractive)
            )
            .await,
        Decision::Escalate
    );
}

#[tokio::test]
async fn admin_rule_provider_rejects_non_policy_rules() {
    let provider = AdminRuleProvider::new(vec![rule(
        "bad-admin-rule",
        RuleSource::Workspace,
        1,
        RuleAction::Deny,
    )]);

    let err = match RuleEngineBroker::builder()
        .with_rule_provider(Arc::new(provider))
        .build()
        .await
    {
        Ok(_) => panic!("admin provider should reject non-Policy rules"),
        Err(err) => err,
    };

    assert!(err.to_string().contains("Policy"));
}

#[tokio::test]
async fn memory_provider_replace_rules_reload_updates_snapshot() {
    let provider = Arc::new(InMemoryRuleProvider::new(
        "memory",
        RuleSource::Session,
        Vec::new(),
    ));
    let broker = RuleEngineBroker::builder()
        .with_rule_provider(provider.clone())
        .build()
        .await
        .unwrap();

    assert_eq!(broker.current_snapshot().generation, 1);
    provider.replace_rules(vec![rule(
        "session-allow",
        RuleSource::Session,
        1,
        RuleAction::Allow,
    )]);
    broker.reload().await.unwrap();

    let snapshot = broker.current_snapshot();
    assert_eq!(snapshot.generation, 2);
    assert_eq!(snapshot.rules[0].id, "session-allow");
}

#[tokio::test]
async fn file_rule_provider_reads_json_toml_and_watches_changes() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("rules.json");
    let toml_path = dir.path().join("rules.toml");
    std::fs::write(
        &json_path,
        r#"[{
            "id": "json-allow",
            "priority": 10,
            "scope": { "tool_name": "shell" },
            "action": "allow",
            "source": "workspace"
        }]"#,
    )
    .unwrap();
    std::fs::write(
        &toml_path,
        r#"
            [[rules]]
            id = "toml-deny"
            priority = 20
            action = "deny"
            source = "project"

            [rules.scope]
            tool_name = "shell"
        "#,
    )
    .unwrap();

    let json_provider =
        FileRuleProvider::new("json", RuleSource::Workspace, json_path.clone()).unwrap();
    let toml_provider = FileRuleProvider::new("toml", RuleSource::Project, toml_path).unwrap();

    assert_eq!(
        json_provider.resolve_rules(TenantId::SHARED).await.unwrap()[0].id,
        "json-allow"
    );
    assert_eq!(
        toml_provider.resolve_rules(TenantId::SHARED).await.unwrap()[0].id,
        "toml-deny"
    );

    let mut updates = json_provider.watch().unwrap();
    std::fs::write(
        &json_path,
        r#"[{
            "id": "json-deny",
            "priority": 30,
            "scope": { "tool_name": "shell" },
            "action": "deny",
            "source": "workspace"
        }]"#,
    )
    .unwrap();

    let update = tokio::time::timeout(Duration::from_secs(5), updates.next())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(update.provider_id, "json");
    assert_eq!(update.new_rules[0].id, "json-deny");
}

fn rule(id: &str, source: RuleSource, priority: i32, action: RuleAction) -> PermissionRule {
    PermissionRule {
        id: id.to_owned(),
        priority,
        scope: DecisionScope::ToolName("shell".to_owned()),
        action,
        source,
    }
}

fn permission_request() -> PermissionRequest {
    let tenant_id = TenantId::SHARED;
    let session_id = SessionId::new();
    PermissionRequest {
        request_id: RequestId::new(),
        tenant_id,
        session_id,
        tool_use_id: ToolUseId::new(),
        tool_name: "shell".to_owned(),
        subject: PermissionSubject::CommandExec {
            command: "pwd".to_owned(),
            argv: vec!["pwd".to_owned()],
            cwd: None,
            fingerprint: None,
        },
        severity: Severity::Low,
        scope_hint: DecisionScope::ToolName("shell".to_owned()),
        created_at: Utc::now(),
    }
}

fn permission_context(interactivity: InteractivityLevel) -> PermissionContext {
    PermissionContext {
        permission_mode: PermissionMode::Default,
        previous_mode: None,
        session_id: SessionId::new(),
        tenant_id: TenantId::SHARED,
        interactivity,
        timeout_policy: None,
        fallback_policy: FallbackPolicy::AskUser,
        rule_snapshot: Arc::new(RuleSnapshot {
            rules: Vec::new(),
            generation: 0,
            built_at: Utc::now(),
        }),
        hook_overrides: Vec::new(),
    }
}
