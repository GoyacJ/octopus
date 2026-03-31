<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import { UiArtifactBlock, UiBadge, UiEmptyState, UiInboxBlock, UiSurface, UiTraceBlock } from '@octopus/ui'

import { countLabel, enumLabel, formatDateTime, resolveCopy, resolveMockField, resolveMockList } from '@/i18n/copy'
import type { ContextPaneTab } from '@/stores/shell'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const shell = useShellStore()
const workbench = useWorkbenchStore()

const activeConversation = computed(() => workbench.activeConversation)
const activeAgentLabel = computed(() => {
  const agentId = activeConversation.value?.activeAgentId
  if (!agentId) {
    return ''
  }

  const agent = workbench.agents.find((item) => item.id === agentId)
  return agent ? resolveMockField('agent', agent.id, 'name', agent.name) : agentId
})

const activeTeamLabel = computed(() => {
  const teamId = activeConversation.value?.activeTeamId
  if (!teamId) {
    return ''
  }

  const team = workbench.teams.find((item) => item.id === teamId)
  return team ? resolveMockField('team', team.id, 'name', team.name) : teamId
})

const selectedArtifact = computed(() =>
  workbench.activeConversationArtifacts.find((artifact: { id: string }) => artifact.id === shell.selectedArtifactId)
  ?? workbench.activeConversationArtifacts[0],
)

const artifactDraft = ref('')

const tabItems = computed(() => [
  {
    id: 'context',
    label: t('contextPane.tabs.context.label'),
    hint: t('contextPane.tabs.context.hint'),
  },
  {
    id: 'artifacts',
    label: t('contextPane.tabs.artifacts.label'),
    hint: t('contextPane.tabs.artifacts.hint'),
  },
  {
    id: 'inbox',
    label: t('contextPane.tabs.inbox.label'),
    hint: t('contextPane.tabs.inbox.hint'),
  },
  {
    id: 'trace',
    label: t('contextPane.tabs.trace.label'),
    hint: t('contextPane.tabs.trace.hint'),
  },
] as const)

watch(
  selectedArtifact,
  (artifact) => {
    artifactDraft.value = artifact ? resolveMockField('artifact', artifact.id, 'content', artifact.content) : ''
  },
  { immediate: true },
)

function updateQuery(pane: ContextPaneTab, artifactId?: string) {
  void router.replace({
    query: {
      ...route.query,
      pane,
      ...(artifactId ? { artifact: artifactId } : {}),
    },
  })
}

function setPane(pane: string) {
  const nextPane = pane as ContextPaneTab
  shell.setContextPane(nextPane)
  updateQuery(nextPane, shell.selectedArtifactId || undefined)
}

function openArtifact(artifactId: string) {
  shell.selectArtifact(artifactId)
  shell.setContextPane('artifacts')
  updateQuery('artifacts', artifactId)
}

function saveArtifactDraft() {
  if (!selectedArtifact.value) {
    return
  }

  workbench.updateArtifactContent(selectedArtifact.value.id, artifactDraft.value)
}

function requestReview() {
  if (!selectedArtifact.value) {
    return
  }

  workbench.requestArtifactReview(selectedArtifact.value.id)
  shell.setContextPane('inbox')
  updateQuery('inbox', selectedArtifact.value.id)
}
</script>

<template>
  <aside class="context-pane scroll-y">
    <UiSurface
      :eyebrow="t('contextPane.host.eyebrow')"
      :title="t('contextPane.host.title')"
      :subtitle="t('contextPane.host.subtitle')"
    >
      <div class="meta-row">
        <UiBadge :label="enumLabel('hostPlatform', shell.hostState.platform)" :tone="shell.hostState.platform === 'tauri' ? 'success' : 'info'" />
        <UiBadge :label="shell.hostState.cargoWorkspace ? t('contextPane.host.cargoWorkspace') : t('contextPane.host.webFallback')" subtle />
        <UiBadge :label="shell.hostState.appVersion" subtle />
      </div>
      <p class="pane-copy">
        {{ t('common.currentShell', { shell: shell.hostState.shell }) }}
        {{ t('common.lastRoute', { route: shell.preferences.lastVisitedRoute }) }}
      </p>
    </UiSurface>

    <div class="tab-row">
      <button
        v-for="tab in tabItems"
        :key="tab.id"
        type="button"
        class="tab-button"
        :class="{ active: shell.contextPane === tab.id }"
        @click="setPane(tab.id)"
      >
        <span>{{ tab.label }}</span>
        <small>{{ tab.hint }}</small>
      </button>
    </div>

    <UiSurface
      v-if="shell.contextPane === 'context' && activeConversation"
      :eyebrow="t('contextPane.conversation.eyebrow')"
      :title="resolveMockField('conversation', activeConversation.id, 'title', activeConversation.title)"
      :subtitle="resolveMockField('conversation', activeConversation.id, 'statusNote', resolveCopy(activeConversation.statusNote))"
    >
      <div class="meta-row">
        <UiBadge :label="enumLabel('conversationIntent', activeConversation.intent)" tone="info" />
        <UiBadge v-if="activeAgentLabel" :label="activeAgentLabel" subtle />
        <UiBadge v-if="activeTeamLabel" :label="activeTeamLabel" subtle />
      </div>
      <div class="context-block">
        <strong>{{ t('contextPane.conversation.goalLabel') }}</strong>
        <p>{{ resolveMockField('conversation', activeConversation.id, 'currentGoal', activeConversation.currentGoal) }}</p>
      </div>
      <div class="context-block">
        <strong>{{ t('contextPane.conversation.constraintsLabel') }}</strong>
        <ul>
          <li
            v-for="(constraint, index) in resolveMockList('conversation', activeConversation.id, 'constraints', activeConversation.constraints)"
            :key="`${activeConversation.id}-${index}`"
          >
            {{ constraint }}
          </li>
        </ul>
      </div>
      <div class="context-block">
        <strong>{{ t('contextPane.conversation.resumePointsLabel') }}</strong>
        <ul>
          <li v-for="resumePoint in activeConversation.resumePoints" :key="resumePoint.id">
            {{ resolveMockField('conversation', activeConversation.id, `resumePoints.${resumePoint.id}.label`, resumePoint.label) }}
            ·
            {{ formatDateTime(resumePoint.timestamp) }}
          </li>
        </ul>
      </div>
      <div class="context-block">
        <strong>{{ t('contextPane.conversation.branchesLabel') }}</strong>
        <ul v-if="activeConversation.branchLinks.length">
          <li v-for="branch in activeConversation.branchLinks" :key="branch.id">
            {{ resolveMockField('conversation', activeConversation.id, `branchLinks.${branch.id}.label`, branch.label) }} → {{ branch.targetConversationId }}
          </li>
        </ul>
        <UiEmptyState
          v-else
          :title="t('contextPane.conversation.emptyBranchTitle')"
          :description="t('contextPane.conversation.emptyBranchDescription')"
        />
      </div>
    </UiSurface>

    <template v-else-if="shell.contextPane === 'artifacts'">
      <UiSurface
        :eyebrow="t('contextPane.artifacts.eyebrow')"
        :title="t('contextPane.artifacts.title')"
        :subtitle="t('contextPane.artifacts.subtitle')"
      >
        <div v-if="workbench.activeConversationArtifacts.length" class="panel-list">
          <UiArtifactBlock
            v-for="artifact in workbench.activeConversationArtifacts"
            :key="artifact.id"
            :title="resolveMockField('artifact', artifact.id, 'title', artifact.title)"
            :excerpt="resolveMockField('artifact', artifact.id, 'excerpt', artifact.excerpt)"
            :type-label="resolveMockField('artifact', artifact.id, 'type', artifact.type)"
            :version-label="`v${artifact.version}`"
            :status-label="enumLabel('artifactStatus', artifact.status)"
          >
            <template #actions>
              <button type="button" class="ghost-button" @click="openArtifact(artifact.id)">
                {{ shell.selectedArtifactId === artifact.id ? t('common.selected') : t('common.open') }}
              </button>
            </template>
          </UiArtifactBlock>
        </div>
        <UiEmptyState
          v-else
          :title="t('contextPane.artifacts.emptyTitle')"
          :description="t('contextPane.artifacts.emptyDescription')"
        />
      </UiSurface>

      <UiSurface
        v-if="selectedArtifact"
        :title="resolveMockField('artifact', selectedArtifact.id, 'title', selectedArtifact.title)"
        :subtitle="t('common.updatedAt', { time: formatDateTime(selectedArtifact.updatedAt) })"
      >
        <div class="meta-row">
          <UiBadge :label="enumLabel('artifactStatus', selectedArtifact.status)" tone="warning" />
          <UiBadge :label="`v${selectedArtifact.version}`" subtle />
          <UiBadge :label="resolveMockField('artifact', selectedArtifact.id, 'type', selectedArtifact.type)" subtle />
        </div>
        <textarea v-model="artifactDraft" rows="12" />
        <div class="action-row">
          <button type="button" class="secondary-button" @click="saveArtifactDraft">{{ t('common.saveDraft') }}</button>
          <button type="button" class="primary-button" @click="requestReview">{{ t('common.requestReview') }}</button>
        </div>
      </UiSurface>
    </template>

    <UiSurface v-else-if="shell.contextPane === 'inbox'" :eyebrow="t('contextPane.inbox.eyebrow')" :title="t('contextPane.inbox.title')">
      <div v-if="workbench.workspaceInbox.length" class="panel-list">
        <UiInboxBlock
          v-for="item in workbench.workspaceInbox"
          :key="item.id"
          :title="resolveMockField('inboxItem', item.id, 'title', resolveCopy(item.title))"
          :description="resolveMockField('inboxItem', item.id, 'description', resolveCopy(item.description))"
          :priority-label="enumLabel('riskLevel', item.priority)"
          :status-label="enumLabel('inboxStatus', item.status)"
          :impact="resolveMockField('inboxItem', item.id, 'impact', resolveCopy(item.impact))"
          :risk-note="resolveMockField('inboxItem', item.id, 'riskNote', resolveCopy(item.riskNote))"
          :status-heading="t('common.status')"
          :impact-heading="t('common.impact')"
          :risk-heading="t('common.risk')"
        >
          <template #actions>
            <button
              v-if="item.status === 'pending'"
              type="button"
              class="primary-button"
              @click="workbench.resolveInboxItem(item.id, 'approve')"
            >
              {{ t('common.approve') }}
            </button>
            <button
              v-if="item.status === 'pending'"
              type="button"
              class="danger-button"
              @click="workbench.resolveInboxItem(item.id, 'reject')"
            >
              {{ t('common.reject') }}
            </button>
          </template>
        </UiInboxBlock>
      </div>
      <UiEmptyState
        v-else
        :title="t('contextPane.inbox.emptyTitle')"
        :description="t('contextPane.inbox.emptyDescription')"
      />
    </UiSurface>

    <UiSurface v-else :eyebrow="t('contextPane.trace.eyebrow')" :title="t('contextPane.trace.title')" :subtitle="t('contextPane.trace.subtitle')">
      <div v-if="workbench.activeTrace.length" class="panel-list">
        <UiTraceBlock
          v-for="trace in workbench.activeTrace"
          :key="trace.id"
          :title="resolveMockField('traceRecord', trace.id, 'title', trace.title)"
          :detail="resolveMockField('traceRecord', trace.id, 'detail', trace.detail)"
          :actor="trace.actor"
          :timestamp-label="formatDateTime(trace.timestamp)"
          :tone="trace.status"
        />
      </div>
      <UiEmptyState
        v-else
        :title="t('contextPane.trace.emptyTitle')"
        :description="t('contextPane.trace.emptyDescription')"
      />
    </UiSurface>
  </aside>
</template>

<style scoped>
.context-pane {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  min-width: 0;
  padding: 1rem;
  border-left: 1px solid var(--border-subtle);
  background:
    linear-gradient(180deg, color-mix(in srgb, var(--bg-sidebar) 96%, white), var(--bg-sidebar)),
    var(--bg-sidebar);
}

.pane-copy,
.context-block p {
  color: var(--text-secondary);
  line-height: 1.6;
  overflow-wrap: anywhere;
}

.context-block {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  min-width: 0;
}

ul {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  padding-left: 1.1rem;
  color: var(--text-secondary);
  overflow-wrap: anywhere;
}
</style>
