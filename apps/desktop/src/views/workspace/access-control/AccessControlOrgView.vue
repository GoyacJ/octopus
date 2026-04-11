<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'

import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiDialog,
  UiEmptyState,
  UiField,
  UiInput,
  UiListDetailWorkspace,
  UiPanelFrame,
  UiSelect,
  UiStatTile,
  UiStatusCallout,
  UiTabs,
  UiToolbarRow,
} from '@octopus/ui'

import type {
  OrgUnitRecord,
  OrgUnitUpsertRequest,
  PositionRecord,
  PositionUpsertRequest,
  UserGroupRecord,
  UserGroupUpsertRequest,
  UserOrgAssignmentRecord,
  UserOrgAssignmentUpsertRequest,
} from '@octopus/schema'

import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'

import { statusOptions } from './helpers'

const accessControlStore = useWorkspaceAccessControlStore()

const metrics = computed(() => ({
  units: accessControlStore.orgUnits.length,
  positions: accessControlStore.positions.length,
  groups: accessControlStore.userGroups.length,
  assignments: accessControlStore.userOrgAssignments.length,
}))

const activeSection = ref('units')
const roleSubsection = ref('positions')
const submitError = ref('')
const successMessage = ref('')

const unitsQuery = ref('')
const positionsQuery = ref('')
const groupsQuery = ref('')
const assignmentsQuery = ref('')

const selectedOrgUnitId = ref('')
const selectedPositionId = ref('')
const selectedGroupId = ref('')
const selectedAssignmentKey = ref('')

const unitDialogOpen = ref(false)
const positionDialogOpen = ref(false)
const groupDialogOpen = ref(false)
const assignmentDialogOpen = ref(false)

const savingOrgUnit = ref(false)
const savingPosition = ref(false)
const savingGroup = ref(false)
const savingAssignment = ref(false)
const deletingOrgUnitId = ref('')
const deletingPositionId = ref('')
const deletingGroupId = ref('')
const deletingAssignmentKey = ref('')

const sectionTabs = [
  { value: 'units', label: '部门' },
  { value: 'roles', label: '岗位与用户组' },
  { value: 'assignments', label: '用户归属' },
]

const roleTabs = [
  { value: 'positions', label: '岗位' },
  { value: 'groups', label: '用户组' },
]

const orgUnitForm = reactive({
  code: '',
  name: '',
  parentId: '',
  status: 'active',
})

const positionForm = reactive({
  code: '',
  name: '',
  status: 'active',
})

const groupForm = reactive({
  code: '',
  name: '',
  status: 'active',
})

const assignmentForm = reactive({
  userId: '',
  orgUnitId: '',
  isPrimary: true,
  positionIds: [] as string[],
  userGroupIds: [] as string[],
})

const orgUnitMap = computed(() => new Map(accessControlStore.orgUnits.map(unit => [unit.id, unit.name])))
const assignmentUserMap = computed(() => new Map(accessControlStore.users.map(user => [user.id, user.displayName])))
const positionMap = computed(() => new Map(accessControlStore.positions.map(position => [position.id, position.name])))
const groupMap = computed(() => new Map(accessControlStore.userGroups.map(group => [group.id, group.name])))

const userOptions = computed(() =>
  accessControlStore.users.map(user => ({ label: user.displayName, value: user.id })),
)

const orgUnitParentOptions = computed(() => [
  { label: '无上级部门', value: '' },
  ...accessControlStore.orgUnits.map(unit => ({ label: unit.name, value: unit.id })),
])

const orgUnitOptions = computed(() =>
  accessControlStore.orgUnits.map(unit => ({ label: unit.name, value: unit.id })),
)

const filteredOrgUnits = computed(() => {
  const normalizedQuery = unitsQuery.value.trim().toLowerCase()
  return [...accessControlStore.orgUnits]
    .sort((left, right) => left.name.localeCompare(right.name))
    .filter(unit => !normalizedQuery || [
      unit.name,
      unit.code,
      orgUnitMap.value.get(unit.parentId ?? '') ?? '',
    ].join(' ').toLowerCase().includes(normalizedQuery))
})

const filteredPositions = computed(() => {
  const normalizedQuery = positionsQuery.value.trim().toLowerCase()
  return [...accessControlStore.positions]
    .sort((left, right) => left.name.localeCompare(right.name))
    .filter(position => !normalizedQuery || [
      position.name,
      position.code,
      position.status,
    ].join(' ').toLowerCase().includes(normalizedQuery))
})

const filteredGroups = computed(() => {
  const normalizedQuery = groupsQuery.value.trim().toLowerCase()
  return [...accessControlStore.userGroups]
    .sort((left, right) => left.name.localeCompare(right.name))
    .filter(group => !normalizedQuery || [
      group.name,
      group.code,
      group.status,
    ].join(' ').toLowerCase().includes(normalizedQuery))
})

const filteredAssignments = computed(() => {
  const normalizedQuery = assignmentsQuery.value.trim().toLowerCase()
  return [...accessControlStore.userOrgAssignments]
    .sort((left, right) => {
      const leftLabel = assignmentUserMap.value.get(left.userId) ?? left.userId
      const rightLabel = assignmentUserMap.value.get(right.userId) ?? right.userId
      return leftLabel.localeCompare(rightLabel)
    })
    .filter(assignment => !normalizedQuery || [
      assignmentUserMap.value.get(assignment.userId) ?? assignment.userId,
      orgUnitMap.value.get(assignment.orgUnitId) ?? assignment.orgUnitId,
      ...assignment.positionIds.map(id => positionMap.value.get(id) ?? id),
      ...assignment.userGroupIds.map(id => groupMap.value.get(id) ?? id),
    ].join(' ').toLowerCase().includes(normalizedQuery))
})

const selectedOrgUnit = computed(() =>
  accessControlStore.orgUnits.find(unit => unit.id === selectedOrgUnitId.value) ?? null,
)
const selectedPosition = computed(() =>
  accessControlStore.positions.find(position => position.id === selectedPositionId.value) ?? null,
)
const selectedGroup = computed(() =>
  accessControlStore.userGroups.find(group => group.id === selectedGroupId.value) ?? null,
)
const selectedAssignment = computed(() =>
  accessControlStore.userOrgAssignments.find(
    assignment => `${assignment.userId}:${assignment.orgUnitId}` === selectedAssignmentKey.value,
  ) ?? null,
)

watch(selectedOrgUnit, (unit) => {
  if (!unit) {
    resetOrgUnitForm()
    return
  }
  populateOrgUnitForm(unit)
}, { immediate: true })

watch(selectedPosition, (position) => {
  if (!position) {
    resetPositionForm()
    return
  }
  populatePositionForm(position)
}, { immediate: true })

watch(selectedGroup, (group) => {
  if (!group) {
    resetGroupForm()
    return
  }
  populateGroupForm(group)
}, { immediate: true })

watch(selectedAssignment, (assignment) => {
  if (!assignment) {
    resetAssignmentForm()
    return
  }
  populateAssignmentForm(assignment)
}, { immediate: true })

watch(filteredOrgUnits, (units) => {
  if (selectedOrgUnitId.value && !units.some(unit => unit.id === selectedOrgUnitId.value)) {
    selectedOrgUnitId.value = ''
  }
})

watch(filteredPositions, (positions) => {
  if (selectedPositionId.value && !positions.some(position => position.id === selectedPositionId.value)) {
    selectedPositionId.value = ''
  }
})

watch(filteredGroups, (groups) => {
  if (selectedGroupId.value && !groups.some(group => group.id === selectedGroupId.value)) {
    selectedGroupId.value = ''
  }
})

watch(filteredAssignments, (assignments) => {
  if (
    selectedAssignmentKey.value
    && !assignments.some(assignment => `${assignment.userId}:${assignment.orgUnitId}` === selectedAssignmentKey.value)
  ) {
    selectedAssignmentKey.value = ''
  }
})

function clearMessages() {
  submitError.value = ''
  successMessage.value = ''
}

function resetOrgUnitForm() {
  Object.assign(orgUnitForm, {
    code: '',
    name: '',
    parentId: '',
    status: 'active',
  })
}

function populateOrgUnitForm(unit: OrgUnitRecord) {
  Object.assign(orgUnitForm, {
    code: unit.code,
    name: unit.name,
    parentId: unit.parentId ?? '',
    status: unit.status,
  })
}

function resetPositionForm() {
  Object.assign(positionForm, {
    code: '',
    name: '',
    status: 'active',
  })
}

function populatePositionForm(position: PositionRecord) {
  Object.assign(positionForm, {
    code: position.code,
    name: position.name,
    status: position.status,
  })
}

function resetGroupForm() {
  Object.assign(groupForm, {
    code: '',
    name: '',
    status: 'active',
  })
}

function populateGroupForm(group: UserGroupRecord) {
  Object.assign(groupForm, {
    code: group.code,
    name: group.name,
    status: group.status,
  })
}

function resetAssignmentForm() {
  Object.assign(assignmentForm, {
    userId: accessControlStore.users[0]?.id ?? '',
    orgUnitId: accessControlStore.orgUnits[0]?.id ?? '',
    isPrimary: true,
    positionIds: [],
    userGroupIds: [],
  })
}

function populateAssignmentForm(assignment: UserOrgAssignmentRecord) {
  Object.assign(assignmentForm, {
    userId: assignment.userId,
    orgUnitId: assignment.orgUnitId,
    isPrimary: assignment.isPrimary,
    positionIds: [...assignment.positionIds],
    userGroupIds: [...assignment.userGroupIds],
  })
}

function openCreateUnitDialog() {
  clearMessages()
  selectedOrgUnitId.value = ''
  resetOrgUnitForm()
  unitDialogOpen.value = true
}

function openCreatePositionDialog() {
  clearMessages()
  selectedPositionId.value = ''
  resetPositionForm()
  positionDialogOpen.value = true
}

function openCreateGroupDialog() {
  clearMessages()
  selectedGroupId.value = ''
  resetGroupForm()
  groupDialogOpen.value = true
}

function openCreateAssignmentDialog() {
  clearMessages()
  selectedAssignmentKey.value = ''
  resetAssignmentForm()
  assignmentDialogOpen.value = true
}

function selectOrgUnit(unitId: string) {
  clearMessages()
  selectedOrgUnitId.value = unitId
}

function selectPosition(positionId: string) {
  clearMessages()
  selectedPositionId.value = positionId
}

function selectGroup(groupId: string) {
  clearMessages()
  selectedGroupId.value = groupId
}

function selectAssignment(assignment: UserOrgAssignmentRecord) {
  clearMessages()
  selectedAssignmentKey.value = `${assignment.userId}:${assignment.orgUnitId}`
}

function validateOrgUnitForm() {
  if (!orgUnitForm.code.trim() || !orgUnitForm.name.trim()) {
    return '请填写完整的部门编码和名称。'
  }
  return ''
}

function validatePositionForm() {
  if (!positionForm.code.trim() || !positionForm.name.trim()) {
    return '请填写完整的岗位编码和名称。'
  }
  return ''
}

function validateGroupForm() {
  if (!groupForm.code.trim() || !groupForm.name.trim()) {
    return '请填写完整的用户组编码和名称。'
  }
  return ''
}

function validateAssignmentForm() {
  if (!assignmentForm.userId || !assignmentForm.orgUnitId) {
    return '请选择用户和部门。'
  }
  return ''
}

async function saveOrgUnit(isCreate = false) {
  submitError.value = validateOrgUnitForm()
  if (submitError.value) {
    return null
  }

  savingOrgUnit.value = true
  try {
    const payload: OrgUnitUpsertRequest = {
      code: orgUnitForm.code.trim(),
      name: orgUnitForm.name.trim(),
      parentId: orgUnitForm.parentId || undefined,
      status: orgUnitForm.status,
    }

    const record = isCreate || !selectedOrgUnit.value
      ? await accessControlStore.createOrgUnit(payload)
      : await accessControlStore.updateOrgUnit(selectedOrgUnit.value.id, payload)

    successMessage.value = `已保存部门 ${record.name}`
    return record
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存部门失败。'
    return null
  } finally {
    savingOrgUnit.value = false
  }
}

async function savePosition(isCreate = false) {
  submitError.value = validatePositionForm()
  if (submitError.value) {
    return null
  }

  savingPosition.value = true
  try {
    const payload: PositionUpsertRequest = {
      code: positionForm.code.trim(),
      name: positionForm.name.trim(),
      status: positionForm.status,
    }

    const record = isCreate || !selectedPosition.value
      ? await accessControlStore.createPosition(payload)
      : await accessControlStore.updatePosition(selectedPosition.value.id, payload)

    successMessage.value = `已保存岗位 ${record.name}`
    return record
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存岗位失败。'
    return null
  } finally {
    savingPosition.value = false
  }
}

async function saveGroup(isCreate = false) {
  submitError.value = validateGroupForm()
  if (submitError.value) {
    return null
  }

  savingGroup.value = true
  try {
    const payload: UserGroupUpsertRequest = {
      code: groupForm.code.trim(),
      name: groupForm.name.trim(),
      status: groupForm.status,
    }

    const record = isCreate || !selectedGroup.value
      ? await accessControlStore.createUserGroup(payload)
      : await accessControlStore.updateUserGroup(selectedGroup.value.id, payload)

    successMessage.value = `已保存用户组 ${record.name}`
    return record
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存用户组失败。'
    return null
  } finally {
    savingGroup.value = false
  }
}

async function saveAssignment(isCreate = false) {
  submitError.value = validateAssignmentForm()
  if (submitError.value) {
    return null
  }

  savingAssignment.value = true
  try {
    const payload: UserOrgAssignmentUpsertRequest = {
      userId: assignmentForm.userId,
      orgUnitId: assignmentForm.orgUnitId,
      isPrimary: assignmentForm.isPrimary,
      positionIds: [...assignmentForm.positionIds],
      userGroupIds: [...assignmentForm.userGroupIds],
    }

    const record = await accessControlStore.upsertUserOrgAssignment(payload)
    successMessage.value = `已保存归属 ${assignmentUserMap.value.get(record.userId) ?? record.userId}`
    if (isCreate) {
      selectedAssignmentKey.value = `${record.userId}:${record.orgUnitId}`
    }
    return record
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存用户归属失败。'
    return null
  } finally {
    savingAssignment.value = false
  }
}

async function handleCreateUnit() {
  const record = await saveOrgUnit(true)
  if (!record) {
    return
  }
  unitDialogOpen.value = false
  selectedOrgUnitId.value = record.id
}

async function handleCreatePosition() {
  const record = await savePosition(true)
  if (!record) {
    return
  }
  positionDialogOpen.value = false
  roleSubsection.value = 'positions'
  selectedPositionId.value = record.id
}

async function handleCreateGroup() {
  const record = await saveGroup(true)
  if (!record) {
    return
  }
  groupDialogOpen.value = false
  roleSubsection.value = 'groups'
  selectedGroupId.value = record.id
}

async function handleCreateAssignment() {
  const record = await saveAssignment(true)
  if (!record) {
    return
  }
  assignmentDialogOpen.value = false
}

async function deleteOrgUnit() {
  if (!selectedOrgUnit.value) {
    return
  }

  deletingOrgUnitId.value = selectedOrgUnit.value.id
  submitError.value = ''
  try {
    const label = selectedOrgUnit.value.name
    await accessControlStore.deleteOrgUnit(selectedOrgUnit.value.id)
    selectedOrgUnitId.value = ''
    successMessage.value = `已删除部门 ${label}`
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除部门失败。'
  } finally {
    deletingOrgUnitId.value = ''
  }
}

async function deletePosition() {
  if (!selectedPosition.value) {
    return
  }

  deletingPositionId.value = selectedPosition.value.id
  submitError.value = ''
  try {
    const label = selectedPosition.value.name
    await accessControlStore.deletePosition(selectedPosition.value.id)
    selectedPositionId.value = ''
    successMessage.value = `已删除岗位 ${label}`
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除岗位失败。'
  } finally {
    deletingPositionId.value = ''
  }
}

async function deleteGroup() {
  if (!selectedGroup.value) {
    return
  }

  deletingGroupId.value = selectedGroup.value.id
  submitError.value = ''
  try {
    const label = selectedGroup.value.name
    await accessControlStore.deleteUserGroup(selectedGroup.value.id)
    selectedGroupId.value = ''
    successMessage.value = `已删除用户组 ${label}`
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除用户组失败。'
  } finally {
    deletingGroupId.value = ''
  }
}

async function deleteAssignment() {
  if (!selectedAssignment.value) {
    return
  }

  const key = `${selectedAssignment.value.userId}:${selectedAssignment.value.orgUnitId}`
  deletingAssignmentKey.value = key
  submitError.value = ''
  try {
    const label = assignmentUserMap.value.get(selectedAssignment.value.userId) ?? selectedAssignment.value.userId
    await accessControlStore.deleteUserOrgAssignment(
      selectedAssignment.value.userId,
      selectedAssignment.value.orgUnitId,
    )
    selectedAssignmentKey.value = ''
    successMessage.value = `已删除归属 ${label}`
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除用户归属失败。'
  } finally {
    deletingAssignmentKey.value = ''
  }
}

function assignmentPositionLabels(assignment: UserOrgAssignmentRecord) {
  return assignment.positionIds.map(id => positionMap.value.get(id) ?? id)
}

function assignmentGroupLabels(assignment: UserOrgAssignmentRecord) {
  return assignment.userGroupIds.map(id => groupMap.value.get(id) ?? id)
}
</script>

<template>
  <div class="space-y-4" data-testid="access-control-org-shell">
    <section class="grid gap-4 md:grid-cols-3">
      <UiStatTile label="部门" :value="String(metrics.units)" />
      <UiStatTile label="岗位 / 用户组" :value="String(metrics.positions + metrics.groups)" />
      <UiStatTile label="组织归属" :value="String(metrics.assignments)" />
    </section>

    <UiStatusCallout v-if="submitError" tone="error" :description="submitError" />
    <UiStatusCallout v-if="successMessage" tone="success" :description="successMessage" />

    <UiTabs v-model="activeSection" :tabs="sectionTabs" data-testid="access-control-org-section-tabs" />

    <UiListDetailWorkspace
      v-if="activeSection === 'units'"
      :has-selection="Boolean(selectedOrgUnit)"
      :detail-title="selectedOrgUnit ? selectedOrgUnit.name : ''"
      detail-subtitle="编辑部门编码、层级和状态。"
      empty-detail-title="请选择部门"
      empty-detail-description="从左侧部门列表中选择一项后即可查看详情，或在右上角新建部门。"
    >
      <template #toolbar>
        <UiToolbarRow>
          <template #search>
            <UiInput v-model="unitsQuery" placeholder="搜索部门名称、编码或上级部门" />
          </template>
          <template #actions>
            <UiButton @click="openCreateUnitDialog">新建部门</UiButton>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame variant="panel" padding="md" title="部门目录" :subtitle="`共 ${filteredOrgUnits.length} 个部门`">
          <div v-if="filteredOrgUnits.length" class="space-y-2">
            <button
              v-for="unit in filteredOrgUnits"
              :key="unit.id"
              type="button"
              class="w-full rounded-[var(--radius-l)] border px-4 py-3 text-left transition-colors"
              :class="selectedOrgUnitId === unit.id ? 'border-primary bg-accent/40' : 'border-border bg-card hover:bg-subtle/60'"
              @click="selectOrgUnit(unit.id)"
            >
              <div class="space-y-1">
                <div class="flex flex-wrap items-center gap-2">
                  <span class="text-sm font-semibold text-foreground">{{ unit.name }}</span>
                  <UiBadge :label="unit.status" subtle />
                </div>
                <div class="text-xs text-muted-foreground">{{ unit.code }}</div>
                <div class="text-xs text-muted-foreground">
                  {{ unit.parentId ? `上级：${orgUnitMap.get(unit.parentId) ?? unit.parentId}` : '根部门' }}
                </div>
              </div>
            </button>
          </div>
          <UiEmptyState v-else title="暂无组织单元" description="当前筛选条件下没有部门记录。" />
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedOrgUnit" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ selectedOrgUnit.name }}</div>
              <UiBadge :label="selectedOrgUnit.status" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">{{ selectedOrgUnit.code }}</div>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField label="部门编码">
              <UiInput v-model="orgUnitForm.code" data-testid="access-control-org-unit-code" />
            </UiField>
            <UiField label="部门名称">
              <UiInput v-model="orgUnitForm.name" data-testid="access-control-org-unit-name" />
            </UiField>
            <UiField label="上级部门">
              <UiSelect v-model="orgUnitForm.parentId" :options="orgUnitParentOptions" data-testid="access-control-org-unit-parent" />
            </UiField>
            <UiField label="状态">
              <UiSelect v-model="orgUnitForm.status" :options="statusOptions" data-testid="access-control-org-unit-status" />
            </UiField>
          </div>

          <div class="flex flex-wrap justify-between gap-2">
            <UiButton
              v-if="selectedOrgUnit.id !== 'org-root'"
              variant="ghost"
              class="text-destructive"
              :loading="deletingOrgUnitId === selectedOrgUnit.id"
              @click="deleteOrgUnit"
            >
              删除部门
            </UiButton>
            <UiButton :loading="savingOrgUnit" data-testid="access-control-org-unit-save" @click="saveOrgUnit()">
              保存部门
            </UiButton>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>

    <div v-else-if="activeSection === 'roles'" class="space-y-4">
      <UiTabs v-model="roleSubsection" :tabs="roleTabs" />

      <UiListDetailWorkspace
        v-if="roleSubsection === 'positions'"
        :has-selection="Boolean(selectedPosition)"
        :detail-title="selectedPosition ? selectedPosition.name : ''"
        detail-subtitle="岗位用于组织归属和角色绑定扩展。"
        empty-detail-title="请选择岗位"
        empty-detail-description="从左侧岗位列表中选择一项后即可查看详情，或在右上角新建岗位。"
      >
        <template #toolbar>
          <UiToolbarRow>
            <template #search>
              <UiInput v-model="positionsQuery" placeholder="搜索岗位名称或编码" />
            </template>
            <template #actions>
              <UiButton @click="openCreatePositionDialog">新建岗位</UiButton>
            </template>
          </UiToolbarRow>
        </template>

        <template #list>
          <UiPanelFrame variant="panel" padding="md" title="岗位列表" :subtitle="`共 ${filteredPositions.length} 个岗位`">
            <div v-if="filteredPositions.length" class="space-y-2">
              <button
                v-for="position in filteredPositions"
                :key="position.id"
                type="button"
                class="w-full rounded-[var(--radius-l)] border px-4 py-3 text-left transition-colors"
                :class="selectedPositionId === position.id ? 'border-primary bg-accent/40' : 'border-border bg-card hover:bg-subtle/60'"
                @click="selectPosition(position.id)"
              >
                <div class="space-y-1">
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="text-sm font-semibold text-foreground">{{ position.name }}</span>
                    <UiBadge :label="position.status" subtle />
                  </div>
                  <div class="text-xs text-muted-foreground">{{ position.code }}</div>
                </div>
              </button>
            </div>
            <UiEmptyState v-else title="暂无岗位" description="当前筛选条件下没有岗位记录。" />
          </UiPanelFrame>
        </template>

        <template #detail>
          <div v-if="selectedPosition" class="space-y-4">
            <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
              <div class="flex flex-wrap items-center gap-2">
                <div class="text-sm font-semibold text-foreground">{{ selectedPosition.name }}</div>
                <UiBadge :label="selectedPosition.status" subtle />
              </div>
              <div class="mt-2 text-xs text-muted-foreground">{{ selectedPosition.code }}</div>
            </div>

            <div class="grid gap-3 md:grid-cols-2">
              <UiField label="岗位编码">
                <UiInput v-model="positionForm.code" />
              </UiField>
              <UiField label="岗位名称">
                <UiInput v-model="positionForm.name" />
              </UiField>
              <UiField label="状态" class="md:col-span-2">
                <UiSelect v-model="positionForm.status" :options="statusOptions" />
              </UiField>
            </div>

            <div class="flex flex-wrap justify-between gap-2">
              <UiButton
                variant="ghost"
                class="text-destructive"
                :loading="deletingPositionId === selectedPosition.id"
                @click="deletePosition"
              >
                删除岗位
              </UiButton>
              <UiButton :loading="savingPosition" data-testid="access-control-position-save" @click="savePosition()">
                保存岗位
              </UiButton>
            </div>
          </div>
        </template>
      </UiListDetailWorkspace>

      <UiListDetailWorkspace
        v-else
        :has-selection="Boolean(selectedGroup)"
        :detail-title="selectedGroup ? selectedGroup.name : ''"
        detail-subtitle="用户组用于补充主体归属和授权范围。"
        empty-detail-title="请选择用户组"
        empty-detail-description="从左侧用户组列表中选择一项后即可查看详情，或在右上角新建用户组。"
      >
        <template #toolbar>
          <UiToolbarRow>
            <template #search>
              <UiInput v-model="groupsQuery" placeholder="搜索用户组名称或编码" />
            </template>
            <template #actions>
              <UiButton @click="openCreateGroupDialog">新建用户组</UiButton>
            </template>
          </UiToolbarRow>
        </template>

        <template #list>
          <UiPanelFrame variant="panel" padding="md" title="用户组列表" :subtitle="`共 ${filteredGroups.length} 个用户组`">
            <div v-if="filteredGroups.length" class="space-y-2">
              <button
                v-for="group in filteredGroups"
                :key="group.id"
                type="button"
                class="w-full rounded-[var(--radius-l)] border px-4 py-3 text-left transition-colors"
                :class="selectedGroupId === group.id ? 'border-primary bg-accent/40' : 'border-border bg-card hover:bg-subtle/60'"
                @click="selectGroup(group.id)"
              >
                <div class="space-y-1">
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="text-sm font-semibold text-foreground">{{ group.name }}</span>
                    <UiBadge :label="group.status" subtle />
                  </div>
                  <div class="text-xs text-muted-foreground">{{ group.code }}</div>
                </div>
              </button>
            </div>
            <UiEmptyState v-else title="暂无用户组" description="当前筛选条件下没有用户组记录。" />
          </UiPanelFrame>
        </template>

        <template #detail>
          <div v-if="selectedGroup" class="space-y-4">
            <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
              <div class="flex flex-wrap items-center gap-2">
                <div class="text-sm font-semibold text-foreground">{{ selectedGroup.name }}</div>
                <UiBadge :label="selectedGroup.status" subtle />
              </div>
              <div class="mt-2 text-xs text-muted-foreground">{{ selectedGroup.code }}</div>
            </div>

            <div class="grid gap-3 md:grid-cols-2">
              <UiField label="用户组编码">
                <UiInput v-model="groupForm.code" />
              </UiField>
              <UiField label="用户组名称">
                <UiInput v-model="groupForm.name" />
              </UiField>
              <UiField label="状态" class="md:col-span-2">
                <UiSelect v-model="groupForm.status" :options="statusOptions" />
              </UiField>
            </div>

            <div class="flex flex-wrap justify-between gap-2">
              <UiButton
                variant="ghost"
                class="text-destructive"
                :loading="deletingGroupId === selectedGroup.id"
                @click="deleteGroup"
              >
                删除用户组
              </UiButton>
              <UiButton :loading="savingGroup" data-testid="access-control-group-save" @click="saveGroup()">
                保存用户组
              </UiButton>
            </div>
          </div>
        </template>
      </UiListDetailWorkspace>
    </div>

    <UiListDetailWorkspace
      v-else
      :has-selection="Boolean(selectedAssignment)"
      :detail-title="selectedAssignment ? (assignmentUserMap.get(selectedAssignment.userId) ?? selectedAssignment.userId) : ''"
      detail-subtitle="维护用户主归属、附属归属、岗位和用户组。"
      empty-detail-title="请选择归属记录"
      empty-detail-description="从左侧归属列表中选择一项后即可查看详情，或在右上角新建归属。"
    >
      <template #toolbar>
        <UiToolbarRow>
          <template #search>
            <UiInput v-model="assignmentsQuery" placeholder="搜索用户、部门、岗位或用户组" />
          </template>
          <template #actions>
            <UiButton @click="openCreateAssignmentDialog">新建归属</UiButton>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame variant="panel" padding="md" title="用户归属" :subtitle="`共 ${filteredAssignments.length} 条归属记录`">
          <div v-if="filteredAssignments.length" class="space-y-2">
            <button
              v-for="assignment in filteredAssignments"
              :key="`${assignment.userId}:${assignment.orgUnitId}`"
              type="button"
              class="w-full rounded-[var(--radius-l)] border px-4 py-3 text-left transition-colors"
              :class="selectedAssignmentKey === `${assignment.userId}:${assignment.orgUnitId}` ? 'border-primary bg-accent/40' : 'border-border bg-card hover:bg-subtle/60'"
              @click="selectAssignment(assignment)"
            >
              <div class="space-y-1">
                <div class="flex flex-wrap items-center gap-2">
                  <span class="text-sm font-semibold text-foreground">{{ assignmentUserMap.get(assignment.userId) ?? assignment.userId }}</span>
                  <UiBadge :label="assignment.isPrimary ? '主归属' : '附属归属'" subtle />
                </div>
                <div class="text-xs text-muted-foreground">{{ orgUnitMap.get(assignment.orgUnitId) ?? assignment.orgUnitId }}</div>
                <div class="text-xs text-muted-foreground">
                  {{ assignmentPositionLabels(assignment).join('、') || '未绑定岗位' }}
                </div>
              </div>
            </button>
          </div>
          <UiEmptyState v-else title="暂无归属记录" description="当前筛选条件下没有归属记录。" />
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedAssignment" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">
                {{ assignmentUserMap.get(selectedAssignment.userId) ?? selectedAssignment.userId }}
              </div>
              <UiBadge :label="selectedAssignment.isPrimary ? '主归属' : '附属归属'" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">
              {{ orgUnitMap.get(selectedAssignment.orgUnitId) ?? selectedAssignment.orgUnitId }}
            </div>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField label="用户">
              <UiSelect v-model="assignmentForm.userId" :options="userOptions" data-testid="access-control-assignment-user" />
            </UiField>
            <UiField label="部门">
              <UiSelect v-model="assignmentForm.orgUnitId" :options="orgUnitOptions" data-testid="access-control-assignment-org-unit" />
            </UiField>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField label="岗位">
              <div class="grid gap-2 rounded-[var(--radius-m)] border border-border bg-card p-3">
                <UiCheckbox
                  v-for="position in accessControlStore.positions"
                  :key="position.id"
                  v-model="assignmentForm.positionIds"
                  :value="position.id"
                >
                  {{ position.name }}
                </UiCheckbox>
                <p v-if="!accessControlStore.positions.length" class="text-xs text-muted-foreground">暂无岗位</p>
              </div>
            </UiField>

            <UiField label="用户组">
              <div class="grid gap-2 rounded-[var(--radius-m)] border border-border bg-card p-3">
                <UiCheckbox
                  v-for="group in accessControlStore.userGroups"
                  :key="group.id"
                  v-model="assignmentForm.userGroupIds"
                  :value="group.id"
                >
                  {{ group.name }}
                </UiCheckbox>
                <p v-if="!accessControlStore.userGroups.length" class="text-xs text-muted-foreground">暂无用户组</p>
              </div>
            </UiField>
          </div>

          <div class="rounded-[var(--radius-m)] border border-border bg-muted/35 p-3">
            <UiCheckbox v-model="assignmentForm.isPrimary">设为主归属</UiCheckbox>
          </div>

          <div class="flex flex-wrap justify-between gap-2">
            <UiButton
              variant="ghost"
              class="text-destructive"
              :loading="deletingAssignmentKey === selectedAssignmentKey"
              @click="deleteAssignment"
            >
              删除归属
            </UiButton>
            <UiButton :loading="savingAssignment" data-testid="access-control-assignment-save" @click="saveAssignment()">
              保存归属
            </UiButton>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <div class="rounded-[var(--radius-l)] border border-border bg-card p-4">
              <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">岗位摘要</div>
              <div class="mt-2 text-sm text-foreground">
                {{ assignmentPositionLabels(selectedAssignment).join('、') || '未设置岗位' }}
              </div>
            </div>
            <div class="rounded-[var(--radius-l)] border border-border bg-card p-4">
              <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">用户组摘要</div>
              <div class="mt-2 text-sm text-foreground">
                {{ assignmentGroupLabels(selectedAssignment).join('、') || '未设置用户组' }}
              </div>
            </div>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>

    <UiDialog
      :open="unitDialogOpen"
      title="新建部门"
      description="创建组织单元并设置层级与状态。"
      @update:open="unitDialogOpen = $event"
    >
      <div class="grid gap-3 md:grid-cols-2">
        <UiField label="部门编码">
          <UiInput v-model="orgUnitForm.code" data-testid="access-control-org-unit-code" />
        </UiField>
        <UiField label="部门名称">
          <UiInput v-model="orgUnitForm.name" data-testid="access-control-org-unit-name" />
        </UiField>
        <UiField label="上级部门">
          <UiSelect v-model="orgUnitForm.parentId" :options="orgUnitParentOptions" data-testid="access-control-org-unit-parent" />
        </UiField>
        <UiField label="状态">
          <UiSelect v-model="orgUnitForm.status" :options="statusOptions" data-testid="access-control-org-unit-status" />
        </UiField>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="unitDialogOpen = false">取消</UiButton>
        <UiButton :loading="savingOrgUnit" data-testid="access-control-org-unit-save" @click="handleCreateUnit">
          创建部门
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="positionDialogOpen"
      title="新建岗位"
      description="创建岗位定义，用于组织归属扩展。"
      @update:open="positionDialogOpen = $event"
    >
      <div class="grid gap-3 md:grid-cols-2">
        <UiField label="岗位编码">
          <UiInput v-model="positionForm.code" />
        </UiField>
        <UiField label="岗位名称">
          <UiInput v-model="positionForm.name" />
        </UiField>
        <UiField label="状态" class="md:col-span-2">
          <UiSelect v-model="positionForm.status" :options="statusOptions" />
        </UiField>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="positionDialogOpen = false">取消</UiButton>
        <UiButton :loading="savingPosition" data-testid="access-control-position-save" @click="handleCreatePosition">
          创建岗位
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="groupDialogOpen"
      title="新建用户组"
      description="创建用户组，用于组织和授权聚合。"
      @update:open="groupDialogOpen = $event"
    >
      <div class="grid gap-3 md:grid-cols-2">
        <UiField label="用户组编码">
          <UiInput v-model="groupForm.code" />
        </UiField>
        <UiField label="用户组名称">
          <UiInput v-model="groupForm.name" />
        </UiField>
        <UiField label="状态" class="md:col-span-2">
          <UiSelect v-model="groupForm.status" :options="statusOptions" />
        </UiField>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="groupDialogOpen = false">取消</UiButton>
        <UiButton :loading="savingGroup" data-testid="access-control-group-save" @click="handleCreateGroup">
          创建用户组
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="assignmentDialogOpen"
      title="新建归属"
      description="为用户设置部门、岗位和用户组归属。"
      @update:open="assignmentDialogOpen = $event"
    >
      <div class="space-y-4">
        <div class="grid gap-3 md:grid-cols-2">
          <UiField label="用户">
            <UiSelect v-model="assignmentForm.userId" :options="userOptions" data-testid="access-control-assignment-user" />
          </UiField>
          <UiField label="部门">
            <UiSelect v-model="assignmentForm.orgUnitId" :options="orgUnitOptions" data-testid="access-control-assignment-org-unit" />
          </UiField>
        </div>

        <div class="grid gap-3 md:grid-cols-2">
          <UiField label="岗位">
            <div class="grid gap-2 rounded-[var(--radius-m)] border border-border bg-card p-3">
              <UiCheckbox
                v-for="position in accessControlStore.positions"
                :key="position.id"
                v-model="assignmentForm.positionIds"
                :value="position.id"
              >
                {{ position.name }}
              </UiCheckbox>
              <p v-if="!accessControlStore.positions.length" class="text-xs text-muted-foreground">暂无岗位</p>
            </div>
          </UiField>

          <UiField label="用户组">
            <div class="grid gap-2 rounded-[var(--radius-m)] border border-border bg-card p-3">
              <UiCheckbox
                v-for="group in accessControlStore.userGroups"
                :key="group.id"
                v-model="assignmentForm.userGroupIds"
                :value="group.id"
              >
                {{ group.name }}
              </UiCheckbox>
              <p v-if="!accessControlStore.userGroups.length" class="text-xs text-muted-foreground">暂无用户组</p>
            </div>
          </UiField>
        </div>

        <UiCheckbox v-model="assignmentForm.isPrimary">设为主归属</UiCheckbox>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="assignmentDialogOpen = false">取消</UiButton>
        <UiButton :loading="savingAssignment" data-testid="access-control-assignment-save" @click="handleCreateAssignment">
          创建归属
        </UiButton>
      </template>
    </UiDialog>
  </div>
</template>
