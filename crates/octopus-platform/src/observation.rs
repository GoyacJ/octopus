use async_trait::async_trait;
use octopus_core::{AppError, AuditRecord, CostLedgerEntry, TraceEventRecord};

#[async_trait]
pub trait ObservationService: Send + Sync {
    async fn list_trace_events(&self) -> Result<Vec<TraceEventRecord>, AppError>;
    async fn list_audit_records(&self) -> Result<Vec<AuditRecord>, AppError>;
    async fn list_cost_entries(&self) -> Result<Vec<CostLedgerEntry>, AppError>;
    async fn append_trace(&self, record: TraceEventRecord) -> Result<(), AppError>;
    async fn append_audit(&self, record: AuditRecord) -> Result<(), AppError>;
    async fn append_cost(&self, record: CostLedgerEntry) -> Result<(), AppError>;
}
