use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::Utc;
use harness_contracts::{CredentialPoolSharedAcrossTenantsEvent, ModelError, TenantId};

use crate::{CredentialError, CredentialKey, CredentialSource, CredentialValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolStrategy {
    FillFirst,
    RoundRobin,
    Random,
    LeastUsed,
}

#[derive(Debug)]
pub struct PickedCredential {
    pub key: CredentialKey,
    pub value: CredentialValue,
}

pub trait CredentialPoolAuditSink: Send + Sync + 'static {
    fn record_shared_across_tenants(&self, event: CredentialPoolSharedAcrossTenantsEvent);
}

#[derive(Default)]
pub struct NoopCredentialPoolAuditSink;

impl CredentialPoolAuditSink for NoopCredentialPoolAuditSink {
    fn record_shared_across_tenants(&self, _event: CredentialPoolSharedAcrossTenantsEvent) {}
}

pub struct CredentialPool {
    strategy: PoolStrategy,
    sources: Vec<Arc<dyn CredentialSource>>,
    audit_sink: Arc<dyn CredentialPoolAuditSink>,
    state: Mutex<CredentialPoolState>,
}

#[derive(Default)]
struct CredentialPoolState {
    round_robin_cursor: usize,
    random_seed: u64,
    cooldown_until: HashMap<CredentialKey, Instant>,
    banned: HashSet<CredentialKey>,
    use_counts: HashMap<CredentialKey, u64>,
    audited_shared: HashSet<CredentialKey>,
}

impl CredentialPool {
    pub fn builder() -> CredentialPoolBuilder {
        CredentialPoolBuilder::default()
    }

    pub async fn pick(
        &self,
        candidates: &[CredentialKey],
    ) -> Result<PickedCredential, CredentialError> {
        let key = {
            let mut state = self.state.lock().expect("credential pool state poisoned");
            state.prune_expired_cooldowns(Instant::now());
            let available: Vec<CredentialKey> = candidates
                .iter()
                .filter(|key| {
                    !state.banned.contains(*key) && !state.cooldown_until.contains_key(*key)
                })
                .cloned()
                .collect();
            select_key(self.strategy, &available, &mut state)
        }
        .ok_or(CredentialError::Model(ModelError::AllCredentialsBanned))?;

        let value = self.fetch_from_sources(&key).await?;
        self.record_success(&key);

        Ok(PickedCredential { key, value })
    }

    pub fn pick_strategy(&self) -> PoolStrategy {
        self.strategy
    }

    pub fn mark_rate_limited(&self, key: &CredentialKey, cooldown: Duration) {
        let mut state = self.state.lock().expect("credential pool state poisoned");
        state
            .cooldown_until
            .insert(key.clone(), Instant::now() + cooldown);
    }

    pub fn mark_banned(&self, key: &CredentialKey) {
        let mut state = self.state.lock().expect("credential pool state poisoned");
        state.banned.insert(key.clone());
    }

    async fn fetch_from_sources(
        &self,
        key: &CredentialKey,
    ) -> Result<CredentialValue, CredentialError> {
        let mut last_error = None;
        for source in &self.sources {
            match source.fetch(key.clone()).await {
                Ok(value) => return Ok(value),
                Err(error) => last_error = Some(error),
            }
        }

        Err(last_error.unwrap_or_else(|| {
            CredentialError::Model(ModelError::InvalidRequest(
                "credential pool has no sources".to_owned(),
            ))
        }))
    }

    fn record_success(&self, key: &CredentialKey) {
        let event = {
            let mut state = self.state.lock().expect("credential pool state poisoned");
            *state.use_counts.entry(key.clone()).or_default() += 1;

            if key.tenant_id == TenantId::SHARED && state.audited_shared.insert(key.clone()) {
                Some(CredentialPoolSharedAcrossTenantsEvent {
                    tenant_id: key.tenant_id,
                    provider_id: key.provider_id.clone(),
                    credential_key_hash: credential_key_hash(key),
                    at: Utc::now(),
                })
            } else {
                None
            }
        };

        if let Some(event) = event {
            self.audit_sink.record_shared_across_tenants(event);
        }
    }
}

#[derive(Default)]
pub struct CredentialPoolBuilder {
    strategy: Option<PoolStrategy>,
    sources: Vec<Arc<dyn CredentialSource>>,
    audit_sink: Option<Arc<dyn CredentialPoolAuditSink>>,
}

impl CredentialPoolBuilder {
    pub fn strategy(mut self, strategy: PoolStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    pub fn add_source(mut self, source: Arc<dyn CredentialSource>) -> Self {
        self.sources.push(source);
        self
    }

    pub fn audit_sink(mut self, sink: Arc<dyn CredentialPoolAuditSink>) -> Self {
        self.audit_sink = Some(sink);
        self
    }

    pub fn build(self) -> CredentialPool {
        CredentialPool {
            strategy: self.strategy.unwrap_or(PoolStrategy::FillFirst),
            sources: self.sources,
            audit_sink: self
                .audit_sink
                .unwrap_or_else(|| Arc::new(NoopCredentialPoolAuditSink)),
            state: Mutex::new(CredentialPoolState {
                random_seed: initial_random_seed(),
                ..CredentialPoolState::default()
            }),
        }
    }
}

fn select_key(
    strategy: PoolStrategy,
    available: &[CredentialKey],
    state: &mut CredentialPoolState,
) -> Option<CredentialKey> {
    if available.is_empty() {
        return None;
    }

    match strategy {
        PoolStrategy::FillFirst => available.first().cloned(),
        PoolStrategy::RoundRobin => {
            let index = state.round_robin_cursor % available.len();
            state.round_robin_cursor = state.round_robin_cursor.wrapping_add(1);
            available.get(index).cloned()
        }
        PoolStrategy::Random => {
            state.random_seed ^= state.random_seed << 13;
            state.random_seed ^= state.random_seed >> 7;
            state.random_seed ^= state.random_seed << 17;
            available
                .get((state.random_seed as usize) % available.len())
                .cloned()
        }
        PoolStrategy::LeastUsed => available
            .iter()
            .min_by_key(|key| state.use_counts.get(*key).copied().unwrap_or(0))
            .cloned(),
    }
}

impl CredentialPoolState {
    fn prune_expired_cooldowns(&mut self, now: Instant) {
        self.cooldown_until.retain(|_, until| *until > now);
    }
}

fn initial_random_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0x9e37_79b9_7f4a_7c15)
        .max(1)
}

fn credential_key_hash(key: &CredentialKey) -> [u8; 32] {
    let mut out = [0; 32];
    for salt in 0..4_u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        salt.hash(&mut hasher);
        key.hash(&mut hasher);
        let bytes = hasher.finish().to_be_bytes();
        out[(salt as usize * 8)..((salt as usize + 1) * 8)].copy_from_slice(&bytes);
    }
    out
}
