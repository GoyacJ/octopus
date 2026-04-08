<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiEmptyState, UiRecordCard, UiSectionHeading } from '@octopus/ui'

import { enumLabel } from '@/i18n/copy'
import { useShellStore } from '@/stores/shell'

const { t } = useI18n()
const shell = useShellStore()

const hostBackendBadges = computed((): Array<{ id: string, label: string, tone: 'info' | 'success' | 'warning' }> => {
  if (!shell.backendConnection) {
    return []
  }

  return [
    {
      id: 'state',
      label: enumLabel('backendConnectionState', shell.backendConnection.state),
      tone: shell.backendConnection.state === 'ready' ? 'success' : 'warning' as const,
    },
    {
      id: 'transport',
      label: enumLabel('backendTransport', shell.backendConnection.transport),
      tone: 'info' as const,
    },
  ]
})

function workspaceLabel(workspaceId: string): string {
  return shell.workspaceConnections.find((item) => item.workspaceId === workspaceId)?.label ?? workspaceId
}
</script>

<template>
  <div class="w-full flex flex-col gap-10 pb-20">
    <header class="px-2 shrink-0">
      <UiSectionHeading
        :eyebrow="t('connections.header.eyebrow')"
        :title="t('connections.header.title')"
        :subtitle="t('connections.header.subtitle')"
      />
    </header>

    <main class="grid gap-8 xl:grid-cols-2 px-2">
      <!-- Product Connections -->
      <section class="space-y-6">
        <div class="space-y-1 border-b border-border-subtle dark:border-white/[0.05] pb-4">
          <h3 class="text-lg font-bold text-text-primary">{{ t('connections.product.title') }}</h3>
          <p class="text-[13px] text-text-secondary">{{ t('connections.product.subtitle') }}</p>
        </div>

        <div data-testid="connections-product-list" class="space-y-4">
          <UiRecordCard
            v-for="connection in shell.workspaceConnections"
            :key="connection.workspaceConnectionId"
            :test-id="`connection-record-${connection.workspaceConnectionId}`"
            :title="connection.label"
            :description="t('common.workspaceLabel', { workspace: workspaceLabel(connection.workspaceId) })"
          >
            <template #badges>
              <UiBadge :label="enumLabel('transportSecurityLevel', connection.transportSecurity)" :tone="connection.transportSecurity === 'loopback' ? 'info' : 'success' as const" />
              <UiBadge :label="enumLabel('workspaceConnectionStatus', connection.status)" subtle />
            </template>
            <template #meta>
              <span class="truncate text-[12px] text-text-tertiary font-mono">{{ connection.baseUrl ?? t('common.noRemoteBaseUrl') }}</span>
            </template>
          </UiRecordCard>

          <UiEmptyState v-if="!shell.workspaceConnections.length" :title="t('connections.empty.title')" :description="t('connections.empty.description')" />
        </div>
      </section>

      <!-- Host Connections -->
      <section class="space-y-6">
        <div class="space-y-1 border-b border-border-subtle dark:border-white/[0.05] pb-4">
          <h3 class="text-lg font-bold text-text-primary">{{ t('connections.host.title') }}</h3>
          <p class="text-[13px] text-text-secondary">{{ t('connections.host.subtitle') }}</p>
        </div>

        <div data-testid="connections-host-list" class="space-y-4">
          <UiRecordCard
            v-if="shell.backendConnection"
            test-id="host-backend-connection"
            :title="t('connections.host.backendTitle')"
            :description="t('connections.host.backendSubtitle')"
          >
            <template #badges>
              <UiBadge
                v-for="badge in hostBackendBadges"
                :key="badge.id"
                :label="badge.label"
                :tone="badge.tone"
              />
            </template>
            <template #meta>
              <span class="truncate text-[12px] text-text-tertiary font-mono">{{ shell.backendConnection.baseUrl ?? t('common.noBaseUrl') }}</span>
            </template>
          </UiRecordCard>

          <UiRecordCard
            v-for="connection in shell.bootstrapConnections"
            :key="connection.id"
            :test-id="`connection-record-${connection.id}`"
            :title="connection.label"
            :description="t('common.workspaceLabel', { workspace: workspaceLabel(connection.workspaceId) })"
          >
            <template #badges>
              <UiBadge :label="enumLabel('connectionMode', connection.mode)" :tone="connection.mode === 'local' ? 'info' : 'success' as const" />
              <UiBadge :label="enumLabel('connectionState', connection.state)" subtle />
            </template>
            <template #meta>
              <span class="truncate text-[12px] text-text-tertiary font-mono">{{ connection.baseUrl ?? t('common.noBaseUrl') }}</span>
            </template>
          </UiRecordCard>

          <UiEmptyState
            v-if="!shell.backendConnection && !shell.bootstrapConnections.length"
            :title="t('connections.host.emptyTitle')"
            :description="t('connections.host.emptyDescription')"
          />
        </div>
      </section>
    </main>
  </div>
</template>
