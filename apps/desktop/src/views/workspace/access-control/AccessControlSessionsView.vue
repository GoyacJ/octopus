<script setup lang="ts">
import { computed, h, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import {
  type ColumnDef,
  UiBadge,
  UiButton,
  UiDataTable,
  UiField,
  UiInput,
  UiListDetailWorkspace,
  UiPagination,
  UiPanelFrame,
  UiSelect,
  UiStatusCallout,
  UiTabs,
  UiToolbarRow,
} from '@octopus/ui'
import type { AccessSessionRecord, AuditRecord } from '@octopus/schema'

import { usePagination } from '@/composables/usePagination'
import { formatDateTime } from '@/i18n/copy'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'

import { getAuditActionLabel, getAuditResourceLabel } from './display-i18n'
import {
  createAuditOutcomeOptions,
  getAuditOutcomeLabel,
  getStatusLabel,
  getSubjectTypeLabel,
} from './helpers'
import { useAccessControlNotifications } from './useAccessControlNotifications'

const { t } = useI18n()
const accessControlStore = useWorkspaceAccessControlStore()
const { notifySuccess } = useAccessControlNotifications('access-control.sessions')

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

const tabs = computed(() => [
  { value: 'sessions', label: t('accessControl.sessions.tabs.sessions') },
  { value: 'audit', label: t('accessControl.sessions.tabs.audit') },
])

const sessionStatusOptions = computed(() => [
  { label: t('accessControl.common.filters.allStatuses'), value: '' },
  { label: getStatusLabel(t, 'active'), value: 'active' },
  { label: getStatusLabel(t, 'revoked'), value: 'revoked' },
  { label: getStatusLabel(t, 'expired'), value: 'expired' },
])

const auditOutcomeOptions = computed(() => [
  { label: t('accessControl.common.filters.allResults'), value: '' },
  ...createAuditOutcomeOptions(t),
])

const filteredSessions = computed(() =>
  [...accessControlStore.sessions]
    .filter(session => !sessionStatusFilter.value || session.status === sessionStatusFilter.value)
    .sort((left, right) => right.createdAt - left.createdAt),
)

const sessionsPagination = usePagination(filteredSessions, {
  pageSize: 10,
  resetOn: [sessionStatusFilter],
})

const auditPagination = usePagination(() => accessControlStore.auditRecords, {
  pageSize: 10,
  resetOn: [
    () => accessControlStore.auditRecords.length,
    () => auditFilters.actorId,
    () => auditFilters.action,
    () => auditFilters.resourceType,
    () => auditFilters.outcome,
    () => auditFilters.from,
    () => auditFilters.to,
  ],
})

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

const sessionColumns = computed<ColumnDef<AccessSessionRecord>[]>(() => [
  {
    accessorKey: 'displayName',
    header: t('accessControl.sessions.sessions.columns.session'),
    cell: ({ row }) => h('div', { class: 'space-y-1' }, [
      h('div', { class: 'text-sm font-medium text-text-primary' }, row.original.displayName),
      h('div', { class: 'text-xs text-text-secondary' }, `${row.original.username} / ${row.original.clientAppId}`),
    ]),
  },
  {
    accessorKey: 'createdAt',
    header: t('accessControl.sessions.sessions.columns.createdAt'),
    cell: ({ row }) => formatDateTime(row.original.createdAt),
  },
  {
    accessorKey: 'status',
    header: t('accessControl.sessions.sessions.columns.status'),
    cell: ({ row }) => h(UiBadge, {
      label: row.original.current ? t('accessControl.common.list.currentSession') : getStatusLabel(t, row.original.status),
      tone: row.original.status === 'active' ? 'success' : 'default',
      subtle: true,
    }),
  },
])

const auditColumns = computed<ColumnDef<AuditRecord>[]>(() => [
  {
    accessorKey: 'action',
    header: t('accessControl.sessions.audit.columns.action'),
    cell: ({ row }) => h('div', { class: 'space-y-1' }, [
      h('div', { class: 'text-sm font-medium text-text-primary' }, getAuditActionLabel(row.original.action)),
      h('div', { class: 'text-xs text-text-secondary' }, `${getSubjectTypeLabel(t, row.original.actorType)} / ${row.original.actorId}`),
    ]),
  },
  {
    accessorKey: 'resource',
    header: t('accessControl.sessions.audit.columns.resource'),
    cell: ({ row }) => h('div', { class: 'text-xs text-text-secondary' }, getAuditResourceLabel(row.original.resource)),
  },
  {
    accessorKey: 'createdAt',
    header: t('accessControl.sessions.audit.columns.createdAt'),
    cell: ({ row }) => formatDateTime(row.original.createdAt),
  },
  {
    accessorKey: 'outcome',
    header: t('accessControl.sessions.audit.columns.outcome'),
    cell: ({ row }) => h(UiBadge, {
      label: getAuditOutcomeLabel(t, row.original.outcome),
      tone: row.original.outcome === 'success' ? 'success' : 'warning',
      subtle: true,
    }),
  },
])

watch(sessionsPagination.pagedItems, (sessions) => {
  if (selectedSessionId.value && !sessions.some(session => session.sessionId === selectedSessionId.value)) {
    selectedSessionId.value = ''
  }
}, { immediate: true })

watch(auditPagination.pagedItems, (records) => {
  if (selectedAuditId.value && !records.some(record => record.id === selectedAuditId.value)) {
    selectedAuditId.value = ''
  }
}, { immediate: true })

function parseTimeInput(value: string): number | undefined {
  if (!value.trim()) {
    return undefined
  }
  const timestamp = Date.parse(value)
  return Number.isFinite(timestamp) ? timestamp : undefined
}

function selectSession(session: AccessSessionRecord) {
  selectedSessionId.value = session.sessionId
}

function selectAuditRecord(record: AuditRecord) {
  selectedAuditId.value = record.id
}

async function handleRevokeSession(sessionId: string) {
  pendingSessionId.value = sessionId
  submitError.value = ''
  try {
    await accessControlStore.revokeSession(sessionId)
    if (selectedSession.value) {
      await notifySuccess(t('accessControl.sessions.feedback.toastSessionRevoked'), selectedSession.value.displayName)
    }
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.sessions.feedback.searchFailed')
  } finally {
    pendingSessionId.value = ''
  }
}

async function handleRevokeUserSessions(userId: string) {
  pendingUserId.value = userId
  submitError.value = ''
  try {
    const label = selectedSession.value?.displayName ?? userId
    await accessControlStore.revokeUserSessions(userId)
    await notifySuccess(t('accessControl.sessions.feedback.toastUserSessionsRevoked'), label)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.sessions.feedback.searchFailed')
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
    submitError.value = error instanceof Error ? error.message : t('accessControl.sessions.feedback.searchFailed')
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
    submitError.value = error instanceof Error ? error.message : t('accessControl.sessions.feedback.loadMoreFailed')
  }
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
      :detail-subtitle="t('accessControl.sessions.sessions.detailSubtitle')"
      :empty-detail-title="t('accessControl.sessions.sessions.emptyTitle')"
      :empty-detail-description="t('accessControl.sessions.sessions.emptyDescription')"
    >
      <template #toolbar>
        <UiToolbarRow test-id="access-control-sessions-toolbar">
          <template #filters>
            <UiField :label="t('accessControl.sessions.sessions.toolbarStatus')" class="w-full md:w-[180px]">
              <UiSelect v-model="sessionStatusFilter" :options="sessionStatusOptions" />
            </UiField>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame
          variant="panel"
          padding="md"
          :title="t('accessControl.sessions.sessions.listTitle')"
          :subtitle="t('accessControl.common.list.totalSessions', { count: sessionsPagination.totalItems.value })"
        >
          <UiDataTable
            :data="sessionsPagination.pagedItems.value"
            :columns="sessionColumns"
            :empty-title="t('accessControl.sessions.sessions.noListTitle')"
            :empty-description="t('accessControl.sessions.sessions.noListDescription')"
            :on-row-click="selectSession"
          />

          <div class="mt-3 pt-2">
            <UiPagination
              v-model:page="sessionsPagination.currentPage.value"
              :page-count="sessionsPagination.pageCount.value"
              :previous-label="t('accessControl.common.pagination.previous')"
              :next-label="t('accessControl.common.pagination.next')"
              :summary-label="t('accessControl.common.pagination.summary', { count: sessionsPagination.totalItems.value })"
            />
          </div>
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedSession" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ selectedSession.displayName }}</div>
              <UiBadge :label="getStatusLabel(t, selectedSession.status)" :tone="selectedSession.status === 'active' ? 'success' : 'default'" subtle />
              <UiBadge v-if="selectedSession.current" :label="t('accessControl.common.list.currentSession')" subtle />
            </div>
            <div class="mt-2 text-xs text-text-secondary">{{ selectedSession.username }} / {{ selectedSession.clientAppId }}</div>
            <div class="mt-1 text-xs text-text-secondary">{{ formatDateTime(selectedSession.createdAt) }}</div>
          </div>

          <div class="flex flex-wrap gap-2">
            <UiButton
              v-if="!selectedSession.current && selectedSession.status === 'active'"
              variant="ghost"
              :loading="pendingSessionId === selectedSession.sessionId"
              @click="handleRevokeSession(selectedSession.sessionId)"
            >
              {{ t('accessControl.sessions.sessions.actions.revokeSession') }}
            </UiButton>
            <UiButton
              v-if="!selectedSession.current"
              variant="ghost"
              :loading="pendingUserId === selectedSession.userId"
              @click="handleRevokeUserSessions(selectedSession.userId)"
            >
              {{ t('accessControl.sessions.sessions.actions.revokeUserSessions') }}
            </UiButton>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>

    <UiListDetailWorkspace
      v-else
      :has-selection="Boolean(selectedAuditRecord)"
      :detail-title="selectedAuditRecord ? getAuditActionLabel(selectedAuditRecord.action) : ''"
      :detail-subtitle="t('accessControl.sessions.audit.detailSubtitle')"
      :empty-detail-title="t('accessControl.sessions.audit.emptyTitle')"
      :empty-detail-description="t('accessControl.sessions.audit.emptyDescription')"
    >
      <template #toolbar>
        <UiToolbarRow test-id="access-control-audit-toolbar">
          <template #filters>
            <UiField :label="t('accessControl.sessions.audit.fields.actorId')" class="w-full md:w-[180px]">
              <UiInput v-model="auditFilters.actorId" data-testid="access-control-audit-actor" />
            </UiField>
            <UiField :label="t('accessControl.sessions.audit.fields.action')" class="w-full md:w-[160px]">
              <UiInput v-model="auditFilters.action" data-testid="access-control-audit-action" />
            </UiField>
            <UiField :label="t('accessControl.sessions.audit.fields.resourceType')" class="w-full md:w-[160px]">
              <UiInput v-model="auditFilters.resourceType" data-testid="access-control-audit-resource-type" />
            </UiField>
            <UiField :label="t('accessControl.sessions.audit.fields.outcome')" class="w-full md:w-[160px]">
              <UiSelect v-model="auditFilters.outcome" :options="auditOutcomeOptions" data-testid="access-control-audit-outcome" />
            </UiField>
            <UiField :label="t('accessControl.sessions.audit.fields.from')" class="w-full md:w-[200px]">
              <UiInput v-model="auditFilters.from" data-testid="access-control-audit-from" />
            </UiField>
            <UiField :label="t('accessControl.sessions.audit.fields.to')" class="w-full md:w-[200px]">
              <UiInput v-model="auditFilters.to" data-testid="access-control-audit-to" />
            </UiField>
          </template>
          <template #actions>
            <div class="flex gap-2">
              <UiButton variant="ghost" @click="resetAuditFilters">
                {{ t('accessControl.sessions.audit.actions.reset') }}
              </UiButton>
              <UiButton :loading="accessControlStore.auditLoading" data-testid="access-control-audit-search" @click="handleSearchAudit">
                {{ t('accessControl.sessions.audit.actions.search') }}
              </UiButton>
            </div>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame
          variant="panel"
          padding="md"
          :title="t('accessControl.sessions.audit.listTitle')"
          :subtitle="t('accessControl.common.list.loadedAudit', { count: accessControlStore.auditRecords.length })"
        >
          <UiDataTable
            :data="auditPagination.pagedItems.value"
            :columns="auditColumns"
            :empty-title="t('accessControl.sessions.audit.noListTitle')"
            :empty-description="t('accessControl.sessions.audit.noListDescription')"
            :on-row-click="selectAuditRecord"
          >
            <template #toolbar>
              <div class="flex flex-wrap items-center gap-3 text-xs text-text-tertiary">
                <span>{{ t('accessControl.common.metrics.deniedAudit', { count: auditStats.denied }) }}</span>
                <span>{{ t('accessControl.common.metrics.sensitiveAudit', { count: auditStats.sensitive }) }}</span>
              </div>
            </template>
          </UiDataTable>

          <div class="mt-3 flex flex-wrap items-center justify-between gap-3 pt-2">
            <UiPagination
              v-model:page="auditPagination.currentPage.value"
              :page-count="auditPagination.pageCount.value"
              :previous-label="t('accessControl.common.pagination.previous')"
              :next-label="t('accessControl.common.pagination.next')"
              :summary-label="t('accessControl.common.pagination.summary', { count: accessControlStore.auditRecords.length })"
            />

            <UiButton
              v-if="accessControlStore.auditNextCursor"
              variant="ghost"
              :loading="accessControlStore.auditLoading"
              data-testid="access-control-audit-load-more"
              @click="handleLoadMoreAudit"
            >
              {{ t('accessControl.sessions.audit.actions.loadMore') }}
            </UiButton>
          </div>
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedAuditRecord" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ getAuditActionLabel(selectedAuditRecord.action) }}</div>
              <UiBadge
                :label="getAuditOutcomeLabel(t, selectedAuditRecord.outcome)"
                :tone="selectedAuditRecord.outcome === 'success' ? 'success' : 'warning'"
                subtle
              />
            </div>
            <div class="mt-2 text-xs text-text-secondary">{{ formatDateTime(selectedAuditRecord.createdAt) }}</div>
          </div>

          <div class="grid gap-3">
            <div class="rounded-[var(--radius-l)] border border-border bg-card p-4">
              <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">
                {{ t('accessControl.sessions.audit.detailBlocks.actor') }}
              </div>
              <div class="mt-2 text-sm text-foreground">
                {{ getSubjectTypeLabel(t, selectedAuditRecord.actorType) }} / {{ selectedAuditRecord.actorId }}
              </div>
            </div>
            <div class="rounded-[var(--radius-l)] border border-border bg-card p-4">
              <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">
                {{ t('accessControl.sessions.audit.detailBlocks.resource') }}
              </div>
              <div class="mt-2 text-sm text-foreground">{{ getAuditResourceLabel(selectedAuditRecord.resource) }}</div>
            </div>
            <div class="rounded-[var(--radius-l)] border border-border bg-card p-4">
              <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">
                {{ t('accessControl.sessions.audit.detailBlocks.recordId') }}
              </div>
              <div class="mt-2 text-sm text-foreground">{{ selectedAuditRecord.id }}</div>
            </div>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>
  </div>
</template>
