use async_trait::async_trait;
use thiserror::Error;

use crate::AskPrompt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskAnswer {
    pub prompt_id: String,
    pub option_id: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AskError {
    #[error("ask prompt is not resolvable in the current host")]
    NotResolvable,
    #[error("ask prompt timed out before receiving an answer")]
    Timeout,
    #[error("ask prompt was cancelled before receiving an answer")]
    Cancelled,
}

#[async_trait]
pub trait AskResolver: Send + Sync {
    async fn resolve(&self, prompt_id: &str, prompt: &AskPrompt) -> Result<AskAnswer, AskError>;
}

#[cfg(test)]
mod tests {
    use super::{AskAnswer, AskError};

    #[test]
    fn ask_answer_keeps_prompt_and_option_binding() {
        let answer = AskAnswer {
            prompt_id: "prompt-1".into(),
            option_id: "approve".into(),
            text: "Proceed".into(),
        };

        assert_eq!(answer.prompt_id, "prompt-1");
        assert_eq!(answer.option_id, "approve");
        assert_eq!(answer.text, "Proceed");
    }

    #[test]
    fn ask_error_display_messages_are_stable() {
        assert_eq!(
            AskError::NotResolvable.to_string(),
            "ask prompt is not resolvable in the current host"
        );
        assert_eq!(
            AskError::Timeout.to_string(),
            "ask prompt timed out before receiving an answer"
        );
        assert_eq!(
            AskError::Cancelled.to_string(),
            "ask prompt was cancelled before receiving an answer"
        );
    }
}
