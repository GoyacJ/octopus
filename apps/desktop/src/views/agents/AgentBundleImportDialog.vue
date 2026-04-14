<script setup lang="ts">
import { computed } from 'vue'
import { Loader2 } from 'lucide-vue-next'

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
  UiListRow,
  UiPanelFrame,
  UiStatusCallout,
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
  : 'Preview the detected teams, agents, skills, MCP servers, and avatars before writing anything.')
const importableItemCount = computed(() =>
  (report.value?.importableAgentCount ?? 0) + (report.value?.importableTeamCount ?? 0),
)
const canConfirm = computed(() => Boolean(props.preview && !props.result && importableItemCount.value > 0 && !props.loading))
const visibleAgents = computed(() => report.value?.agents.slice(0, 12) ?? [])
const visibleTeams = computed(() => report.value?.teams.slice(0, 12) ?? [])
const visibleSkills = computed(() => report.value?.skills.slice(0, 12) ?? [])
const visibleMcps = computed(() => report.value?.mcps.slice(0, 12) ?? [])
const visibleAvatars = computed(() => report.value?.avatars.slice(0, 12) ?? [])
const visibleIssues = computed(() => report.value?.issues.slice(0, 12) ?? [])
const hiddenAgentCount = computed(() => Math.max(0, (report.value?.agents.length ?? 0) - visibleAgents.value.length))
const hiddenTeamCount = computed(() => Math.max(0, (report.value?.teams.length ?? 0) - visibleTeams.value.length))
const hiddenSkillCount = computed(() => Math.max(0, (report.value?.skills.length ?? 0) - visibleSkills.value.length))
const hiddenMcpCount = computed(() => Math.max(0, (report.value?.mcps.length ?? 0) - visibleMcps.value.length))
const hiddenAvatarCount = computed(() => Math.max(0, (report.value?.avatars.length ?? 0) - visibleAvatars.value.length))
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
    <UiPanelFrame variant="subtle" padding="md" class="grid gap-3 md:grid-cols-2 xl:grid-cols-5">
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
        <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-text-tertiary">Teams</div>
        <div class="text-2xl font-semibold text-text-primary">{{ report?.importableTeamCount ?? 0 }}</div>
        <div class="text-sm text-text-secondary">
          {{ report?.detectedTeamCount ?? 0 }} detected
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
        <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-text-tertiary">MCP</div>
        <div class="text-2xl font-semibold text-text-primary">{{ report?.uniqueMcpCount ?? 0 }}</div>
        <div class="text-sm text-text-secondary">
          {{ report?.avatarCount ?? 0 }} avatars
        </div>
      </div>
      <div class="space-y-1">
        <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-text-tertiary">Issues</div>
        <div class="text-2xl font-semibold text-text-primary">{{ report?.failureCount ?? 0 }}</div>
        <div class="text-sm text-text-secondary">
          {{ report?.departmentCount ?? 0 }} departments detected
        </div>
      </div>
    </UiPanelFrame>

    <UiStatusCallout
      v-if="props.errorMessage"
      tone="error"
      :description="props.errorMessage"
    />

    <div
      v-if="props.loading && !report && !props.errorMessage"
      class="flex min-h-56 flex-col items-center justify-center gap-3 rounded-[var(--radius-l)] border border-border bg-surface-muted/60 px-6 py-10 text-center"
    >
      <Loader2 class="h-5 w-5 animate-spin text-primary" />
      <div class="space-y-1">
        <div class="text-sm font-semibold text-text-primary">正在分析导入内容</div>
        <div class="text-xs text-text-secondary">已读取所选模板，正在生成导入预览，请稍候。</div>
      </div>
    </div>

    <div v-if="report" class="grid gap-4 xl:grid-cols-[minmax(0,1.25fr)_minmax(0,1fr)]">
      <div class="space-y-4">
        <UiPanelFrame variant="raised" padding="md" class="space-y-3">
          <div class="flex items-center justify-between gap-3">
            <div class="text-sm font-semibold text-text-primary">Agents</div>
            <UiBadge :label="`${report.agents.length} total`" subtle />
          </div>
          <div v-if="visibleAgents.length" class="space-y-2">
            <UiListRow
              v-for="agent in visibleAgents"
              :key="agent.sourceId"
              :title="agent.name"
              :subtitle="agent.sourceId"
            >
              <div class="text-xs text-text-tertiary">{{ agent.skillSlugs.length }} linked skills</div>
              <template #meta>
                <UiBadge :label="agent.action" :tone="actionTone(agent.action)" />
              </template>
            </UiListRow>
          </div>
          <UiEmptyState
            v-else
            title="No agents"
            description="No compatible agents were detected in the selected folder."
          />
          <div v-if="hiddenAgentCount" class="text-xs text-text-tertiary">
            +{{ hiddenAgentCount }} more agents not shown
          </div>
        </UiPanelFrame>

        <UiPanelFrame variant="raised" padding="md" class="space-y-3">
          <div class="flex items-center justify-between gap-3">
            <div class="text-sm font-semibold text-text-primary">Teams</div>
            <UiBadge :label="`${report.teams.length} total`" subtle />
          </div>
          <div v-if="visibleTeams.length" class="space-y-2">
            <UiListRow
              v-for="team in visibleTeams"
              :key="team.sourceId"
              :title="team.name"
              :subtitle="team.leaderName || team.sourceId"
            >
              <div class="text-xs text-text-tertiary">{{ team.memberNames.length }} members</div>
              <template #meta>
                <UiBadge :label="team.action" :tone="actionTone(team.action)" />
              </template>
            </UiListRow>
          </div>
          <UiEmptyState
            v-else
            title="No teams"
            description="No compatible digital teams were detected in the selected bundle."
          />
          <div v-if="hiddenTeamCount" class="text-xs text-text-tertiary">
            +{{ hiddenTeamCount }} more teams not shown
          </div>
        </UiPanelFrame>

        <UiPanelFrame variant="raised" padding="md" class="space-y-3">
          <div class="flex items-center justify-between gap-3">
            <div class="text-sm font-semibold text-text-primary">Skills</div>
            <UiBadge :label="`${report.skills.length} total`" subtle />
          </div>
          <div v-if="visibleSkills.length" class="space-y-2">
            <UiListRow
              v-for="skill in visibleSkills"
              :key="skill.slug"
              :title="skill.name"
              :subtitle="skill.slug"
            >
              <div class="text-xs text-text-tertiary">{{ skill.sourceIds.length }} sources</div>
              <template #meta>
                <UiBadge :label="skill.action" :tone="actionTone(skill.action)" />
              </template>
            </UiListRow>
          </div>
          <UiEmptyState
            v-else
            title="No skills"
            description="No managed skills will be written from this bundle."
          />
          <div v-if="hiddenSkillCount" class="text-xs text-text-tertiary">
            +{{ hiddenSkillCount }} more skills not shown
          </div>
        </UiPanelFrame>
      </div>

      <div class="space-y-4">
        <UiPanelFrame variant="raised" padding="md" class="space-y-3">
          <div class="flex items-center justify-between gap-3">
            <div class="text-sm font-semibold text-text-primary">MCP</div>
            <UiBadge :label="`${report.mcps.length} total`" subtle />
          </div>
          <div v-if="visibleMcps.length" class="space-y-2">
            <UiListRow
              v-for="mcp in visibleMcps"
              :key="mcp.serverName"
              :title="mcp.serverName"
              :subtitle="mcp.consumerNames.join(', ') || mcp.sourceIds.join(', ')"
            >
              <div class="text-xs text-text-tertiary">
                {{ mcp.referencedOnly ? 'Referenced only' : 'Bundled config' }}
              </div>
              <template #meta>
                <UiBadge :label="mcp.action" :tone="actionTone(mcp.action)" />
              </template>
            </UiListRow>
          </div>
          <UiEmptyState
            v-else
            title="No MCP servers"
            description="No MCP configuration will be imported from this bundle."
          />
          <div v-if="hiddenMcpCount" class="text-xs text-text-tertiary">
            +{{ hiddenMcpCount }} more MCP entries not shown
          </div>
        </UiPanelFrame>

        <UiPanelFrame variant="raised" padding="md" class="space-y-3">
          <div class="flex items-center justify-between gap-3">
            <div class="text-sm font-semibold text-text-primary">Avatars</div>
            <UiBadge :label="`${report.avatars.length} total`" subtle />
          </div>
          <div v-if="visibleAvatars.length" class="space-y-2">
            <UiListRow
              v-for="avatar in visibleAvatars"
              :key="`${avatar.ownerKind}:${avatar.sourceId}`"
              :title="avatar.ownerName"
              :subtitle="avatar.fileName"
            >
              <div class="text-xs text-text-tertiary">{{ avatar.ownerKind }}</div>
              <template #meta>
                <UiBadge :label="avatar.generated ? 'generated' : 'provided'" :tone="avatar.generated ? 'warning' : 'default'" />
              </template>
            </UiListRow>
          </div>
          <UiEmptyState
            v-else
            title="No avatars"
            description="No avatar files were detected in the selected bundle."
          />
          <div v-if="hiddenAvatarCount" class="text-xs text-text-tertiary">
            +{{ hiddenAvatarCount }} more avatars not shown
          </div>
        </UiPanelFrame>

        <UiPanelFrame variant="raised" padding="md" class="space-y-3">
          <div class="flex items-center justify-between gap-3">
            <div class="text-sm font-semibold text-text-primary">Issues</div>
            <UiBadge :label="`${report.issues.length} total`" subtle />
          </div>
          <div v-if="visibleIssues.length" class="space-y-2">
            <UiListRow
              v-for="issue in visibleIssues"
              :key="`${issue.scope}:${issue.sourceId ?? issue.message}`"
              :title="issue.sourceId ? `${issue.scope} - ${issue.sourceId}` : issue.scope"
              :subtitle="issue.message"
            >
              <template #meta>
                <UiBadge :label="issue.severity" :tone="issueTone(issue)" />
              </template>
            </UiListRow>
          </div>
          <UiEmptyState
            v-else
            title="No issues"
            description="The bundle parsed cleanly."
          />
          <div v-if="hiddenIssueCount" class="text-xs text-text-tertiary">
            +{{ hiddenIssueCount }} more issues not shown
          </div>
        </UiPanelFrame>
      </div>
    </div>

    <template #footer>
      <div class="flex w-full items-center justify-between gap-3">
        <div class="text-xs text-text-tertiary">
          {{ props.result ? 'The digital employee, digital team, and resource catalogs have been refreshed.' : 'The import writes digital employees, digital teams, managed skills, and MCP resources together.' }}
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
