use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::{PermissionMode, SessionId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubagentSpec {
    pub id: String,
    pub system_prompt: String,
    pub allowed_tools: Vec<String>,
    pub agent_role: String,
    pub model_role: String,
    pub permission_mode: PermissionMode,
    pub task_budget: TaskBudget,
    pub max_turns: u16,
    pub depth: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskBudget {
    pub total: u32,
    pub completion_threshold: f32,
}

impl Default for TaskBudget {
    fn default() -> Self {
        Self {
            total: 0,
            completion_threshold: 0.9,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentSummary {
    pub session_id: SessionId,
    pub parent_session_id: SessionId,
    pub resume_session_id: Option<SessionId>,
    pub spec_id: String,
    pub agent_role: String,
    pub parent_agent_role: String,
    pub turns: u16,
    pub tokens_used: u32,
    pub duration_ms: u64,
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: String,
    pub model_id: String,
    pub model_version: String,
    pub config_snapshot_id: String,
    pub permission_mode: PermissionMode,
    pub allowed_tools: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubagentOutput {
    Summary {
        text: String,
        meta: SubagentSummary,
    },
    FileRef {
        path: PathBuf,
        bytes: u64,
        meta: SubagentSummary,
    },
    Json {
        value: Value,
        meta: SubagentSummary,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubagentError {
    #[error("subagent depth exceeded: {depth}")]
    DepthExceeded { depth: u8 },
    #[error("subagent budget exceeded: used {used} of {total}")]
    BudgetExceeded { used: u32, total: u32 },
    #[error("evaluator exhausted after {rounds} rounds")]
    EvaluatorExhausted { rounds: u16 },
    #[error("subagent permission error: {reason}")]
    Permission { reason: String },
    #[error("subagent provider error: {reason}")]
    Provider { reason: String },
    #[error("subagent storage error: {reason}")]
    Storage { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SprintContract {
    pub scope: String,
    pub done_definition: String,
    pub out_of_scope: Vec<String>,
    pub invariants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Verdict {
    Pass {
        notes: Vec<String>,
    },
    Fail {
        reasons: Vec<String>,
        next_actions: Vec<String>,
    },
}
