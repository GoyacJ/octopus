<script setup lang="ts">
import { computed, reactive, ref } from 'vue'

import { UiBadge, UiButton, UiEmptyState, UiField, UiInput, UiPanelFrame, UiSelect, UiStatTile, UiStatusCallout, UiTabs } from '@octopus/ui'

import { formatDateTime } from '@/i18n/copy'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'

const accessControlStore = useWorkspaceAccessControlStore()

const activeTab = ref('sessions')
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
  { value: 'sessions', label: '会话管理' },
  { value: 'audit', label: '审计日志' },
]
const auditOutcomeOptions = [
  { label: '全部结果', value: '' },
  { label: '成功', value: 'success' },
  { label: '拒绝', value: 'denied' },
  { label: '失败', value: 'failure' },
  { label: '锁定', value: 'locked' },
]
const auditStats = computed(() => ({
  total: accessControlStore.auditRecords.length,
  denied: accessControlStore.auditRecords.filter(record => record.outcome === 'denied').length,
  sensitive: accessControlStore.auditRecords.filter(record =>
    /(run|invoke|publish|delete|grant|export|retrieve|bind-credential)/.test(record.action),
  ).length,
}))

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
</script>

<template>
  <div class="space-y-4" data-testid="access-control-sessions-shell">
    <section class="grid gap-4 md:grid-cols-3">
      <UiStatTile label="会话总数" :value="String(accessControlStore.sessions.length)" />
      <UiStatTile label="活跃会话" :value="String(accessControlStore.sessions.filter(session => session.status === 'active').length)" tone="success" />
      <UiStatTile label="审计记录" :value="String(auditStats.total)" />
    </section>

    <UiStatusCallout
      v-if="submitError || accessControlStore.auditError"
      tone="error"
      :description="submitError || accessControlStore.auditError"
    />

    <UiPanelFrame variant="subtle" padding="sm">
      <UiTabs v-model="activeTab" :tabs="tabs" data-testid="access-control-sessions-tabs" />
    </UiPanelFrame>

    <UiPanelFrame
      v-if="activeTab === 'sessions'"
      variant="panel"
      padding="md"
      title="会话管理"
      subtitle="策略变更与会话撤销会在下一请求生效。"
    >
      <div v-if="accessControlStore.sessions.length" class="space-y-3">
        <article
          v-for="session in accessControlStore.sessions"
          :key="session.sessionId"
          class="rounded-[var(--radius-l)] border border-border bg-card p-4"
        >
          <div class="flex flex-wrap items-start justify-between gap-3">
            <div>
              <div class="flex flex-wrap items-center gap-2">
                <h3 class="text-sm font-semibold text-foreground">{{ session.displayName }}</h3>
                <UiBadge :label="session.status" :tone="session.status === 'active' ? 'success' : 'warning'" subtle />
                <UiBadge v-if="session.current" label="当前会话" subtle />
              </div>
              <p class="text-xs text-muted-foreground">{{ session.username }} / {{ session.clientAppId }}</p>
              <p class="mt-1 text-xs text-muted-foreground">{{ formatDateTime(session.createdAt) }}</p>
            </div>
            <div class="flex flex-wrap gap-2">
              <UiButton
                v-if="!session.current && session.status === 'active'"
                size="sm"
                variant="ghost"
                :loading="pendingSessionId === session.sessionId"
                @click="handleRevokeSession(session.sessionId)"
              >
                撤销会话
              </UiButton>
              <UiButton
                v-if="!session.current"
                size="sm"
                variant="ghost"
                :loading="pendingUserId === session.userId"
                @click="handleRevokeUserSessions(session.userId)"
              >
                撤销该用户全部会话
              </UiButton>
            </div>
          </div>
        </article>
      </div>
      <UiEmptyState v-else title="暂无会话" description="当前工作区没有会话记录。" />
    </UiPanelFrame>

    <UiPanelFrame
      v-else
      variant="panel"
      padding="md"
      title="审计日志"
      subtitle="登录、验证码、会话撤销、策略变更与敏感资源访问都在这里查询。"
    >
      <div class="space-y-4">
        <section class="grid gap-4 md:grid-cols-3">
          <UiStatTile label="已加载记录" :value="String(auditStats.total)" />
          <UiStatTile label="拒绝事件" :value="String(auditStats.denied)" tone="warning" />
          <UiStatTile label="敏感动作" :value="String(auditStats.sensitive)" />
        </section>

        <div class="grid gap-3 xl:grid-cols-3">
          <UiField label="Actor ID">
            <UiInput v-model="auditFilters.actorId" data-testid="access-control-audit-actor" />
          </UiField>
          <UiField label="动作">
            <UiInput v-model="auditFilters.action" data-testid="access-control-audit-action" />
          </UiField>
          <UiField label="资源类型">
            <UiInput v-model="auditFilters.resourceType" data-testid="access-control-audit-resource-type" />
          </UiField>
          <UiField label="结果">
            <UiSelect v-model="auditFilters.outcome" :options="auditOutcomeOptions" data-testid="access-control-audit-outcome" />
          </UiField>
          <UiField label="开始时间" hint="支持 ISO 日期时间。">
            <UiInput v-model="auditFilters.from" data-testid="access-control-audit-from" />
          </UiField>
          <UiField label="结束时间" hint="支持 ISO 日期时间。">
            <UiInput v-model="auditFilters.to" data-testid="access-control-audit-to" />
          </UiField>
        </div>

        <div class="flex justify-end gap-2">
          <UiButton variant="ghost" @click="resetAuditFilters">
            重置
          </UiButton>
          <UiButton :loading="accessControlStore.auditLoading" data-testid="access-control-audit-search" @click="handleSearchAudit">
            查询审计
          </UiButton>
        </div>

        <div v-if="accessControlStore.auditRecords.length" class="space-y-3">
          <article
            v-for="record in accessControlStore.auditRecords"
            :key="record.id"
            class="rounded-[var(--radius-l)] border border-border bg-card p-4"
          >
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div>
                <div class="flex flex-wrap items-center gap-2">
                  <div class="text-sm font-semibold text-foreground">{{ record.action }}</div>
                  <UiBadge :label="record.outcome" :tone="record.outcome === 'success' ? 'success' : 'warning'" subtle />
                </div>
                <div class="mt-1 text-xs text-muted-foreground">
                  {{ record.actorType }} / {{ record.actorId }}
                </div>
                <div class="mt-1 text-xs text-muted-foreground">
                  {{ record.resource }} / {{ formatDateTime(record.createdAt) }}
                </div>
              </div>
              <div class="text-xs text-muted-foreground">
                {{ record.id }}
              </div>
            </div>
          </article>
          <div class="flex justify-end">
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
        </div>
        <UiEmptyState v-else title="暂无审计记录" description="当前筛选条件下没有命中的审计事件。" />
      </div>
    </UiPanelFrame>
  </div>
</template>
