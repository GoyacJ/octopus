use std::sync::Arc;

use async_trait::async_trait;
use octopus_sdk_contracts::{
    ContentBlock, Message, Role, SessionEvent, SprintContract, SubagentError, SubagentOutput,
    Verdict,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ParentSessionContext, SubagentContext};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Draft {
    pub content: SubagentOutput,
    pub metadata: Value,
}

impl Draft {
    #[must_use]
    pub fn strip_thinking(&self) -> Self {
        let mut metadata = self.metadata.clone();
        if let Value::Object(map) = &mut metadata {
            map.remove("generator_thinking");
        }

        Self {
            content: self.content.clone(),
            metadata,
        }
    }
}

#[async_trait]
pub trait Planner: Send + Sync {
    async fn expand(&self, prompt: &str) -> Result<SprintContract, SubagentError>;
}

#[async_trait]
pub trait Generator: Send + Sync {
    async fn run(
        &self,
        contract: &SprintContract,
        feedback: Option<&Verdict>,
    ) -> Result<Draft, SubagentError>;
}

#[async_trait]
pub trait Evaluator: Send + Sync {
    async fn judge(&self, draft: &Draft) -> Result<Verdict, SubagentError>;
}

pub struct GeneratorEvaluator {
    planner: Arc<dyn Planner>,
    generator: Arc<dyn Generator>,
    evaluator: Arc<dyn Evaluator>,
    max_rounds: u16,
    evaluator_parent: Option<ParentSessionContext>,
}

impl GeneratorEvaluator {
    #[must_use]
    pub fn new(
        planner: Arc<dyn Planner>,
        generator: Arc<dyn Generator>,
        evaluator: Arc<dyn Evaluator>,
        max_rounds: u16,
    ) -> Self {
        Self {
            planner,
            generator,
            evaluator,
            max_rounds: max_rounds.max(1),
            evaluator_parent: None,
        }
    }

    #[must_use]
    pub fn with_evaluator_parent(mut self, parent: ParentSessionContext) -> Self {
        self.evaluator_parent = Some(parent);
        self
    }

    pub async fn run(&self, prompt: &str) -> Result<Draft, SubagentError> {
        let contract = self.planner.expand(prompt).await?;
        let mut feedback = None;

        for round in 0..self.max_rounds {
            let draft = self.generator.run(&contract, feedback.as_ref()).await?;
            let evaluator_draft = draft.strip_thinking();
            let verdict = self.judge_with_session(&evaluator_draft).await?;

            match verdict {
                Verdict::Pass { .. } => return Ok(evaluator_draft),
                Verdict::Fail { .. } if round + 1 == self.max_rounds => {
                    return Err(SubagentError::EvaluatorExhausted {
                        rounds: self.max_rounds,
                    });
                }
                Verdict::Fail { .. } => {
                    feedback = Some(verdict);
                }
            }
        }

        Err(SubagentError::EvaluatorExhausted {
            rounds: self.max_rounds,
        })
    }

    async fn judge_with_session(&self, draft: &Draft) -> Result<Verdict, SubagentError> {
        let Some(parent) = &self.evaluator_parent else {
            return self.evaluator.judge(draft).await;
        };

        let context = SubagentContext::for_evaluator(parent.clone(), draft);
        let session_id = context
            .session_store
            .new_child_session(&context.parent_session, &context.spec)
            .await
            .map_err(storage_error)?;
        context
            .session_store
            .append(
                &session_id,
                SessionEvent::UserMessage(draft_message(draft)?),
            )
            .await
            .map_err(storage_error)?;
        let verdict = self.evaluator.judge(draft).await?;
        context
            .session_store
            .append(
                &session_id,
                SessionEvent::AssistantMessage(verdict_message(&verdict)),
            )
            .await
            .map_err(storage_error)?;
        Ok(verdict)
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub struct MockEvaluator {
    rubric: Arc<dyn Fn(&Draft) -> Verdict + Send + Sync>,
}

#[cfg(any(test, feature = "test-utils"))]
impl MockEvaluator {
    #[must_use]
    pub fn new<F>(rubric: F) -> Self
    where
        F: Fn(&Draft) -> Verdict + Send + Sync + 'static,
    {
        Self {
            rubric: Arc::new(rubric),
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl Evaluator for MockEvaluator {
    async fn judge(&self, draft: &Draft) -> Result<Verdict, SubagentError> {
        Ok((self.rubric)(draft))
    }
}

fn draft_message(draft: &Draft) -> Result<Message, SubagentError> {
    Ok(Message {
        role: Role::User,
        content: vec![ContentBlock::Text {
            text: serde_json::to_string(draft).map_err(|error| SubagentError::Storage {
                reason: error.to_string(),
            })?,
        }],
    })
}

fn verdict_message(verdict: &Verdict) -> Message {
    let text = match verdict {
        Verdict::Pass { notes } => format!("pass: {}", notes.join("; ")),
        Verdict::Fail {
            reasons,
            next_actions,
        } => format!(
            "fail: {}; next: {}",
            reasons.join("; "),
            next_actions.join("; ")
        ),
    };

    Message {
        role: Role::Assistant,
        content: vec![ContentBlock::Text { text }],
    }
}

fn storage_error(error: octopus_sdk_session::SessionError) -> SubagentError {
    SubagentError::Storage {
        reason: error.to_string(),
    }
}
