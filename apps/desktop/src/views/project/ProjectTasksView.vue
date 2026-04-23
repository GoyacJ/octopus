<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRoute, useRouter } from 'vue-router'

import {
  UiBadge,
  UiButton,
  UiDialog,
  UiEmptyState,
  UiField,
  UiInput,
  UiListDetailWorkspace,
  UiListRow,
  UiPageHeader,
  UiPageShell,
  UiPanelFrame,
  UiStatusCallout,
  UiTextarea,
  UiTimelineList,
  UiToolbarRow,
} from '@octopus/ui'

import { enumLabel, formatDateTime } from '@/i18n/copy'
import { createProjectConversationTarget } from '@/i18n/navigation'
import { useProjectTaskStore } from '@/stores/project_task'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceStore } from '@/stores/workspace'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const shell = useShellStore()
const workspaceStore = useWorkspaceStore()
const taskStore = useProjectTaskStore()

interface TaskFormState {
  title: string
  goal: string
  brief: string
  defaultActorRef: string
  scheduleSpec: string
}

const searchQuery = ref('')
const createDialogOpen = ref(false)
const editDialogOpen = ref(false)
const briefDialogOpen = ref(false)
const commentDialogOpen = ref(false)
const changeActorDialogOpen = ref(false)
const editorError = ref('')
const commentInterventionError = ref('')
const commentInterventionValue = ref('')
const briefInterventionError = ref('')
const briefInterventionValue = ref('')
const changeActorInterventionError = ref('')
const changeActorInterventionValue = ref('')
const executionActorRef = ref('')
const createForm = reactive<TaskFormState>(createEmptyTaskForm())
const editForm = reactive<TaskFormState>(createEmptyTaskForm())
const emptyTaskContextBundle = {
  refs: [],
  pinnedInstructions: '',
  resolutionMode: 'explicit_only',
  lastResolvedAt: null,
}

const projectId = computed(() =>
  typeof route.params.projectId === 'string' ? route.params.projectId : workspaceStore.currentProjectId,
)
const workspaceId = computed(() =>
  typeof route.params.workspaceId === 'string' ? route.params.workspaceId : workspaceStore.currentWorkspaceId,
)
const sourceConversationId = computed(() =>
  typeof route.query.conversationId === 'string' ? route.query.conversationId : '',
)
const openedFromConversation = computed(() =>
  route.query.from === 'conversation' && Boolean(sourceConversationId.value),
)
const projectRecord = computed(() =>
  workspaceStore.projects.find(project => project.id === projectId.value) ?? null,
)
const tasks = computed(() => taskStore.projectTasksFor(projectId.value))
const selectedTaskId = computed(() =>
  typeof route.query.taskId === 'string'
    ? route.query.taskId
    : taskStore.selectedTaskIdByProjectId[projectId.value] ?? '',
)
const filteredTasks = computed(() => {
  const query = searchQuery.value.trim().toLowerCase()
  if (!query) {
    return tasks.value
  }

  return tasks.value.filter((task) =>
    [
      task.title,
      task.goal,
      task.status,
      task.defaultActorRef,
      task.latestResultSummary ?? '',
      task.latestFailureCategory ?? '',
      ...(task.attentionReasons ?? []),
    ].join(' ').toLowerCase().includes(query),
  )
})
const selectedTask = computed(() =>
  (selectedTaskId.value ? taskStore.getCachedDetail(selectedTaskId.value) : null)
  ?? filteredTasks.value.find(task => task.id === selectedTaskId.value)
  ?? tasks.value.find(task => task.id === selectedTaskId.value)
  ?? null,
)
const selectedTaskDetail = computed(() =>
  selectedTaskId.value ? taskStore.getCachedDetail(selectedTaskId.value) : null,
)
const selectedTaskRuns = computed(() =>
  selectedTaskId.value ? taskStore.getCachedRunHistory(selectedTaskId.value) : [],
)
const selectedTaskInterventions = computed(() =>
  selectedTaskDetail.value?.interventionHistory ?? [],
)
const selectedTaskNeedsApproval = computed(() => {
  if (!selectedTask.value) {
    return false
  }

  const activeRun = selectedTaskDetail.value?.activeRun ?? null
  return selectedTask.value.attentionReasons.includes('needs_approval')
    || activeRun?.status === 'waiting_approval'
    || activeRun?.attentionReasons.includes('needs_approval')
})
const selectedTaskCanResume = computed(() => {
  if (!selectedTask.value) {
    return false
  }

  const activeRun = selectedTaskDetail.value?.activeRun ?? null
  return selectedTask.value.attentionReasons.includes('waiting_input')
    || activeRun?.status === 'waiting_input'
    || activeRun?.attentionReasons.includes('waiting_input')
})
const selectedTaskContextBundle = computed(() =>
  selectedTaskDetail.value?.contextBundle ?? emptyTaskContextBundle,
)
const createSaving = computed(() =>
  Boolean(projectId.value) && Boolean(taskStore.loading.saveByTaskId[`create:${projectId.value}`]),
)
const selectedTaskSaveLoading = computed(() =>
  selectedTaskId.value ? Boolean(taskStore.loading.saveByTaskId[selectedTaskId.value]) : false,
)
const selectedTaskLaunchLoading = computed(() =>
  selectedTaskId.value ? Boolean(taskStore.loading.launchByTaskId[selectedTaskId.value]) : false,
)
const selectedTaskBusy = computed(() =>
  selectedTaskSaveLoading.value || selectedTaskLaunchLoading.value,
)
const resolvedExecutionActorRef = computed(() =>
  executionActorRef.value.trim()
  || selectedTaskDetail.value?.defaultActorRef
  || selectedTask.value?.defaultActorRef
  || '',
)

watch(
  () => [shell.activeWorkspaceConnectionId, projectId.value] as const,
  ([connectionId, nextProjectId]) => {
    if (!connectionId || !nextProjectId) {
      return
    }

    void taskStore.loadProjectTasks(nextProjectId)
  },
  { immediate: true },
)

watch(
  () => [tasks.value, selectedTaskId.value] as const,
  async ([records, queryTaskId]) => {
    if (!records.length) {
      if (queryTaskId) {
        await replaceTaskQuery(null)
      }
      return
    }

    if (queryTaskId && records.some(task => task.id === queryTaskId)) {
      return
    }

    await replaceTaskQuery(records[0]?.id ?? null)
  },
  { immediate: true },
)

watch(
  () => [shell.activeWorkspaceConnectionId, projectId.value, selectedTaskId.value] as const,
  ([connectionId, nextProjectId, taskId]) => {
    if (!connectionId || !nextProjectId || !taskId) {
      return
    }

    taskStore.selectTask(nextProjectId, taskId)
    void taskStore.getTaskDetail(nextProjectId, taskId)
  },
  { immediate: true },
)

watch(
  () => [selectedTaskId.value, selectedTaskDetail.value?.defaultActorRef ?? selectedTask.value?.defaultActorRef ?? ''] as const,
  ([, nextActorRef]) => {
    executionActorRef.value = nextActorRef
  },
  { immediate: true },
)

function taskStatusLabel(status?: string | null) {
  return enumLabel('taskLifecycleStatus', status)
}

function taskFailureLabel(category?: string | null) {
  return enumLabel('taskFailureCategory', category)
}

function taskAttentionReasonLabel(reason: string) {
  return enumLabel('taskAttentionReason', reason)
}

function taskTriggerLabel(triggerType?: string | null) {
  return enumLabel('taskTriggerType', triggerType)
}

function taskRunStatusLabel(status?: string | null) {
  return enumLabel('taskRunStatus', status)
}

function taskInterventionTypeLabel(type?: string | null) {
  return enumLabel('taskInterventionType', type)
}

function taskInterventionStatusLabel(status?: string | null) {
  return enumLabel('taskInterventionStatus', status)
}

function translatedTaskContextLabel(key: string, fallback?: string | null) {
  const label = t(key)
  return label === key ? (fallback ?? '') : label
}

function taskContextResolutionLabel(mode?: string | null) {
  return translatedTaskContextLabel(`tasks.detail.context.resolutionModes.${mode ?? 'explicit_only'}`, mode)
}

function taskContextRefKindLabel(kind?: string | null) {
  return translatedTaskContextLabel(`tasks.detail.context.refKinds.${kind ?? 'resource'}`, kind)
}

function taskContextPinModeLabel(mode?: string | null) {
  return translatedTaskContextLabel(`tasks.detail.context.pinModes.${mode ?? 'snapshot'}`, mode)
}

function viewStatusLabel(status?: string | null) {
  return enumLabel('resourceStatus', status)
}

function taskRunConversationTarget(conversationId?: string | null) {
  if (!workspaceId.value || !projectId.value || !conversationId) {
    return null
  }

  return createProjectConversationTarget(workspaceId.value, projectId.value, conversationId)
}

function taskConversationTargetFromDetail(detail: typeof selectedTaskDetail.value) {
  if (!detail) {
    return null
  }

  const conversationId = detail.activeRun?.conversationId
    ?? findTaskRun(detail.activeTaskRunId)?.conversationId
    ?? detail.runHistory[0]?.conversationId
    ?? null

  return taskRunConversationTarget(conversationId)
}

function findTaskRun(taskRunId?: string | null) {
  if (!taskRunId) {
    return null
  }

  if (selectedTaskDetail.value?.activeRun?.id === taskRunId) {
    return selectedTaskDetail.value.activeRun
  }

  return selectedTaskRuns.value.find(run => run.id === taskRunId) ?? null
}

function taskInterventionConversationTarget(taskRunId?: string | null) {
  return taskRunConversationTarget(findTaskRun(taskRunId)?.conversationId)
}

function latestTaskConversationTarget() {
  if (!selectedTaskId.value) {
    return null
  }

  const detail = taskStore.getCachedDetail(selectedTaskId.value)
  return taskConversationTargetFromDetail(detail)
    ?? taskRunConversationTarget(taskStore.getCachedRunHistory(selectedTaskId.value)[0]?.conversationId ?? null)
}

function taskInterventionDescription(record: (typeof selectedTaskInterventions.value)[number]) {
  const payloadNote = typeof record.payload?.note === 'string'
    ? record.payload.note.trim()
    : ''
  const payloadBrief = typeof record.payload?.brief === 'string'
    ? record.payload.brief.trim()
    : ''
  const payloadActorRef = typeof record.payload?.actorRef === 'string'
    ? record.payload.actorRef.trim()
    : ''

  if (payloadNote) {
    return payloadNote
  }

  if (payloadBrief) {
    return payloadBrief
  }

  if (payloadActorRef) {
    return payloadActorRef
  }

  if (record.taskRunId) {
    return t('tasks.detail.interventionRunLabel', { runId: record.taskRunId })
  }

  return t('tasks.detail.interventionNoPayload')
}

function createEmptyTaskForm(): TaskFormState {
  return {
    title: '',
    goal: '',
    brief: '',
    defaultActorRef: '',
    scheduleSpec: '',
  }
}

function applyFormState(target: TaskFormState, source: Partial<TaskFormState>) {
  target.title = source.title ?? ''
  target.goal = source.goal ?? ''
  target.brief = source.brief ?? ''
  target.defaultActorRef = source.defaultActorRef ?? ''
  target.scheduleSpec = source.scheduleSpec ?? ''
}

function validateForm(form: TaskFormState) {
  return Boolean(
    form.title.trim()
    && form.goal.trim()
    && form.brief.trim()
    && form.defaultActorRef.trim(),
  )
}

function normalizeOptionalText(value: string) {
  const trimmed = value.trim()
  return trimmed ? trimmed : null
}

async function replaceTaskQuery(taskId: string | null) {
  await router.replace({
    query: {
      ...route.query,
      taskId: taskId || undefined,
    },
  })
}

async function selectTaskRow(taskId: string) {
  if (!taskId || taskId === selectedTaskId.value) {
    return
  }

  await replaceTaskQuery(taskId)
}

async function returnToSourceConversation() {
  if (!workspaceId.value || !projectId.value || !sourceConversationId.value) {
    return
  }

  await router.push(createProjectConversationTarget(workspaceId.value, projectId.value, sourceConversationId.value))
}

function openCreateDialog() {
  editorError.value = ''
  applyFormState(createForm, {
    ...createEmptyTaskForm(),
    defaultActorRef: selectedTask.value?.defaultActorRef ?? '',
  })
  createDialogOpen.value = true
}

function closeCreateDialog() {
  createDialogOpen.value = false
  editorError.value = ''
}

async function ensureSelectedTaskDetail() {
  if (!projectId.value || !selectedTaskId.value) {
    return null
  }

  const cached = taskStore.getCachedDetail(selectedTaskId.value)
  if (cached) {
    return cached
  }

  return await taskStore.getTaskDetail(projectId.value, selectedTaskId.value)
}

async function openEditDialog() {
  const detail = await ensureSelectedTaskDetail()
  if (!detail) {
    return
  }

  editorError.value = ''
  applyFormState(editForm, {
    title: detail.title,
    goal: detail.goal,
    brief: detail.brief,
    defaultActorRef: detail.defaultActorRef,
    scheduleSpec: detail.scheduleSpec ?? '',
  })
  editDialogOpen.value = true
}

function closeEditDialog() {
  editDialogOpen.value = false
  editorError.value = ''
}

async function openCommentDialog() {
  const detail = await ensureSelectedTaskDetail()
  if (!detail) {
    return
  }

  commentInterventionError.value = ''
  commentInterventionValue.value = ''
  commentDialogOpen.value = true
}

function closeCommentDialog() {
  commentDialogOpen.value = false
  commentInterventionError.value = ''
}

async function openBriefDialog() {
  const detail = await ensureSelectedTaskDetail()
  if (!detail) {
    return
  }

  briefInterventionError.value = ''
  briefInterventionValue.value = detail.brief ?? ''
  briefDialogOpen.value = true
}

function closeBriefDialog() {
  briefDialogOpen.value = false
  briefInterventionError.value = ''
}

async function openChangeActorDialog() {
  const detail = await ensureSelectedTaskDetail()
  if (!detail) {
    return
  }

  changeActorInterventionError.value = ''
  changeActorInterventionValue.value = detail.activeRun?.actorRef ?? detail.defaultActorRef ?? ''
  changeActorDialogOpen.value = true
}

function closeChangeActorDialog() {
  changeActorDialogOpen.value = false
  changeActorInterventionError.value = ''
}

async function submitCreate() {
  if (!projectId.value) {
    return
  }

  if (!validateForm(createForm)) {
    editorError.value = t('tasks.form.validationMessage')
    return
  }

  editorError.value = ''
  const created = await taskStore.createTask(projectId.value, {
    title: createForm.title.trim(),
    goal: createForm.goal.trim(),
    brief: createForm.brief.trim(),
    defaultActorRef: createForm.defaultActorRef.trim(),
    scheduleSpec: normalizeOptionalText(createForm.scheduleSpec),
    contextBundle: {
      refs: [],
      pinnedInstructions: '',
      resolutionMode: 'explicit_only',
      lastResolvedAt: null,
    },
  })

  if (!created) {
    editorError.value = taskStore.error || t('tasks.status.createFailed')
    return
  }

  await taskStore.getTaskDetail(projectId.value, created.id)
  closeCreateDialog()
  await replaceTaskQuery(created.id)
}

async function submitEdit() {
  if (!projectId.value || !selectedTaskId.value) {
    return
  }

  if (!validateForm(editForm)) {
    editorError.value = t('tasks.form.validationMessage')
    return
  }

  editorError.value = ''
  const updated = await taskStore.updateTask(projectId.value, selectedTaskId.value, {
    title: editForm.title.trim(),
    goal: editForm.goal.trim(),
    brief: editForm.brief.trim(),
    defaultActorRef: editForm.defaultActorRef.trim(),
    scheduleSpec: normalizeOptionalText(editForm.scheduleSpec),
  })

  if (!updated) {
    editorError.value = taskStore.error || t('tasks.status.saveFailed')
    return
  }

  closeEditDialog()
}

async function launchSelectedTask() {
  if (!projectId.value || !selectedTaskId.value) {
    return
  }

  await taskStore.launchTask(projectId.value, selectedTaskId.value, {
    actorRef: resolvedExecutionActorRef.value || undefined,
  })
}

async function rerunSelectedTask() {
  if (!projectId.value || !selectedTaskId.value) {
    return
  }

  const detail = await ensureSelectedTaskDetail()
  await taskStore.rerunTask(projectId.value, selectedTaskId.value, {
    actorRef: resolvedExecutionActorRef.value || undefined,
    sourceTaskRunId: detail?.activeTaskRunId ?? detail?.runHistory[0]?.id ?? null,
  })
}

async function submitBriefIntervention() {
  if (!projectId.value || !selectedTaskId.value) {
    return
  }

  const brief = briefInterventionValue.value.trim()
  if (!brief) {
    briefInterventionError.value = t('tasks.form.briefValidationMessage')
    return
  }

  const detail = await ensureSelectedTaskDetail()
  briefInterventionError.value = ''
  const intervention = await taskStore.createIntervention(projectId.value, selectedTaskId.value, {
    type: 'edit_brief',
    taskRunId: detail?.activeTaskRunId ?? null,
    payload: {
      brief,
    },
  })

  if (!intervention) {
    briefInterventionError.value = taskStore.error || t('tasks.status.interventionFailed')
    return
  }

  closeBriefDialog()
}

async function submitCommentIntervention() {
  if (!projectId.value || !selectedTaskId.value) {
    return
  }

  const note = commentInterventionValue.value.trim()
  if (!note) {
    commentInterventionError.value = t('tasks.form.commentValidationMessage')
    return
  }

  const detail = await ensureSelectedTaskDetail()
  commentInterventionError.value = ''
  const intervention = await taskStore.createIntervention(projectId.value, selectedTaskId.value, {
    type: 'comment',
    taskRunId: detail?.activeTaskRunId ?? null,
    payload: {
      note,
    },
  })

  if (!intervention) {
    commentInterventionError.value = taskStore.error || t('tasks.status.interventionFailed')
    return
  }

  closeCommentDialog()
}

async function submitChangeActorIntervention() {
  if (!projectId.value || !selectedTaskId.value) {
    return
  }

  const actorRef = changeActorInterventionValue.value.trim()
  if (!actorRef) {
    changeActorInterventionError.value = t('tasks.form.actorValidationMessage')
    return
  }

  const detail = await ensureSelectedTaskDetail()
  changeActorInterventionError.value = ''
  const intervention = await taskStore.createIntervention(projectId.value, selectedTaskId.value, {
    type: 'change_actor',
    taskRunId: detail?.activeTaskRunId ?? null,
    payload: {
      actorRef,
    },
  })

  if (!intervention) {
    changeActorInterventionError.value = taskStore.error || t('tasks.status.interventionFailed')
    return
  }

  closeChangeActorDialog()
}

async function submitSelectedTaskIntervention(type: 'approve' | 'reject' | 'resume') {
  if (!projectId.value || !selectedTaskId.value) {
    return
  }

  const detail = await ensureSelectedTaskDetail()
  await taskStore.createIntervention(projectId.value, selectedTaskId.value, {
    type,
    taskRunId: detail?.activeTaskRunId ?? null,
    approvalId: type === 'approve' ? detail?.activeRun?.pendingApprovalId ?? null : null,
    payload: {},
  })
}

async function approveSelectedTask() {
  await submitSelectedTaskIntervention('approve')
}

async function rejectSelectedTask() {
  await submitSelectedTaskIntervention('reject')
}

async function resumeSelectedTask() {
  await submitSelectedTaskIntervention('resume')
}

async function takeOverSelectedTask() {
  if (!projectId.value || !selectedTaskId.value) {
    return
  }

  const detail = await ensureSelectedTaskDetail()
  const fallbackTarget = taskConversationTargetFromDetail(detail)
  const intervention = await taskStore.createIntervention(projectId.value, selectedTaskId.value, {
    type: 'takeover',
    taskRunId: detail?.activeTaskRunId ?? null,
    payload: {},
  })

  if (!intervention) {
    return
  }

  const target = latestTaskConversationTarget() ?? fallbackTarget
  if (target) {
    await router.push(target)
  }
}
</script>

<template>
  <UiPageShell width="wide" test-id="project-tasks-view">
    <UiPageHeader
      :eyebrow="t('tasks.header.eyebrow')"
      :title="projectRecord?.name ?? t('tasks.header.titleFallback')"
      :description="projectRecord?.description || t('tasks.header.subtitle')"
    />

    <UiStatusCallout
      v-if="taskStore.error"
      tone="error"
      :description="taskStore.error"
    />

    <UiStatusCallout
      v-if="openedFromConversation"
      tone="info"
      :title="t('tasks.detail.fromConversationTitle')"
      :description="t('tasks.detail.fromConversationDescription')"
      data-testid="project-tasks-conversation-callout"
    >
      <div class="flex flex-wrap gap-2">
        <UiButton
          size="sm"
          variant="secondary"
          data-testid="project-tasks-back-to-conversation"
          @click="returnToSourceConversation"
        >
          {{ t('tasks.detail.backToConversation') }}
        </UiButton>
      </div>
    </UiStatusCallout>

    <UiListDetailWorkspace
      :has-selection="Boolean(selectedTask)"
      :detail-title="selectedTask?.title"
      :detail-subtitle="selectedTask ? taskStatusLabel(selectedTask.status) : t('common.na')"
      :empty-detail-title="t('tasks.detail.emptyTitle')"
      :empty-detail-description="t('tasks.detail.emptyDescription')"
      detail-class="xl:min-w-[420px]"
    >
      <template #toolbar>
        <UiToolbarRow test-id="project-tasks-toolbar">
          <template #search>
            <UiInput
              v-model="searchQuery"
              data-testid="project-tasks-search-input"
              :aria-label="t('tasks.filters.searchPlaceholder')"
              :placeholder="t('tasks.filters.searchPlaceholder')"
            />
          </template>
          <template #actions>
            <UiButton data-testid="project-task-create-button" @click="openCreateDialog">
              {{ t('tasks.actions.create') }}
            </UiButton>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <div class="space-y-3">
          <div v-if="filteredTasks.length" class="space-y-2">
            <div
              v-for="task in filteredTasks"
              :key="task.id"
              :data-testid="`project-task-row-${task.id}`"
              class="rounded-[var(--radius-l)]"
              @click="selectTaskRow(task.id)"
            >
              <UiListRow
                :title="task.title"
                :subtitle="task.goal"
                :eyebrow="taskStatusLabel(task.status)"
                interactive
                :active="selectedTaskId === task.id"
              >
                <template #meta>
                  <UiBadge :label="viewStatusLabel(task.viewStatus)" subtle />
                  <UiBadge
                    v-if="task.scheduleSpec"
                    :label="t('tasks.list.scheduled')"
                    subtle
                  />
                  <span class="text-xs text-text-tertiary">{{ formatDateTime(task.updatedAt) }}</span>
                </template>
              </UiListRow>
            </div>
          </div>

          <UiEmptyState
            v-else
            :title="t('tasks.empty.title')"
            :description="t('tasks.empty.description')"
          />
        </div>
      </template>

      <template #detail>
        <section
          v-if="selectedTask"
          data-testid="project-task-detail"
          class="space-y-4"
        >
          <div class="flex flex-wrap items-start justify-between gap-3">
            <div class="flex flex-wrap items-center gap-2">
              <UiBadge :label="taskStatusLabel(selectedTask.status)" subtle />
              <UiBadge :label="viewStatusLabel(selectedTask.viewStatus)" subtle />
              <UiBadge
                v-if="selectedTask.latestFailureCategory"
                :label="taskFailureLabel(selectedTask.latestFailureCategory)"
                subtle
              />
            </div>

            <div class="flex flex-wrap items-center gap-2">
              <UiButton
                variant="ghost"
                data-testid="project-task-detail-edit"
                :disabled="selectedTaskBusy"
                @click="openEditDialog"
              >
                {{ t('common.edit') }}
              </UiButton>
              <UiButton
                variant="ghost"
                data-testid="project-task-detail-comment"
                :disabled="selectedTaskBusy"
                @click="openCommentDialog"
              >
                {{ t('tasks.actions.comment') }}
              </UiButton>
              <UiButton
                variant="ghost"
                data-testid="project-task-detail-edit-brief"
                :disabled="selectedTaskBusy"
                @click="openBriefDialog"
              >
                {{ t('tasks.actions.editBrief') }}
              </UiButton>
              <UiButton
                variant="ghost"
                data-testid="project-task-detail-change-actor"
                :disabled="selectedTaskBusy"
                @click="openChangeActorDialog"
              >
                {{ t('tasks.actions.changeActor') }}
              </UiButton>
              <UiButton
                v-if="selectedTaskNeedsApproval"
                variant="ghost"
                data-testid="project-task-detail-approve"
                :disabled="selectedTaskBusy"
                @click="approveSelectedTask"
              >
                {{ t('tasks.actions.approve') }}
              </UiButton>
              <UiButton
                v-if="selectedTaskNeedsApproval"
                variant="ghost"
                data-testid="project-task-detail-reject"
                :disabled="selectedTaskBusy"
                @click="rejectSelectedTask"
              >
                {{ t('tasks.actions.reject') }}
              </UiButton>
              <UiButton
                v-if="selectedTaskCanResume"
                variant="ghost"
                data-testid="project-task-detail-resume"
                :disabled="selectedTaskBusy"
                @click="resumeSelectedTask"
              >
                {{ t('tasks.actions.resume') }}
              </UiButton>
              <UiButton
                variant="ghost"
                data-testid="project-task-detail-takeover"
                :disabled="selectedTaskBusy"
                @click="takeOverSelectedTask"
              >
                {{ t('tasks.actions.takeover') }}
              </UiButton>
              <UiButton
                variant="ghost"
                data-testid="project-task-detail-rerun"
                :loading="selectedTaskLaunchLoading"
                :disabled="selectedTaskSaveLoading"
                @click="rerunSelectedTask"
              >
                {{ t('tasks.actions.rerun') }}
              </UiButton>
              <UiButton
                data-testid="project-task-detail-launch"
                :loading="selectedTaskLaunchLoading"
                :disabled="selectedTaskSaveLoading"
                @click="launchSelectedTask"
              >
                {{ t('tasks.actions.launch') }}
              </UiButton>
            </div>
          </div>

          <div class="space-y-2">
            <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">
              {{ t('common.goal') }}
            </div>
            <p class="text-sm leading-6 text-text-primary">
              {{ selectedTask.goal }}
            </p>
          </div>

          <div class="space-y-2">
            <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">
              {{ t('tasks.detail.briefLabel') }}
            </div>
            <p class="text-sm leading-6 text-text-primary">
              {{ selectedTaskDetail?.brief || t('tasks.detail.noBrief') }}
            </p>
          </div>

          <UiPanelFrame
            variant="panel"
            padding="md"
            :title="t('tasks.detail.latestResultTitle')"
            :subtitle="selectedTask.latestResultSummary || t('tasks.detail.noResult')"
          >
            <div class="space-y-3 text-sm text-text-secondary">
              <p class="leading-6 text-text-primary">
                {{ selectedTask.latestTransition?.summary || t('tasks.detail.noTransition') }}
              </p>
              <div class="flex flex-wrap items-center gap-3 text-xs text-text-tertiary">
                <span>{{ t('common.updatedAt', { time: formatDateTime(selectedTask.updatedAt) }) }}</span>
                <span v-if="selectedTask.latestTransition">{{ formatDateTime(selectedTask.latestTransition.at) }}</span>
              </div>
            </div>
          </UiPanelFrame>

          <div class="grid gap-4 xl:grid-cols-2">
          <UiPanelFrame
            variant="panel"
            padding="md"
            :title="t('tasks.detail.scheduleTitle')"
            :subtitle="selectedTask.scheduleSpec || t('tasks.detail.unscheduled')"
          >
              <div class="space-y-3 text-sm text-text-secondary">
                <div>
                  <span class="font-medium text-text-primary">{{ t('tasks.detail.nextRunLabel') }}</span>
                  <span class="ml-2">{{ formatDateTime(selectedTask.nextRunAt ?? undefined) }}</span>
                </div>
                <div>
                  <span class="font-medium text-text-primary">{{ t('tasks.detail.lastRunLabel') }}</span>
                  <span class="ml-2">{{ formatDateTime(selectedTask.lastRunAt ?? undefined) }}</span>
                </div>
                <div>
                  <span class="font-medium text-text-primary">{{ t('tasks.detail.actorLabel') }}</span>
                  <span class="ml-2">{{ selectedTask.defaultActorRef }}</span>
                </div>
                <UiField
                  :label="t('tasks.detail.executionActorLabel')"
                  :hint="t('tasks.detail.executionActorHint')"
                >
                  <UiInput
                    v-model="executionActorRef"
                    data-testid="project-task-detail-execution-actor"
                    :placeholder="selectedTask.defaultActorRef"
                  />
                </UiField>
              </div>
            </UiPanelFrame>

            <UiPanelFrame
              variant="panel"
              padding="md"
              :title="t('tasks.detail.attentionTitle')"
              :subtitle="selectedTask.attentionReasons.length ? t('tasks.detail.attentionSubtitle') : t('tasks.detail.noAttention')"
            >
              <div v-if="selectedTask.attentionReasons.length" class="flex flex-wrap gap-2">
                <UiBadge
                  v-for="reason in selectedTask.attentionReasons"
                  :key="reason"
                  :label="taskAttentionReasonLabel(reason)"
                  subtle
                />
              </div>
              <p v-else class="text-sm text-text-secondary">
                {{ t('tasks.detail.noAttention') }}
              </p>
            </UiPanelFrame>
          </div>

          <div data-testid="project-task-context-panel">
            <UiPanelFrame
              variant="panel"
              padding="md"
              :title="t('tasks.detail.contextTitle')"
              :subtitle="t('tasks.detail.contextSubtitle', { count: selectedTaskContextBundle.refs.length })"
            >
              <div class="space-y-4 text-sm text-text-secondary">
                <div class="space-y-2">
                  <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">
                    {{ t('tasks.detail.contextInstructionsLabel') }}
                  </div>
                  <p class="leading-6 text-text-primary">
                    {{ selectedTaskContextBundle.pinnedInstructions || t('tasks.detail.contextNoInstructions') }}
                  </p>
                </div>

                <div class="flex flex-wrap items-center gap-3 text-xs text-text-tertiary">
                  <span>
                    <span class="font-medium text-text-primary">{{ t('tasks.detail.contextResolutionLabel') }}</span>
                    <span class="ml-2">{{ taskContextResolutionLabel(selectedTaskContextBundle.resolutionMode) }}</span>
                  </span>
                  <span>
                    <span class="font-medium text-text-primary">{{ t('tasks.detail.contextResolvedAtLabel') }}</span>
                    <span class="ml-2">
                      {{
                        selectedTaskContextBundle.lastResolvedAt
                          ? formatDateTime(selectedTaskContextBundle.lastResolvedAt)
                          : t('tasks.detail.contextNotResolved')
                      }}
                    </span>
                  </span>
                </div>

                <div v-if="selectedTaskContextBundle.refs.length" class="space-y-2">
                  <div
                    v-for="reference in selectedTaskContextBundle.refs"
                    :key="`${reference.kind}:${reference.refId}`"
                    :data-testid="`project-task-context-ref-${reference.refId}`"
                    class="rounded-[var(--radius-m)] border border-border bg-surface-muted px-3 py-3"
                  >
                    <div class="flex flex-wrap items-center gap-2">
                      <UiBadge :label="taskContextRefKindLabel(reference.kind)" subtle />
                      <UiBadge :label="taskContextPinModeLabel(reference.pinMode)" subtle />
                      <UiBadge
                        v-if="reference.versionRef"
                        :label="t('tasks.detail.contextVersionBadge', { version: reference.versionRef })"
                        subtle
                      />
                    </div>
                    <div class="mt-2 text-sm font-medium text-text-primary">{{ reference.title }}</div>
                    <div v-if="reference.subtitle" class="mt-1 text-sm text-text-secondary">
                      {{ reference.subtitle }}
                    </div>
                  </div>
                </div>

                <p v-else class="text-sm text-text-secondary">
                  {{ t('tasks.detail.contextNoRefs') }}
                </p>
              </div>
            </UiPanelFrame>
          </div>

          <UiPanelFrame
            variant="panel"
            padding="md"
            :title="t('tasks.detail.runsTitle')"
            :subtitle="t('tasks.detail.runsSubtitle', { count: selectedTaskRuns.length })"
          >
            <div v-if="selectedTaskRuns.length" class="space-y-2">
              <div
                v-for="run in selectedTaskRuns"
                :key="run.id"
                class="rounded-[var(--radius-m)] border border-border bg-surface-muted px-3 py-3"
              >
                <div class="flex flex-wrap items-center gap-2">
                  <UiBadge :label="taskTriggerLabel(run.triggerType)" subtle />
                  <UiBadge :label="viewStatusLabel(run.viewStatus)" subtle />
                  <UiBadge :label="taskRunStatusLabel(run.status)" subtle />
                </div>
                <div class="mt-2 text-sm font-medium text-text-primary">{{ run.id }}</div>
                <div class="mt-1 text-sm text-text-secondary">
                  {{ run.resultSummary || run.latestTransition?.summary || t('tasks.detail.noRunSummary') }}
                </div>
                <div class="mt-2 text-xs text-text-tertiary">
                  <span class="font-medium text-text-primary">{{ t('tasks.detail.runActorLabel') }}</span>
                  <span class="ml-2">{{ run.actorRef }}</span>
                </div>
                <div class="mt-2 text-xs text-text-tertiary">
                  {{ formatDateTime(run.startedAt) }}
                </div>
                <RouterLink
                  v-if="taskRunConversationTarget(run.conversationId)"
                  :to="taskRunConversationTarget(run.conversationId)!"
                  :data-testid="`project-task-run-conversation-${run.id}`"
                  class="mt-2 inline-flex items-center text-sm font-medium text-primary hover:underline"
                >
                  {{ t('tasks.detail.openConversation') }}
                </RouterLink>
              </div>
            </div>
            <UiEmptyState
              v-else
              :title="t('tasks.detail.runsEmptyTitle')"
              :description="t('tasks.detail.runsEmptyDescription')"
            />
          </UiPanelFrame>

          <UiPanelFrame
            variant="panel"
            padding="md"
            :title="t('tasks.detail.interventionsTitle')"
            :subtitle="t('tasks.detail.interventionsSubtitle', { count: selectedTaskInterventions.length })"
          >
            <UiTimelineList
              v-if="selectedTaskInterventions.length"
              :items="selectedTaskInterventions.map(record => ({
                id: record.id,
                title: taskInterventionTypeLabel(record.type),
                description: taskInterventionDescription(record),
                timestamp: formatDateTime(record.createdAt),
                helper: taskInterventionStatusLabel(record.status),
              }))"
              density="compact"
            >
              <template #item="{ item }">
                <div class="mt-1.5 size-2 shrink-0 rounded-full bg-primary" />
                <div
                  :data-testid="`project-task-intervention-${item.id}`"
                  class="min-w-0 flex-1"
                >
                  <small
                    v-if="item.helper"
                    class="block pb-1 text-[0.68rem] font-semibold uppercase tracking-[0.08em] text-text-tertiary"
                  >
                    {{ item.helper }}
                  </small>
                  <strong class="block text-sm font-semibold text-text-primary">{{ item.title }}</strong>
                  <small
                    v-if="item.description"
                    class="block pt-1 text-sm leading-6 text-text-secondary"
                  >
                    {{ item.description }}
                  </small>
                  <RouterLink
                    v-if="taskInterventionConversationTarget(selectedTaskInterventions.find(record => record.id === item.id)?.taskRunId)"
                    :to="taskInterventionConversationTarget(selectedTaskInterventions.find(record => record.id === item.id)?.taskRunId)!"
                    class="mt-2 inline-flex items-center text-sm font-medium text-primary hover:underline"
                  >
                    {{ t('tasks.detail.openConversation') }}
                  </RouterLink>
                </div>
                <span
                  v-if="item.timestamp"
                  class="shrink-0 text-xs leading-5 text-text-tertiary"
                >
                  {{ item.timestamp }}
                </span>
              </template>
            </UiTimelineList>
            <UiEmptyState
              v-else
              :title="t('tasks.detail.interventionsEmptyTitle')"
              :description="t('tasks.detail.interventionsEmptyDescription')"
            />
          </UiPanelFrame>
        </section>
      </template>
    </UiListDetailWorkspace>

    <UiDialog
      v-model:open="createDialogOpen"
      :title="t('tasks.dialogs.create.title')"
      :description="t('tasks.dialogs.create.description')"
      content-test-id="project-task-create-dialog"
    >
      <div class="space-y-4">
        <UiStatusCallout
          v-if="editorError"
          tone="error"
          :description="editorError"
        />

        <UiField :label="t('tasks.form.titleLabel')">
          <UiInput
            v-model="createForm.title"
            data-testid="project-task-create-title"
            :placeholder="t('tasks.form.titlePlaceholder')"
          />
        </UiField>

        <UiField :label="t('common.goal')">
          <UiTextarea
            v-model="createForm.goal"
            data-testid="project-task-create-goal"
            :rows="3"
            :placeholder="t('tasks.form.goalPlaceholder')"
          />
        </UiField>

        <UiField :label="t('tasks.form.briefLabel')">
          <UiTextarea
            v-model="createForm.brief"
            data-testid="project-task-create-brief"
            :rows="4"
            :placeholder="t('tasks.form.briefPlaceholder')"
          />
        </UiField>

        <UiField :label="t('tasks.form.actorLabel')">
          <UiInput
            v-model="createForm.defaultActorRef"
            data-testid="project-task-create-actor"
            :placeholder="t('tasks.form.actorPlaceholder')"
          />
        </UiField>

        <UiField :label="t('tasks.form.scheduleLabel')" :hint="t('tasks.form.scheduleHint')">
          <UiInput
            v-model="createForm.scheduleSpec"
            data-testid="project-task-create-schedule"
            :placeholder="t('tasks.form.schedulePlaceholder')"
          />
        </UiField>
      </div>

      <template #footer>
        <UiButton variant="ghost" :disabled="createSaving" @click="closeCreateDialog">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton
          data-testid="project-task-create-submit"
          :loading="createSaving"
          @click="submitCreate"
        >
          {{ t('tasks.actions.create') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      v-model:open="editDialogOpen"
      :title="t('tasks.dialogs.edit.title')"
      :description="t('tasks.dialogs.edit.description')"
      content-test-id="project-task-edit-dialog"
    >
      <div class="space-y-4">
        <UiStatusCallout
          v-if="editorError"
          tone="error"
          :description="editorError"
        />

        <UiField :label="t('tasks.form.titleLabel')">
          <UiInput
            v-model="editForm.title"
            data-testid="project-task-edit-title"
            :placeholder="t('tasks.form.titlePlaceholder')"
          />
        </UiField>

        <UiField :label="t('common.goal')">
          <UiTextarea
            v-model="editForm.goal"
            data-testid="project-task-edit-goal"
            :rows="3"
            :placeholder="t('tasks.form.goalPlaceholder')"
          />
        </UiField>

        <UiField :label="t('tasks.form.briefLabel')">
          <UiTextarea
            v-model="editForm.brief"
            data-testid="project-task-edit-brief"
            :rows="4"
            :placeholder="t('tasks.form.briefPlaceholder')"
          />
        </UiField>

        <UiField :label="t('tasks.form.actorLabel')">
          <UiInput
            v-model="editForm.defaultActorRef"
            data-testid="project-task-edit-actor"
            :placeholder="t('tasks.form.actorPlaceholder')"
          />
        </UiField>

        <UiField :label="t('tasks.form.scheduleLabel')" :hint="t('tasks.form.scheduleHint')">
          <UiInput
            v-model="editForm.scheduleSpec"
            data-testid="project-task-edit-schedule"
            :placeholder="t('tasks.form.schedulePlaceholder')"
          />
        </UiField>
      </div>

      <template #footer>
        <UiButton variant="ghost" :disabled="selectedTaskSaveLoading" @click="closeEditDialog">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton
          data-testid="project-task-edit-submit"
          :loading="selectedTaskSaveLoading"
          @click="submitEdit"
        >
          {{ t('common.save') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      v-model:open="commentDialogOpen"
      :title="t('tasks.dialogs.comment.title')"
      :description="t('tasks.dialogs.comment.description')"
      content-test-id="project-task-comment-dialog"
    >
      <div class="space-y-4">
        <UiStatusCallout
          v-if="commentInterventionError"
          tone="error"
          :description="commentInterventionError"
        />

        <UiField :label="t('tasks.form.commentLabel')">
          <UiTextarea
            v-model="commentInterventionValue"
            data-testid="project-task-comment-input"
            :rows="4"
            :placeholder="t('tasks.form.commentPlaceholder')"
          />
        </UiField>
      </div>

      <template #footer>
        <UiButton variant="ghost" :disabled="selectedTaskSaveLoading" @click="closeCommentDialog">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton
          data-testid="project-task-comment-submit"
          :loading="selectedTaskSaveLoading"
          @click="submitCommentIntervention"
        >
          {{ t('common.save') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      v-model:open="changeActorDialogOpen"
      :title="t('tasks.dialogs.changeActor.title')"
      :description="t('tasks.dialogs.changeActor.description')"
      content-test-id="project-task-change-actor-dialog"
    >
      <div class="space-y-4">
        <UiStatusCallout
          v-if="changeActorInterventionError"
          tone="error"
          :description="changeActorInterventionError"
        />

        <UiField :label="t('tasks.form.actorLabel')">
          <UiInput
            v-model="changeActorInterventionValue"
            data-testid="project-task-change-actor-input"
            :placeholder="t('tasks.form.actorPlaceholder')"
          />
        </UiField>
      </div>

      <template #footer>
        <UiButton variant="ghost" :disabled="selectedTaskSaveLoading" @click="closeChangeActorDialog">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton
          data-testid="project-task-change-actor-submit"
          :loading="selectedTaskSaveLoading"
          @click="submitChangeActorIntervention"
        >
          {{ t('common.save') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      v-model:open="briefDialogOpen"
      :title="t('tasks.dialogs.brief.title')"
      :description="t('tasks.dialogs.brief.description')"
      content-test-id="project-task-brief-dialog"
    >
      <div class="space-y-4">
        <UiStatusCallout
          v-if="briefInterventionError"
          tone="error"
          :description="briefInterventionError"
        />

        <UiField :label="t('tasks.form.briefLabel')">
          <UiTextarea
            v-model="briefInterventionValue"
            data-testid="project-task-brief-input"
            :rows="5"
            :placeholder="t('tasks.form.briefPlaceholder')"
          />
        </UiField>
      </div>

      <template #footer>
        <UiButton variant="ghost" :disabled="selectedTaskSaveLoading" @click="closeBriefDialog">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton
          data-testid="project-task-brief-submit"
          :loading="selectedTaskSaveLoading"
          @click="submitBriefIntervention"
        >
          {{ t('common.save') }}
        </UiButton>
      </template>
    </UiDialog>
  </UiPageShell>
</template>
