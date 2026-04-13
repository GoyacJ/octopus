use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use runtime::{JsonValue as RuntimeJsonValue, Session};
use serde::{Deserialize, Serialize};

const CAPABILITY_RUNTIME_SESSION_EXTENSION_KEY: &str = "capability_runtime";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilityProfile {
    allowed_tools: BTreeSet<String>,
}

impl CapabilityProfile {
    #[must_use]
    pub fn from_tools(allowed_tools: BTreeSet<String>) -> Self {
        Self { allowed_tools }
    }

    #[must_use]
    pub fn allows_tool(&self, tool_name: &str) -> bool {
        self.allowed_tools.contains(tool_name)
    }

    #[must_use]
    pub fn allowed_tools(&self) -> &BTreeSet<String> {
        &self.allowed_tools
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityActivation {
    Tool(String),
}

impl CapabilityActivation {
    #[must_use]
    pub fn tool(name: impl Into<String>) -> Self {
        Self::Tool(name.into())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionCapabilityState {
    activated_tools: BTreeSet<String>,
    granted_tools: BTreeSet<String>,
    pending_tools: BTreeSet<String>,
    approved_tools: BTreeSet<String>,
    auth_resolved_tools: BTreeSet<String>,
    hidden_tools: BTreeSet<String>,
    injected_skill_messages: Vec<String>,
    skill_state_updates: Vec<crate::SkillStateUpdate>,
    model_override: Option<String>,
    effort_override: Option<String>,
}

impl SessionCapabilityState {
    pub fn persist_into_session(&self, session: &mut Session) -> Result<(), String> {
        let mut serialized_state = self.clone();
        serialized_state.refresh_fork_skill_state_updates();

        if serialized_state == Self::default() {
            session.remove_extension(CAPABILITY_RUNTIME_SESSION_EXTENSION_KEY);
            return Ok(());
        }

        let serialized =
            serde_json::to_value(&serialized_state).map_err(|error| error.to_string())?;
        session.set_extension(
            CAPABILITY_RUNTIME_SESSION_EXTENSION_KEY,
            runtime_json_from_serde_value(serialized),
        );
        Ok(())
    }

    pub fn restore_from_session(session: &Session) -> Result<Self, String> {
        let Some(value) = session.extension(CAPABILITY_RUNTIME_SESSION_EXTENSION_KEY) else {
            return Ok(Self::default());
        };

        let mut restored: Self = serde_json::from_value(serde_value_from_runtime_json(value))
            .map_err(|error| error.to_string())?;
        restored.record_restored_fork_skill_state_updates();
        restored.refresh_fork_skill_state_updates();
        Ok(restored)
    }

    pub fn activate(&mut self, activation: CapabilityActivation) {
        match activation {
            CapabilityActivation::Tool(name) => {
                self.activated_tools.insert(name);
            }
        }
    }

    #[must_use]
    pub fn is_tool_activated(&self, tool_name: &str) -> bool {
        self.activated_tools.contains(tool_name)
    }

    #[must_use]
    pub fn activated_tools(&self) -> &BTreeSet<String> {
        &self.activated_tools
    }

    pub fn grant_tool(&mut self, tool_name: impl Into<String>) {
        self.granted_tools.insert(tool_name.into());
    }

    #[must_use]
    pub fn is_tool_granted(&self, tool_name: &str) -> bool {
        self.granted_tools.contains(tool_name)
    }

    #[must_use]
    pub fn granted_tools(&self) -> &BTreeSet<String> {
        &self.granted_tools
    }

    pub fn push_injected_skill_message(&mut self, message: String) {
        if !message.trim().is_empty() {
            self.injected_skill_messages.push(message);
        }
    }

    pub fn mark_tool_pending(&mut self, tool_name: impl Into<String>) {
        self.pending_tools.insert(tool_name.into());
    }

    pub fn clear_tool_pending(&mut self, tool_name: &str) {
        self.pending_tools.remove(tool_name);
    }

    #[must_use]
    pub fn is_tool_pending(&self, tool_name: &str) -> bool {
        self.pending_tools.contains(tool_name)
    }

    #[must_use]
    pub fn pending_tools(&self) -> &BTreeSet<String> {
        &self.pending_tools
    }

    pub fn approve_tool(&mut self, tool_name: impl Into<String>) {
        let tool_name = tool_name.into();
        self.pending_tools.remove(&tool_name);
        self.approved_tools.insert(tool_name);
    }

    #[must_use]
    pub fn is_tool_approved(&self, tool_name: &str) -> bool {
        self.approved_tools.contains(tool_name)
    }

    #[must_use]
    pub fn approved_tools(&self) -> &BTreeSet<String> {
        &self.approved_tools
    }

    pub fn resolve_tool_auth(&mut self, tool_name: impl Into<String>) {
        let tool_name = tool_name.into();
        self.pending_tools.remove(&tool_name);
        self.auth_resolved_tools.insert(tool_name);
    }

    #[must_use]
    pub fn is_tool_auth_resolved(&self, tool_name: &str) -> bool {
        self.auth_resolved_tools.contains(tool_name)
    }

    #[must_use]
    pub fn auth_resolved_tools(&self) -> &BTreeSet<String> {
        &self.auth_resolved_tools
    }

    #[must_use]
    pub fn injected_skill_messages(&self) -> &[String] {
        &self.injected_skill_messages
    }

    #[must_use]
    pub fn skill_state_updates(&self) -> &[crate::SkillStateUpdate] {
        &self.skill_state_updates
    }

    pub fn set_model_override(&mut self, model: Option<String>) {
        self.model_override = model.filter(|value| !value.trim().is_empty());
    }

    #[must_use]
    pub fn model_override(&self) -> Option<&str> {
        self.model_override.as_deref()
    }

    pub fn set_effort_override(&mut self, effort: Option<String>) {
        self.effort_override = effort.filter(|value| !value.trim().is_empty());
    }

    #[must_use]
    pub fn effort_override(&self) -> Option<&str> {
        self.effort_override.as_deref()
    }

    pub fn apply_skill_execution_result(&mut self, result: &crate::SkillExecutionResult) {
        for tool in &result.tool_grants {
            self.grant_tool(tool.clone());
        }
        for message in result.injected_system_sections() {
            self.push_injected_skill_message(message);
        }
        self.apply_skill_state_updates(&result.state_updates);
        self.set_model_override(result.model_override.clone());
        self.set_effort_override(result.effort_override.clone());
    }

    pub fn apply_skill_state_updates(&mut self, updates: &[crate::SkillStateUpdate]) {
        self.skill_state_updates.extend(updates.iter().cloned());
    }

    fn refresh_fork_skill_state_updates(&mut self) {
        self.skill_state_updates
            .extend(crate::skill_runtime::reconcile_fork_lifecycle_updates(
                &self.skill_state_updates,
                false,
            ));
    }

    fn record_restored_fork_skill_state_updates(&mut self) {
        self.skill_state_updates
            .extend(crate::skill_runtime::reconcile_fork_lifecycle_updates(
                &self.skill_state_updates,
                true,
            ));
    }
}

pub type SharedSessionCapabilityState = Arc<Mutex<SessionCapabilityState>>;

#[derive(Debug, Clone, Default)]
pub struct SessionCapabilityStore {
    shared: SharedSessionCapabilityState,
}

impl SessionCapabilityStore {
    #[must_use]
    pub fn from_shared(shared: SharedSessionCapabilityState) -> Self {
        Self { shared }
    }

    #[must_use]
    pub fn shared(&self) -> &SharedSessionCapabilityState {
        &self.shared
    }

    #[must_use]
    pub fn snapshot(&self) -> SessionCapabilityState {
        self.mutate(|state| {
            state.refresh_fork_skill_state_updates();
            state.clone()
        })
    }

    pub fn with_state<T>(&self, f: impl FnOnce(&SessionCapabilityState) -> T) -> T {
        let state = self
            .shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&state)
    }

    pub fn mutate<T>(&self, f: impl FnOnce(&mut SessionCapabilityState) -> T) -> T {
        let mut state = self
            .shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&mut state)
    }

    pub fn persist_into_session(&self, session: &mut Session) -> Result<(), String> {
        self.mutate(|state| {
            state.refresh_fork_skill_state_updates();
            state.persist_into_session(session)
        })
    }

    pub fn restore_from_session(session: &Session) -> Result<Self, String> {
        Ok(Self::from_shared(Arc::new(Mutex::new(
            SessionCapabilityState::restore_from_session(session)?,
        ))))
    }

    pub fn activate(&self, activation: CapabilityActivation) {
        self.mutate(|state| state.activate(activation));
    }

    pub fn apply_skill_execution_result(&self, result: &crate::SkillExecutionResult) {
        self.mutate(|state| state.apply_skill_execution_result(result));
    }

    pub fn apply_skill_state_updates(&self, updates: &[crate::SkillStateUpdate]) {
        self.mutate(|state| state.apply_skill_state_updates(updates));
    }

    pub fn mark_tool_pending(&self, tool_name: impl Into<String>) {
        self.mutate(|state| state.mark_tool_pending(tool_name.into()));
    }

    pub fn clear_tool_pending(&self, tool_name: &str) {
        self.mutate(|state| state.clear_tool_pending(tool_name));
    }

    pub fn approve_tool(&self, tool_name: impl Into<String>) {
        self.mutate(|state| state.approve_tool(tool_name.into()));
    }

    pub fn resolve_tool_auth(&self, tool_name: impl Into<String>) {
        self.mutate(|state| state.resolve_tool_auth(tool_name.into()));
    }
}

pub type CapabilitySurfaceProjection = super::provider::CapabilitySurface;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityRequestOverride {
    pub model: String,
    pub system_sections: Vec<String>,
    pub reasoning_effort: Option<String>,
}

#[must_use]
pub fn apply_skill_session_overrides(
    base_model: &str,
    base_system_sections: Vec<String>,
    state: &SessionCapabilityState,
) -> CapabilityRequestOverride {
    let mut system_sections = base_system_sections;
    system_sections.extend(state.injected_skill_messages().iter().cloned());

    CapabilityRequestOverride {
        model: state.model_override().unwrap_or(base_model).to_string(),
        system_sections,
        reasoning_effort: state.effort_override().map(str::to_string),
    }
}

fn runtime_json_from_serde_value(value: serde_json::Value) -> RuntimeJsonValue {
    match value {
        serde_json::Value::Null => RuntimeJsonValue::Null,
        serde_json::Value::Bool(value) => RuntimeJsonValue::Bool(value),
        serde_json::Value::Number(value) => {
            RuntimeJsonValue::Number(value.as_i64().unwrap_or_default())
        }
        serde_json::Value::String(value) => RuntimeJsonValue::String(value),
        serde_json::Value::Array(values) => RuntimeJsonValue::Array(
            values
                .into_iter()
                .map(runtime_json_from_serde_value)
                .collect(),
        ),
        serde_json::Value::Object(entries) => RuntimeJsonValue::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key, runtime_json_from_serde_value(value)))
                .collect(),
        ),
    }
}

fn serde_value_from_runtime_json(value: &RuntimeJsonValue) -> serde_json::Value {
    match value {
        RuntimeJsonValue::Null => serde_json::Value::Null,
        RuntimeJsonValue::Bool(value) => serde_json::Value::Bool(*value),
        RuntimeJsonValue::Number(value) => serde_json::Value::Number((*value).into()),
        RuntimeJsonValue::String(value) => serde_json::Value::String(value.clone()),
        RuntimeJsonValue::Array(values) => {
            serde_json::Value::Array(values.iter().map(serde_value_from_runtime_json).collect())
        }
        RuntimeJsonValue::Object(entries) => serde_json::Value::Object(
            entries
                .iter()
                .map(|(key, value)| (key.clone(), serde_value_from_runtime_json(value)))
                .collect(),
        ),
    }
}
