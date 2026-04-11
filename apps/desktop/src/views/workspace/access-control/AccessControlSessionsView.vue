<script setup lang="ts">
import { computed, h, reactive, ref } from 'vue'

import {
  type ColumnDef,
  UiBadge,
  UiButton,
  UiDataTable,
  UiField,
  UiInput,
  UiListDetailWorkspace,
  UiPanelFrame,
  UiSelect,
  UiStatusCallout,
  UiTabs,
  UiToolbarRow,
} from '@octopus/ui'
import type { AccessSessionRecord, AuditRecord } from '@octopus/schema'

import { formatDateTime } from '@/i18n/copy'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'

const accessControlStore = useWorkspaceAccessControlStore()

const activeTab = ref('sessions')
const selectedSessionId = ref('')
const selectedAuditId = ref('')
const sessionStatusFilter = ref('')
const pendingSessionId = ref('')
const pendingUserId = ref('')
const submitError = ref('')
const auditFilters = reactive({
  actorId: '',
  action: '',
  resourceType: '',
  outcome: '',
  from: '',
  to: '',
})

const tabs = [
  { value: 'sessions', label: '会话' },
  { value: 'audit', label: '审计' },
]
const auditOutcomeOptions = [
  { label: '全部结果', value: '' },
  { label: '成功', value: 'success' },
  { label: '拒绝', value: 'denied' },
  { label: '失败', value: 'failure' },
  { label: '锁定', value: 'locked' },
]
const sessionStatusOptions = [
  { label: '全部状态', value: '' },
  { label: '活跃', value: 'active' },
  { label: '已撤销', value: 'revoked' },
  { label: '已过期', value: 'expired' },
]

const filteredSessions = computed(() =>
  accessControlStore.sessions.filter(session =>
    !sessionStatusFilter.value || session.status === sessionStatusFilter.value,
  ),
)
const selectedSession = computed(() =>
  accessControlStore.sessions.find(session => session.sessionId === selectedSessionId.value) ?? null,
)
const selectedAuditRecord = computed(() =>
  accessControlStore.auditRecords.find(record => record.id === selectedAuditId.value) ?? null,
)
const auditStats = computed(() => ({
  total: accessControlStore.auditRecords.length,
  denied: accessControlStore.auditRecords.filter(record => record.outcome === 'denied').length,
  sensitive: accessControlStore.auditRecords.filter(record =>
    /(run|invoke|publish|delete|grant|export|retrieve|bind-credential)/.test(record.action),
  ).length,
}))

const sessionColumns: ColumnDef<AccessSessionRecord>[] = [
  {
    accessorKey: 'displayName',
    header: '会话',
    cell: ({ row }) => h('div', { class: 'space-y-1' }, [
      h('div', { class: 'text-sm font-medium text-text-primary' }, row.original.displayName),
      h('div', { class: 'text-xs text-text-secondary' }, `${row.original.username} / ${row.original.clientAppId}`),
    ]),
  },
  {
    accessorKey: 'createdAt',
    header: '创建时间',
    cell: ({ row }) => formatDateTime(row.original.createdAt),
  },
  {
    accessorKey: 'status',
    header: '状态',
    cell: ({ row }) => h(UiBadge, {
      label: row.original.current ? '当前会话' : row.original.status,
      tone: row.original.status === 'active' ? 'success' : 'default',
      subtle: true,
    }),
  },
]

const auditColumns: ColumnDef<AuditRecord>[] = [
  {
    accessorKey: 'action',
    header: '动作',
    cell: ({ row }) => h('div', { class: 'space-y-1' }, [
      h('div', { class: 'text-sm font-medium text-text-primary' }, row.original.action),
      h('div', { class: 'text-xs text-text-secondary' }, `${row.original.actorType} / ${row.original.actorId}`),
    ]),
  },
  {
    accessorKey: 'resource',
    header: '资源',
    cell: ({ row }) => h('div', { class: 'text-xs text-text-secondary' }, row.original.resource),
  },
  {
    accessorKey: 'createdAt',
    header: '时间',
    cell: ({ row }) => formatDateTime(row.original.createdAt),
  },
  {
    accessorKey: 'outcome',
    header: '结果',
    cell: ({ row }) => h(UiBadge, {
      label: row.original.outcome,
      tone: row.original.outcome === 'success' ? 'success' : 'warning',
      subtle: true,
    }),
  },
]

function parseTimeInput(value: string): number | undefined {
  if (!value.trim()) {
    return undefined
  }
  const timestamp = Date.parse(value)
  return Number.isFinite(timestamp) ? timestamp : undefined
}

async function handleRevokeSession(sessionId: string) {
  pendingSessionId.value = sessionId
  try {
    await accessControlStore.revokeSession(sessionId)
  } finally {
    pendingSessionId.value = ''
  }
}

async function handleRevokeUserSessions(userId: string) {
  pendingUserId.value = userId
  try {
    await accessControlStore.revokeUserSessions(userId)
  } finally {
    pendingUserId.value = ''
  }
}

async function handleSearchAudit() {
  submitError.value = ''
  try {
    await accessControlStore.loadAudit({
      actorId: auditFilters.actorId.trim() || undefined,
      action: auditFilters.action.trim() || undefined,
      resourceType: auditFilters.resourceType.trim() || undefined,
      outcome: auditFilters.outcome || undefined,
      from: parseTimeInput(auditFilters.from),
      to: parseTimeInput(auditFilters.to),
    })
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '加载审计日志失败。'
  }
}

function resetAuditFilters() {
  Object.assign(auditFilters, {
    actorId: '',
    action: '',
    resourceType: '',
    outcome: '',
    from: '',
    to: '',
  })
}

async function handleLoadMoreAudit() {
  submitError.value = ''
  try {
    await accessControlStore.loadMoreAudit()
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '加载更多审计日志失败。'
  }
}

function handleSessionRowClick(row: AccessSessionRecord) {
  selectedSessionId.value = row.sessionId
}

function handleAuditRowClick(row: AuditRecord) {
  selectedAuditId.value = row.id
}
</script>

<template>
  <div class="space-y-4" data-testid="access-control-sessions-shell">
    <UiStatusCallout
      v-if="submitError || accessControlStore.auditError"
      tone="error"
      :description="submitError || accessControlStore.auditError"
    />

    <UiTabs v-model="activeTab" :tabs="tabs" data-testid="access-control-sessions-tabs" />

    <UiListDetailWorkspace
      v-if="activeTab === 'sessions'"
      :has-selection="Boolean(selectedSession)"
      :detail-title="selectedSession ? selectedSession.displayName : ''"
      detail-subtitle="会话撤销会在下一次请求生效。"
      empty-detail-title="请选择会话"
      empty-detail-description="从左侧会话列表中选择一项后即可查看详情并执行撤销操作。"
    >
      <template #toolbar>
        <UiToolbarRow test-id="access-control-sessions-toolbar">
          <template #filters>
            <UiField label="会话状态" class="w-full md:w-[180px]">
              <UiSelect v-model="sessionStatusFilter" :options="sessionStatusOptions" />
            </UiField>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame variant="panel" padding="md" title="会话管理" :subtitle="`共 ${filteredSessions.length} 条会话记录`">
          <UiDataTable
            :data="filteredSessions"
            :columns="sessionColumns"
            empty-title="暂无会话"
            empty-description="当前筛选条件下没有会话记录。"
            :on-row-click="handleSessionRowClick"
          />
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedSession" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ selectedSession.displayName }}</div>
              <UiBadge :label="selectedSession.status" :tone="selectedSession.status === 'active' ? 'success' : 'default'" subtle />
              <UiBadge v-if="selectedSession.current" label="当前会话" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">{{ selectedSession.username }} / {{ selectedSession.clientAppId }}</div>
            <div class="mt-1 text-xs text-muted-foreground">{{ formatDateTime(selectedSession.createdAt) }}</div>
          </div>

          <div class="flex flex-wrap gap-2">
            <UiButton
              v-if="!selectedSession.current && selectedSession.status === 'active'"
              variant="ghost"
              :loading="pendingSessionId === selectedSession.sessionId"
              @click="handleRevokeSession(selectedSession.sessionId)"
            >
              撤销当前会话
            </UiButton>
            <UiButton
              v-if="!selectedSession.current"
              variant="ghost"
              :loading="pendingUserId === selectedSession.userId"
              @click="handleRevokeUserSessions(selectedSession.userId)"
            >
              撤销该用户全部会话
            </UiButton>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>

    <UiListDetailWorkspace
      v-else
      :has-selection="Boolean(selectedAuditRecord)"
      :detail-title="selectedAuditRecord ? selectedAuditRecord.action : ''"
      detail-subtitle="查看事件主体、资源和结果，用于排查授权与审计问题。"
      empty-detail-title="请选择审计记录"
      empty-detail-description="从左侧审计结果中选择一项后即可查看详情。"
    >
      <template #toolbar>
        <UiToolbarRow test-id="access-control-sessions-toolbar">
          <template #filters>
            <UiField label="Actor ID" class="w-full md:w-[180px]">
              <UiInput v-model="auditFilters.actorId" data-testid="access-control-audit-actor" />
            </UiField>
            <UiField label="动作" class="w-full md:w-[160px]">
              <UiInput v-model="auditFilters.action" data-testid="access-control-audit-action" />
            </UiField>
            <UiField label="资源类型" class="w-full md:w-[160px]">
              <UiInput v-model="auditFilters.resourceType" data-testid="access-control-audit-resource-type" />
            </UiField>
            <UiField label="结果" class="w-full md:w-[160px]">
              <UiSelect v-model="auditFilters.outcome" :options="auditOutcomeOptions" data-testid="access-control-audit-outcome" />
            </UiField>
            <UiField label="开始时间" class="w-full md:w-[200px]">
              <UiInput v-model="auditFilters.from" data-testid="access-control-audit-from" />
            </UiField>
            <UiField label="结束时间" class="w-full md:w-[200px]">
              <UiInput v-model="auditFilters.to" data-testid="access-control-audit-to" />
            </UiField>
          </template>
          <template #actions>
            <div class="flex gap-2">
              <UiButton variant="ghost" @click="resetAuditFilters">
                重置
              </UiButton>
              <UiButton :loading="accessControlStore.auditLoading" data-testid="access-control-audit-search" @click="handleSearchAudit">
                查询审计
              </UiButton>
            </div>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame variant="panel" padding="md" title="审计日志" :subtitle="`已加载 ${auditStats.total} 条记录`">
          <UiDataTable
            :data="accessControlStore.auditRecords"
            :columns="auditColumns"
            empty-title="暂无审计记录"
            empty-description="当前筛选条件下没有命中的审计事件。"
            :on-row-click="handleAuditRowClick"
          >
            <template #toolbar>
              <div class="flex flex-wrap items-center gap-3 text-xs text-text-tertiary">
                <span>拒绝事件 {{ auditStats.denied }}</span>
                <span>敏感动作 {{ auditStats.sensitive }}</span>
              </div>
            </template>
          </UiDataTable>

          <div class="mt-3 flex justify-end">
            <UiButton
              v-if="accessControlStore.auditNextCursor"
              variant="ghost"
              :loading="accessControlStore.auditLoading"
              data-testid="access-control-audit-load-more"
              @click="handleLoadMoreAudit"
            >
              加载更多
            </UiButton>
          </div>
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedAuditRecord" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ selectedAuditRecord.action }}</div>
              <UiBadge :label="selectedAuditRecord.outcome" :tone="selectedAuditRecord.outcome === 'success' ? 'success' : 'warning'" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">{{ formatDateTime(selectedAuditRecord.createdAt) }}</div>
          </div>

          <div class="grid gap-3">
            <div class="rounded-[var(--radius-l)] border border-border bg-card p-4">
              <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">主体</div>
              <div class="mt-2 text-sm text-foreground">{{ selectedAuditRecord.actorType }} / {{ selectedAuditRecord.actorId }}</div>
            </div>
            <div class="rounded-[var(--radius-l)] border border-border bg-card p-4">
              <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">资源</div>
              <div class="mt-2 text-sm text-foreground">{{ selectedAuditRecord.resource }}</div>
            </div>
            <div class="rounded-[var(--radius-l)] border border-border bg-card p-4">
              <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">记录 ID</div>
              <div class="mt-2 break-all text-sm text-foreground">{{ selectedAuditRecord.id }}</div>
            </div>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>
  </div>
</template>
