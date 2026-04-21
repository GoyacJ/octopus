//! Permission gate implementation for canUseTool-style decisions.

use std::sync::Arc;

use async_trait::async_trait;
use octopus_sdk_contracts::{
    AskOption, AskPrompt, AskQuestion, PermissionGate, PermissionMode, PermissionOutcome,
    ToolCallRequest, ToolCategory,
};

use crate::{ApprovalBroker, PermissionBehavior, PermissionContext, PermissionPolicy};

#[derive(Clone)]
pub struct DefaultPermissionGate {
    pub policy: PermissionPolicy,
    pub mode: PermissionMode,
    pub broker: Arc<ApprovalBroker>,
    pub category_resolver: Arc<dyn Fn(&str) -> ToolCategory + Send + Sync>,
}

impl DefaultPermissionGate {
    #[must_use]
    pub fn new(
        policy: PermissionPolicy,
        mode: PermissionMode,
        broker: Arc<ApprovalBroker>,
        category_resolver: Arc<dyn Fn(&str) -> ToolCategory + Send + Sync>,
    ) -> Self {
        Self {
            policy,
            mode,
            broker,
            category_resolver,
        }
    }
}

#[async_trait]
impl PermissionGate for DefaultPermissionGate {
    async fn check(&self, call: &ToolCallRequest) -> PermissionOutcome {
        let category = (self.category_resolver)(call.name.as_str());
        let ctx = PermissionContext::new(call.clone(), self.mode, category);
        let (allow_matches, deny_matches, ask_matches) = self.policy.match_rules(call);

        if let Some(rule) = deny_matches.first() {
            return PermissionOutcome::Deny {
                reason: format!("tool '{}' denied by {:?} rule", call.name, rule.source),
            };
        }

        if !allow_matches.is_empty() {
            return PermissionOutcome::Allow;
        }

        if matches!(self.mode, PermissionMode::BypassPermissions) {
            return PermissionOutcome::Allow;
        }

        if matches!(self.mode, PermissionMode::Plan) && tool_has_side_effects(category) {
            return PermissionOutcome::Deny {
                reason: format!("tool '{}' is not allowed in plan mode", call.name),
            };
        }

        if !ask_matches.is_empty() {
            return self
                .broker
                .request_approval(call, ask_prompt(&ctx, PermissionBehavior::Ask))
                .await;
        }

        match default_mode_outcome(&ctx) {
            PermissionOutcome::AskApproval { prompt }
            | PermissionOutcome::RequireAuth { prompt } => {
                self.broker.request_approval(call, prompt).await
            }
            outcome => outcome,
        }
    }
}

fn default_mode_outcome(ctx: &PermissionContext) -> PermissionOutcome {
    match ctx.mode {
        PermissionMode::Default => {
            if matches!(ctx.category, ToolCategory::Read) {
                PermissionOutcome::Allow
            } else {
                PermissionOutcome::AskApproval {
                    prompt: ask_prompt(ctx, PermissionBehavior::Ask),
                }
            }
        }
        PermissionMode::AcceptEdits => match ctx.category {
            ToolCategory::Read | ToolCategory::Write | ToolCategory::Subagent => {
                PermissionOutcome::Allow
            }
            _ => PermissionOutcome::AskApproval {
                prompt: ask_prompt(ctx, PermissionBehavior::Ask),
            },
        },
        PermissionMode::BypassPermissions => PermissionOutcome::Allow,
        PermissionMode::Plan => {
            if matches!(ctx.category, ToolCategory::Read) {
                PermissionOutcome::Allow
            } else {
                PermissionOutcome::Deny {
                    reason: format!("tool '{}' is not allowed in plan mode", ctx.call.name),
                }
            }
        }
    }
}

fn tool_has_side_effects(category: ToolCategory) -> bool {
    !matches!(category, ToolCategory::Read)
}

fn ask_prompt(ctx: &PermissionContext, behavior: PermissionBehavior) -> AskPrompt {
    let header = match behavior {
        PermissionBehavior::Allow => "Permission allow",
        PermissionBehavior::Deny => "Permission deny",
        PermissionBehavior::Ask => "Permission approval",
    };

    AskPrompt {
        kind: match behavior {
            PermissionBehavior::Ask => "permission-approval".into(),
            PermissionBehavior::Allow => "permission-allow".into(),
            PermissionBehavior::Deny => "permission-deny".into(),
        },
        questions: vec![AskQuestion {
            id: format!("permission-{}", ctx.call.id.0),
            header: header.into(),
            question: format!(
                "Allow '{}' ({:?}) while mode is {:?}?",
                ctx.call.name, ctx.category, ctx.mode
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
