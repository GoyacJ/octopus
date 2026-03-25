export interface CoreObjectContract {
  name: string
  bounded_context: string
  required_fields: string[]
  notes: string
}

export interface EventSkeleton {
  name: string
  category: string
  required_fields: string[]
}

export const runTypeValues = [
  'task',
  'discussion',
  'automation',
  'watch',
  'delegation',
  'review',
] as const

export const runStatusValues = [
  'queued',
  'planning',
  'running',
  'waiting_input',
  'waiting_approval',
  'waiting_dependency',
  'paused',
  'recovering',
  'completed',
  'failed',
  'terminated',
  'cancelled',
] as const

export const approvalTypeValues = [
  'execution',
  'knowledge_promotion',
  'external_delegation',
  'export_sharing',
] as const

export const triggerSourceValues = ['cron', 'webhook', 'manual_event', 'mcp_event'] as const

export const sandboxTierValues = [
  'local_trusted',
  'tenant_sandboxed',
  'ephemeral_restricted',
  'external_delegated',
] as const

export const knowledgeStatusValues = [
  'candidate',
  'verified_shared',
  'promoted_org',
  'revoked_or_tombstoned',
] as const

export const trustLevelValues = ['low', 'medium', 'high', 'verified'] as const

export type RunType = (typeof runTypeValues)[number]
export type RunStatus = (typeof runStatusValues)[number]
export type ApprovalType = (typeof approvalTypeValues)[number]
export type TriggerSource = (typeof triggerSourceValues)[number]
export type SandboxTier = (typeof sandboxTierValues)[number]
export type KnowledgeStatus = (typeof knowledgeStatusValues)[number]
export type TrustLevel = (typeof trustLevelValues)[number]

export const contractCatalog = {
  version: '1.0.0',
  coreObjects: [
    {
      name: 'HubConnection',
      bounded_context: 'Access & Identity',
      required_fields: ['id', 'name', 'endpoint', 'mode', 'auth_strategy', 'last_status'],
      notes: 'Client 持有的连接配置，不是远程业务事实源。',
    },
    {
      name: 'Workspace',
      bounded_context: 'Workspace & Project',
      required_fields: ['id', 'tenant_id', 'name', 'default_budget_policy_id', 'state'],
      notes: '共享协作、共享知识和共享治理的基本边界。',
    },
    {
      name: 'Project',
      bounded_context: 'Workspace & Project',
      required_fields: ['id', 'workspace_id', 'name', 'attached_space_ids', 'state'],
      notes: '承载 Run、Artifact 和 KnowledgeSpace 视图。',
    },
    {
      name: 'Agent',
      bounded_context: 'Agent Registry',
      required_fields: ['id', 'workspace_id', 'identity', 'capability_profile', 'knowledge_scope', 'state'],
      notes: '统一数字协作者对象。',
    },
    {
      name: 'Team',
      bounded_context: 'Collaboration Mesh',
      required_fields: [
        'id',
        'workspace_id',
        'coordination_mode',
        'member_ids',
        'authority_scope',
        'knowledge_scope',
        'delegation_edges',
      ],
      notes: '协作拓扑单元，不作为 Shared Knowledge 主属边界。',
    },
    {
      name: 'Run',
      bounded_context: 'Run Orchestration',
      required_fields: ['id', 'project_id', 'run_type', 'status', 'idempotency_key', 'requested_by', 'created_at'],
      notes: '所有正式执行的权威外壳。',
    },
    {
      name: 'Task',
      bounded_context: 'Run Orchestration',
      required_fields: ['id', 'run_id', 'title', 'goal', 'state'],
      notes: '交付导向业务对象。',
    },
    {
      name: 'Automation',
      bounded_context: 'Run Orchestration',
      required_fields: ['id', 'workspace_id', 'name', 'trigger_ids', 'budget_policy_id', 'state'],
      notes: '可重复执行定义。',
    },
    {
      name: 'Trigger',
      bounded_context: 'Run Orchestration',
      required_fields: ['id', 'source_type', 'dedupe_key', 'owner_ref', 'state'],
      notes: '启动或续接 Run 的正式触发器。',
    },
    {
      name: 'CapabilityGrant',
      bounded_context: 'Governance & Policy',
      required_fields: [
        'id',
        'subject_ref',
        'scope',
        'tool_set',
        'environment_scope',
        'protocol_scope',
        'effective_window',
      ],
      notes: '主体能力授权窗口。',
    },
    {
      name: 'BudgetPolicy',
      bounded_context: 'Governance & Policy',
      required_fields: [
        'id',
        'owner_ref',
        'model_budget',
        'token_budget',
        'tool_action_budget',
        'runtime_ceiling',
        'escalation_condition',
      ],
      notes: '约束成本、动作次数和时窗。',
    },
    {
      name: 'ApprovalRequest',
      bounded_context: 'Governance & Policy',
      required_fields: ['id', 'run_id', 'approval_type', 'state', 'target_ref', 'requested_at'],
      notes: '高风险动作、知识晋升和导出的正式审批对象。',
    },
    {
      name: 'EnvironmentLease',
      bounded_context: 'Execution Management',
      required_fields: ['id', 'run_id', 'sandbox_tier', 'state', 'expiry_at', 'resume_token'],
      notes: '环境租约与恢复锚点。',
    },
    {
      name: 'Artifact',
      bounded_context: 'Artifact & Inbox',
      required_fields: ['id', 'project_id', 'run_id', 'version', 'title', 'content_ref', 'state'],
      notes: '正式结果对象。',
    },
    {
      name: 'InboxItem',
      bounded_context: 'Artifact & Inbox',
      required_fields: ['id', 'workspace_id', 'owner_ref', 'state', 'priority', 'target_ref', 'dedupe_key'],
      notes: '正式待处理项。',
    },
    {
      name: 'KnowledgeSpace',
      bounded_context: 'Knowledge System',
      required_fields: ['id', 'workspace_id', 'name', 'owner_refs', 'scope', 'state'],
      notes: 'Shared Knowledge 权威容器。',
    },
    {
      name: 'KnowledgeAsset',
      bounded_context: 'Knowledge System',
      required_fields: ['id', 'knowledge_space_id', 'layer', 'status', 'trust_level', 'source_ref'],
      notes: '正式知识条目。',
    },
    {
      name: 'DelegationGrant',
      bounded_context: 'Collaboration Mesh',
      required_fields: ['id', 'run_id', 'authority_scope', 'budget_policy_id', 'expiry_at', 'state'],
      notes: '某次委托的 authority、budget 和 expiry 约束。',
    },
    {
      name: 'A2APeer',
      bounded_context: 'Interop Gateway',
      required_fields: ['id', 'tenant_id', 'display_name', 'trust_level', 'peer_status', 'capability_summary'],
      notes: '已登记的外部 Agent 对端。',
    },
    {
      name: 'ExternalAgentIdentity',
      bounded_context: 'Interop Gateway',
      required_fields: ['id', 'peer_id', 'subject_claim', 'delegated_authority', 'identity_status'],
      notes: '绑定在具体 A2A 对端上的外部主体身份声明。',
    },
  ] satisfies CoreObjectContract[],
  enums: {
    run_type: [...runTypeValues],
    run_status: [...runStatusValues],
    approval_type: [...approvalTypeValues],
    trigger_source: [...triggerSourceValues],
    sandbox_tier: [...sandboxTierValues],
    knowledge_status: [...knowledgeStatusValues],
    trust_level: [...trustLevelValues],
  },
  events: [
    {
      name: 'RunStateChanged',
      category: 'runtime',
      required_fields: ['run_id', 'previous_status', 'next_status', 'run_type', 'occurred_at'],
    },
    {
      name: 'PolicyDecisionRecorded',
      category: 'governance',
      required_fields: ['subject_ref', 'action', 'decision', 'policy_layer', 'reason', 'occurred_at'],
    },
    {
      name: 'ApprovalRequested',
      category: 'governance',
      required_fields: ['approval_id', 'run_id', 'approval_type', 'target_ref', 'occurred_at'],
    },
    {
      name: 'ApprovalResolved',
      category: 'governance',
      required_fields: ['approval_id', 'run_id', 'decision', 'reviewed_by', 'occurred_at'],
    },
    {
      name: 'KnowledgeCandidateCreated',
      category: 'knowledge',
      required_fields: ['candidate_id', 'source_ref', 'knowledge_space_id', 'trust_level', 'occurred_at'],
    },
    {
      name: 'KnowledgeCandidatePromoted',
      category: 'knowledge',
      required_fields: ['knowledge_asset_id', 'knowledge_space_id', 'promoted_to', 'approved_by', 'occurred_at'],
    },
    {
      name: 'TriggerDelivered',
      category: 'runtime',
      required_fields: ['trigger_id', 'source_type', 'dedupe_key', 'delivery_state', 'occurred_at'],
    },
    {
      name: 'DelegationIssued',
      category: 'mesh',
      required_fields: ['delegation_grant_id', 'run_id', 'issuer_ref', 'delegate_ref', 'occurred_at'],
    },
  ] satisfies EventSkeleton[],
}

export const interactionSurfaces = [
  'Chat',
  'Board',
  'Trace',
  'Inbox',
  'Knowledge',
  'Workspace',
  'Connections',
] as const

