use std::{
    collections::HashMap,
    fmt::Write as _,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use futures::{stream, StreamExt};
use harness_context::{ContextEngine, ContextSessionView};
use harness_contracts::{
    CapabilityRegistry, ConfigHash, CorrelationId, DecidedBy, Decision, DecisionId, DecisionScope,
    EndReason, Event, EventId, FallbackPolicy, InteractivityLevel, MessageId as HarnessMessageId,
    MessagePart, MessageRole, ModelProvider as HarnessModelProvider, NoopRedactor,
    PermissionRequestedEvent, PermissionResolvedEvent, PermissionSubject, ProviderRestriction,
    RunEndedEvent, RunId, RunStartedEvent, StopReason as HarnessStopReason, TenantId,
    ToolDescriptor, ToolGroup, ToolOrigin, ToolProperties, ToolResult, ToolUseApprovedEvent,
    ToolUseCompletedEvent, ToolUseId, ToolUseRequestedEvent, TrustLevel,
};
use harness_journal::{EventStore, InMemoryEventStore};
use harness_model::{
    ApiMode, InferContext, MockProvider, ModelProvider as HarnessModelProviderTrait,
    ModelRequest as HarnessModelRequest,
};
use harness_permission::{PermissionBroker, PermissionContext, PermissionRequest, RuleSnapshot};
use harness_session::{Session, SessionOptions};
use harness_tool::{
    default_result_budget, BuiltinToolset, InterruptToken, OrchestratorContext,
    SchemaResolverContext, Tool, ToolCall, ToolContext, ToolEvent, ToolOrchestrator, ToolPool,
    ToolPoolFilter, ToolPoolModelProfile, ToolRegistry as HarnessToolRegistry, ToolSearchMode,
    ToolStream, ValidationError,
};
use octopus_sdk::{
    default_backend_for_host, register_builtins, AgentRuntime, AnthropicMessagesAdapter, AskAnswer,
    AskError, AskPrompt, AskResolver, AssistantEvent, ContentBlock, DefaultModelProvider,
    EventRange, GeminiNativeAdapter, Message, ModelCatalog, ModelError, ModelId, ModelProvider,
    ModelRequest, ModelStream, OpenAiChatAdapter, OpenAiResponsesAdapter, PermissionGate,
    PermissionMode, PermissionOutcome, ProtocolAdapter, ProtocolFamily, ProviderDescriptor,
    ProviderId, RegistryError, Role, SecretValue, SecretVault, SessionEvent, SessionHandle,
    SessionId, SqliteJsonlSessionStore, StartSessionInput, StopReason, SubmitTurnInput,
    ToolCallRequest, ToolRegistry, VaultError, VendorNativeAdapter,
};
use serde_json::{json, Value};
use thiserror::Error;
use tokio::sync::Mutex as TokioMutex;

use crate::automation::{
    render_slash_command_help, render_slash_command_help_detail, suggest_slash_commands,
    SlashCommand,
};
use crate::init::initialize_repo;
use crate::workspace::{handle_agents_slash_command, handle_skills_slash_command};

const CLI_CONFIG_SNAPSHOT_ID: &str = "octopus-cli:local-run";
const DEFAULT_TOKEN_BUDGET: u32 = 8_192;
const SCRIPTED_RESPONSE_ENV: &str = "OCTOPUS_CLI_SCRIPTED_RESPONSE";
const PERMISSION_MODE_ENV: &str = "OCTOPUS_CLI_PERMISSION_MODE";

#[derive(Debug, Error)]
pub enum CliError {
    #[error("{0}")]
    Usage(String),
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

#[derive(Debug, Clone)]
enum CliCommand {
    Help,
    Init { root: PathBuf },
    Slash { input: String, cwd: PathBuf },
    Agents { args: Option<String>, cwd: PathBuf },
    Skills { args: Option<String>, cwd: PathBuf },
    Run(CliArgs),
    HarnessRunOnce { prompt: String, cwd: PathBuf },
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
        Err(VaultError::Backend(
            "octopus-cli secret vault is read-only".into(),
        ))
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
    match parse_args(args)? {
        CliCommand::Help => writeln!(out, "{}", cli_usage()).map_err(CliError::from),
        CliCommand::Init { root } => {
            let report =
                initialize_repo(&root).map_err(|error| CliError::Setup(error.to_string()))?;
            writeln!(out, "{}", report.render()).map_err(CliError::from)
        }
        CliCommand::Slash { input, cwd } => {
            writeln!(out, "{}", execute_slash_command(&input, &cwd)?).map_err(CliError::from)
        }
        CliCommand::Agents { args, cwd } => writeln!(
            out,
            "{}",
            handle_agents_slash_command(args.as_deref(), &cwd)?
        )
        .map_err(CliError::from),
        CliCommand::Skills { args, cwd } => writeln!(
            out,
            "{}",
            handle_skills_slash_command(args.as_deref(), &cwd)?
        )
        .map_err(CliError::from),
        CliCommand::HarnessRunOnce { prompt, cwd } => run_harness_m3_once(&cwd, &prompt, out).await,
        CliCommand::Run(cli) => {
            let runtime = build_runtime(&cli.working_dir)?;
            let handle = run_once(
                Arc::clone(&runtime),
                StartSessionInput {
                    session_id: None,
                    working_dir: cli.working_dir.clone(),
                    permission_mode: cli.permission_mode,
                    model: cli.model.clone(),
                    config_snapshot_id: CLI_CONFIG_SNAPSHOT_ID.into(),
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
    }
}

pub async fn main_from_env() -> Result<(), CliError> {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    main_with_args(std::env::args(), &mut lock).await
}

fn parse_args<I>(args: I) -> Result<CliCommand, CliError>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let _bin = args.next();
    let Some(first) = args.next() else {
        return Ok(CliCommand::Help);
    };

    if matches!(first.as_str(), "-h" | "--help" | "help") {
        return Ok(CliCommand::Help);
    }

    if first == "init" {
        let root = match args.next() {
            Some(path) => {
                if args.next().is_some() {
                    return Err(CliError::Usage("usage: octopus-cli init [path]".into()));
                }
                PathBuf::from(path)
            }
            None => std::env::current_dir()?,
        };
        return Ok(CliCommand::Init { root });
    }

    if first == "slash" {
        let input = collect_remainder(args)?;
        return Ok(CliCommand::Slash {
            input,
            cwd: std::env::current_dir()?,
        });
    }

    if first.starts_with('/') && !first[1..].contains('/') {
        let mut input = vec![first];
        input.extend(args);
        return Ok(CliCommand::Slash {
            input: input.join(" "),
            cwd: std::env::current_dir()?,
        });
    }

    if first == "agents" {
        return Ok(CliCommand::Agents {
            args: join_optional_args(args),
            cwd: std::env::current_dir()?,
        });
    }

    if first == "skills" {
        return Ok(CliCommand::Skills {
            args: join_optional_args(args),
            cwd: std::env::current_dir()?,
        });
    }

    if first == "run" {
        let Some(flag) = args.next() else {
            return Err(CliError::Usage(
                "usage: octopus-cli run --once <prompt>".into(),
            ));
        };
        if flag != "--once" {
            return Err(CliError::Usage(
                "usage: octopus-cli run --once <prompt>".into(),
            ));
        }
        let prompt = collect_run_once_prompt(args)?;
        return Ok(CliCommand::HarnessRunOnce {
            prompt,
            cwd: std::env::current_dir()?,
        });
    }

    let Some(model) = args.next() else {
        return Err(CliError::Usage(cli_usage()));
    };
    let prompt_parts: Vec<String> = args.collect();
    if prompt_parts.is_empty() {
        return Err(CliError::Usage(cli_usage()));
    }

    Ok(CliCommand::Run(CliArgs {
        working_dir: PathBuf::from(first),
        model: ModelId(model),
        prompt: prompt_parts.join(" "),
        permission_mode: parse_permission_mode()?,
    }))
}

fn cli_usage() -> String {
    [
        "usage:",
        "  octopus-cli <working-dir> <model> <prompt>",
        "  octopus-cli run --once <prompt>",
        "  octopus-cli init [path]",
        "  octopus-cli agents [list|help]",
        "  octopus-cli skills [list|install <path>|help]",
        "  octopus-cli slash </command ...>",
        "  octopus-cli /help",
    ]
    .join("\n")
}

fn collect_run_once_prompt<I>(args: I) -> Result<String, CliError>
where
    I: IntoIterator<Item = String>,
{
    let prompt = args.into_iter().collect::<Vec<_>>().join(" ");
    if prompt.trim().is_empty() {
        return Err(CliError::Usage(
            "usage: octopus-cli run --once <prompt>".into(),
        ));
    }
    Ok(prompt)
}

fn join_optional_args<I>(args: I) -> Option<String>
where
    I: IntoIterator<Item = String>,
{
    let joined = args.into_iter().collect::<Vec<_>>().join(" ");
    (!joined.trim().is_empty()).then_some(joined)
}

fn collect_remainder<I>(args: I) -> Result<String, CliError>
where
    I: IntoIterator<Item = String>,
{
    let joined = args.into_iter().collect::<Vec<_>>().join(" ");
    if joined.trim().is_empty() {
        return Err(CliError::Usage(
            "usage: octopus-cli slash </command ...>".into(),
        ));
    }
    Ok(joined)
}

fn execute_slash_command(input: &str, cwd: &Path) -> Result<String, CliError> {
    let command = SlashCommand::parse(input).map_err(|error| CliError::Usage(error.to_string()))?;
    let Some(command) = command else {
        return Err(CliError::Usage("slash input must start with '/'".into()));
    };

    match command {
        SlashCommand::Help => Ok(render_slash_command_help()),
        SlashCommand::Init => {
            let report =
                initialize_repo(cwd).map_err(|error| CliError::Setup(error.to_string()))?;
            Ok(report.render())
        }
        SlashCommand::Agents { args } => {
            handle_agents_slash_command(args.as_deref(), cwd).map_err(CliError::from)
        }
        SlashCommand::Skills { args } => {
            handle_skills_slash_command(args.as_deref(), cwd).map_err(CliError::from)
        }
        SlashCommand::Unknown(name) => {
            let suggestions = suggest_slash_commands(&name, 3);
            let mut message = format!("Unknown slash command `/{name}`.");
            if !suggestions.is_empty() {
                let _ = write!(message, "\n\nDid you mean {}?", suggestions.join(", "));
            }
            message.push_str("\n\n");
            message.push_str(&render_slash_command_help());
            Ok(message)
        }
        other => {
            let detail = render_slash_command_help_detail(other.name())
                .unwrap_or_else(|| format!("/{}", other.name()));
            Ok(format!(
                "Slash command `/{}` is recognized but not implemented in octopus-cli yet.\n\n{}",
                other.name(),
                detail
            ))
        }
    }
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

async fn run_harness_m3_once<W: Write>(
    workspace_root: &Path,
    prompt: &str,
    out: &mut W,
) -> Result<(), CliError> {
    let workspace_root = workspace_root.canonicalize()?;
    let tenant_id = TenantId::SINGLE;
    let session_id = harness_contracts::SessionId::new();
    let event_store = Arc::new(InMemoryEventStore::new(Arc::new(NoopRedactor)));
    let session = Session::builder()
        .with_options(
            SessionOptions::new(&workspace_root)
                .with_tenant_id(tenant_id)
                .with_session_id(session_id),
        )
        .with_event_store(event_store.clone())
        .build()
        .await
        .map_err(|error| CliError::Setup(error.to_string()))?;
    let driver =
        HarnessM3RunOnceDriver::new(workspace_root, tenant_id, session_id, event_store).await?;

    let answer = driver
        .run_turn(&session, prompt)
        .await
        .map_err(CliError::Setup)?;
    writeln!(out, "[tool.executed] name=ListDir")?;
    writeln!(out, "{answer}")?;
    Ok(())
}

struct HarnessM3RunOnceDriver {
    workspace_root: PathBuf,
    tenant_id: TenantId,
    session_id: harness_contracts::SessionId,
    event_store: Arc<InMemoryEventStore>,
    context: ContextEngine,
    model: MockProvider,
    tools: ToolPool,
    broker: Arc<HarnessAllowBroker>,
}

impl HarnessM3RunOnceDriver {
    async fn new(
        workspace_root: PathBuf,
        tenant_id: TenantId,
        session_id: harness_contracts::SessionId,
        event_store: Arc<InMemoryEventStore>,
    ) -> Result<Self, CliError> {
        let context = ContextEngine::builder()
            .build()
            .map_err(|error| CliError::Setup(error.to_string()))?;
        let registry = HarnessToolRegistry::builder()
            .with_builtin_toolset(BuiltinToolset::Custom(vec![
                Box::new(CliListDirTool::new()),
            ]))
            .build()
            .map_err(|error| CliError::Setup(error.to_string()))?;
        let tools = ToolPool::assemble(
            &registry.snapshot(),
            &ToolPoolFilter::default(),
            &ToolSearchMode::Disabled,
            &ToolPoolModelProfile {
                provider: HarnessModelProvider("mock".to_owned()),
                supports_tool_reference: false,
                max_context_tokens: Some(DEFAULT_TOKEN_BUDGET),
            },
            &SchemaResolverContext {
                run_id: RunId::new(),
                session_id,
                tenant_id,
            },
        )
        .await
        .map_err(|error| CliError::Setup(error.to_string()))?;

        Ok(Self {
            workspace_root,
            tenant_id,
            session_id,
            event_store,
            context,
            model: MockProvider::default(),
            tools,
            broker: Arc::new(HarnessAllowBroker::default()),
        })
    }

    async fn run_turn(&self, session: &Session, prompt: &str) -> Result<String, String> {
        session
            .run_turn(prompt)
            .await
            .map_err(|error| error.to_string())?;

        let run_id = RunId::new();
        let user_message = harness_message(
            MessageRole::User,
            vec![MessagePart::Text(prompt.to_owned())],
        );
        let turn_input = harness_contracts::TurnInput {
            message: user_message.clone(),
            metadata: json!({ "cli": "run-once" }),
        };
        self.event_store
            .append(
                self.tenant_id,
                self.session_id,
                &[Event::RunStarted(RunStartedEvent {
                    run_id,
                    session_id: self.session_id,
                    tenant_id: self.tenant_id,
                    parent_run_id: None,
                    input: turn_input.clone(),
                    snapshot_id: session.snapshot_id(),
                    effective_config_hash: ConfigHash([0; 32]),
                    started_at: harness_contracts::now(),
                    correlation_id: CorrelationId::new(),
                })],
            )
            .await
            .map_err(|error| error.to_string())?;

        let prompt_view = CliPromptView {
            tenant_id: self.tenant_id,
            session_id: self.session_id,
            descriptors: self
                .tools
                .iter()
                .map(|tool| tool.descriptor().clone())
                .collect(),
        };
        let assembled = self
            .context
            .assemble(&prompt_view, &turn_input)
            .await
            .map_err(|error| error.to_string())?;
        let mut stream = self
            .model
            .infer(
                HarnessModelRequest {
                    model_id: "mock".to_owned(),
                    messages: assembled.messages,
                    tools: Some(assembled.tools_snapshot),
                    system: assembled.system,
                    temperature: None,
                    max_tokens: Some(256),
                    stream: true,
                    cache_breakpoints: assembled.cache_breakpoints,
                    api_mode: ApiMode::Responses,
                    extra: json!({ "prompt": prompt }),
                },
                InferContext::for_test(),
            )
            .await
            .map_err(|error| error.to_string())?;
        while let Some(_event) = stream.next().await {}

        let tool_call = ToolCall {
            tool_use_id: ToolUseId::new(),
            tool_name: "ListDir".to_owned(),
            input: json!({ "path": self.workspace_root }),
        };
        let descriptor = self
            .tools
            .descriptor(&tool_call.tool_name)
            .ok_or_else(|| "ListDir descriptor missing".to_owned())?
            .clone();
        self.event_store
            .append(
                self.tenant_id,
                self.session_id,
                &[
                    Event::UserMessageAppended(harness_contracts::UserMessageAppendedEvent {
                        run_id,
                        message_id: user_message.id,
                        content: harness_contracts::MessageContent::Text(prompt.to_owned()),
                        metadata: harness_contracts::MessageMetadata::default(),
                        at: harness_contracts::now(),
                    }),
                    Event::ToolUseRequested(ToolUseRequestedEvent {
                        run_id,
                        tool_use_id: tool_call.tool_use_id,
                        tool_name: tool_call.tool_name.clone(),
                        input: tool_call.input.clone(),
                        properties: descriptor.properties.clone(),
                        causation_id: EventId::new(),
                        at: harness_contracts::now(),
                    }),
                ],
            )
            .await
            .map_err(|error| error.to_string())?;

        let tool_results = ToolOrchestrator::default()
            .dispatch(vec![tool_call.clone()], self.orchestrator_context(run_id))
            .await;
        let result = tool_results
            .into_iter()
            .next()
            .ok_or_else(|| "ListDir result missing".to_owned())?;
        let tool_result = result.result.map_err(|error| error.to_string())?;
        let permission = self
            .broker
            .take_requests()
            .await
            .pop()
            .ok_or_else(|| "permission request missing".to_owned())?;
        let decision_id = DecisionId::new();
        self.event_store
            .append(
                self.tenant_id,
                self.session_id,
                &[
                    Event::PermissionRequested(PermissionRequestedEvent {
                        request_id: permission.request_id,
                        run_id,
                        session_id: self.session_id,
                        tenant_id: self.tenant_id,
                        tool_use_id: permission.tool_use_id,
                        tool_name: permission.tool_name.clone(),
                        subject: permission.subject.clone(),
                        severity: permission.severity,
                        scope_hint: permission.scope_hint.clone(),
                        fingerprint: None,
                        presented_options: vec![Decision::AllowOnce, Decision::DenyOnce],
                        interactivity: InteractivityLevel::NoInteractive,
                        causation_id: EventId::new(),
                        at: harness_contracts::now(),
                    }),
                    Event::PermissionResolved(PermissionResolvedEvent {
                        request_id: permission.request_id,
                        decision: Decision::AllowOnce,
                        decided_by: DecidedBy::Broker {
                            broker_id: "octopus-cli-m3-run-once".to_owned(),
                        },
                        scope: permission.scope_hint,
                        fingerprint: None,
                        rationale: None,
                        at: harness_contracts::now(),
                    }),
                    Event::ToolUseApproved(ToolUseApprovedEvent {
                        tool_use_id: tool_call.tool_use_id,
                        decision_id,
                        scope: DecisionScope::ToolName(tool_call.tool_name.clone()),
                        at: harness_contracts::now(),
                    }),
                    Event::ToolUseCompleted(ToolUseCompletedEvent {
                        tool_use_id: tool_call.tool_use_id,
                        result: tool_result.clone(),
                        usage: None,
                        duration_ms: result.duration.as_millis().min(u128::from(u64::MAX)) as u64,
                        at: harness_contracts::now(),
                    }),
                ],
            )
            .await
            .map_err(|error| error.to_string())?;

        let answer = list_dir_summary(&tool_result);
        self.event_store
            .append(
                self.tenant_id,
                self.session_id,
                &[
                    Event::AssistantMessageCompleted(
                        harness_contracts::AssistantMessageCompletedEvent {
                            run_id,
                            message_id: HarnessMessageId::new(),
                            content: harness_contracts::MessageContent::Text(answer.clone()),
                            tool_uses: vec![harness_contracts::ToolUseSummary {
                                tool_use_id: tool_call.tool_use_id,
                                tool_name: tool_call.tool_name.clone(),
                            }],
                            usage: harness_contracts::UsageSnapshot::default(),
                            pricing_snapshot_id: None,
                            stop_reason: HarnessStopReason::EndTurn,
                            at: harness_contracts::now(),
                        },
                    ),
                    Event::RunEnded(RunEndedEvent {
                        run_id,
                        reason: EndReason::Completed,
                        usage: Some(harness_contracts::UsageSnapshot::default()),
                        ended_at: harness_contracts::now(),
                    }),
                ],
            )
            .await
            .map_err(|error| error.to_string())?;

        Ok(answer)
    }

    fn orchestrator_context(&self, run_id: RunId) -> OrchestratorContext {
        OrchestratorContext {
            pool: self.tools.clone(),
            tool_context: ToolContext {
                tool_use_id: ToolUseId::new(),
                run_id,
                session_id: self.session_id,
                tenant_id: self.tenant_id,
                sandbox: None,
                permission_broker: self.broker.clone(),
                cap_registry: Arc::new(CapabilityRegistry::default()),
                interrupt: InterruptToken::new(),
                parent_run: None,
            },
            permission_context: PermissionContext {
                permission_mode: harness_contracts::PermissionMode::Default,
                previous_mode: None,
                session_id: self.session_id,
                tenant_id: self.tenant_id,
                interactivity: InteractivityLevel::NoInteractive,
                timeout_policy: None,
                fallback_policy: FallbackPolicy::DenyAll,
                rule_snapshot: Arc::new(RuleSnapshot {
                    rules: Vec::new(),
                    generation: 0,
                    built_at: harness_contracts::now(),
                }),
                hook_overrides: Vec::new(),
            },
            blob_store: None,
            event_emitter: Arc::new(harness_tool::NoopToolEventEmitter),
        }
    }
}

#[derive(Default)]
struct HarnessAllowBroker {
    requests: TokioMutex<Vec<PermissionRequest>>,
}

impl HarnessAllowBroker {
    async fn take_requests(&self) -> Vec<PermissionRequest> {
        std::mem::take(&mut *self.requests.lock().await)
    }
}

#[async_trait]
impl PermissionBroker for HarnessAllowBroker {
    async fn decide(&self, request: PermissionRequest, _ctx: PermissionContext) -> Decision {
        self.requests.lock().await.push(request);
        Decision::AllowOnce
    }

    async fn persist(
        &self,
        _decision_id: DecisionId,
        _scope: DecisionScope,
    ) -> Result<(), harness_contracts::PermissionError> {
        Ok(())
    }
}

struct CliPromptView {
    tenant_id: TenantId,
    session_id: harness_contracts::SessionId,
    descriptors: Vec<ToolDescriptor>,
}

impl ContextSessionView for CliPromptView {
    fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }

    fn session_id(&self) -> Option<harness_contracts::SessionId> {
        Some(self.session_id)
    }

    fn system(&self) -> Option<String> {
        Some("Octopus CLI M3 run-once driver".to_owned())
    }

    fn messages(&self) -> Vec<harness_contracts::Message> {
        Vec::new()
    }

    fn tools_snapshot(&self) -> Vec<ToolDescriptor> {
        self.descriptors.clone()
    }
}

#[derive(Clone)]
struct CliListDirTool {
    descriptor: ToolDescriptor,
}

impl CliListDirTool {
    fn new() -> Self {
        Self {
            descriptor: ToolDescriptor {
                name: "ListDir".to_owned(),
                display_name: "ListDir".to_owned(),
                description: "List directory entries".to_owned(),
                category: "filesystem".to_owned(),
                group: ToolGroup::FileSystem,
                version: "0.0.1".to_owned(),
                input_schema: json!({
                    "type": "object",
                    "required": ["path"],
                    "properties": {
                        "path": { "type": "string" }
                    }
                }),
                output_schema: None,
                dynamic_schema: false,
                properties: ToolProperties {
                    is_concurrency_safe: true,
                    is_read_only: true,
                    is_destructive: false,
                    long_running: None,
                    defer_policy: harness_contracts::DeferPolicy::AlwaysLoad,
                },
                trust_level: TrustLevel::AdminTrusted,
                required_capabilities: Vec::new(),
                budget: default_result_budget(),
                provider_restriction: ProviderRestriction::All,
                origin: ToolOrigin::Builtin,
                search_hint: None,
            },
        }
    }
}

#[async_trait]
impl Tool for CliListDirTool {
    fn descriptor(&self) -> &ToolDescriptor {
        &self.descriptor
    }

    async fn validate(&self, input: &Value, _ctx: &ToolContext) -> Result<(), ValidationError> {
        if input.get("path").and_then(Value::as_str).is_none() {
            return Err(ValidationError::from("path is required"));
        }
        Ok(())
    }

    async fn check_permission(
        &self,
        input: &Value,
        _ctx: &ToolContext,
    ) -> harness_permission::PermissionCheck {
        harness_permission::PermissionCheck::AskUser {
            subject: PermissionSubject::ToolInvocation {
                tool: self.descriptor.name.clone(),
                input: input.clone(),
            },
            scope: DecisionScope::ToolName(self.descriptor.name.clone()),
        }
    }

    async fn execute(
        &self,
        input: Value,
        _ctx: ToolContext,
    ) -> Result<ToolStream, harness_contracts::ToolError> {
        let root = input.get("path").and_then(Value::as_str).ok_or_else(|| {
            harness_contracts::ToolError::Validation("path is required".to_owned())
        })?;
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(root)
            .map_err(|error| harness_contracts::ToolError::Message(error.to_string()))?
        {
            let entry =
                entry.map_err(|error| harness_contracts::ToolError::Message(error.to_string()))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let metadata = entry
                .metadata()
                .map_err(|error| harness_contracts::ToolError::Message(error.to_string()))?;
            entries.push(json!({
                "path": name,
                "kind": if metadata.is_dir() { "dir" } else { "file" },
                "size": metadata.len(),
            }));
        }
        entries.sort_by(|left, right| {
            left["path"]
                .as_str()
                .unwrap_or_default()
                .cmp(right["path"].as_str().unwrap_or_default())
        });
        Ok(Box::pin(stream::iter([ToolEvent::Final(
            ToolResult::Structured(Value::Array(entries)),
        )])))
    }
}

fn harness_message(role: MessageRole, parts: Vec<MessagePart>) -> harness_contracts::Message {
    harness_contracts::Message {
        id: HarnessMessageId::new(),
        role,
        parts,
        created_at: harness_contracts::now(),
    }
}

fn list_dir_summary(result: &ToolResult) -> String {
    match result {
        ToolResult::Structured(Value::Array(entries)) => entries
            .iter()
            .filter_map(|entry| entry.get("path").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        ToolResult::Text(text) => text.clone(),
        _ => String::new(),
    }
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
        SessionEvent::Render { blocks, lifecycle } => {
            for block in blocks {
                writeln!(
                    out,
                    "[render.block] lifecycle={lifecycle:?} kind={:?} payload={}",
                    block.kind,
                    serde_json::to_string(&block.payload)
                        .map_err(|error| CliError::Setup(error.to_string()))?
                )?;
            }
        }
        SessionEvent::PermissionDecision {
            name,
            mode,
            outcome,
            ..
        } => {
            writeln!(
                out,
                "[permission.decision] name={name} mode={mode:?} outcome={outcome:?}"
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
    #[allow(clippy::await_holding_lock)]
    async fn main_with_args_prints_scripted_reply() {
        let _guard = env_lock().lock().expect("env lock should remain available");
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
        assert!(rendered.contains("cli scripted reply"));
    }

    #[tokio::test]
    async fn main_with_args_without_args_prints_help() {
        let mut out = Vec::new();
        main_with_args(vec!["octopus-cli".into()], &mut out)
            .await
            .expect("no args should print help");

        let rendered = String::from_utf8(out).expect("stdout buffer should stay utf8");
        assert!(rendered.contains("octopus-cli init"));
    }

    #[tokio::test]
    async fn main_with_args_requires_prompt_for_run_mode() {
        let mut out = Vec::new();
        let error = main_with_args(
            vec![
                "octopus-cli".into(),
                "/tmp/workspace".into(),
                "test-model".into(),
            ],
            &mut out,
        )
        .await
        .expect_err("missing prompt should fail");

        assert!(matches!(error, CliError::Usage(_)));
    }
}
