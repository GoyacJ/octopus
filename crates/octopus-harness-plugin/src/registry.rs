use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use harness_contracts::PluginId;
use parking_lot::RwLock;
use serde_json::Value;

use crate::{
    CapabilityRegistrationState, CapabilitySlot, DiscoverySource, ManifestRecord, ManifestSigner,
    Plugin, PluginActivationContext, PluginActivationResult, PluginError, PluginManifest,
    PluginManifestLoader, PluginRuntimeLoader, RegistrationError, RuntimeLoaderError,
    ScopedCoordinatorStrategyRegistration, ScopedHookRegistration, ScopedMcpRegistration,
    ScopedMemoryProviderRegistration, ScopedSkillRegistration, ScopedToolRegistration,
    SignatureAlgorithm, SignerProvenance, StaticLinkRuntimeLoader, StaticTrustedSignerStore,
    TrustedSigner, TrustedSignerStore,
};

#[derive(Clone)]
pub struct PluginRegistry {
    inner: Arc<RwLock<PluginRegistryInner>>,
    manifest_loaders: Arc<Vec<Arc<dyn PluginManifestLoader>>>,
    runtime_loaders: Arc<Vec<Arc<dyn PluginRuntimeLoader>>>,
    discovery_sources: Arc<Vec<DiscoverySource>>,
    manifest_signer: ManifestSigner,
}

impl std::fmt::Debug for PluginRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PluginRegistry")
            .field("snapshot", &self.snapshot())
            .finish()
    }
}

#[derive(Default)]
struct PluginRegistryInner {
    discovered: BTreeMap<PluginId, DiscoveredPlugin>,
    activated: BTreeMap<PluginId, ActivatedPlugin>,
    state: BTreeMap<PluginId, PluginLifecycleState>,
    slots: CapabilitySlotManager,
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum PluginLifecycleState {
    Validated,
    Activating,
    Activated,
    Deactivating,
    Deactivated,
    Rejected,
    Failed,
}

#[derive(Clone)]
struct ActivatedPlugin {
    plugin: Arc<dyn Plugin>,
    slots: Vec<CapabilitySlot>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredPlugin {
    pub record: ManifestRecord,
    pub source: DiscoverySource,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PluginRegistrySnapshot {
    pub discovered: Vec<PluginId>,
    pub activated: Vec<PluginId>,
    pub states: BTreeMap<PluginId, PluginLifecycleState>,
    pub occupied_slots: HashMap<CapabilitySlot, PluginId>,
}

#[derive(Default)]
pub struct PluginRegistryBuilder {
    manifest_loaders: Vec<Arc<dyn PluginManifestLoader>>,
    runtime_loaders: Vec<Arc<dyn PluginRuntimeLoader>>,
    discovery_sources: Vec<DiscoverySource>,
    signer_store: Option<Arc<dyn TrustedSignerStore>>,
    trusted_signers: Vec<Vec<u8>>,
}

impl PluginRegistry {
    pub fn builder() -> PluginRegistryBuilder {
        PluginRegistryBuilder::default()
    }

    pub async fn discover(&self) -> Result<Vec<DiscoveredPlugin>, PluginError> {
        let mut discovered = Vec::new();

        for loader in self.manifest_loaders.iter() {
            for source in self.discovery_sources.iter() {
                for record in loader.enumerate(source).await? {
                    let plugin_id = record.manifest.plugin_id();
                    if let Err(error) = self.manifest_signer.verify_manifest(&record.manifest).await
                    {
                        self.inner
                            .write()
                            .state
                            .insert(plugin_id, PluginLifecycleState::Rejected);
                        return Err(error);
                    }
                    let plugin = DiscoveredPlugin {
                        record,
                        source: source.clone(),
                    };
                    self.inner
                        .write()
                        .state
                        .insert(plugin_id.clone(), PluginLifecycleState::Validated);
                    self.inner
                        .write()
                        .discovered
                        .insert(plugin_id, plugin.clone());
                    discovered.push(plugin);
                }
            }
        }

        Ok(discovered)
    }

    pub async fn activate(&self, id: &PluginId) -> Result<(), PluginError> {
        let discovered = {
            let mut inner = self.inner.write();
            if inner.activated.contains_key(id) {
                return Ok(());
            }
            let discovered = inner.discovered.get(id).cloned().ok_or_else(|| {
                PluginError::ActivateFailed(format!("plugin not discovered: {}", id.0))
            })?;
            inner
                .state
                .insert(id.clone(), PluginLifecycleState::Activating);
            discovered
        };

        let plugin = match self.load_plugin(&discovered.record).await {
            Ok(plugin) => plugin,
            Err(error) => {
                self.mark_failed(id);
                return Err(error);
            }
        };

        let activation = Arc::new(CapabilityRegistrationState::default());
        let ctx = self.activation_context(&discovered.record.manifest, Arc::clone(&activation));
        let result = match plugin.activate(ctx).await {
            Ok(result) => result,
            Err(error) => {
                self.mark_failed(id);
                return Err(error);
            }
        };

        if let Err(error) =
            validate_activation_result(&discovered.record.manifest, &result, &activation)
        {
            self.mark_failed(id);
            return Err(error.into());
        }

        if let Err(error) = self.occupy_slots(id, &result.occupied_slots) {
            self.mark_failed(id);
            return Err(error);
        }

        let mut inner = self.inner.write();
        inner.activated.insert(
            id.clone(),
            ActivatedPlugin {
                plugin,
                slots: result.occupied_slots,
            },
        );
        inner
            .state
            .insert(id.clone(), PluginLifecycleState::Activated);
        Ok(())
    }

    pub async fn deactivate(&self, id: &PluginId) -> Result<(), PluginError> {
        let activated = {
            let mut inner = self.inner.write();
            let Some(activated) = inner.activated.remove(id) else {
                if inner.state.contains_key(id) {
                    inner
                        .state
                        .insert(id.clone(), PluginLifecycleState::Deactivated);
                }
                return Ok(());
            };
            inner
                .state
                .insert(id.clone(), PluginLifecycleState::Deactivating);
            activated
        };

        if let Err(error) = activated.plugin.deactivate().await {
            self.mark_failed(id);
            return Err(PluginError::DeactivateFailed(error.to_string()));
        }

        let mut inner = self.inner.write();
        for slot in &activated.slots {
            inner.slots.release(slot, id);
        }
        inner
            .state
            .insert(id.clone(), PluginLifecycleState::Deactivated);
        Ok(())
    }

    pub fn list_activated(&self) -> Vec<PluginManifest> {
        self.inner
            .read()
            .activated
            .values()
            .map(|activated| activated.plugin.manifest().clone())
            .collect()
    }

    pub fn snapshot(&self) -> PluginRegistrySnapshot {
        let inner = self.inner.read();
        PluginRegistrySnapshot {
            discovered: inner.discovered.keys().cloned().collect(),
            activated: inner.activated.keys().cloned().collect(),
            states: inner.state.clone(),
            occupied_slots: inner.slots.occupied.clone(),
        }
    }

    pub fn state(&self, id: &PluginId) -> Option<PluginLifecycleState> {
        self.inner.read().state.get(id).cloned()
    }

    pub fn activation_context_for_test(
        &self,
        manifest: &PluginManifest,
    ) -> PluginActivationContext {
        self.activation_context(manifest, Arc::new(CapabilityRegistrationState::default()))
    }

    fn activation_context(
        &self,
        manifest: &PluginManifest,
        activation: Arc<CapabilityRegistrationState>,
    ) -> PluginActivationContext {
        PluginActivationContext {
            trust_level: manifest.trust_level,
            plugin_id: manifest.plugin_id(),
            config: Value::Null,
            workspace_root: None,
            tools: (!manifest.capabilities.tools.is_empty()).then(|| {
                Arc::new(ScopedToolRegistration::new(
                    manifest,
                    Arc::clone(&activation),
                )) as Arc<_>
            }),
            hooks: (!manifest.capabilities.hooks.is_empty()).then(|| {
                Arc::new(ScopedHookRegistration::new(
                    manifest,
                    Arc::clone(&activation),
                )) as Arc<_>
            }),
            mcp: (!manifest.capabilities.mcp_servers.is_empty()).then(|| {
                Arc::new(ScopedMcpRegistration::new(
                    manifest,
                    Arc::clone(&activation),
                )) as Arc<_>
            }),
            skills: (!manifest.capabilities.skills.is_empty()).then(|| {
                Arc::new(ScopedSkillRegistration::new(
                    manifest,
                    Arc::clone(&activation),
                )) as Arc<_>
            }),
            memory: manifest.capabilities.memory_provider.is_some().then(|| {
                Arc::new(ScopedMemoryProviderRegistration::new(Arc::clone(
                    &activation,
                ))) as Arc<_>
            }),
            coordinator: manifest
                .capabilities
                .coordinator_strategy
                .is_some()
                .then(|| {
                    Arc::new(ScopedCoordinatorStrategyRegistration::new(Arc::clone(
                        &activation,
                    ))) as Arc<_>
                }),
        }
    }

    async fn load_plugin(&self, record: &ManifestRecord) -> Result<Arc<dyn Plugin>, PluginError> {
        for loader in self.runtime_loaders.iter() {
            if loader.can_load(&record.manifest, &record.origin) {
                let plugin = loader.load(&record.manifest, &record.origin).await?;
                if plugin.manifest() != &record.manifest {
                    return Err(RuntimeLoaderError::LoadFailed(format!(
                        "manifest mismatch: expected {}, got {}",
                        record.manifest.plugin_id().0,
                        plugin.manifest().plugin_id().0
                    ))
                    .into());
                }
                return Ok(plugin);
            }
        }

        Err(PluginError::ActivateFailed(format!(
            "no runtime loader can handle origin: {}",
            record.origin
        )))
    }

    fn occupy_slots(&self, id: &PluginId, slots: &[CapabilitySlot]) -> Result<(), PluginError> {
        let mut inner = self.inner.write();
        for slot in slots {
            if let Err(error) = inner.slots.try_occupy(slot.clone(), id) {
                for occupied in slots {
                    inner.slots.release(occupied, id);
                    if occupied == slot {
                        break;
                    }
                }
                return Err(error);
            }
        }
        Ok(())
    }

    fn mark_failed(&self, id: &PluginId) {
        self.inner
            .write()
            .state
            .insert(id.clone(), PluginLifecycleState::Failed);
    }
}

impl PluginRegistryBuilder {
    #[must_use]
    pub fn with_manifest_loader(mut self, loader: Arc<dyn PluginManifestLoader>) -> Self {
        self.manifest_loaders.push(loader);
        self
    }

    #[must_use]
    pub fn with_runtime_loader(mut self, loader: Arc<dyn PluginRuntimeLoader>) -> Self {
        self.runtime_loaders.push(loader);
        self
    }

    #[must_use]
    pub fn with_source(mut self, source: DiscoverySource) -> Self {
        self.discovery_sources.push(source);
        self
    }

    #[must_use]
    pub fn with_signer_store(mut self, store: Arc<dyn TrustedSignerStore>) -> Self {
        self.signer_store = Some(store);
        self
    }

    #[must_use]
    pub fn with_trusted_signer(mut self, public_key: impl Into<Vec<u8>>) -> Self {
        self.trusted_signers.push(public_key.into());
        self
    }

    pub fn build(self) -> Result<PluginRegistry, PluginError> {
        let Self {
            manifest_loaders,
            runtime_loaders,
            discovery_sources,
            signer_store,
            trusted_signers,
        } = self;

        if signer_store.is_some() && !trusted_signers.is_empty() {
            return Err(PluginError::Builder(
                "with_signer_store and with_trusted_signer are mutually exclusive".to_owned(),
            ));
        }

        let signer_store = match signer_store {
            Some(store) => store,
            None => Arc::new(StaticTrustedSignerStore::new(builder_trusted_signers(
                &trusted_signers,
            ))?),
        };

        Ok(PluginRegistry {
            inner: Arc::new(RwLock::new(PluginRegistryInner::default())),
            manifest_loaders: Arc::new(default_manifest_loaders(manifest_loaders)),
            runtime_loaders: Arc::new(default_runtime_loaders(runtime_loaders)),
            discovery_sources: Arc::new(if discovery_sources.is_empty() {
                vec![DiscoverySource::Inline]
            } else {
                discovery_sources
            }),
            manifest_signer: ManifestSigner::new(signer_store),
        })
    }
}

fn default_manifest_loaders(
    manifest_loaders: Vec<Arc<dyn PluginManifestLoader>>,
) -> Vec<Arc<dyn PluginManifestLoader>> {
    if manifest_loaders.is_empty() {
        vec![Arc::new(crate::FileManifestLoader)]
    } else {
        manifest_loaders
    }
}

fn default_runtime_loaders(
    runtime_loaders: Vec<Arc<dyn PluginRuntimeLoader>>,
) -> Vec<Arc<dyn PluginRuntimeLoader>> {
    if runtime_loaders.is_empty() {
        vec![Arc::new(StaticLinkRuntimeLoader::default())]
    } else {
        runtime_loaders
    }
}

fn builder_trusted_signers(public_keys: &[Vec<u8>]) -> Vec<TrustedSigner> {
    public_keys
        .iter()
        .enumerate()
        .map(|(index, public_key)| TrustedSigner {
            id: crate::SignerId::new(format!("user-injected-{index}"))
                .expect("generated signer id is valid"),
            algorithm: SignatureAlgorithm::Ed25519,
            public_key: public_key.clone(),
            activated_at: chrono::DateTime::UNIX_EPOCH,
            retired_at: None,
            revoked_at: None,
            provenance: SignerProvenance::BuilderInjected,
        })
        .collect()
}

#[derive(Debug, Clone, Default)]
pub struct CapabilitySlotManager {
    occupied: HashMap<CapabilitySlot, PluginId>,
}

impl CapabilitySlotManager {
    pub fn try_occupy(
        &mut self,
        slot: CapabilitySlot,
        plugin_id: &PluginId,
    ) -> Result<(), PluginError> {
        if let Some(occupant) = self.occupied.get(&slot) {
            if occupant != plugin_id {
                return Err(PluginError::SlotOccupied {
                    slot,
                    occupant: occupant.clone(),
                });
            }
        }
        self.occupied.insert(slot, plugin_id.clone());
        Ok(())
    }

    pub fn release(&mut self, slot: &CapabilitySlot, plugin_id: &PluginId) {
        if self.occupied.get(slot) == Some(plugin_id) {
            self.occupied.remove(slot);
        }
    }
}

fn validate_activation_result(
    manifest: &PluginManifest,
    result: &PluginActivationResult,
    activation: &CapabilityRegistrationState,
) -> Result<(), RegistrationError> {
    validate_subset(
        "tool",
        result.registered_tools.iter().cloned(),
        manifest
            .capabilities
            .tools
            .iter()
            .map(|entry| entry.name.clone()),
    )?;
    validate_subset(
        "hook",
        result.registered_hooks.iter().cloned(),
        manifest
            .capabilities
            .hooks
            .iter()
            .map(|entry| entry.name.clone()),
    )?;
    validate_subset(
        "skill",
        result.registered_skills.iter().cloned(),
        manifest
            .capabilities
            .skills
            .iter()
            .map(|entry| entry.name.clone()),
    )?;
    validate_subset(
        "mcp",
        result.registered_mcp.iter().map(|id| id.0.clone()),
        manifest
            .capabilities
            .mcp_servers
            .iter()
            .map(|entry| entry.name.clone()),
    )?;
    validate_subset(
        "tool",
        activation.registered_tools(),
        manifest
            .capabilities
            .tools
            .iter()
            .map(|entry| entry.name.clone()),
    )?;
    validate_subset(
        "hook",
        activation.registered_hooks(),
        manifest
            .capabilities
            .hooks
            .iter()
            .map(|entry| entry.name.clone()),
    )?;
    validate_subset(
        "skill",
        activation.registered_skills(),
        manifest
            .capabilities
            .skills
            .iter()
            .map(|entry| entry.name.clone()),
    )?;
    validate_subset(
        "mcp",
        activation.registered_mcp(),
        manifest
            .capabilities
            .mcp_servers
            .iter()
            .map(|entry| entry.name.clone()),
    )?;
    for slot in &result.occupied_slots {
        if !slot_declared(manifest, slot) {
            return Err(RegistrationError::UndeclaredResult {
                kind: "slot",
                name: format!("{slot:?}"),
            });
        }
    }
    if activation.coordinator_registered()
        && !result
            .occupied_slots
            .contains(&CapabilitySlot::CoordinatorStrategy)
    {
        return Err(RegistrationError::UndeclaredResult {
            kind: "slot",
            name: "CoordinatorStrategy registration missing occupied slot".to_owned(),
        });
    }
    Ok(())
}

fn slot_declared(manifest: &PluginManifest, slot: &CapabilitySlot) -> bool {
    match slot {
        CapabilitySlot::MemoryProvider => manifest.capabilities.memory_provider.is_some(),
        CapabilitySlot::CustomToolset(name) => manifest
            .capabilities
            .tools
            .iter()
            .any(|entry| &entry.name == name),
        CapabilitySlot::CoordinatorStrategy => manifest.capabilities.coordinator_strategy.is_some(),
    }
}

fn validate_subset(
    kind: &'static str,
    registered: impl IntoIterator<Item = String>,
    declared: impl IntoIterator<Item = String>,
) -> Result<(), RegistrationError> {
    let declared = declared
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    for name in registered {
        if !declared.contains(&name) {
            return Err(RegistrationError::UndeclaredResult { kind, name });
        }
    }
    Ok(())
}
