<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import { UiBadge, UiEmptyState, UiRecordCard, UiSectionHeading } from '@octopus/ui'

import { enumLabel, resolveMockField } from '@/i18n/copy'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const shell = useShellStore()
const workbench = useWorkbenchStore()

function workspaceLabel(workspaceId: string): string {
  const workspace = workbench.workspaces.find((item) => item.id === workspaceId)
  return workspace ? resolveMockField('workspace', workspace.id, 'name', workspace.name) : workspaceId
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
        <div class="space-y-1 border-b border-border-subtle pb-4">
          <h3 class="text-lg font-bold text-text-primary">{{ t('connections.product.title') }}</h3>
          <p class="text-[13px] text-text-secondary">{{ t('connections.product.subtitle') }}</p>
        </div>

        <div data-testid="connections-product-list" class="space-y-4">
          <UiRecordCard
            v-for="connection in workbench.connections"
            :key="connection.id"
            :test-id="`connection-record-${connection.id}`"
            :title="resolveMockField('connection', connection.id, 'label', connection.label)"
            :description="t('common.workspaceLabel', { workspace: workspaceLabel(connection.workspaceId) })"
          >
            <template #badges>
              <UiBadge :label="enumLabel('connectionMode', connection.mode)" :tone="connection.mode === 'local' ? 'info' : 'success'" />
              <UiBadge :label="enumLabel('connectionState', connection.state)" subtle />
            </template>
            <template #meta>
              <span class="truncate text-[12px] text-text-tertiary font-mono">{{ connection.baseUrl ?? t('common.noRemoteBaseUrl') }}</span>
            </template>
          </UiRecordCard>

          <UiEmptyState v-if="!workbench.connections.length" :title="t('connections.empty.title')" :description="t('connections.empty.description')" />
        </div>
      </section>

      <!-- Host Connections -->
      <section class="space-y-6">
        <div class="space-y-1 border-b border-border-subtle pb-4">
          <h3 class="text-lg font-bold text-text-primary">{{ t('connections.host.title') }}</h3>
          <p class="text-[13px] text-text-secondary">{{ t('connections.host.subtitle') }}</p>
        </div>

        <div data-testid="connections-host-list" class="space-y-4">
          <UiRecordCard
            v-for="connection in shell.bootstrapConnections"
            :key="connection.id"
            :test-id="`connection-record-${connection.id}`"
            :title="resolveMockField('connection', connection.id, 'label', connection.label)"
            :description="t('common.workspaceLabel', { workspace: workspaceLabel(connection.workspaceId) })"
          >
            <template #badges>
              <UiBadge :label="enumLabel('connectionMode', connection.mode)" :tone="connection.mode === 'local' ? 'info' : 'success'" />
              <UiBadge :label="enumLabel('connectionState', connection.state)" subtle />
            </template>
            <template #meta>
              <span class="truncate text-[12px] text-text-tertiary font-mono">{{ connection.baseUrl ?? t('common.noBaseUrl') }}</span>
            </template>
          </UiRecordCard>

          <UiEmptyState
            v-if="!shell.bootstrapConnections.length"
            :title="t('connections.host.emptyTitle')"
            :description="t('connections.host.emptyDescription')"
          />
        </div>
      </section>
    </main>
  </div>
</template>
