<script setup lang="ts">
import { computed } from 'vue'

import type {
  ImportIssue,
  ImportWorkspaceAgentBundlePreview,
  ImportWorkspaceAgentBundleResult,
} from '@octopus/schema'
import {
  UiBadge,
  UiButton,
  UiDialog,
  UiEmptyState,
  UiSurface,
} from '@octopus/ui'

type AgentBundleReport = ImportWorkspaceAgentBundlePreview | ImportWorkspaceAgentBundleResult

const props = withDefaults(defineProps<{
  open: boolean
  preview: ImportWorkspaceAgentBundlePreview | null
  result: ImportWorkspaceAgentBundleResult | null
  loading?: boolean
  errorMessage?: string
}>(), {
  loading: false,
  errorMessage: '',
})

const emit = defineEmits<{
  'update:open': [value: boolean]
  confirm: []
}>()

const report = computed<AgentBundleReport | null>(() => props.result ?? props.preview)
const reportTitle = computed(() => props.result ? 'Import Result' : 'Import Agent Bundle')
const reportDescription = computed(() => props.result
  ? 'Review created, updated, skipped, and failed items from the latest import run.'
  : 'Preview the detected agents and deduplicated skills before writing anything into the workspace.')
const importableAgentCount = computed(() => report.value?.importableAgentCount ?? 0)
const canConfirm = computed(() => Boolean(props.preview && !props.result && importableAgentCount.value > 0 && !props.loading))
const visibleAgents = computed(() => report.value?.agents.slice(0, 12) ?? [])
const visibleSkills = computed(() => report.value?.skills.slice(0, 12) ?? [])
const visibleIssues = computed(() => report.value?.issues.slice(0, 12) ?? [])
const hiddenAgentCount = computed(() => Math.max(0, (report.value?.agents.length ?? 0) - visibleAgents.value.length))
const hiddenSkillCount = computed(() => Math.max(0, (report.value?.skills.length ?? 0) - visibleSkills.value.length))
const hiddenIssueCount = computed(() => Math.max(0, (report.value?.issues.length ?? 0) - visibleIssues.value.length))

function actionTone(action: string): 'default' | 'success' | 'warning' | 'error' | 'info' {
  switch (action) {
    case 'create':
      return 'success'
    case 'update':
      return 'info'
    case 'skip':
      return 'default'
    case 'failed':
      return 'error'
    default:
      return 'default'
  }
}

function issueTone(issue: ImportIssue): 'default' | 'success' | 'warning' | 'error' | 'info' {
  return issue.severity === 'error' ? 'error' : 'warning'
}
</script>

<template>
  <UiDialog
    :open="props.open"
    :title="reportTitle"
    :description="reportDescription"
    content-class="max-w-5xl"
    body-class="max-h-[75vh] space-y-4 overflow-y-auto pr-1"
    @update:open="emit('update:open', $event)"
  >
    <UiSurface class="grid gap-3 border-border/50 p-4 md:grid-cols-3 dark:border-white/[0.08]">
      <div class="space-y-1">
        <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-text-tertiary">Agents</div>
        <div class="text-2xl font-semibold text-text-primary">{{ report?.importableAgentCount ?? 0 }}</div>
        <div class="flex flex-wrap gap-2">
          <UiBadge :label="`Create ${report?.createCount ?? 0}`" tone="success" />
          <UiBadge :label="`Update ${report?.updateCount ?? 0}`" tone="info" />
          <UiBadge :label="`Skip ${report?.skipCount ?? 0}`" />
        </div>
      </div>
      <div class="space-y-1">
        <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-text-tertiary">Skills</div>
        <div class="text-2xl font-semibold text-text-primary">{{ report?.uniqueSkillCount ?? 0 }}</div>
        <div class="text-sm text-text-secondary">
          {{ report?.filteredFileCount ?? 0 }} filtered files
        </div>
      </div>
      <div class="space-y-1">
        <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-text-tertiary">Issues</div>
        <div class="text-2xl font-semibold text-text-primary">{{ report?.failureCount ?? 0 }}</div>
        <div class="text-sm text-text-secondary">
          {{ report?.departmentCount ?? 0 }} departments detected
        </div>
      </div>
    </UiSurface>

    <UiSurface
      v-if="props.errorMessage"
      class="border-status-error/20 bg-status-error/5 p-4 dark:border-status-error/20"
    >
      <div class="text-sm font-medium text-status-error">{{ props.errorMessage }}</div>
    </UiSurface>

    <div v-if="report" class="grid gap-4 xl:grid-cols-[minmax(0,1.25fr)_minmax(0,1fr)]">
      <div class="space-y-4">
        <UiSurface class="space-y-3 border-border/50 p-4 dark:border-white/[0.08]">
          <div class="flex items-center justify-between gap-3">
            <div class="text-sm font-semibold text-text-primary">Agents</div>
            <UiBadge :label="`${report.agents.length} total`" subtle />
          </div>
          <div v-if="visibleAgents.length" class="space-y-2">
            <div
              v-for="agent in visibleAgents"
              :key="agent.sourceId"
              class="flex items-start justify-between gap-3 rounded-md border border-border/40 px-3 py-2 dark:border-white/[0.08]"
            >
              <div class="min-w-0 space-y-1">
                <div class="truncate text-sm font-medium text-text-primary">{{ agent.name }}</div>
                <div class="truncate text-xs text-text-secondary">{{ agent.sourceId }}</div>
                <div class="text-xs text-text-tertiary">{{ agent.skillSlugs.length }} linked skills</div>
              </div>
              <UiBadge :label="agent.action" :tone="actionTone(agent.action)" />
            </div>
          </div>
          <UiEmptyState
            v-else
            title="No agents"
            description="No compatible agents were detected in the selected folder."
          />
          <div v-if="hiddenAgentCount" class="text-xs text-text-tertiary">
            +{{ hiddenAgentCount }} more agents not shown
          </div>
        </UiSurface>

        <UiSurface class="space-y-3 border-border/50 p-4 dark:border-white/[0.08]">
          <div class="flex items-center justify-between gap-3">
            <div class="text-sm font-semibold text-text-primary">Skills</div>
            <UiBadge :label="`${report.skills.length} total`" subtle />
          </div>
          <div v-if="visibleSkills.length" class="space-y-2">
            <div
              v-for="skill in visibleSkills"
              :key="skill.slug"
              class="flex items-start justify-between gap-3 rounded-md border border-border/40 px-3 py-2 dark:border-white/[0.08]"
            >
              <div class="min-w-0 space-y-1">
                <div class="truncate text-sm font-medium text-text-primary">{{ skill.name }}</div>
                <div class="truncate text-xs text-text-secondary">{{ skill.slug }}</div>
                <div class="text-xs text-text-tertiary">{{ skill.sourceIds.length }} sources</div>
              </div>
              <UiBadge :label="skill.action" :tone="actionTone(skill.action)" />
            </div>
          </div>
          <UiEmptyState
            v-else
            title="No skills"
            description="No managed skills will be written from this bundle."
          />
          <div v-if="hiddenSkillCount" class="text-xs text-text-tertiary">
            +{{ hiddenSkillCount }} more skills not shown
          </div>
        </UiSurface>
      </div>

      <UiSurface class="space-y-3 border-border/50 p-4 dark:border-white/[0.08]">
        <div class="flex items-center justify-between gap-3">
          <div class="text-sm font-semibold text-text-primary">Issues</div>
          <UiBadge :label="`${report.issues.length} total`" subtle />
        </div>
        <div v-if="visibleIssues.length" class="space-y-2">
          <div
            v-for="issue in visibleIssues"
            :key="`${issue.scope}:${issue.sourceId ?? issue.message}`"
            class="space-y-2 rounded-md border border-border/40 px-3 py-2 dark:border-white/[0.08]"
          >
            <div class="flex items-center justify-between gap-2">
              <div class="truncate text-sm font-medium text-text-primary">
                {{ issue.scope }}
                <span v-if="issue.sourceId" class="text-text-secondary"> - {{ issue.sourceId }}</span>
              </div>
              <UiBadge :label="issue.severity" :tone="issueTone(issue)" />
            </div>
            <div class="text-sm text-text-secondary">{{ issue.message }}</div>
          </div>
        </div>
        <UiEmptyState
          v-else
          title="No issues"
          description="The bundle parsed cleanly."
        />
        <div v-if="hiddenIssueCount" class="text-xs text-text-tertiary">
          +{{ hiddenIssueCount }} more issues not shown
        </div>
      </UiSurface>
    </div>

    <template #footer>
      <div class="flex w-full items-center justify-between gap-3">
        <div class="text-xs text-text-tertiary">
          {{ props.result ? 'The workspace agents list and skill catalog have been refreshed.' : 'The import writes workspace agents plus deduplicated managed skills.' }}
        </div>
        <div class="flex items-center gap-2">
          <UiButton variant="ghost" @click="emit('update:open', false)">
            {{ props.result ? 'Close' : 'Cancel' }}
          </UiButton>
          <UiButton
            v-if="!props.result"
            :disabled="!canConfirm"
            :loading="props.loading"
            loading-label="Importing"
            @click="emit('confirm')"
          >
            Import
          </UiButton>
        </div>
      </div>
    </template>
  </UiDialog>
</template>
