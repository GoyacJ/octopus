use std::{collections::HashMap, sync::Arc};

use octopus_core::{AppError, RuntimeExecutionClass, RuntimeExecutionProfile};

use super::{
    drivers::{
        AnthropicMessagesDriver, GeminiNativeDriver, OpenAiChatDriver, OpenAiResponsesDriver,
    },
    ConversationModelDriver, GenerationModelDriver,
};

#[derive(Clone, Default)]
pub struct ModelDriverRegistry {
    conversation_drivers: HashMap<&'static str, Arc<dyn ConversationModelDriver>>,
    generation_drivers: HashMap<&'static str, Arc<dyn GenerationModelDriver>>,
}

impl ModelDriverRegistry {
    pub fn new(
        conversation_drivers: Vec<Arc<dyn ConversationModelDriver>>,
        generation_drivers: Vec<Arc<dyn GenerationModelDriver>>,
    ) -> Self {
        let conversation_drivers = conversation_drivers
            .into_iter()
            .map(|driver| (driver.protocol_family(), driver))
            .collect();
        let generation_drivers = generation_drivers
            .into_iter()
            .map(|driver| (driver.protocol_family(), driver))
            .collect();
        Self {
            conversation_drivers,
            generation_drivers,
        }
    }

    pub fn installed() -> Self {
        Self::new(
            vec![
                Arc::new(AnthropicMessagesDriver),
                Arc::new(OpenAiChatDriver),
            ],
            vec![
                Arc::new(AnthropicMessagesDriver),
                Arc::new(OpenAiChatDriver),
                Arc::new(OpenAiResponsesDriver),
                Arc::new(GeminiNativeDriver),
            ],
        )
    }

    pub fn conversation_driver_for(
        &self,
        protocol_family: &str,
    ) -> Result<Arc<dyn ConversationModelDriver>, AppError> {
        self.conversation_drivers
            .get(protocol_family)
            .cloned()
            .ok_or_else(|| {
                AppError::runtime(format!(
                    "runtime execution does not support protocol family `{protocol_family}` for conversation turns"
                ))
            })
    }

    pub fn generation_driver_for(
        &self,
        protocol_family: &str,
    ) -> Result<Arc<dyn GenerationModelDriver>, AppError> {
        self.generation_drivers
            .get(protocol_family)
            .cloned()
            .ok_or_else(|| {
                AppError::runtime(format!(
                    "runtime execution does not support protocol family `{protocol_family}` yet"
                ))
            })
    }

    pub fn execution_profile_for(&self, protocol_family: &str) -> RuntimeExecutionProfile {
        if let Some(driver) = self.conversation_drivers.get(protocol_family) {
            let profile = driver.execution_profile();
            if profile.execution_class == RuntimeExecutionClass::AgentConversation {
                return profile;
            }
        }

        self.generation_drivers
            .get(protocol_family)
            .map(|driver| driver.execution_profile())
            .unwrap_or_default()
    }
}
