<script setup lang="ts">
import { enumLabel } from '@/i18n/copy'
import { UiBadge, UiEmptyState, UiRecordCard } from '@octopus/ui'

defineProps<{
  workspaceConnections: Array<any>
  backendConnection: any
  bootstrapConnections: Array<any>
  hostBackendBadges: Array<{ id: string, label: string, tone: 'info' | 'success' | 'warning' }>
  workspaceLabel: (workspaceId: string) => string
}>()
</script>

<template>
  <section class="space-y-10">
    <div class="space-y-1">
      <h3 class="text-xl font-bold text-text-primary">{{ $t('connections.header.title') }}</h3>
      <p class="text-[14px] text-text-secondary">{{ $t('connections.header.subtitle') }}</p>
    </div>

    <div class="grid gap-10 xl:grid-cols-2">
      <section class="space-y-6">
        <div class="space-y-1 border-b border-border-subtle pb-4">
          <h3 class="text-lg font-bold text-text-primary">{{ $t('connections.product.title') }}</h3>
          <p class="text-[13px] text-text-secondary">{{ $t('connections.product.subtitle') }}</p>
        </div>

        <div data-testid="connections-product-list" class="space-y-4">
          <UiRecordCard
            v-for="connection in workspaceConnections"
            :key="connection.workspaceConnectionId"
            :test-id="`connection-record-${connection.workspaceConnectionId}`"
            :title="connection.label"
            :description="$t('common.workspaceLabel', { workspace: workspaceLabel(connection.workspaceId) })"
          >
            <template #badges>
              <UiBadge :label="enumLabel('transportSecurityLevel', connection.transportSecurity)" :tone="connection.transportSecurity === 'loopback' ? 'info' : 'success'" />
              <UiBadge :label="enumLabel('workspaceConnectionStatus', connection.status)" subtle />
            </template>
            <template #meta>
              <span class="truncate font-mono text-[12px] text-text-tertiary">{{ connection.baseUrl ?? $t('common.noRemoteBaseUrl') }}</span>
            </template>
          </UiRecordCard>

          <UiEmptyState v-if="!workspaceConnections.length" :title="$t('connections.empty.title')" :description="$t('connections.empty.description')" />
        </div>
      </section>

      <section class="space-y-6">
        <div class="space-y-1 border-b border-border-subtle pb-4">
          <h3 class="text-lg font-bold text-text-primary">{{ $t('connections.host.title') }}</h3>
          <p class="text-[13px] text-text-secondary">{{ $t('connections.host.subtitle') }}</p>
        </div>

        <div data-testid="connections-host-list" class="space-y-4">
          <UiRecordCard
            v-if="backendConnection"
            test-id="host-backend-connection"
            :title="$t('connections.host.backendTitle')"
            :description="$t('connections.host.backendSubtitle')"
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
              <span class="truncate font-mono text-[12px] text-text-tertiary">{{ backendConnection.baseUrl ?? $t('common.noBaseUrl') }}</span>
            </template>
          </UiRecordCard>

          <UiRecordCard
            v-for="connection in bootstrapConnections"
            :key="connection.id"
            :test-id="`connection-record-${connection.id}`"
            :title="connection.label"
            :description="$t('common.workspaceLabel', { workspace: workspaceLabel(connection.workspaceId) })"
          >
            <template #badges>
              <UiBadge :label="enumLabel('connectionMode', connection.mode)" :tone="connection.mode === 'local' ? 'info' : 'success'" />
              <UiBadge :label="enumLabel('connectionState', connection.state)" subtle />
            </template>
            <template #meta>
              <span class="truncate font-mono text-[12px] text-text-tertiary">{{ connection.baseUrl ?? $t('common.noBaseUrl') }}</span>
            </template>
          </UiRecordCard>

          <UiEmptyState
            v-if="!backendConnection && !bootstrapConnections.length"
            :title="$t('connections.host.emptyTitle')"
            :description="$t('connections.host.emptyDescription')"
          />
        </div>
      </section>
    </div>
  </section>
</template>
