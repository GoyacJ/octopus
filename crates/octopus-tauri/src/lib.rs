use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventBridgeConfig {
    pub channel: String,
    pub replay_last_event: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopBootstrap {
    pub default_window_label: String,
    pub local_hub_enabled: bool,
    pub event_bridge: EventBridgeConfig,
}

impl Default for DesktopBootstrap {
    fn default() -> Self {
        Self {
            default_window_label: "octopus-control-plane".into(),
            local_hub_enabled: true,
            event_bridge: EventBridgeConfig {
                channel: "octopus://runtime-events".into(),
                replay_last_event: true,
            },
        }
    }
}

