use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceValue {
    String(String),
    I64(i64),
    U64(u64),
    Bool(bool),
    Json(serde_json::Value),
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TraceSpan {
    pub name: String,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub parent_span_id: Option<String>,
    pub agent_role: Option<String>,
    pub fields: BTreeMap<String, TraceValue>,
}

impl TraceSpan {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            trace_id: None,
            span_id: None,
            parent_span_id: None,
            agent_role: None,
            fields: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_field(mut self, key: impl Into<String>, value: TraceValue) -> Self {
        self.fields.insert(key.into(), value);
        self
    }

    #[must_use]
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    #[must_use]
    pub fn with_span_id(mut self, span_id: impl Into<String>) -> Self {
        self.span_id = Some(span_id.into());
        self
    }

    #[must_use]
    pub fn with_parent_span_id(mut self, parent_span_id: impl Into<String>) -> Self {
        self.parent_span_id = Some(parent_span_id.into());
        self
    }

    #[must_use]
    pub fn with_agent_role(mut self, agent_role: impl Into<String>) -> Self {
        self.agent_role = Some(agent_role.into());
        self
    }
}

#[must_use]
pub fn session_trace_id(session_id: &str) -> String {
    format!("trace:{session_id}")
}

#[must_use]
pub fn session_span_id(session_id: &str) -> String {
    format!("session:{session_id}")
}

#[must_use]
pub fn brain_iteration_span_id(session_id: &str, iteration: usize) -> String {
    format!("brain:{session_id}:{iteration}")
}

#[must_use]
pub fn tool_span_id(tool_call_id: &str) -> String {
    format!("tool:{tool_call_id}")
}

#[must_use]
pub fn subagent_span_id(session_id: &str) -> String {
    format!("subagent:{session_id}")
}

#[must_use]
pub fn subagent_permission_span_id(tool_call_id: &str) -> String {
    format!("subagent-permission:{tool_call_id}")
}

#[must_use]
pub fn stable_input_hash(value: &serde_json::Value) -> String {
    let bytes = value.to_string().into_bytes();
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;

    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }

    format!("{hash:016x}")
}

pub trait Tracer: Send + Sync {
    fn record(&self, span: TraceSpan);
}

#[derive(Debug, Default)]
pub struct NoopTracer;

impl Tracer for NoopTracer {
    fn record(&self, _span: TraceSpan) {}
}
