use std::{
    collections::HashMap,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use futures::{stream, StreamExt};
use octopus_sdk::{
    default_backend_for_host, register_builtins, AgentRuntime, AnthropicMessagesAdapter,
    AskAnswer, AskError, AskPrompt, AskResolver, AssistantEvent, ContentBlock,
    DefaultModelProvider, EventRange, GeminiNativeAdapter, Message, ModelCatalog, ModelError,
    ModelId, ModelProvider, ModelRequest, ModelStream, OpenAiChatAdapter,
    OpenAiResponsesAdapter, PermissionGate, PermissionMode, PermissionOutcome, ProtocolAdapter,
    ProtocolFamily, ProviderDescriptor, ProviderId, RegistryError, Role, SecretValue, SecretVault,
    SessionEvent, SessionHandle, SessionId, SqliteJsonlSessionStore, StartSessionInput,
    StopReason, SubmitTurnInput, ToolCallRequest, ToolRegistry, VaultError, VendorNativeAdapter,
};
use thiserror::Error;

const DEFAULT_CONFIG_SNAPSHOT_ID: &str = "octopus-cli:minimal";
const DEFAULT_TOKEN_BUDGET: u32 = 8_192;
const SCRIPTED_RESPONSE_ENV: &str = "OCTOPUS_CLI_SCRIPTED_RESPONSE";
const PERMISSION_MODE_ENV: &str = "OCTOPUS_CLI_PERMISSION_MODE";

#[derive(Debug, Error)]
pub enum CliError {
    #[error("usage: octopus-cli <working-dir> <model> <prompt>")]
    Usage,
    #[error(transparent)]
    Runtime(#[from] octopus_sdk::RuntimeError),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("{0}")]
    Setup(String),
}

#[derive(Debug, Clone)]
struct CliArgs {
    working_dir: PathBuf,
    model: ModelId,
    prompt: String,
    permission_mode: PermissionMode,
}

struct AllowAllGate;

#[async_trait]
impl PermissionGate for AllowAllGate {
    async fn check(&self, _call: &ToolCallRequest) -> PermissionOutcome {
        PermissionOutcome::Allow
    }
}

struct NoopAskResolver;

#[async_trait]
impl AskResolver for NoopAskResolver {
    async fn resolve(&self, _prompt_id: &str, _prompt: &AskPrompt) -> Result<AskAnswer, AskError> {
        Err(AskError::NotResolvable)
    }
}

struct EnvSecretVault;

#[async_trait]
impl SecretVault for EnvSecretVault {
    async fn get(&self, ref_id: &str) -> Result<SecretValue, VaultError> {
        let env_key = secret_env_key(ref_id);
        std::env::var(&env_key)
            .map(SecretValue::new)
            .map_err(|_| VaultError::NotFound)
    }

    async fn put(&self, _ref_id: &str, _value: SecretValue) -> Result<(), VaultError> {
        Err(VaultError::Backend("octopus-cli secret vault is read-only".into()))
    }
}

struct ScriptedModelProvider {
    response: String,
}

#[async_trait]
impl ModelProvider for ScriptedModelProvider {
    async fn complete(&self, _req: ModelRequest) -> Result<ModelStream, ModelError> {
        Ok(Box::pin(stream::iter(vec![
            Ok(AssistantEvent::TextDelta(self.response.clone())),
            Ok(AssistantEvent::MessageStop {
                stop_reason: StopReason::EndTurn,
            }),
        ])))
    }

    fn describe(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            id: ProviderId("scripted".into()),
            supported_families: vec![ProtocolFamily::VendorNative],
            catalog_version: "scripted".into(),
        }
    }
}

pub async fn run_once(
    runtime: Arc<AgentRuntime>,
    session: StartSessionInput,
    prompt: String,
) -> Result<SessionHandle, CliError> {
    let handle = runtime.start_session(session).await?;
    runtime
        .submit_turn(SubmitTurnInput {
            session_id: handle.session_id.clone(),
            message: Message {
                role: Role::User,
                content: vec![ContentBlock::Text { text: prompt }],
            },
        })
        .await?;
    Ok(handle)
}

pub async fn main_with_args<I, W>(args: I, out: &mut W) -> Result<(), CliError>
where
    I: IntoIterator<Item = String>,
    W: Write,
{
    let cli = parse_args(args)?;
    let runtime = build_runtime(&cli.working_dir)?;
    let handle = run_once(
        Arc::clone(&runtime),
        StartSessionInput {
            session_id: None,
            working_dir: cli.working_dir.clone(),
            permission_mode: cli.permission_mode,
            model: cli.model.clone(),
            config_snapshot_id: DEFAULT_CONFIG_SNAPSHOT_ID.into(),
            effective_config_hash: format!(
                "octopus-cli:{}:{:?}",
                cli.model.0, cli.permission_mode
            ),
            token_budget: DEFAULT_TOKEN_BUDGET,
        },
        cli.prompt,
    )
    .await?;

    print_session_events(runtime, &handle.session_id, out).await
}

pub async fn main_from_env() -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    main_with_args(std::env::args(), &mut lock).await
}

fn parse_args<I>(args: I) -> Result<CliArgs, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let _bin = args.next();
    let Some(first) = args.next() else {
        return Err(CliError::Usage);
    };

    if matches!(first.as_str(), "-h" | "--help") {
        return Err(CliError::Usage);
    }

    let Some(model) = args.next() else {
        return Err(CliError::Usage);
    };
    let prompt_parts: Vec<String> = args.collect();
    if prompt_parts.is_empty() {
        return Err(CliError::Usage);
    }

    Ok(CliArgs {
        working_dir: PathBuf::from(first),
        model: ModelId(model),
        prompt: prompt_parts.join(" "),
        permission_mode: parse_permission_mode()?,
    })
}

fn parse_permission_mode() -> Result<PermissionMode, CliError> {
    match std::env::var(PERMISSION_MODE_ENV) {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "default" => Ok(PermissionMode::Default),
            "accept_edits" | "accept-edits" => Ok(PermissionMode::AcceptEdits),
            "bypass_permissions" | "bypass-permissions" | "bypass" => {
                Ok(PermissionMode::BypassPermissions)
            }
            "plan" => Ok(PermissionMode::Plan),
            other => Err(CliError::Setup(format!(
                "invalid {PERMISSION_MODE_ENV}: {other}"
            ))),
        },
        Err(std::env::VarError::NotPresent) => Ok(PermissionMode::BypassPermissions),
        Err(error) => Err(CliError::Setup(format!(
            "failed to read {PERMISSION_MODE_ENV}: {error}"
        ))),
    }
}

fn build_runtime(working_dir: &Path) -> Result<Arc<AgentRuntime>, CliError> {
    let store = Arc::new(
        SqliteJsonlSessionStore::open(
            &working_dir.join("data/main.db"),
            &working_dir.join("runtime/events"),
        )
        .map_err(|error| CliError::Setup(error.to_string()))?,
    );
    let secret_vault: Arc<dyn SecretVault> = Arc::new(EnvSecretVault);
    let model_provider = build_model_provider(Arc::clone(&secret_vault));

    let mut tools = ToolRegistry::new();
    register_builtins(&mut tools)
        .map_err(|error: RegistryError| CliError::Setup(error.to_string()))?;

    let runtime = AgentRuntime::builder()
        .with_session_store(store)
        .with_model_provider(model_provider)
        .with_secret_vault(secret_vault)
        .with_tool_registry(tools)
        .with_permission_gate(Arc::new(AllowAllGate))
        .with_ask_resolver(Arc::new(NoopAskResolver))
        .with_sandbox_backend(default_backend_for_host())
        .build()?;

    Ok(Arc::new(runtime))
}

fn build_model_provider(secret_vault: Arc<dyn SecretVault>) -> Arc<dyn ModelProvider> {
    if let Ok(response) = std::env::var(SCRIPTED_RESPONSE_ENV) {
        return Arc::new(ScriptedModelProvider { response });
    }

    Arc::new(DefaultModelProvider::new(
        Arc::new(ModelCatalog::new_builtin()),
        default_protocol_adapters(),
        reqwest::Client::new(),
        secret_vault,
    ))
}

fn default_protocol_adapters() -> HashMap<ProtocolFamily, Arc<dyn ProtocolAdapter>> {
    HashMap::from([
        (
            ProtocolFamily::AnthropicMessages,
            Arc::new(AnthropicMessagesAdapter) as Arc<dyn ProtocolAdapter>,
        ),
        (
            ProtocolFamily::OpenAiChat,
            Arc::new(OpenAiChatAdapter) as Arc<dyn ProtocolAdapter>,
        ),
        (
            ProtocolFamily::OpenAiResponses,
            Arc::new(OpenAiResponsesAdapter) as Arc<dyn ProtocolAdapter>,
        ),
        (
            ProtocolFamily::GeminiNative,
            Arc::new(GeminiNativeAdapter) as Arc<dyn ProtocolAdapter>,
        ),
        (
            ProtocolFamily::VendorNative,
            Arc::new(VendorNativeAdapter) as Arc<dyn ProtocolAdapter>,
        ),
    ])
}

async fn print_session_events<W: Write>(
    runtime: Arc<AgentRuntime>,
    session_id: &SessionId,
    out: &mut W,
) -> Result<(), CliError> {
    let mut stream = runtime.events(session_id, EventRange::default()).await?;
    while let Some(event) = stream.next().await {
        render_event(
            event.map_err(|error| CliError::Setup(error.to_string()))?,
            out,
        )?;
    }
    Ok(())
}

fn render_event<W: Write>(event: SessionEvent, out: &mut W) -> Result<(), CliError> {
    match event {
        SessionEvent::SessionStarted { .. } => {
            writeln!(out, "[session.started]")?;
        }
        SessionEvent::AssistantMessage(message) if message.role == Role::Assistant => {
            for block in &message.content {
                render_content_block(block, out)?;
            }
        }
        SessionEvent::ToolExecuted {
            name,
            duration_ms,
            is_error,
            ..
        } => {
            writeln!(
                out,
                "[tool.executed] name={name} duration_ms={duration_ms} error={is_error}"
            )?;
        }
        SessionEvent::Render { block, lifecycle } => {
            writeln!(
                out,
                "[render.block] lifecycle={lifecycle:?} kind={:?} payload={}",
                block.kind,
                serde_json::to_string(&block.payload)
                    .map_err(|error| CliError::Setup(error.to_string()))?
            )?;
        }
        SessionEvent::Ask { prompt } => {
            writeln!(out, "[ask] kind={}", prompt.kind)?;
        }
        _ => {}
    }

    Ok(())
}

fn render_content_block<W: Write>(block: &ContentBlock, out: &mut W) -> Result<(), CliError> {
    match block {
        ContentBlock::Text { text } | ContentBlock::Thinking { text } => {
            if serde_json::from_str::<AssistantEvent>(text).is_err() && !text.trim().is_empty() {
                writeln!(out, "{text}")?;
            }
        }
        ContentBlock::ToolResult { content, .. } => {
            for nested in content {
                render_content_block(nested, out)?;
            }
        }
        ContentBlock::ToolUse { .. } => {}
    }

    Ok(())
}

fn secret_env_key(ref_id: &str) -> String {
    ref_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, OnceLock};

    use super::{main_with_args, CliError, SCRIPTED_RESPONSE_ENV};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[tokio::test]
    async fn main_with_args_prints_scripted_reply() {
        let _guard = env_lock()
            .lock()
            .expect("env lock should remain available");
        let root = tempfile::tempdir().expect("tempdir should exist");
        std::env::set_var(SCRIPTED_RESPONSE_ENV, "cli scripted reply");

        let mut out = Vec::new();
        let result = main_with_args(
            vec![
                "octopus-cli".into(),
                root.path().display().to_string(),
                "scripted-model".into(),
                "hello".into(),
            ],
            &mut out,
        )
        .await;

        std::env::remove_var(SCRIPTED_RESPONSE_ENV);
        result.expect("scripted cli path should succeed");

        let rendered = String::from_utf8(out).expect("stdout buffer should stay utf8");
        assert!(rendered.contains("[session.started]"));
        assert!(rendered.contains("cli scripted reply"));
    }

    #[tokio::test]
    async fn main_with_args_requires_model_and_prompt() {
        let mut out = Vec::new();
        let error = main_with_args(vec!["octopus-cli".into()], &mut out)
            .await
            .expect_err("missing args should fail");

        assert!(matches!(error, CliError::Usage));
    }
}
