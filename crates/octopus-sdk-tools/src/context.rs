use std::{path::PathBuf, sync::Arc};

use octopus_sdk_contracts::{AskResolver, EventSink, PermissionGate, SecretVault, SessionId};
use octopus_sdk_session::SessionStore;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SandboxHandle {
    pub cwd: PathBuf,
    pub env_allowlist: Vec<String>,
}

#[derive(Clone)]
pub struct ToolContext {
    pub session_id: SessionId,
    pub permissions: Arc<dyn PermissionGate>,
    pub sandbox: SandboxHandle,
    pub session_store: Arc<dyn SessionStore>,
    pub secret_vault: Arc<dyn SecretVault>,
    pub ask_resolver: Arc<dyn AskResolver>,
    pub event_sink: Arc<dyn EventSink>,
    pub working_dir: PathBuf,
    pub cancellation: CancellationToken,
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use async_trait::async_trait;
    use futures::stream;
    use octopus_sdk_contracts::{
        AskAnswer, AskError, AskPrompt, AskResolver, EventId, EventSink, PermissionGate,
        PermissionOutcome, SecretValue, SecretVault, SessionEvent, SessionId, ToolCallRequest,
        VaultError,
    };
    use octopus_sdk_session::{
        EventRange, EventStream, SessionError, SessionSnapshot, SessionStore,
    };
    use tokio_util::sync::CancellationToken;

    use super::{SandboxHandle, ToolContext};

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
        let context = ToolContext {
            session_id: SessionId("session-1".into()),
            permissions: Arc::new(MockPermissionGate),
            sandbox: SandboxHandle {
                cwd: PathBuf::from("/tmp/workspace"),
                env_allowlist: vec!["PATH".into(), "HOME".into()],
            },
            session_store: Arc::new(MockSessionStore),
            secret_vault: Arc::new(MockSecretVault),
            ask_resolver: Arc::new(MockAskResolver),
            event_sink: Arc::new(MockEventSink),
            working_dir: PathBuf::from("/tmp/workspace"),
            cancellation: CancellationToken::new(),
        };

        assert_eq!(context.session_id.0, "session-1");
        assert_eq!(context.sandbox.cwd, PathBuf::from("/tmp/workspace"));
        assert_eq!(context.sandbox.env_allowlist, vec!["PATH", "HOME"]);
        assert_eq!(context.working_dir, PathBuf::from("/tmp/workspace"));
        assert!(!context.cancellation.is_cancelled());
    }
}
