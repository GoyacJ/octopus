use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use async_trait::async_trait;
use harness_contracts::{
    DeferPolicy, McpServerId, McpServerSource, PluginId, ProviderRestriction, ToolDescriptor,
    ToolGroup, ToolOrigin, ToolProperties, TrustLevel,
};
use harness_hook::{HookContext, HookEvent, HookHandler, HookOutcome};
use harness_mcp::{McpServerSpec, TransportChoice};
use harness_plugin::{
    CapabilitySlot, CoordinatorStrategy, CoordinatorStrategyManifestEntry, DiscoverySource,
    ManifestLoaderError, ManifestOrigin, ManifestRecord, Plugin, PluginActivationContext,
    PluginActivationResult, PluginCapabilities, PluginError, PluginManifest, PluginManifestLoader,
    PluginName, PluginRegistry, PluginRuntimeLoader, RegistrationError, RuntimeLoaderError,
};
use harness_skill::{Skill, SkillFrontmatter, SkillPrerequisites, SkillSource};
use harness_tool::{
    default_result_budget, PermissionCheck, SchemaResolverContext, Tool, ToolContext, ToolStream,
    ValidationError,
};
use serde_json::{json, Value};

#[tokio::test]
async fn discover_keeps_runtime_loader_idle() {
    let runtime = Arc::new(CountingRuntimeLoader::new(Arc::new(NoopPlugin::new(record(
        "manifest-only",
        PluginCapabilities::default(),
    ))) as Arc<dyn Plugin>));
    let registry = PluginRegistry::builder()
        .with_source(DiscoverySource::Workspace("/workspace".into()))
        .with_manifest_loader(Arc::new(StaticManifestLoader::new(vec![record(
            "manifest-only",
            PluginCapabilities::default(),
        )])))
        .with_runtime_loader(runtime.clone())
        .build()
        .unwrap();

    let discovered = registry.discover().await.unwrap();

    assert_eq!(discovered.len(), 1);
    assert_eq!(runtime.load_count(), 0);
    assert_eq!(
        registry.state(&plugin_id("manifest-only")).unwrap(),
        harness_plugin::PluginLifecycleState::Validated
    );
}

#[tokio::test]
async fn activate_injects_only_declared_capability_handles() {
    let manifest = record(
        "tool-only",
        PluginCapabilities {
            tools: vec![harness_plugin::ToolManifestEntry {
                name: "declared-tool".to_owned(),
                destructive: false,
            }],
            ..PluginCapabilities::default()
        },
    );
    let plugin = Arc::new(CapturingPlugin::new(manifest.clone()));
    let runtime_plugin: Arc<dyn Plugin> = plugin.clone();
    let runtime = Arc::new(CountingRuntimeLoader::new(runtime_plugin));
    let registry = registry_for(manifest, runtime.clone());

    registry.discover().await.unwrap();
    registry.activate(&plugin_id("tool-only")).await.unwrap();

    assert_eq!(runtime.load_count(), 1);
    let ctx = plugin.captured_context().unwrap();
    assert!(ctx.tools.is_some());
    assert!(ctx.hooks.is_none());
    assert!(ctx.mcp.is_none());
    assert!(ctx.skills.is_none());
    assert!(ctx.memory.is_none());
    assert!(ctx.coordinator.is_none());
    assert_eq!(
        registry.state(&plugin_id("tool-only")).unwrap(),
        harness_plugin::PluginLifecycleState::Activated
    );
}

#[tokio::test]
async fn activate_injects_declared_coordinator_handle() {
    let manifest = record(
        "coordinator-plugin",
        PluginCapabilities {
            coordinator_strategy: Some(CoordinatorStrategyManifestEntry {
                name: "coordinator".to_owned(),
            }),
            ..PluginCapabilities::default()
        },
    );
    let plugin = Arc::new(CapturingPlugin::new(manifest.clone()));
    let runtime_plugin: Arc<dyn Plugin> = plugin.clone();
    let registry = registry_for(
        manifest,
        Arc::new(CountingRuntimeLoader::new(runtime_plugin)),
    );

    registry.discover().await.unwrap();
    registry
        .activate(&plugin_id("coordinator-plugin"))
        .await
        .unwrap();

    let ctx = plugin.captured_context().unwrap();
    assert!(ctx.coordinator.is_some());
    assert!(ctx.memory.is_none());
}

#[tokio::test]
async fn capability_handles_reject_undeclared_registrations() {
    let manifest = record(
        "declared-tool",
        PluginCapabilities {
            tools: vec![harness_plugin::ToolManifestEntry {
                name: "allowed".to_owned(),
                destructive: false,
            }],
            hooks: vec![harness_plugin::HookManifestEntry {
                name: "allowed-hook".to_owned(),
            }],
            mcp_servers: vec![harness_plugin::McpManifestEntry {
                name: "allowed-mcp".to_owned(),
            }],
            skills: vec![harness_plugin::SkillManifestEntry {
                name: "allowed-skill".to_owned(),
            }],
            ..PluginCapabilities::default()
        },
    );
    let registry = PluginRegistry::builder().build().unwrap();
    let ctx = registry.activation_context_for_test(&manifest.manifest);

    let tool_error = ctx
        .tools
        .unwrap()
        .register(Box::new(FakeTool::new("not-declared")))
        .await
        .unwrap_err();
    assert_eq!(
        tool_error,
        RegistrationError::UndeclaredTool {
            name: "not-declared".to_owned()
        }
    );

    let hook_error = ctx
        .hooks
        .unwrap()
        .register(Box::new(FakeHook::new("not-declared-hook")))
        .await
        .unwrap_err();
    assert_eq!(
        hook_error,
        RegistrationError::UndeclaredHook {
            name: "not-declared-hook".to_owned()
        }
    );

    let mcp_error = ctx
        .mcp
        .unwrap()
        .register(mcp_spec("not-declared-mcp"))
        .await
        .unwrap_err();
    assert_eq!(
        mcp_error,
        RegistrationError::UndeclaredMcp {
            name: "not-declared-mcp".to_owned()
        }
    );

    let skill_error = ctx
        .skills
        .unwrap()
        .register(fake_skill("not-declared-skill"))
        .await
        .unwrap_err();
    assert_eq!(
        skill_error,
        RegistrationError::UndeclaredSkill {
            name: "not-declared-skill".to_owned()
        }
    );
}

#[tokio::test]
async fn activate_rejects_result_registrations_outside_manifest() {
    let manifest = record("bad-result", PluginCapabilities::default());
    let plugin: Arc<dyn Plugin> = Arc::new(ResultPlugin::new(
        manifest.clone(),
        PluginActivationResult {
            registered_tools: vec!["extra-tool".to_owned()],
            ..PluginActivationResult::default()
        },
    ));
    let registry = registry_for(manifest, Arc::new(CountingRuntimeLoader::new(plugin)));

    registry.discover().await.unwrap();
    let error = registry
        .activate(&plugin_id("bad-result"))
        .await
        .unwrap_err();

    assert!(matches!(error, PluginError::Registration(_)));
    assert_eq!(
        registry.state(&plugin_id("bad-result")).unwrap(),
        harness_plugin::PluginLifecycleState::Failed
    );
}

#[tokio::test]
async fn failed_activation_can_be_retried_from_validated_manifest() {
    let manifest = record("retryable", PluginCapabilities::default());
    let plugin = Arc::new(RetryPlugin::new(manifest.clone()));
    let runtime_plugin: Arc<dyn Plugin> = plugin.clone();
    let registry = registry_for(
        manifest,
        Arc::new(CountingRuntimeLoader::new(runtime_plugin)),
    );

    registry.discover().await.unwrap();
    let first = registry
        .activate(&plugin_id("retryable"))
        .await
        .unwrap_err();
    assert!(matches!(first, PluginError::ActivateFailed(_)));
    assert_eq!(
        registry.state(&plugin_id("retryable")).unwrap(),
        harness_plugin::PluginLifecycleState::Failed
    );

    registry.activate(&plugin_id("retryable")).await.unwrap();
    assert_eq!(
        registry.state(&plugin_id("retryable")).unwrap(),
        harness_plugin::PluginLifecycleState::Activated
    );
}

#[tokio::test]
async fn activation_rejects_slots_not_declared_by_manifest() {
    let manifest = record("undeclared-slot", PluginCapabilities::default());
    let plugin: Arc<dyn Plugin> = Arc::new(ResultPlugin::new(
        manifest.clone(),
        PluginActivationResult {
            occupied_slots: vec![CapabilitySlot::MemoryProvider],
            ..PluginActivationResult::default()
        },
    ));
    let registry = registry_for(manifest, Arc::new(CountingRuntimeLoader::new(plugin)));

    registry.discover().await.unwrap();
    let error = registry
        .activate(&plugin_id("undeclared-slot"))
        .await
        .unwrap_err();

    assert!(matches!(error, PluginError::Registration(_)));
    assert_eq!(
        registry.state(&plugin_id("undeclared-slot")).unwrap(),
        harness_plugin::PluginLifecycleState::Failed
    );
}

#[tokio::test]
async fn slot_conflicts_reject_second_activation_and_deactivate_releases_slot() {
    let first = memory_plugin("memory-one");
    let second = memory_plugin("memory-two");
    let registry = PluginRegistry::builder()
        .with_source(DiscoverySource::Workspace("/workspace".into()))
        .with_manifest_loader(Arc::new(StaticManifestLoader::new(vec![
            first.clone(),
            second.clone(),
        ])))
        .with_runtime_loader(Arc::new(MultiRuntimeLoader::new(vec![
            Arc::new(ResultPlugin::new(
                first.clone(),
                PluginActivationResult {
                    occupied_slots: vec![CapabilitySlot::MemoryProvider],
                    ..PluginActivationResult::default()
                },
            )),
            Arc::new(ResultPlugin::new(
                second.clone(),
                PluginActivationResult {
                    occupied_slots: vec![CapabilitySlot::MemoryProvider],
                    ..PluginActivationResult::default()
                },
            )),
        ])))
        .build()
        .unwrap();

    registry.discover().await.unwrap();
    registry.activate(&plugin_id("memory-one")).await.unwrap();
    let error = registry
        .activate(&plugin_id("memory-two"))
        .await
        .unwrap_err();
    assert!(matches!(error, PluginError::SlotOccupied { .. }));

    registry.deactivate(&plugin_id("memory-one")).await.unwrap();
    registry.activate(&plugin_id("memory-two")).await.unwrap();
    registry.deactivate(&plugin_id("memory-two")).await.unwrap();
    registry.deactivate(&plugin_id("memory-two")).await.unwrap();

    assert_eq!(
        registry.state(&plugin_id("memory-two")).unwrap(),
        harness_plugin::PluginLifecycleState::Deactivated
    );
}

#[tokio::test]
async fn coordinator_slot_conflicts_reject_second_activation() {
    let first = coordinator_plugin("coordinator-one");
    let second = coordinator_plugin("coordinator-two");
    let registry = PluginRegistry::builder()
        .with_source(DiscoverySource::Workspace("/workspace".into()))
        .with_manifest_loader(Arc::new(StaticManifestLoader::new(vec![
            first.clone(),
            second.clone(),
        ])))
        .with_runtime_loader(Arc::new(MultiRuntimeLoader::new(vec![
            Arc::new(ResultPlugin::new(
                first.clone(),
                PluginActivationResult {
                    occupied_slots: vec![CapabilitySlot::CoordinatorStrategy],
                    ..PluginActivationResult::default()
                },
            )),
            Arc::new(ResultPlugin::new(
                second.clone(),
                PluginActivationResult {
                    occupied_slots: vec![CapabilitySlot::CoordinatorStrategy],
                    ..PluginActivationResult::default()
                },
            )),
        ])))
        .build()
        .unwrap();

    registry.discover().await.unwrap();
    registry
        .activate(&plugin_id("coordinator-one"))
        .await
        .unwrap();
    let error = registry
        .activate(&plugin_id("coordinator-two"))
        .await
        .unwrap_err();
    assert!(matches!(error, PluginError::SlotOccupied { .. }));

    registry
        .deactivate(&plugin_id("coordinator-one"))
        .await
        .unwrap();
    registry
        .activate(&plugin_id("coordinator-two"))
        .await
        .unwrap();
}

fn registry_for(record: ManifestRecord, runtime: Arc<CountingRuntimeLoader>) -> PluginRegistry {
    PluginRegistry::builder()
        .with_source(DiscoverySource::Workspace("/workspace".into()))
        .with_manifest_loader(Arc::new(StaticManifestLoader::new(vec![record])))
        .with_runtime_loader(runtime)
        .build()
        .unwrap()
}

fn memory_plugin(name: &str) -> ManifestRecord {
    record(
        name,
        PluginCapabilities {
            memory_provider: Some(harness_plugin::MemoryProviderManifestEntry {
                name: "memory".to_owned(),
            }),
            ..PluginCapabilities::default()
        },
    )
}

fn coordinator_plugin(name: &str) -> ManifestRecord {
    record(
        name,
        PluginCapabilities {
            coordinator_strategy: Some(CoordinatorStrategyManifestEntry {
                name: "coordinator".to_owned(),
            }),
            ..PluginCapabilities::default()
        },
    )
}

fn record(name: &str, capabilities: PluginCapabilities) -> ManifestRecord {
    ManifestRecord::new(
        PluginManifest {
            manifest_schema_version: 1,
            name: PluginName::new(name).unwrap(),
            version: "0.1.0".to_owned(),
            trust_level: TrustLevel::UserControlled,
            description: None,
            authors: Vec::new(),
            repository: None,
            signature: None,
            capabilities,
            dependencies: Vec::new(),
            min_harness_version: ">=0.0.0".to_owned(),
        },
        ManifestOrigin::File {
            path: format!("/plugins/{name}/plugin.json").into(),
        },
        [3; 32],
    )
    .unwrap()
}

fn plugin_id(name: &str) -> PluginId {
    PluginId(format!("{name}@0.1.0"))
}

struct StaticManifestLoader {
    records: Vec<ManifestRecord>,
}

impl StaticManifestLoader {
    fn new(records: Vec<ManifestRecord>) -> Self {
        Self { records }
    }
}

#[async_trait]
impl PluginManifestLoader for StaticManifestLoader {
    async fn enumerate(
        &self,
        _source: &DiscoverySource,
    ) -> Result<Vec<ManifestRecord>, ManifestLoaderError> {
        Ok(self.records.clone())
    }
}

struct CountingRuntimeLoader {
    plugin: Arc<dyn Plugin>,
    load_count: AtomicUsize,
}

impl CountingRuntimeLoader {
    fn new(plugin: Arc<dyn Plugin>) -> Self {
        Self {
            plugin,
            load_count: AtomicUsize::new(0),
        }
    }

    fn load_count(&self) -> usize {
        self.load_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl PluginRuntimeLoader for CountingRuntimeLoader {
    fn can_load(&self, manifest: &PluginManifest, _origin: &ManifestOrigin) -> bool {
        self.plugin.manifest().plugin_id() == manifest.plugin_id()
    }

    async fn load(
        &self,
        manifest: &PluginManifest,
        origin: &ManifestOrigin,
    ) -> Result<Arc<dyn Plugin>, RuntimeLoaderError> {
        if !self.can_load(manifest, origin) {
            return Err(RuntimeLoaderError::UnsupportedOrigin(origin.to_string()));
        }
        self.load_count.fetch_add(1, Ordering::SeqCst);
        Ok(Arc::clone(&self.plugin))
    }
}

struct MultiRuntimeLoader {
    plugins: Vec<Arc<dyn Plugin>>,
}

impl MultiRuntimeLoader {
    fn new(plugins: Vec<Arc<dyn Plugin>>) -> Self {
        Self { plugins }
    }
}

#[async_trait]
impl PluginRuntimeLoader for MultiRuntimeLoader {
    fn can_load(&self, manifest: &PluginManifest, _origin: &ManifestOrigin) -> bool {
        self.plugins
            .iter()
            .any(|plugin| plugin.manifest().plugin_id() == manifest.plugin_id())
    }

    async fn load(
        &self,
        manifest: &PluginManifest,
        origin: &ManifestOrigin,
    ) -> Result<Arc<dyn Plugin>, RuntimeLoaderError> {
        self.plugins
            .iter()
            .find(|plugin| plugin.manifest().plugin_id() == manifest.plugin_id())
            .cloned()
            .ok_or_else(|| RuntimeLoaderError::UnsupportedOrigin(origin.to_string()))
    }
}

struct NoopPlugin {
    manifest: PluginManifest,
}

impl NoopPlugin {
    fn new(record: ManifestRecord) -> Self {
        Self {
            manifest: record.manifest,
        }
    }
}

#[async_trait]
impl Plugin for NoopPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn activate(
        &self,
        _ctx: PluginActivationContext,
    ) -> Result<PluginActivationResult, PluginError> {
        Ok(PluginActivationResult::default())
    }

    async fn deactivate(&self) -> Result<(), PluginError> {
        Ok(())
    }
}

struct CapturingPlugin {
    manifest: PluginManifest,
    captured: tokio::sync::Mutex<Option<PluginActivationContext>>,
}

impl CapturingPlugin {
    fn new(record: ManifestRecord) -> Self {
        Self {
            manifest: record.manifest,
            captured: tokio::sync::Mutex::new(None),
        }
    }

    fn captured_context(&self) -> Option<PluginActivationContext> {
        self.captured.try_lock().ok()?.clone()
    }
}

#[async_trait]
impl Plugin for CapturingPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn activate(
        &self,
        ctx: PluginActivationContext,
    ) -> Result<PluginActivationResult, PluginError> {
        let coordinator = ctx.coordinator.clone();
        *self.captured.lock().await = Some(ctx);
        if let Some(coordinator) = coordinator {
            coordinator
                .register(Arc::new(FakeCoordinatorStrategy))
                .await?;
            return Ok(PluginActivationResult {
                occupied_slots: vec![CapabilitySlot::CoordinatorStrategy],
                ..PluginActivationResult::default()
            });
        }
        Ok(PluginActivationResult {
            registered_tools: vec!["declared-tool".to_owned()],
            ..PluginActivationResult::default()
        })
    }

    async fn deactivate(&self) -> Result<(), PluginError> {
        Ok(())
    }
}

struct FakeCoordinatorStrategy;

impl CoordinatorStrategy for FakeCoordinatorStrategy {}

struct ResultPlugin {
    manifest: PluginManifest,
    result: PluginActivationResult,
}

impl ResultPlugin {
    fn new(record: ManifestRecord, result: PluginActivationResult) -> Self {
        Self {
            manifest: record.manifest,
            result,
        }
    }
}

struct RetryPlugin {
    manifest: PluginManifest,
    attempts: AtomicUsize,
}

impl RetryPlugin {
    fn new(record: ManifestRecord) -> Self {
        Self {
            manifest: record.manifest,
            attempts: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl Plugin for RetryPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn activate(
        &self,
        _ctx: PluginActivationContext,
    ) -> Result<PluginActivationResult, PluginError> {
        if self.attempts.fetch_add(1, Ordering::SeqCst) == 0 {
            return Err(PluginError::ActivateFailed("first attempt".to_owned()));
        }
        Ok(PluginActivationResult::default())
    }

    async fn deactivate(&self) -> Result<(), PluginError> {
        Ok(())
    }
}

#[async_trait]
impl Plugin for ResultPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn activate(
        &self,
        _ctx: PluginActivationContext,
    ) -> Result<PluginActivationResult, PluginError> {
        Ok(self.result.clone())
    }

    async fn deactivate(&self) -> Result<(), PluginError> {
        Ok(())
    }
}

struct FakeTool {
    descriptor: ToolDescriptor,
}

impl FakeTool {
    fn new(name: &str) -> Self {
        Self {
            descriptor: ToolDescriptor {
                name: name.to_owned(),
                display_name: name.to_owned(),
                description: "fake".to_owned(),
                category: "test".to_owned(),
                group: ToolGroup::Custom("test".to_owned()),
                version: "0.1.0".to_owned(),
                input_schema: json!({ "type": "object" }),
                output_schema: None,
                dynamic_schema: false,
                properties: ToolProperties {
                    is_concurrency_safe: true,
                    is_read_only: true,
                    is_destructive: false,
                    long_running: None,
                    defer_policy: DeferPolicy::AlwaysLoad,
                },
                trust_level: TrustLevel::UserControlled,
                required_capabilities: Vec::new(),
                budget: default_result_budget(),
                provider_restriction: ProviderRestriction::All,
                origin: ToolOrigin::Plugin {
                    plugin_id: plugin_id("declared-tool"),
                    trust: TrustLevel::UserControlled,
                },
                search_hint: None,
            },
        }
    }
}

#[async_trait]
impl Tool for FakeTool {
    fn descriptor(&self) -> &ToolDescriptor {
        &self.descriptor
    }

    async fn resolve_schema(
        &self,
        _ctx: &SchemaResolverContext,
    ) -> Result<Value, harness_contracts::ToolError> {
        Ok(self.descriptor.input_schema.clone())
    }

    async fn validate(&self, _input: &Value, _ctx: &ToolContext) -> Result<(), ValidationError> {
        Ok(())
    }

    async fn check_permission(&self, _input: &Value, _ctx: &ToolContext) -> PermissionCheck {
        PermissionCheck::Allowed
    }

    async fn execute(
        &self,
        _input: Value,
        _ctx: ToolContext,
    ) -> Result<ToolStream, harness_contracts::ToolError> {
        Ok(Box::pin(futures::stream::empty()))
    }
}

struct FakeHook {
    id: String,
}

impl FakeHook {
    fn new(id: &str) -> Self {
        Self { id: id.to_owned() }
    }
}

#[async_trait]
impl HookHandler for FakeHook {
    fn handler_id(&self) -> &str {
        &self.id
    }

    fn interested_events(&self) -> &[harness_contracts::HookEventKind] {
        &[harness_contracts::HookEventKind::UserPromptSubmit]
    }

    async fn handle(
        &self,
        _event: HookEvent,
        _ctx: HookContext,
    ) -> Result<HookOutcome, harness_contracts::HookError> {
        Ok(HookOutcome::Continue)
    }
}

fn mcp_spec(id: &str) -> McpServerSpec {
    McpServerSpec::new(
        McpServerId(id.to_owned()),
        id,
        TransportChoice::InProcess,
        McpServerSource::Plugin(plugin_id("declared-tool")),
    )
}

fn fake_skill(name: &str) -> Skill {
    Skill {
        id: harness_contracts::SkillId(format!("skill:{name}")),
        name: name.to_owned(),
        description: "fake skill".to_owned(),
        source: SkillSource::Plugin(plugin_id("declared-tool")),
        frontmatter: SkillFrontmatter {
            name: name.to_owned(),
            description: "fake skill".to_owned(),
            allowlist_agents: None,
            parameters: Vec::new(),
            config: Vec::new(),
            platforms: Vec::new(),
            prerequisites: SkillPrerequisites::default(),
            hooks: Vec::new(),
            tags: Vec::new(),
            category: None,
            metadata: HashMap::default(),
        },
        body: String::new(),
        raw_path: None,
    }
}
