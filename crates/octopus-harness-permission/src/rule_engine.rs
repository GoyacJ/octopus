use std::sync::Arc;

use arc_swap::ArcSwap;
use async_trait::async_trait;
use chrono::Utc;
use harness_contracts::{
    Decision, DecisionId, DecisionScope, FallbackPolicy, InteractivityLevel, PermissionError,
    RuleSource, TenantId,
};

use crate::{
    InlineRuleProvider, NoopDecisionPersistence, PermissionBroker, PermissionContext,
    PermissionRequest, PermissionRule, RuleAction, RuleProvider, RuleSnapshot,
};

pub struct RuleEngineBroker {
    snapshot: ArcSwap<RuleSnapshot>,
    rule_providers: Vec<Arc<dyn RuleProvider>>,
    fallback: FallbackPolicy,
    tenant: TenantId,
    persistence: Arc<dyn crate::DecisionPersistence>,
}

pub struct RuleEngineBrokerBuilder {
    tenant: TenantId,
    rule_providers: Vec<Arc<dyn RuleProvider>>,
    fallback: FallbackPolicy,
}

impl RuleEngineBroker {
    pub fn builder() -> RuleEngineBrokerBuilder {
        RuleEngineBrokerBuilder {
            tenant: TenantId::SHARED,
            rule_providers: Vec::new(),
            fallback: FallbackPolicy::AskUser,
        }
    }

    pub async fn reload(&self) -> Result<(), PermissionError> {
        let generation = self.snapshot.load().generation + 1;
        let snapshot = build_snapshot(&self.rule_providers, self.tenant, generation).await?;
        self.snapshot.store(Arc::new(snapshot));
        Ok(())
    }

    pub fn current_snapshot(&self) -> Arc<RuleSnapshot> {
        self.snapshot.load_full()
    }
}

impl RuleEngineBrokerBuilder {
    #[must_use]
    pub fn with_tenant(mut self, tenant: TenantId) -> Self {
        self.tenant = tenant;
        self
    }

    #[must_use]
    pub fn with_rule_provider(mut self, provider: Arc<dyn RuleProvider>) -> Self {
        self.rule_providers.push(provider);
        self
    }

    #[must_use]
    pub fn with_rules(mut self, rules: Vec<PermissionRule>) -> Self {
        self.rule_providers.push(Arc::new(InlineRuleProvider::new(
            "inline",
            RuleSource::Session,
            rules,
        )));
        self
    }

    #[must_use]
    pub fn with_fallback(mut self, fallback: FallbackPolicy) -> Self {
        self.fallback = fallback;
        self
    }

    pub async fn build(self) -> Result<RuleEngineBroker, PermissionError> {
        let snapshot = build_snapshot(&self.rule_providers, self.tenant, 1).await?;
        Ok(RuleEngineBroker {
            snapshot: ArcSwap::from_pointee(snapshot),
            rule_providers: self.rule_providers,
            fallback: self.fallback,
            tenant: self.tenant,
            persistence: Arc::new(NoopDecisionPersistence),
        })
    }
}

#[async_trait]
impl PermissionBroker for RuleEngineBroker {
    async fn decide(&self, request: PermissionRequest, ctx: PermissionContext) -> Decision {
        let snapshot = self.current_snapshot();
        let Some(rule) = select_rule(&snapshot.rules, &request.scope_hint) else {
            return fallback_decision(self.fallback);
        };

        match &rule.action {
            RuleAction::Allow => Decision::AllowOnce,
            RuleAction::Deny => Decision::DenyOnce,
            RuleAction::AskWithDefault(default) => match ctx.interactivity {
                InteractivityLevel::NoInteractive => default.clone(),
                InteractivityLevel::FullyInteractive
                | InteractivityLevel::DeferredInteractive
                | _ => Decision::Escalate,
            },
        }
    }

    async fn persist(
        &self,
        decision_id: DecisionId,
        scope: DecisionScope,
    ) -> Result<(), PermissionError> {
        self.persistence.persist(decision_id, scope).await
    }
}

async fn build_snapshot(
    providers: &[Arc<dyn RuleProvider>],
    tenant: TenantId,
    generation: u64,
) -> Result<RuleSnapshot, PermissionError> {
    let mut rules = Vec::new();
    for provider in providers {
        let provider_rules = provider.resolve_rules(tenant).await?;
        if provider.source() == RuleSource::Policy {
            validate_policy_provider(provider.provider_id(), &provider_rules)?;
        }
        rules.extend(provider_rules);
    }

    rules.sort_by(compare_rules);
    Ok(RuleSnapshot {
        rules,
        generation,
        built_at: Utc::now(),
    })
}

fn validate_policy_provider(
    provider_id: &str,
    rules: &[PermissionRule],
) -> Result<(), PermissionError> {
    if let Some(rule) = rules.iter().find(|rule| rule.source != RuleSource::Policy) {
        return Err(PermissionError::Message(format!(
            "Policy provider `{provider_id}` returned non-Policy rule `{}`",
            rule.id
        )));
    }
    Ok(())
}

fn select_rule<'a>(
    rules: &'a [PermissionRule],
    scope: &DecisionScope,
) -> Option<&'a PermissionRule> {
    if let Some(policy_deny) = rules.iter().find(|rule| {
        rule.source == RuleSource::Policy
            && rule.scope == *scope
            && matches!(rule.action, RuleAction::Deny)
    }) {
        return Some(policy_deny);
    }

    rules.iter().find(|rule| rule.scope == *scope)
}

fn compare_rules(left: &PermissionRule, right: &PermissionRule) -> std::cmp::Ordering {
    source_rank(right.source)
        .cmp(&source_rank(left.source))
        .then_with(|| right.priority.cmp(&left.priority))
        .then_with(|| left.id.cmp(&right.id))
}

fn source_rank(source: RuleSource) -> u8 {
    const USER_RANK: u8 = 0;
    const UNKNOWN_RANK: u8 = 0;

    match source {
        RuleSource::User => USER_RANK,
        RuleSource::Workspace => 1,
        RuleSource::Project => 2,
        RuleSource::Local => 3,
        RuleSource::Flag => 4,
        RuleSource::Policy => 5,
        RuleSource::CliArg => 6,
        RuleSource::Command => 7,
        RuleSource::Session => 8,
        _ => UNKNOWN_RANK,
    }
}

fn fallback_decision(fallback: FallbackPolicy) -> Decision {
    match fallback {
        FallbackPolicy::DenyAll => Decision::DenyOnce,
        FallbackPolicy::AskUser
        | FallbackPolicy::AllowReadOnly
        | FallbackPolicy::ClosestMatchingRule
        | _ => Decision::Escalate,
    }
}
