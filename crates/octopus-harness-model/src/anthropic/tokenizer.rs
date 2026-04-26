use harness_contracts::{Message, MessagePart};

use crate::{ImageMeta, TokenCounter};

#[derive(Debug, Clone, Copy, Default)]
pub struct AnthropicTokenCounter;

impl TokenCounter for AnthropicTokenCounter {
    fn count_tokens(&self, text: &str, _model: &str) -> usize {
        text.chars().count().div_ceil(4)
    }

    fn count_messages(&self, messages: &[Message], model: &str) -> usize {
        messages
            .iter()
            .map(|message| {
                let parts = message
                    .parts
                    .iter()
                    .map(|part| match part {
                        MessagePart::Text(text) => self.count_tokens(text, model),
                        MessagePart::Thinking(thinking) => thinking
                            .text
                            .as_deref()
                            .map_or(0, |text| self.count_tokens(text, model)),
                        MessagePart::ToolUse { input, .. } => {
                            self.count_tokens(&input.to_string(), model)
                        }
                        MessagePart::ToolResult { content, .. } => {
                            self.count_tokens(&format!("{content:?}"), model)
                        }
                        MessagePart::Image { .. } => 0,
                        _ => 0,
                    })
                    .sum::<usize>();
                parts + 4
            })
            .sum::<usize>()
            + 2
    }

    fn count_image(&self, _image: &ImageMeta, _model: &str) -> Option<usize> {
        None
    }
}
