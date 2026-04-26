use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use dashmap::DashMap;
use harness_contracts::{Decision, DecisionId, DecisionScope, PermissionError, RequestId};
use tokio::sync::{mpsc, oneshot};

use crate::{
    DecisionPersistence, NoopDecisionPersistence, PermissionBroker, PermissionContext,
    PermissionRequest,
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);

pub struct StreamBasedBroker {
    requests: mpsc::Sender<PermissionRequest>,
    pending: Arc<DashMap<RequestId, PendingResolution>>,
    persistence: Arc<dyn DecisionPersistence>,
    config: StreamBrokerConfig,
}

#[derive(Debug, Clone)]
pub struct StreamBrokerConfig {
    pub default_timeout: Option<Duration>,
    pub heartbeat_interval: Option<Duration>,
    pub max_pending: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancelReason {
    UiClosed,
    SessionEnded,
    Cancelled(String),
}

struct PendingResolution {
    sender: oneshot::Sender<Decision>,
    request: PermissionRequest,
    enqueued_at: Instant,
    last_heartbeat_at: Instant,
}

impl PendingResolution {
    fn observe_metadata(&self) {
        let _ = (&self.request, self.enqueued_at, self.last_heartbeat_at);
    }
}

#[derive(Clone)]
pub struct ResolverHandle {
    pending: Arc<DashMap<RequestId, PendingResolution>>,
}

impl Default for StreamBrokerConfig {
    fn default() -> Self {
        Self {
            default_timeout: Some(DEFAULT_TIMEOUT),
            heartbeat_interval: None,
            max_pending: 1024,
        }
    }
}

impl StreamBasedBroker {
    pub fn new(
        config: StreamBrokerConfig,
    ) -> (Self, mpsc::Receiver<PermissionRequest>, ResolverHandle) {
        let channel_capacity = config.max_pending.max(1);
        let (requests, receiver) = mpsc::channel(channel_capacity);
        let pending = Arc::new(DashMap::new());
        let resolver = ResolverHandle {
            pending: pending.clone(),
        };
        let broker = Self {
            requests,
            pending,
            persistence: Arc::new(NoopDecisionPersistence),
            config,
        };

        (broker, receiver, resolver)
    }

    #[must_use]
    pub fn with_persistence(mut self, persistence: Arc<dyn DecisionPersistence>) -> Self {
        self.persistence = persistence;
        self
    }

    fn timeout_for(&self, ctx: &PermissionContext) -> (Duration, Decision) {
        if let Some(policy) = &ctx.timeout_policy {
            return (
                Duration::from_millis(policy.deadline_ms),
                policy.default_on_timeout.clone(),
            );
        }

        (
            self.config.default_timeout.unwrap_or(DEFAULT_TIMEOUT),
            Decision::DenyOnce,
        )
    }
}

impl ResolverHandle {
    pub async fn resolve(
        &self,
        request_id: RequestId,
        decision: Decision,
    ) -> Result<(), PermissionError> {
        let Some((_request_id, pending)) = self.pending.remove(&request_id) else {
            return Err(PermissionError::Message(format!(
                "permission request `{request_id}` is not pending"
            )));
        };

        pending.observe_metadata();
        pending.sender.send(decision).map_err(|_| {
            PermissionError::Message(format!(
                "permission request `{request_id}` receiver is closed"
            ))
        })
    }

    pub async fn cancel(
        &self,
        request_id: RequestId,
        _reason: CancelReason,
    ) -> Result<(), PermissionError> {
        let Some((_request_id, pending)) = self.pending.remove(&request_id) else {
            return Err(PermissionError::Message(format!(
                "permission request `{request_id}` is not pending"
            )));
        };

        pending.observe_metadata();
        let _ = pending.sender.send(Decision::DenyOnce);
        Ok(())
    }
}

#[async_trait]
impl PermissionBroker for StreamBasedBroker {
    async fn decide(&self, request: PermissionRequest, ctx: PermissionContext) -> Decision {
        if self.pending.len() >= self.config.max_pending {
            return Decision::DenyOnce;
        }

        let request_id = request.request_id;
        let (sender, receiver) = oneshot::channel();
        let now = Instant::now();
        self.pending.insert(
            request_id,
            PendingResolution {
                sender,
                request: request.clone(),
                enqueued_at: now,
                last_heartbeat_at: now,
            },
        );

        if self.requests.send(request).await.is_err() {
            self.pending.remove(&request_id);
            return Decision::DenyOnce;
        }

        let (timeout, default_on_timeout) = self.timeout_for(&ctx);
        match tokio::time::timeout(timeout, receiver).await {
            Ok(Ok(decision)) => decision,
            Ok(Err(_closed)) => Decision::DenyOnce,
            Err(_elapsed) => {
                self.pending.remove(&request_id);
                default_on_timeout
            }
        }
    }

    async fn persist(
        &self,
        decision_id: DecisionId,
        scope: DecisionScope,
    ) -> Result<(), PermissionError> {
        self.persistence.persist(decision_id, scope).await
    }
}
