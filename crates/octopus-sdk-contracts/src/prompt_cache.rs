use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptCacheEvent {
    pub cache_read_input_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub breakpoint_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheBreakpoint {
    pub position: usize,
    pub ttl: CacheTtl,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheTtl {
    FiveMinutes,
    OneHour,
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{CacheBreakpoint, CacheTtl, PromptCacheEvent};

    #[test]
    fn prompt_cache_event_serializes_expected_fields() {
        let event = PromptCacheEvent {
            cache_read_input_tokens: 21,
            cache_creation_input_tokens: 13,
            breakpoint_count: 2,
        };

        let value = serde_json::to_value(&event).expect("prompt cache event should serialize");

        assert_eq!(
            value.get("cache_read_input_tokens"),
            Some(&Value::Number(21_u32.into()))
        );
        assert_eq!(
            value.get("cache_creation_input_tokens"),
            Some(&Value::Number(13_u32.into()))
        );
        assert_eq!(
            value.get("breakpoint_count"),
            Some(&Value::Number(2_u32.into()))
        );
    }

    #[test]
    fn cache_breakpoint_serializes_ttl_variant_in_snake_case() {
        let breakpoint = CacheBreakpoint {
            position: 7,
            ttl: CacheTtl::FiveMinutes,
        };

        let value = serde_json::to_value(&breakpoint).expect("cache breakpoint should serialize");

        assert_eq!(
            value.get("ttl"),
            Some(&Value::String("five_minutes".into()))
        );
    }
}
