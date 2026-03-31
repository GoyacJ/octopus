<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import { UiBadge, UiEmptyState, UiSectionHeading, UiSurface } from '@octopus/ui'

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
  <section class="section-stack">
    <UiSectionHeading
      :eyebrow="t('connections.header.eyebrow')"
      :title="t('connections.header.title')"
      :subtitle="t('connections.header.subtitle')"
    />

    <div class="surface-grid two">
      <UiSurface :title="t('connections.product.title')" :subtitle="t('connections.product.subtitle')">
        <div v-if="workbench.connections.length" class="panel-list">
          <article v-for="connection in workbench.connections" :key="connection.id" class="connection-card">
            <div class="meta-row">
              <UiBadge :label="enumLabel('connectionMode', connection.mode)" :tone="connection.mode === 'local' ? 'info' : 'success'" />
              <UiBadge :label="enumLabel('connectionState', connection.state)" subtle />
            </div>
            <strong>{{ resolveMockField('connection', connection.id, 'label', connection.label) }}</strong>
            <p>{{ t('common.workspaceLabel', { workspace: workspaceLabel(connection.workspaceId) }) }}</p>
            <small>{{ connection.baseUrl ?? t('common.noRemoteBaseUrl') }}</small>
          </article>
        </div>
        <UiEmptyState v-else :title="t('connections.empty.title')" :description="t('connections.empty.description')" />
      </UiSurface>

      <UiSurface :title="t('connections.host.title')" :subtitle="t('connections.host.subtitle')">
        <div v-if="shell.bootstrapConnections.length" class="panel-list">
          <article v-for="connection in shell.bootstrapConnections" :key="connection.id" class="connection-card">
            <div class="meta-row">
              <UiBadge :label="enumLabel('connectionMode', connection.mode)" :tone="connection.mode === 'local' ? 'info' : 'success'" />
              <UiBadge :label="enumLabel('connectionState', connection.state)" subtle />
            </div>
            <strong>{{ resolveMockField('connection', connection.id, 'label', connection.label) }}</strong>
            <p>{{ t('common.workspaceLabel', { workspace: workspaceLabel(connection.workspaceId) }) }}</p>
            <small>{{ connection.baseUrl ?? t('common.noBaseUrl') }}</small>
          </article>
        </div>
        <UiEmptyState
          v-else
          :title="t('connections.host.emptyTitle')"
          :description="t('connections.host.emptyDescription')"
        />
      </UiSurface>
    </div>
  </section>
</template>

<style scoped>
.connection-card {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  min-width: 0;
  padding: 1rem;
  border-radius: var(--radius-l);
  border: 1px solid var(--border-subtle);
  background: color-mix(in srgb, var(--bg-subtle) 78%, transparent);
}

.connection-card p,
.connection-card small {
  color: var(--text-secondary);
  line-height: 1.6;
  overflow-wrap: anywhere;
}
</style>
