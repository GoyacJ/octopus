use std::{collections::HashMap, sync::Arc};

use octopus_core::{AppError, RuntimeExecutionSupport};

use super::{
    drivers::{
        AnthropicMessagesDriver, GeminiNativeDriver, OpenAiChatDriver, OpenAiResponsesDriver,
    },
    ProtocolDriver, ProtocolDriverCapability,
};

#[derive(Clone, Default)]
pub struct ModelDriverRegistry {
    drivers: HashMap<&'static str, Arc<dyn ProtocolDriver>>,
}

impl ModelDriverRegistry {
    pub fn new(drivers: Vec<Arc<dyn ProtocolDriver>>) -> Self {
        let drivers = drivers
            .into_iter()
            .map(|driver| (driver.protocol_family(), driver))
            .collect();
        Self { drivers }
    }

    pub fn installed() -> Self {
        Self::new(vec![
            Arc::new(AnthropicMessagesDriver),
            Arc::new(OpenAiChatDriver),
            Arc::new(OpenAiResponsesDriver),
            Arc::new(GeminiNativeDriver),
        ])
    }

    pub fn driver_for(&self, protocol_family: &str) -> Result<Arc<dyn ProtocolDriver>, AppError> {
        self.drivers.get(protocol_family).cloned().ok_or_else(|| {
            AppError::runtime(format!(
                "runtime execution does not support protocol family `{protocol_family}` yet"
            ))
        })
    }

    pub fn capability_for(
        &self,
        protocol_family: &str,
    ) -> Result<ProtocolDriverCapability, AppError> {
        Ok(self.driver_for(protocol_family)?.capability())
    }

    pub fn runtime_support_for(&self, protocol_family: &str) -> RuntimeExecutionSupport {
        self.drivers
            .get(protocol_family)
            .map(|driver| driver.capability().runtime_support())
            .unwrap_or_default()
    }
}
