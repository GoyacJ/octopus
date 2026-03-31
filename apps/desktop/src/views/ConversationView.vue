<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import { ConversationIntent } from '@octopus/schema'
import { UiArtifactBlock, UiBadge, UiEmptyState, UiSectionHeading, UiSurface } from '@octopus/ui'

import { enumLabel, resolveCopy, resolveMockField, resolveMockList } from '@/i18n/copy'
import { useShellStore } from '@/stores/shell'
import { useWorkbenchStore } from '@/stores/workbench'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const shell = useShellStore()
const workbench = useWorkbenchStore()

const messageDraft = ref('')
const activeConversation = computed(() => workbench.activeConversation)

const activeAgent = computed(() =>
  workbench.agents.find((agent) => agent.id === activeConversation.value?.activeAgentId),
)

const activeTeam = computed(() =>
  workbench.teams.find((team) => team.id === activeConversation.value?.activeTeamId),
)

function sendMessage() {
  workbench.sendMessage(messageDraft.value)
  messageDraft.value = ''
}

function openArtifact(artifactId: string) {
  shell.selectArtifact(artifactId)
  shell.setContextPane('artifacts')
  void router.replace({
    query: {
      ...route.query,
      pane: 'artifacts',
      artifact: artifactId,
    },
  })
}

function requestReview(artifactId: string) {
  workbench.requestArtifactReview(artifactId)
  shell.setContextPane('inbox')
  void router.replace({
    query: {
      ...route.query,
      pane: 'inbox',
      artifact: artifactId,
    },
  })
}

function senderLabel(senderId: string, senderType: 'user' | 'agent' | 'system'): string {
  if (senderType === 'user') {
    return t('conversation.senderType.user')
  }

  if (senderType === 'system') {
    return t('conversation.senderType.system')
  }

  const agent = workbench.agents.find((item) => item.id === senderId)
  return agent ? resolveMockField('agent', agent.id, 'name', agent.name) : senderId
}
</script>

<template>
  <section class="section-stack">
    <UiSectionHeading
      :eyebrow="t('conversation.header.eyebrow')"
      :title="activeConversation ? resolveMockField('conversation', activeConversation.id, 'title', activeConversation.title) : t('conversation.header.titleFallback')"
      :subtitle="activeConversation ? resolveMockField('conversation', activeConversation.id, 'summary', activeConversation.summary) : t('conversation.header.subtitleFallback')"
    />

    <UiSurface
      v-if="activeConversation"
      :title="t('conversation.controls.title')"
      :subtitle="resolveMockField('conversation', activeConversation.id, 'statusNote', resolveCopy(activeConversation.statusNote))"
    >
      <div class="meta-row">
        <UiBadge :label="enumLabel('conversationIntent', activeConversation.intent)" tone="info" />
        <UiBadge v-if="activeAgent" :label="resolveMockField('agent', activeAgent.id, 'name', activeAgent.name)" subtle />
        <UiBadge v-if="activeTeam" :label="resolveMockField('team', activeTeam.id, 'name', activeTeam.name)" subtle />
        <UiBadge :label="t('common.progress', { value: activeConversation.stageProgress })" subtle />
      </div>
      <div class="action-row">
        <button
          v-if="activeConversation.intent !== ConversationIntent.PAUSED"
          type="button"
          class="secondary-button"
          @click="workbench.pauseConversation()"
        >
          {{ t('common.pause') }}
        </button>
        <button
          v-else
          type="button"
          class="primary-button"
          @click="workbench.resumeConversation()"
        >
          {{ t('common.resume') }}
        </button>
      </div>
      <div class="surface-grid two">
        <div class="context-copy">
          <strong>{{ t('conversation.controls.goalLabel') }}</strong>
          <p>{{ resolveMockField('conversation', activeConversation.id, 'currentGoal', activeConversation.currentGoal) }}</p>
        </div>
        <div class="context-copy">
          <strong>{{ t('conversation.controls.constraintsLabel') }}</strong>
          <ul>
            <li
              v-for="(constraint, index) in resolveMockList('conversation', activeConversation.id, 'constraints', activeConversation.constraints)"
              :key="`${activeConversation.id}-constraint-${index}`"
            >
              {{ constraint }}
            </li>
          </ul>
        </div>
      </div>
    </UiSurface>

    <UiSurface :title="t('conversation.stream.title')" :subtitle="t('conversation.stream.subtitle')">
      <div v-if="workbench.conversationMessages.length" class="message-stream">
        <article
          v-for="message in workbench.conversationMessages"
          :key="message.id"
          class="message-card"
          :class="message.senderType"
        >
          <header>
            <strong>{{ senderLabel(message.senderId, message.senderType) }}</strong>
            <span>{{ t(`conversation.senderType.${message.senderType}`) }}</span>
          </header>
          <p>{{ resolveMockField('message', message.id, 'content', resolveCopy(message.content)) }}</p>
          <div v-if="message.artifacts?.length" class="action-row">
            <button
              v-for="artifactId in message.artifacts"
              :key="artifactId"
              type="button"
              class="ghost-button"
              @click="openArtifact(artifactId)"
            >
              {{ t('common.open') }} {{ artifactId }}
            </button>
          </div>
        </article>
      </div>
      <UiEmptyState v-else :title="t('conversation.stream.emptyTitle')" :description="t('conversation.stream.emptyDescription')" />
      <div class="composer">
        <textarea
          v-model="messageDraft"
          rows="5"
          :placeholder="t('conversation.stream.placeholder')"
        />
        <button type="button" class="primary-button" @click="sendMessage">{{ t('common.send') }}</button>
      </div>
    </UiSurface>

    <UiSurface :title="t('conversation.artifacts.title')" :subtitle="t('conversation.artifacts.subtitle')">
      <div v-if="workbench.activeConversationArtifacts.length" class="surface-grid two">
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
            <button type="button" class="ghost-button" @click="openArtifact(artifact.id)">{{ t('common.open') }}</button>
            <button type="button" class="secondary-button" @click="requestReview(artifact.id)">{{ t('common.requestReview') }}</button>
          </template>
        </UiArtifactBlock>
      </div>
      <UiEmptyState v-else :title="t('conversation.artifacts.emptyTitle')" :description="t('conversation.artifacts.emptyDescription')" />
      <div v-if="activeConversation" class="surface-grid two">
        <UiSurface :title="t('conversation.artifacts.resumeTitle')">
          <ul>
            <li v-for="resumePoint in activeConversation.resumePoints" :key="resumePoint.id">
              {{ resolveMockField('conversation', activeConversation.id, `resumePoints.${resumePoint.id}.label`, resumePoint.label) }}
            </li>
          </ul>
        </UiSurface>
        <UiSurface :title="t('conversation.artifacts.branchTitle')">
          <ul v-if="activeConversation.branchLinks.length">
            <li v-for="branch in activeConversation.branchLinks" :key="branch.id">
              {{ resolveMockField('conversation', activeConversation.id, `branchLinks.${branch.id}.label`, branch.label) }} → {{ branch.targetConversationId }}
            </li>
          </ul>
          <UiEmptyState v-else :title="t('conversation.artifacts.emptyBranchTitle')" :description="t('conversation.artifacts.emptyBranchDescription')" />
        </UiSurface>
      </div>
    </UiSurface>
  </section>
</template>

<style scoped>
.context-copy p,
.message-card p {
  color: var(--text-secondary);
  line-height: 1.6;
  overflow-wrap: anywhere;
}

ul {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  padding-left: 1rem;
  color: var(--text-secondary);
}

.message-stream {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}

.message-card {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  min-width: 0;
  padding: 1rem;
  border-radius: var(--radius-l);
  border: 1px solid var(--border-subtle);
  background: color-mix(in srgb, var(--bg-subtle) 78%, transparent);
}

.message-card.user {
  border-color: color-mix(in srgb, var(--brand-primary) 30%, var(--border-subtle));
}

.message-card header {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: 1rem;
}

.composer {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}
</style>
