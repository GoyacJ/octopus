use std::{collections::HashMap, sync::Arc};

use harness_contracts::{ModelRef, PricingSnapshotId, RunId, SessionId, TenantId, UsageSnapshot};
use parking_lot::RwLock;

const NOOP_CALCULATOR_ID: &str = "noop";

pub trait CostCalculator: Send + Sync + 'static {
    fn calculator_id(&self) -> &str;

    fn compute(
        &self,
        model_ref: &ModelRef,
        pricing_snapshot_id: Option<&PricingSnapshotId>,
        usage: &UsageSnapshot,
    ) -> Option<UsageCost>;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UsageCost {
    pub cost_micros: u64,
    pub pricing_snapshot_id: Option<PricingSnapshotId>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoopCostCalculator;

impl CostCalculator for NoopCostCalculator {
    fn calculator_id(&self) -> &str {
        NOOP_CALCULATOR_ID
    }

    fn compute(
        &self,
        _model_ref: &ModelRef,
        _pricing_snapshot_id: Option<&PricingSnapshotId>,
        _usage: &UsageSnapshot,
    ) -> Option<UsageCost> {
        None
    }
}

#[derive(Clone)]
pub struct UsageAccumulator {
    inner: Arc<RwLock<UsageState>>,
    cost_calculator: Arc<dyn CostCalculator>,
}

impl UsageAccumulator {
    #[must_use]
    pub fn builder() -> UsageAccumulatorBuilder {
        UsageAccumulatorBuilder::default()
    }

    pub fn record(&self, scope: UsageScope, model_ref: Option<ModelRef>, delta: UsageSnapshot) {
        self.record_with_pricing(scope, model_ref, None, delta);
    }

    pub fn record_with_pricing(
        &self,
        scope: UsageScope,
        model_ref: Option<ModelRef>,
        pricing_snapshot_id: Option<PricingSnapshotId>,
        mut delta: UsageSnapshot,
    ) {
        if let Some(model_ref) = &model_ref {
            if let Some(cost) =
                self.cost_calculator
                    .compute(model_ref, pricing_snapshot_id.as_ref(), &delta)
            {
                delta.cost_micros = cost.cost_micros;
            }
        }

        let mut state = self.inner.write();
        add_usage(&mut state.global, &delta);
        match scope {
            UsageScope::Global => {}
            UsageScope::Tenant(tenant_id) => {
                add_usage(state.by_tenant.entry(tenant_id).or_default(), &delta);
            }
            UsageScope::Session(session_id) => {
                add_usage(state.by_session.entry(session_id).or_default(), &delta);
            }
            UsageScope::Run(run_id) => {
                add_usage(state.by_run.entry(run_id).or_default(), &delta);
            }
            UsageScope::Model(model_id) => {
                add_usage(state.by_model.entry(model_id).or_default(), &delta);
            }
        }
    }

    #[must_use]
    pub fn snapshot(&self, scope: UsageScope) -> UsageSnapshot {
        let state = self.inner.read();
        match scope {
            UsageScope::Global => Some(state.global.clone()),
            UsageScope::Tenant(tenant_id) => state.by_tenant.get(&tenant_id).cloned(),
            UsageScope::Session(session_id) => state.by_session.get(&session_id).cloned(),
            UsageScope::Run(run_id) => state.by_run.get(&run_id).cloned(),
            UsageScope::Model(model_id) => state.by_model.get(&model_id).cloned(),
        }
        .unwrap_or_default()
    }

    pub fn reset(&self, scope: UsageScope) {
        let mut state = self.inner.write();
        match scope {
            UsageScope::Global => state.global = UsageSnapshot::default(),
            UsageScope::Tenant(tenant_id) => {
                state.by_tenant.remove(&tenant_id);
            }
            UsageScope::Session(session_id) => {
                state.by_session.remove(&session_id);
            }
            UsageScope::Run(run_id) => {
                state.by_run.remove(&run_id);
            }
            UsageScope::Model(model_id) => {
                state.by_model.remove(&model_id);
            }
        }
    }

    #[must_use]
    pub fn report(&self) -> UsageReport {
        let state = self.inner.read();
        UsageReport {
            global: state.global.clone(),
            tenants: state.by_tenant.clone(),
            sessions: state.by_session.clone(),
            runs: state.by_run.clone(),
            models: state.by_model.clone(),
        }
    }
}

impl Default for UsageAccumulator {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(Clone, Default)]
pub struct UsageAccumulatorBuilder {
    cost_calculator: Option<Arc<dyn CostCalculator>>,
}

impl UsageAccumulatorBuilder {
    #[must_use]
    pub fn with_cost_calculator(mut self, calculator: Arc<dyn CostCalculator>) -> Self {
        self.cost_calculator = Some(calculator);
        self
    }

    #[must_use]
    pub fn build(self) -> UsageAccumulator {
        UsageAccumulator {
            inner: Arc::new(RwLock::new(UsageState::default())),
            cost_calculator: self
                .cost_calculator
                .unwrap_or_else(|| Arc::new(NoopCostCalculator)),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum UsageScope {
    Global,
    Tenant(TenantId),
    Session(SessionId),
    Run(RunId),
    Model(String),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct UsageReport {
    pub global: UsageSnapshot,
    pub tenants: HashMap<TenantId, UsageSnapshot>,
    pub sessions: HashMap<SessionId, UsageSnapshot>,
    pub runs: HashMap<RunId, UsageSnapshot>,
    pub models: HashMap<String, UsageSnapshot>,
}

#[derive(Debug, Clone, Default)]
struct UsageState {
    global: UsageSnapshot,
    by_tenant: HashMap<TenantId, UsageSnapshot>,
    by_session: HashMap<SessionId, UsageSnapshot>,
    by_run: HashMap<RunId, UsageSnapshot>,
    by_model: HashMap<String, UsageSnapshot>,
}

fn add_usage(total: &mut UsageSnapshot, delta: &UsageSnapshot) {
    total.input_tokens = total.input_tokens.saturating_add(delta.input_tokens);
    total.output_tokens = total.output_tokens.saturating_add(delta.output_tokens);
    total.cache_read_tokens = total
        .cache_read_tokens
        .saturating_add(delta.cache_read_tokens);
    total.cache_write_tokens = total
        .cache_write_tokens
        .saturating_add(delta.cache_write_tokens);
    total.cost_micros = total.cost_micros.saturating_add(delta.cost_micros);
}
