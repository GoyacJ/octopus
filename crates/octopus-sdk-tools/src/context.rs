use std::{path::PathBuf, sync::Arc};

use octopus_sdk_contracts::{
    AskResolver, EventSink, PermissionGate, SecretVault, SessionId, ToolCallId,
    ToolPermissionContext,
};
use octopus_sdk_hooks::HookRunner;
pub use octopus_sdk_sandbox::SandboxHandle;
use octopus_sdk_session::SessionStore;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct ToolUseContext {
    pub session_id: SessionId,
    pub tool_call_id: Option<ToolCallId>,
    pub permissions: Arc<dyn PermissionGate>,
    pub sandbox: SandboxHandle,
    pub session_store: Arc<dyn SessionStore>,
    pub secret_vault: Arc<dyn SecretVault>,
    pub ask_resolver: Arc<dyn AskResolver>,
    pub event_sink: Arc<dyn EventSink>,
    pub working_dir: PathBuf,
    pub hooks: Arc<HookRunner>,
    pub permission_context: ToolPermissionContext,
    pub cancellation: CancellationToken,
}

pub type ToolContext = ToolUseContext;

impl ToolUseContext {
    #[must_use]
    pub fn permission_context(&self) -> &ToolPermissionContext {
        &self.permission_context
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use async_trait::async_trait;
    use futures::stream;
    use octopus_sdk_contracts::{
        AskAnswer, AskError, AskPrompt, AskResolver, EventId, EventSink, PermissionGate,
        PermissionMode, PermissionOutcome, SecretValue, SecretVault, SessionEvent, SessionId,
        ToolCallId, ToolCallRequest, ToolPermissionContext, VaultError,
    };
    use octopus_sdk_hooks::HookRunner;
    use octopus_sdk_session::{
        EventRange, EventStream, SessionError, SessionSnapshot, SessionStore,
    };
    use tokio_util::sync::CancellationToken;

    use super::{SandboxHandle, ToolUseContext};

    struct MockPermissionGate;
    struct MockAskResolver;
    struct MockEventSink;
    struct MockSecretVault;
    struct MockSessionStore;

    #[async_trait]
    impl PermissionGate for MockPermissionGate {
        async fn check(&self, _call: &ToolCallRequest) -> PermissionOutcome {
            PermissionOutcome::Allow
        }
    }

    #[async_trait]
    impl AskResolver for MockAskResolver {
        async fn resolve(
            &self,
            prompt_id: &str,
            _prompt: &AskPrompt,
        ) -> Result<AskAnswer, AskError> {
            Ok(AskAnswer {
                prompt_id: prompt_id.to_string(),
                option_id: "approve".into(),
                text: "Proceed".into(),
            })
        }
    }

    impl EventSink for MockEventSink {
        fn emit(&self, _event: SessionEvent) {}
    }

    #[async_trait]
    impl SecretVault for MockSecretVault {
        async fn get(&self, _ref_id: &str) -> Result<SecretValue, VaultError> {
            Ok(SecretValue::new(b"secret"))
        }

        async fn put(&self, _ref_id: &str, _value: SecretValue) -> Result<(), VaultError> {
            Ok(())
        }
    }

    #[async_trait]
    impl SessionStore for MockSessionStore {
        async fn append(
            &self,
            _id: &SessionId,
            _event: SessionEvent,
        ) -> Result<EventId, SessionError> {
            Err(SessionError::NotFound)
        }

        async fn append_session_started(
            &self,
            _id: &SessionId,
            _working_dir: std::path::PathBuf,
            _permission_mode: octopus_sdk_contracts::PermissionMode,
            _model: String,
            _config_snapshot_id: String,
            _effective_config_hash: String,
            _token_budget: u32,
            _plugins_snapshot: Option<octopus_sdk_contracts::PluginsSnapshot>,
        ) -> Result<EventId, SessionError> {
            Err(SessionError::NotFound)
        }

        async fn new_child_session(
            &self,
            _parent_id: &SessionId,
            _spec: &octopus_sdk_contracts::SubagentSpec,
        ) -> Result<SessionId, SessionError> {
            Err(SessionError::NotFound)
        }

        async fn stream(
            &self,
            _id: &SessionId,
            _range: EventRange,
        ) -> Result<EventStream, SessionError> {
            Ok(Box::pin(stream::empty()))
        }

        async fn snapshot(&self, _id: &SessionId) -> Result<SessionSnapshot, SessionError> {
            Err(SessionError::NotFound)
        }

        async fn fork(&self, _id: &SessionId, _from: EventId) -> Result<SessionId, SessionError> {
            Err(SessionError::NotFound)
        }

        async fn wake(&self, _id: &SessionId) -> Result<SessionSnapshot, SessionError> {
            Err(SessionError::NotFound)
        }
    }

    #[test]
    fn tool_context_constructs_with_mock_services() {
        let context = ToolUseContext {
            session_id: SessionId("session-1".into()),
            tool_call_id: Some(ToolCallId("tool-1".into())),
            permissions: Arc::new(MockPermissionGate),
            sandbox: SandboxHandle::new(
                PathBuf::from("/tmp/workspace"),
                vec!["PATH".into(), "HOME".into()],
                "noop",
            ),
            session_store: Arc::new(MockSessionStore),
            secret_vault: Arc::new(MockSecretVault),
            ask_resolver: Arc::new(MockAskResolver),
            event_sink: Arc::new(MockEventSink),
            working_dir: PathBuf::from("/tmp/workspace"),
            hooks: Arc::new(HookRunner::new()),
            permission_context: ToolPermissionContext::for_mode(PermissionMode::Default),
            cancellation: CancellationToken::new(),
        };

        assert_eq!(context.session_id.0, "session-1");
        assert_eq!(
            context.tool_call_id.as_ref().map(|id| id.0.as_str()),
            Some("tool-1")
        );
        assert_eq!(context.sandbox.cwd(), PathBuf::from("/tmp/workspace"));
        assert_eq!(context.sandbox.env_allowlist(), ["PATH", "HOME"]);
        assert_eq!(context.working_dir, PathBuf::from("/tmp/workspace"));
        assert!(!context.cancellation.is_cancelled());
    }

    #[test]
    fn tool_permission_context_is_a_session_snapshot() {
        let context = ToolUseContext {
            session_id: SessionId("session-1".into()),
            tool_call_id: None,
            permissions: Arc::new(MockPermissionGate),
            sandbox: SandboxHandle::new(
                PathBuf::from("/tmp/workspace"),
                vec!["PATH".into()],
                "noop",
            ),
            session_store: Arc::new(MockSessionStore),
            secret_vault: Arc::new(MockSecretVault),
            ask_resolver: Arc::new(MockAskResolver),
            event_sink: Arc::new(MockEventSink),
            working_dir: PathBuf::from("/tmp/workspace"),
            hooks: Arc::new(HookRunner::new()),
            permission_context: ToolPermissionContext::for_mode(PermissionMode::Default),
            cancellation: CancellationToken::new(),
        };

        let permission = ToolPermissionContext::for_mode(PermissionMode::DontAsk);
        let context = ToolUseContext {
            permission_context: permission.clone(),
            ..context
        };

        assert_eq!(context.permission_context(), &permission);
        assert_eq!(context.permission_context().mode, PermissionMode::DontAsk);
        assert_eq!(
            context.permission_context().should_avoid_permission_prompts,
            Some(true)
        );
    }
}
