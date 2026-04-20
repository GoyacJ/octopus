use std::ops::Add;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_creation_input_tokens: u32,
    pub cache_read_input_tokens: u32,
}

impl Add<&Usage> for &Usage {
    type Output = Usage;

    fn add(self, rhs: &Usage) -> Self::Output {
        Usage {
            input_tokens: self.input_tokens + rhs.input_tokens,
            output_tokens: self.output_tokens + rhs.output_tokens,
            cache_creation_input_tokens: self.cache_creation_input_tokens
                + rhs.cache_creation_input_tokens,
            cache_read_input_tokens: self.cache_read_input_tokens + rhs.cache_read_input_tokens,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Usage;

    #[test]
    fn usage_default_starts_with_zeroed_counters() {
        let usage = Usage::default();

        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.output_tokens, 0);
        assert_eq!(usage.cache_creation_input_tokens, 0);
        assert_eq!(usage.cache_read_input_tokens, 0);
    }

    #[test]
    fn usage_addition_accumulates_every_counter() {
        let left = Usage {
            input_tokens: 10,
            output_tokens: 5,
            cache_creation_input_tokens: 3,
            cache_read_input_tokens: 2,
        };
        let right = Usage {
            input_tokens: 7,
            output_tokens: 11,
            cache_creation_input_tokens: 13,
            cache_read_input_tokens: 17,
        };

        let total = &left + &right;

        assert_eq!(total.input_tokens, 17);
        assert_eq!(total.output_tokens, 16);
        assert_eq!(total.cache_creation_input_tokens, 16);
        assert_eq!(total.cache_read_input_tokens, 19);
    }

    #[test]
    fn usage_serializes_fields_in_declared_order() {
        let usage = Usage {
            input_tokens: 1,
            output_tokens: 2,
            cache_creation_input_tokens: 3,
            cache_read_input_tokens: 4,
        };

        let serialized = serde_json::to_string(&usage).expect("usage should serialize");

        assert!(
            serialized
                .find("\"input_tokens\"")
                .expect("input_tokens key should exist")
                < serialized
                    .find("\"output_tokens\"")
                    .expect("output_tokens key should exist")
        );
        assert!(
            serialized
                .find("\"output_tokens\"")
                .expect("output_tokens key should exist")
                < serialized
                    .find("\"cache_creation_input_tokens\"")
                    .expect("cache_creation_input_tokens key should exist")
        );
        assert!(
            serialized
                .find("\"cache_creation_input_tokens\"")
                .expect("cache_creation_input_tokens key should exist")
                < serialized
                    .find("\"cache_read_input_tokens\"")
                    .expect("cache_read_input_tokens key should exist")
        );
    }
}
