use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use thiserror::Error;

pub const DECISION_ALLOW: &str = "allow";
pub const DECISION_REQUIRE_APPROVAL: &str = "require_approval";
pub const DECISION_DENY: &str = "deny";
pub const EXECUTION_STATE_EXECUTABLE: &str = "executable";
pub const EXECUTION_STATE_APPROVAL_REQUIRED: &str = "approval_required";
pub const EXECUTION_STATE_DENIED: &str = "denied";

pub const APPROVAL_TYPE_EXECUTION: &str = "execution";
pub const APPROVAL_TYPE_KNOWLEDGE_PROMOTION: &str = "knowledge_promotion";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityDescriptorRecord {
    pub id: String,
    pub slug: String,
    pub kind: String,
    pub source: String,
    pub platform: String,
    pub risk_level: String,
    pub requires_approval: bool,
    pub input_schema_uri: Option<String>,
    pub output_schema_uri: Option<String>,
    pub fallback_capability_id: Option<String>,
    pub trust_level: String,
    pub created_at: String,
    pub updated_at: String,
}

impl CapabilityDescriptorRecord {
    pub fn new(
        id: impl Into<String>,
        slug: impl Into<String>,
        risk_level: impl Into<String>,
        requires_approval: bool,
    ) -> Self {
        let now = current_timestamp();
        Self {
            id: id.into(),
            slug: slug.into(),
            kind: "core".to_string(),
            source: "builtin".to_string(),
            platform: "runtime_local".to_string(),
            risk_level: risk_level.into(),
            requires_approval,
            input_schema_uri: None,
            output_schema_uri: None,
            fallback_capability_id: None,
            trust_level: "trusted".to_string(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn new_connector_backed(
        id: impl Into<String>,
        slug: impl Into<String>,
        platform: impl Into<String>,
        source: impl Into<String>,
        risk_level: impl Into<String>,
        requires_approval: bool,
        trust_level: impl Into<String>,
    ) -> Self {
        let now = current_timestamp();
        Self {
            id: id.into(),
            slug: slug.into(),
            kind: "connector_backed".to_string(),
            source: source.into(),
            platform: platform.into(),
            risk_level: risk_level.into(),
            requires_approval,
            input_schema_uri: None,
            output_schema_uri: None,
            fallback_capability_id: None,
            trust_level: trust_level.into(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityBindingRecord {
    pub id: String,
    pub capability_id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub scope_ref: String,
    pub binding_status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl CapabilityBindingRecord {
    pub fn project_scope(
        id: impl Into<String>,
        capability_id: impl Into<String>,
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
    ) -> Self {
        let workspace_id = workspace_id.into();
        let project_id = project_id.into();
        let now = current_timestamp();
        Self {
            id: id.into(),
            capability_id: capability_id.into(),
            workspace_id: workspace_id.clone(),
            project_id: project_id.clone(),
            scope_ref: project_scope_ref(&workspace_id, &project_id),
            binding_status: "active".to_string(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityGrantRecord {
    pub id: String,
    pub capability_id: String,
    pub subject_ref: String,
    pub grant_status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl CapabilityGrantRecord {
    pub fn project_scope(
        id: impl Into<String>,
        capability_id: impl Into<String>,
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
    ) -> Self {
        let workspace_id = workspace_id.into();
        let project_id = project_id.into();
        let now = current_timestamp();
        Self {
            id: id.into(),
            capability_id: capability_id.into(),
            subject_ref: project_scope_ref(&workspace_id, &project_id),
            grant_status: "active".to_string(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BudgetPolicyRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub soft_cost_limit: i64,
    pub hard_cost_limit: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl BudgetPolicyRecord {
    pub fn project_scope(
        id: impl Into<String>,
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
        soft_cost_limit: i64,
        hard_cost_limit: i64,
    ) -> Self {
        let now = current_timestamp();
        Self {
            id: id.into(),
            workspace_id: workspace_id.into(),
            project_id: project_id.into(),
            soft_cost_limit,
            hard_cost_limit,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalRequestRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub run_id: String,
    pub task_id: String,
    pub approval_type: String,
    pub target_ref: String,
    pub status: String,
    pub reason: String,
    pub dedupe_key: String,
    pub decided_by: Option<String>,
    pub decision_note: Option<String>,
    pub decided_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl ApprovalRequestRecord {
    pub fn new_execution(
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
        run_id: impl Into<String>,
        task_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        let run_id = run_id.into();
        let now = current_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            project_id: project_id.into(),
            run_id: run_id.clone(),
            task_id: task_id.into(),
            approval_type: APPROVAL_TYPE_EXECUTION.to_string(),
            target_ref: format!("run:{run_id}"),
            status: "pending".to_string(),
            reason: reason.into(),
            dedupe_key: format!("approval:{run_id}"),
            decided_by: None,
            decision_note: None,
            decided_at: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn new_knowledge_promotion(
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
        run_id: impl Into<String>,
        task_id: impl Into<String>,
        candidate_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let candidate_id = candidate_id.into();
        let now = current_timestamp();
        Self {
            id: id.clone(),
            workspace_id: workspace_id.into(),
            project_id: project_id.into(),
            run_id: run_id.into(),
            task_id: task_id.into(),
            approval_type: APPROVAL_TYPE_KNOWLEDGE_PROMOTION.to_string(),
            target_ref: format!("knowledge_candidate:{candidate_id}"),
            status: "pending".to_string(),
            reason: reason.into(),
            dedupe_key: format!("knowledge_promotion:{candidate_id}:{id}"),
            decided_by: None,
            decision_note: None,
            decided_at: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecision {
    Approve,
    Reject,
    Expire,
    Cancel,
}

impl ApprovalDecision {
    pub fn target_status(self) -> &'static str {
        match self {
            Self::Approve => "approved",
            Self::Reject => "rejected",
            Self::Expire => "expired",
            Self::Cancel => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovernanceDecision {
    Allow { reason: String },
    RequireApproval { reason: String },
    Deny { reason: String },
}

impl GovernanceDecision {
    pub fn decision(&self) -> &'static str {
        match self {
            Self::Allow { .. } => DECISION_ALLOW,
            Self::RequireApproval { .. } => DECISION_REQUIRE_APPROVAL,
            Self::Deny { .. } => DECISION_DENY,
        }
    }

    pub fn reason(&self) -> &str {
        match self {
            Self::Allow { reason } | Self::RequireApproval { reason } | Self::Deny { reason } => {
                reason.as_str()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskGovernanceInput {
    pub workspace_id: String,
    pub project_id: String,
    pub capability_id: String,
    pub estimated_cost: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityResolutionRecord {
    pub descriptor: CapabilityDescriptorRecord,
    pub scope_ref: String,
    pub execution_state: String,
    pub reason_code: String,
    pub explanation: String,
}

#[derive(Debug, Error)]
pub enum GovernanceStoreError {
    #[error("approval request `{0}` not found")]
    ApprovalRequestNotFound(String),
    #[error("invalid approval transition for `{approval_id}`: `{from}` -> `{to}`")]
    InvalidApprovalTransition {
        approval_id: String,
        from: String,
        to: String,
    },
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Debug, Clone)]
pub struct SqliteGovernanceStore {
    pool: SqlitePool,
}

impl SqliteGovernanceStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn upsert_capability_descriptor(
        &self,
        record: &CapabilityDescriptorRecord,
    ) -> Result<(), GovernanceStoreError> {
        sqlx::query(
            r#"
            INSERT INTO capability_descriptors (
                id, slug, kind, source, platform, risk_level, requires_approval,
                input_schema_uri, output_schema_uri, fallback_capability_id, trust_level,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(id) DO UPDATE SET
                slug = excluded.slug,
                kind = excluded.kind,
                source = excluded.source,
                platform = excluded.platform,
                risk_level = excluded.risk_level,
                requires_approval = excluded.requires_approval,
                input_schema_uri = excluded.input_schema_uri,
                output_schema_uri = excluded.output_schema_uri,
                fallback_capability_id = excluded.fallback_capability_id,
                trust_level = excluded.trust_level,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&record.id)
        .bind(&record.slug)
        .bind(&record.kind)
        .bind(&record.source)
        .bind(&record.platform)
        .bind(&record.risk_level)
        .bind(record.requires_approval)
        .bind(&record.input_schema_uri)
        .bind(&record.output_schema_uri)
        .bind(&record.fallback_capability_id)
        .bind(&record.trust_level)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn upsert_capability_binding(
        &self,
        record: &CapabilityBindingRecord,
    ) -> Result<(), GovernanceStoreError> {
        sqlx::query(
            r#"
            INSERT INTO capability_bindings (
                id, capability_id, workspace_id, project_id, scope_ref, binding_status, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO UPDATE SET
                capability_id = excluded.capability_id,
                workspace_id = excluded.workspace_id,
                project_id = excluded.project_id,
                scope_ref = excluded.scope_ref,
                binding_status = excluded.binding_status,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&record.id)
        .bind(&record.capability_id)
        .bind(&record.workspace_id)
        .bind(&record.project_id)
        .bind(&record.scope_ref)
        .bind(&record.binding_status)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn upsert_capability_grant(
        &self,
        record: &CapabilityGrantRecord,
    ) -> Result<(), GovernanceStoreError> {
        sqlx::query(
            r#"
            INSERT INTO capability_grants (
                id, capability_id, subject_ref, grant_status, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(id) DO UPDATE SET
                capability_id = excluded.capability_id,
                subject_ref = excluded.subject_ref,
                grant_status = excluded.grant_status,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&record.id)
        .bind(&record.capability_id)
        .bind(&record.subject_ref)
        .bind(&record.grant_status)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn upsert_budget_policy(
        &self,
        record: &BudgetPolicyRecord,
    ) -> Result<(), GovernanceStoreError> {
        sqlx::query(
            r#"
            INSERT INTO budget_policies (
                id, workspace_id, project_id, soft_cost_limit, hard_cost_limit, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO UPDATE SET
                workspace_id = excluded.workspace_id,
                project_id = excluded.project_id,
                soft_cost_limit = excluded.soft_cost_limit,
                hard_cost_limit = excluded.hard_cost_limit,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&record.id)
        .bind(&record.workspace_id)
        .bind(&record.project_id)
        .bind(record.soft_cost_limit)
        .bind(record.hard_cost_limit)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn evaluate_task(
        &self,
        input: &TaskGovernanceInput,
    ) -> Result<GovernanceDecision, GovernanceStoreError> {
        let Some(descriptor) = self
            .fetch_capability_descriptor(input.capability_id.as_str())
            .await?
        else {
            return Ok(GovernanceDecision::Deny {
                reason: "capability_not_registered".to_string(),
            });
        };

        let resolution = self
            .resolve_project_bound_capability(
                descriptor,
                input.workspace_id.as_str(),
                input.project_id.as_str(),
                input.estimated_cost,
            )
            .await?;

        Ok(match resolution.execution_state.as_str() {
            EXECUTION_STATE_EXECUTABLE => GovernanceDecision::Allow {
                reason: resolution.reason_code,
            },
            EXECUTION_STATE_APPROVAL_REQUIRED => GovernanceDecision::RequireApproval {
                reason: resolution.reason_code,
            },
            _ => GovernanceDecision::Deny {
                reason: resolution.reason_code,
            },
        })
    }

    pub async fn find_approval_request_by_dedupe_key(
        &self,
        dedupe_key: &str,
    ) -> Result<Option<ApprovalRequestRecord>, GovernanceStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, run_id, task_id, approval_type, target_ref,
                   status, reason, dedupe_key, decided_by, decision_note, decided_at,
                   created_at, updated_at
            FROM approval_requests
            WHERE dedupe_key = ?1
            "#,
        )
        .bind(dedupe_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| approval_request_from_row(&row)).transpose()?)
    }

    pub async fn find_open_approval_request_by_target_ref(
        &self,
        approval_type: &str,
        target_ref: &str,
    ) -> Result<Option<ApprovalRequestRecord>, GovernanceStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, run_id, task_id, approval_type, target_ref,
                   status, reason, dedupe_key, decided_by, decision_note, decided_at,
                   created_at, updated_at
            FROM approval_requests
            WHERE approval_type = ?1
              AND target_ref = ?2
              AND status = 'pending'
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            "#,
        )
        .bind(approval_type)
        .bind(target_ref)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| approval_request_from_row(&row)).transpose()?)
    }

    pub async fn create_approval_request(
        &self,
        request: &ApprovalRequestRecord,
    ) -> Result<(), GovernanceStoreError> {
        sqlx::query(
            r#"
            INSERT INTO approval_requests (
                id, workspace_id, project_id, run_id, task_id, approval_type, target_ref,
                status, reason, dedupe_key, decided_by, decision_note, decided_at, created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            "#,
        )
        .bind(&request.id)
        .bind(&request.workspace_id)
        .bind(&request.project_id)
        .bind(&request.run_id)
        .bind(&request.task_id)
        .bind(&request.approval_type)
        .bind(&request.target_ref)
        .bind(&request.status)
        .bind(&request.reason)
        .bind(&request.dedupe_key)
        .bind(&request.decided_by)
        .bind(&request.decision_note)
        .bind(&request.decided_at)
        .bind(&request.created_at)
        .bind(&request.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn fetch_approval_request(
        &self,
        approval_id: &str,
    ) -> Result<Option<ApprovalRequestRecord>, GovernanceStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, run_id, task_id, approval_type, target_ref,
                   status, reason, dedupe_key, decided_by, decision_note, decided_at,
                   created_at, updated_at
            FROM approval_requests
            WHERE id = ?1
            "#,
        )
        .bind(approval_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| approval_request_from_row(&row)).transpose()?)
    }

    pub async fn list_approval_requests_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<ApprovalRequestRecord>, GovernanceStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, run_id, task_id, approval_type, target_ref,
                   status, reason, dedupe_key, decided_by, decision_note, decided_at,
                   created_at, updated_at
            FROM approval_requests
            WHERE run_id = ?1
            ORDER BY created_at, id
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(approval_request_from_row)
            .collect::<Result<Vec<_>, _>>()
            .map_err(GovernanceStoreError::from)
    }

    pub async fn resolve_approval_request(
        &self,
        approval_id: &str,
        decision: ApprovalDecision,
        actor_ref: &str,
        note: &str,
    ) -> Result<ApprovalRequestRecord, GovernanceStoreError> {
        let mut request = self
            .fetch_approval_request(approval_id)
            .await?
            .ok_or_else(|| {
                GovernanceStoreError::ApprovalRequestNotFound(approval_id.to_string())
            })?;

        let target = decision.target_status().to_string();
        if request.status == target {
            return Ok(request);
        }
        if request.status != "pending" {
            return Err(GovernanceStoreError::InvalidApprovalTransition {
                approval_id: approval_id.to_string(),
                from: request.status,
                to: target,
            });
        }

        request.status = target;
        request.decided_by = Some(actor_ref.to_string());
        request.decision_note = Some(note.to_string());
        request.decided_at = Some(current_timestamp());
        request.updated_at = request.decided_at.clone().unwrap();

        sqlx::query(
            r#"
            UPDATE approval_requests
            SET status = ?2,
                decided_by = ?3,
                decision_note = ?4,
                decided_at = ?5,
                updated_at = ?6
            WHERE id = ?1
            "#,
        )
        .bind(&request.id)
        .bind(&request.status)
        .bind(&request.decided_by)
        .bind(&request.decision_note)
        .bind(&request.decided_at)
        .bind(&request.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(request)
    }

    pub async fn fetch_capability_descriptor(
        &self,
        capability_id: &str,
    ) -> Result<Option<CapabilityDescriptorRecord>, GovernanceStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, slug, kind, source, platform, risk_level, requires_approval,
                   input_schema_uri, output_schema_uri, fallback_capability_id, trust_level,
                   created_at, updated_at
            FROM capability_descriptors
            WHERE id = ?1
            "#,
        )
        .bind(capability_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row
            .map(|row| capability_descriptor_from_row(&row))
            .transpose()?)
    }

    pub async fn list_project_bound_capability_descriptors(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Vec<CapabilityDescriptorRecord>, GovernanceStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT d.id, d.slug, d.kind, d.source, d.platform, d.risk_level,
                   d.requires_approval, d.input_schema_uri, d.output_schema_uri,
                   d.fallback_capability_id, d.trust_level, d.created_at, d.updated_at
            FROM capability_descriptors d
            INNER JOIN capability_bindings b
                ON b.capability_id = d.id
            WHERE b.workspace_id = ?1
              AND b.project_id = ?2
              AND b.binding_status = 'active'
            ORDER BY d.slug ASC
            "#,
        )
        .bind(workspace_id)
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| capability_descriptor_from_row(&row))
            .collect::<Result<Vec<_>, _>>()
            .map_err(GovernanceStoreError::from)
    }

    pub async fn list_visible_capability_descriptors(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Vec<CapabilityDescriptorRecord>, GovernanceStoreError> {
        self.list_project_bound_capability_descriptors(workspace_id, project_id)
            .await
    }

    pub async fn resolve_project_bound_capability(
        &self,
        descriptor: CapabilityDescriptorRecord,
        workspace_id: &str,
        project_id: &str,
        estimated_cost: i64,
    ) -> Result<CapabilityResolutionRecord, GovernanceStoreError> {
        let default_scope_ref = project_scope_ref(workspace_id, project_id);
        let Some(binding) = self
            .fetch_active_binding(descriptor.id.as_str(), workspace_id, project_id)
            .await?
        else {
            return Ok(build_capability_resolution(
                descriptor,
                default_scope_ref,
                EXECUTION_STATE_DENIED,
                "capability_not_bound",
                project_id,
                estimated_cost,
                None,
                None,
            ));
        };

        if binding.binding_status != "active" {
            return Ok(build_capability_resolution(
                descriptor,
                binding.scope_ref,
                EXECUTION_STATE_DENIED,
                "capability_not_bound",
                project_id,
                estimated_cost,
                None,
                None,
            ));
        }

        let scope_ref = binding.scope_ref;
        let subject_ref = project_scope_ref(workspace_id, project_id);
        let Some(grant) = self
            .fetch_active_grant(descriptor.id.as_str(), subject_ref.as_str())
            .await?
        else {
            return Ok(build_capability_resolution(
                descriptor,
                scope_ref,
                EXECUTION_STATE_DENIED,
                "capability_not_granted",
                project_id,
                estimated_cost,
                None,
                None,
            ));
        };

        if grant.grant_status != "active" {
            return Ok(build_capability_resolution(
                descriptor,
                scope_ref,
                EXECUTION_STATE_DENIED,
                "capability_not_granted",
                project_id,
                estimated_cost,
                None,
                None,
            ));
        }

        let Some(policy) = self.fetch_budget_policy(workspace_id, project_id).await? else {
            return Ok(build_capability_resolution(
                descriptor,
                scope_ref,
                EXECUTION_STATE_DENIED,
                "budget_policy_missing",
                project_id,
                estimated_cost,
                None,
                None,
            ));
        };

        if estimated_cost > policy.hard_cost_limit {
            return Ok(build_capability_resolution(
                descriptor,
                scope_ref,
                EXECUTION_STATE_DENIED,
                "budget_hard_limit_exceeded",
                project_id,
                estimated_cost,
                Some(policy.soft_cost_limit),
                Some(policy.hard_cost_limit),
            ));
        }

        if descriptor.requires_approval || descriptor.risk_level == "high" {
            return Ok(build_capability_resolution(
                descriptor,
                scope_ref,
                EXECUTION_STATE_APPROVAL_REQUIRED,
                "risk_level_high",
                project_id,
                estimated_cost,
                Some(policy.soft_cost_limit),
                Some(policy.hard_cost_limit),
            ));
        }

        if estimated_cost > policy.soft_cost_limit {
            return Ok(build_capability_resolution(
                descriptor,
                scope_ref,
                EXECUTION_STATE_APPROVAL_REQUIRED,
                "budget_soft_limit_exceeded",
                project_id,
                estimated_cost,
                Some(policy.soft_cost_limit),
                Some(policy.hard_cost_limit),
            ));
        }

        Ok(build_capability_resolution(
            descriptor,
            scope_ref,
            EXECUTION_STATE_EXECUTABLE,
            "within_budget",
            project_id,
            estimated_cost,
            Some(policy.soft_cost_limit),
            Some(policy.hard_cost_limit),
        ))
    }

    async fn fetch_active_binding(
        &self,
        capability_id: &str,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Option<CapabilityBindingRecord>, GovernanceStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, capability_id, workspace_id, project_id, scope_ref, binding_status, created_at, updated_at
            FROM capability_bindings
            WHERE capability_id = ?1 AND workspace_id = ?2 AND project_id = ?3
            "#,
        )
        .bind(capability_id)
        .bind(workspace_id)
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row
            .map(|row| capability_binding_from_row(&row))
            .transpose()?)
    }

    async fn fetch_active_grant(
        &self,
        capability_id: &str,
        subject_ref: &str,
    ) -> Result<Option<CapabilityGrantRecord>, GovernanceStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, capability_id, subject_ref, grant_status, created_at, updated_at
            FROM capability_grants
            WHERE capability_id = ?1 AND subject_ref = ?2
            "#,
        )
        .bind(capability_id)
        .bind(subject_ref)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| capability_grant_from_row(&row)).transpose()?)
    }

    async fn fetch_budget_policy(
        &self,
        workspace_id: &str,
        project_id: &str,
    ) -> Result<Option<BudgetPolicyRecord>, GovernanceStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, soft_cost_limit, hard_cost_limit, created_at, updated_at
            FROM budget_policies
            WHERE workspace_id = ?1 AND project_id = ?2
            ORDER BY created_at, id
            LIMIT 1
            "#,
        )
        .bind(workspace_id)
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| budget_policy_from_row(&row)).transpose()?)
    }
}

pub fn project_scope_ref(workspace_id: &str, project_id: &str) -> String {
    format!("workspace:{workspace_id}/project:{project_id}")
}

fn capability_resolution(
    descriptor: CapabilityDescriptorRecord,
    scope_ref: String,
    execution_state: &str,
    reason_code: &str,
    explanation: String,
) -> CapabilityResolutionRecord {
    CapabilityResolutionRecord {
        descriptor,
        scope_ref,
        execution_state: execution_state.to_string(),
        reason_code: reason_code.to_string(),
        explanation,
    }
}

fn build_capability_resolution(
    descriptor: CapabilityDescriptorRecord,
    scope_ref: String,
    execution_state: &str,
    reason_code: &str,
    project_id: &str,
    estimated_cost: i64,
    soft_limit: Option<i64>,
    hard_limit: Option<i64>,
) -> CapabilityResolutionRecord {
    let explanation = capability_resolution_explanation(
        reason_code,
        &descriptor,
        project_id,
        estimated_cost,
        soft_limit,
        hard_limit,
    );

    capability_resolution(
        descriptor,
        scope_ref,
        execution_state,
        reason_code,
        explanation,
    )
}

fn capability_resolution_explanation(
    reason_code: &str,
    descriptor: &CapabilityDescriptorRecord,
    project_id: &str,
    estimated_cost: i64,
    soft_limit: Option<i64>,
    hard_limit: Option<i64>,
) -> String {
    match reason_code {
        "capability_not_bound" => format!(
            "Denied because capability `{}` is not actively bound to project `{project_id}`.",
            descriptor.slug
        ),
        "capability_not_granted" => format!(
            "Denied because capability `{}` does not have an active project grant for `{project_id}`.",
            descriptor.slug
        ),
        "budget_policy_missing" => format!(
            "Denied because project `{project_id}` does not have a budget policy for capability `{}`.",
            descriptor.slug
        ),
        "budget_hard_limit_exceeded" => format!(
            "Denied because the estimated cost {estimated_cost} exceeds the hard cost limit {}.",
            hard_limit.unwrap_or_default()
        ),
        "risk_level_high" => {
            if descriptor.requires_approval {
                format!(
                    "Approval required because capability `{}` is marked to require approval.",
                    descriptor.slug
                )
            } else {
                format!(
                    "Approval required because capability `{}` is classified as high risk.",
                    descriptor.slug
                )
            }
        }
        "budget_soft_limit_exceeded" => format!(
            "Approval required because the estimated cost {estimated_cost} exceeds the soft cost limit {}.",
            soft_limit.unwrap_or_default()
        ),
        "within_budget" => format!(
            "Executable because capability `{}` is bound, granted, and the estimated cost {estimated_cost} is within the current budget.",
            descriptor.slug
        ),
        "capability_not_registered" => format!(
            "Denied because capability `{}` is not registered in the governed runtime.",
            descriptor.id
        ),
        _ => format!(
            "Capability `{}` resolved with governance reason `{reason_code}` for project `{project_id}`.",
            descriptor.slug
        ),
    }
}

fn capability_descriptor_from_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<CapabilityDescriptorRecord, sqlx::Error> {
    Ok(CapabilityDescriptorRecord {
        id: row.try_get("id")?,
        slug: row.try_get("slug")?,
        kind: row.try_get("kind")?,
        source: row.try_get("source")?,
        platform: row.try_get("platform")?,
        risk_level: row.try_get("risk_level")?,
        requires_approval: row.try_get("requires_approval")?,
        input_schema_uri: row.try_get("input_schema_uri")?,
        output_schema_uri: row.try_get("output_schema_uri")?,
        fallback_capability_id: row.try_get("fallback_capability_id")?,
        trust_level: row.try_get("trust_level")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn capability_binding_from_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<CapabilityBindingRecord, sqlx::Error> {
    Ok(CapabilityBindingRecord {
        id: row.try_get("id")?,
        capability_id: row.try_get("capability_id")?,
        workspace_id: row.try_get("workspace_id")?,
        project_id: row.try_get("project_id")?,
        scope_ref: row.try_get("scope_ref")?,
        binding_status: row.try_get("binding_status")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn capability_grant_from_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<CapabilityGrantRecord, sqlx::Error> {
    Ok(CapabilityGrantRecord {
        id: row.try_get("id")?,
        capability_id: row.try_get("capability_id")?,
        subject_ref: row.try_get("subject_ref")?,
        grant_status: row.try_get("grant_status")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn budget_policy_from_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<BudgetPolicyRecord, sqlx::Error> {
    Ok(BudgetPolicyRecord {
        id: row.try_get("id")?,
        workspace_id: row.try_get("workspace_id")?,
        project_id: row.try_get("project_id")?,
        soft_cost_limit: row.try_get("soft_cost_limit")?,
        hard_cost_limit: row.try_get("hard_cost_limit")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn approval_request_from_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<ApprovalRequestRecord, sqlx::Error> {
    Ok(ApprovalRequestRecord {
        id: row.try_get("id")?,
        workspace_id: row.try_get("workspace_id")?,
        project_id: row.try_get("project_id")?,
        run_id: row.try_get("run_id")?,
        task_id: row.try_get("task_id")?,
        approval_type: row.try_get("approval_type")?,
        target_ref: row.try_get("target_ref")?,
        status: row.try_get("status")?,
        reason: row.try_get("reason")?,
        dedupe_key: row.try_get("dedupe_key")?,
        decided_by: row.try_get("decided_by")?,
        decision_note: row.try_get("decision_note")?,
        decided_at: row.try_get("decided_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}
