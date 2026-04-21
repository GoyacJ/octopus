use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpLifecyclePhase {
    Starting,
    Ready,
    Degraded,
    Stopped,
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::McpLifecyclePhase;

    #[test]
    fn lifecycle_phase_serializes_in_snake_case() {
        let value = serde_json::to_value(McpLifecyclePhase::Ready).expect("phase should serialize");
        assert_eq!(value, Value::String("ready".into()));
    }
}
