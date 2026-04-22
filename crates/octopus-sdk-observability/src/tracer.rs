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
    pub fields: BTreeMap<String, TraceValue>,
}

impl TraceSpan {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_field(mut self, key: impl Into<String>, value: TraceValue) -> Self {
        self.fields.insert(key.into(), value);
        self
    }
}

pub trait Tracer: Send + Sync {
    fn record(&self, span: TraceSpan);
}

#[derive(Debug, Default)]
pub struct NoopTracer;

impl Tracer for NoopTracer {
    fn record(&self, _span: TraceSpan) {}
}
