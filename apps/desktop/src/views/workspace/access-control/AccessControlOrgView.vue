<script setup lang="ts">
import { computed, reactive, ref } from 'vue'

import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiEmptyState,
  UiField,
  UiInput,
  UiPanelFrame,
  UiSelect,
  UiStatTile,
  UiStatusCallout,
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

const submitError = ref('')

const editingOrgUnitId = ref('')
const savingOrgUnit = ref(false)
const deletingOrgUnitId = ref('')
const orgUnitForm = reactive({
  code: '',
  name: '',
  parentId: '',
  status: 'active',
})

const editingPositionId = ref('')
const savingPosition = ref(false)
const deletingPositionId = ref('')
const positionForm = reactive({
  code: '',
  name: '',
  status: 'active',
})

const editingGroupId = ref('')
const savingGroup = ref(false)
const deletingGroupId = ref('')
const groupForm = reactive({
  code: '',
  name: '',
  status: 'active',
})

const editingAssignmentKey = ref('')
const savingAssignment = ref(false)
const deletingAssignmentKey = ref('')
const assignmentForm = reactive({
  userId: '',
  orgUnitId: '',
  isPrimary: true,
  positionIds: [] as string[],
  userGroupIds: [] as string[],
})

const userOptions = computed(() => accessControlStore.users.map(user => ({
  label: user.displayName,
  value: user.id,
})))

const orgUnitParentOptions = computed(() => [
  { label: '无上级部门', value: '' },
  ...accessControlStore.orgUnits.map(unit => ({ label: unit.name, value: unit.id })),
])

const assignmentUserMap = computed(() => new Map(accessControlStore.users.map(user => [user.id, user.displayName])))
const assignmentUnitMap = computed(() => new Map(accessControlStore.orgUnits.map(unit => [unit.id, unit.name])))

function resetOrgUnitForm() {
  Object.assign(orgUnitForm, { code: '', name: '', parentId: '', status: 'active' })
  editingOrgUnitId.value = ''
}

function populateOrgUnitForm(unit: OrgUnitRecord) {
  Object.assign(orgUnitForm, {
    code: unit.code,
    name: unit.name,
    parentId: unit.parentId ?? '',
    status: unit.status,
  })
  editingOrgUnitId.value = unit.id
}

async function saveOrgUnit() {
  submitError.value = ''
  if (!orgUnitForm.code.trim() || !orgUnitForm.name.trim()) {
    submitError.value = '请填写完整的部门编码和名称。'
    return
  }

  savingOrgUnit.value = true
  try {
    const payload: OrgUnitUpsertRequest = {
      code: orgUnitForm.code.trim(),
      name: orgUnitForm.name.trim(),
      parentId: orgUnitForm.parentId || undefined,
      status: orgUnitForm.status,
    }
    if (editingOrgUnitId.value) {
      await accessControlStore.updateOrgUnit(editingOrgUnitId.value, payload)
    } else {
      await accessControlStore.createOrgUnit(payload)
    }
    resetOrgUnitForm()
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存部门失败。'
  } finally {
    savingOrgUnit.value = false
  }
}

async function deleteOrgUnit(orgUnitId: string) {
  deletingOrgUnitId.value = orgUnitId
  submitError.value = ''
  try {
    await accessControlStore.deleteOrgUnit(orgUnitId)
    if (editingOrgUnitId.value === orgUnitId) {
      resetOrgUnitForm()
    }
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除部门失败。'
  } finally {
    deletingOrgUnitId.value = ''
  }
}

function resetPositionForm() {
  Object.assign(positionForm, { code: '', name: '', status: 'active' })
  editingPositionId.value = ''
}

function populatePositionForm(position: PositionRecord) {
  Object.assign(positionForm, position)
  editingPositionId.value = position.id
}

async function savePosition() {
  submitError.value = ''
  if (!positionForm.code.trim() || !positionForm.name.trim()) {
    submitError.value = '请填写完整的岗位编码和名称。'
    return
  }

  savingPosition.value = true
  try {
    const payload: PositionUpsertRequest = {
      code: positionForm.code.trim(),
      name: positionForm.name.trim(),
      status: positionForm.status,
    }
    if (editingPositionId.value) {
      await accessControlStore.updatePosition(editingPositionId.value, payload)
    } else {
      await accessControlStore.createPosition(payload)
    }
    resetPositionForm()
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存岗位失败。'
  } finally {
    savingPosition.value = false
  }
}

async function deletePosition(positionId: string) {
  deletingPositionId.value = positionId
  submitError.value = ''
  try {
    await accessControlStore.deletePosition(positionId)
    if (editingPositionId.value === positionId) {
      resetPositionForm()
    }
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除岗位失败。'
  } finally {
    deletingPositionId.value = ''
  }
}

function resetGroupForm() {
  Object.assign(groupForm, { code: '', name: '', status: 'active' })
  editingGroupId.value = ''
}

function populateGroupForm(group: UserGroupRecord) {
  Object.assign(groupForm, group)
  editingGroupId.value = group.id
}

async function saveGroup() {
  submitError.value = ''
  if (!groupForm.code.trim() || !groupForm.name.trim()) {
    submitError.value = '请填写完整的用户组编码和名称。'
    return
  }

  savingGroup.value = true
  try {
    const payload: UserGroupUpsertRequest = {
      code: groupForm.code.trim(),
      name: groupForm.name.trim(),
      status: groupForm.status,
    }
    if (editingGroupId.value) {
      await accessControlStore.updateUserGroup(editingGroupId.value, payload)
    } else {
      await accessControlStore.createUserGroup(payload)
    }
    resetGroupForm()
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存用户组失败。'
  } finally {
    savingGroup.value = false
  }
}

async function deleteGroup(groupId: string) {
  deletingGroupId.value = groupId
  submitError.value = ''
  try {
    await accessControlStore.deleteUserGroup(groupId)
    if (editingGroupId.value === groupId) {
      resetGroupForm()
    }
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除用户组失败。'
  } finally {
    deletingGroupId.value = ''
  }
}

function resetAssignmentForm() {
  Object.assign(assignmentForm, {
    userId: accessControlStore.users[0]?.id ?? '',
    orgUnitId: accessControlStore.orgUnits[0]?.id ?? '',
    isPrimary: true,
    positionIds: [] as string[],
    userGroupIds: [] as string[],
  })
  editingAssignmentKey.value = ''
}

function populateAssignmentForm(assignment: UserOrgAssignmentRecord) {
  Object.assign(assignmentForm, {
    userId: assignment.userId,
    orgUnitId: assignment.orgUnitId,
    isPrimary: assignment.isPrimary,
    positionIds: [...assignment.positionIds],
    userGroupIds: [...assignment.userGroupIds],
  })
  editingAssignmentKey.value = `${assignment.userId}:${assignment.orgUnitId}`
}

async function saveAssignment() {
  submitError.value = ''
  if (!assignmentForm.userId || !assignmentForm.orgUnitId) {
    submitError.value = '请选择用户和部门。'
    return
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
    await accessControlStore.upsertUserOrgAssignment(payload)
    resetAssignmentForm()
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '保存用户归属失败。'
  } finally {
    savingAssignment.value = false
  }
}

async function deleteAssignment(assignment: UserOrgAssignmentRecord) {
  deletingAssignmentKey.value = `${assignment.userId}:${assignment.orgUnitId}`
  submitError.value = ''
  try {
    await accessControlStore.deleteUserOrgAssignment(assignment.userId, assignment.orgUnitId)
    if (editingAssignmentKey.value === deletingAssignmentKey.value) {
      resetAssignmentForm()
    }
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : '删除用户归属失败。'
  } finally {
    deletingAssignmentKey.value = ''
  }
}

resetAssignmentForm()
</script>

<template>
  <div class="space-y-4" data-testid="access-control-org-shell">
    <section class="grid gap-4 md:grid-cols-4">
      <UiStatTile label="部门" :value="String(metrics.units)" />
      <UiStatTile label="岗位" :value="String(metrics.positions)" />
      <UiStatTile label="用户组" :value="String(metrics.groups)" />
      <UiStatTile label="组织归属" :value="String(metrics.assignments)" />
    </section>

    <UiStatusCallout
      v-if="submitError"
      tone="error"
      :description="submitError"
    />

    <div class="grid gap-4 xl:grid-cols-2">
      <UiPanelFrame variant="panel" padding="md" title="部门管理" subtitle="维护部门树和部门基础属性。">
        <div class="space-y-4">
          <div class="grid gap-3 md:grid-cols-2">
            <UiField label="部门编码">
              <UiInput v-model="orgUnitForm.code" data-testid="access-control-org-unit-code" />
            </UiField>
            <UiField label="部门名称">
              <UiInput v-model="orgUnitForm.name" data-testid="access-control-org-unit-name" />
            </UiField>
          </div>
          <div class="grid gap-3 md:grid-cols-2">
            <UiField label="上级部门">
              <UiSelect v-model="orgUnitForm.parentId" :options="orgUnitParentOptions" data-testid="access-control-org-unit-parent" />
            </UiField>
            <UiField label="状态">
              <UiSelect v-model="orgUnitForm.status" :options="statusOptions" data-testid="access-control-org-unit-status" />
            </UiField>
          </div>
          <div class="flex justify-end gap-2">
            <UiButton variant="ghost" @click="resetOrgUnitForm">重置</UiButton>
            <UiButton :loading="savingOrgUnit" data-testid="access-control-org-unit-save" @click="saveOrgUnit">
              {{ editingOrgUnitId ? '保存部门' : '创建部门' }}
            </UiButton>
          </div>

          <div v-if="accessControlStore.orgUnits.length" class="space-y-3">
            <article
              v-for="unit in accessControlStore.orgUnits"
              :key="unit.id"
              class="rounded-[12px] border border-border bg-card p-4"
            >
              <div class="flex items-start justify-between gap-3">
                <div>
                  <h3 class="text-sm font-semibold text-foreground">{{ unit.name }}</h3>
                  <p class="text-xs text-muted-foreground">{{ unit.code }}</p>
                  <p v-if="unit.parentId" class="mt-1 text-xs text-muted-foreground">上级: {{ assignmentUnitMap.get(unit.parentId) ?? unit.parentId }}</p>
                </div>
                <div class="flex gap-2">
                  <UiButton size="sm" variant="ghost" @click="populateOrgUnitForm(unit)">编辑</UiButton>
                  <UiButton
                    v-if="unit.id !== 'org-root'"
                    size="sm"
                    variant="ghost"
                    class="text-destructive"
                    :loading="deletingOrgUnitId === unit.id"
                    @click="deleteOrgUnit(unit.id)"
                  >
                    删除
                  </UiButton>
                </div>
              </div>
            </article>
          </div>
          <UiEmptyState v-else title="暂无组织单元" description="组织结构尚未配置。" />
        </div>
      </UiPanelFrame>

      <UiPanelFrame variant="panel" padding="md" title="岗位与用户组" subtitle="岗位、用户组用于组织归属和角色绑定扩展。">
        <div class="grid gap-4">
          <section class="space-y-3 rounded-[12px] border border-border bg-muted/30 p-4">
            <div class="grid gap-3 md:grid-cols-3">
              <UiField label="岗位编码">
                <UiInput v-model="positionForm.code" />
              </UiField>
              <UiField label="岗位名称">
                <UiInput v-model="positionForm.name" />
              </UiField>
              <UiField label="状态">
                <UiSelect v-model="positionForm.status" :options="statusOptions" />
              </UiField>
            </div>
            <div class="flex justify-end gap-2">
              <UiButton variant="ghost" @click="resetPositionForm">重置</UiButton>
              <UiButton :loading="savingPosition" data-testid="access-control-position-save" @click="savePosition">
                {{ editingPositionId ? '保存岗位' : '创建岗位' }}
              </UiButton>
            </div>
            <div v-if="accessControlStore.positions.length" class="space-y-2">
              <article
                v-for="position in accessControlStore.positions"
                :key="position.id"
                class="flex items-center justify-between rounded-[8px] border border-border bg-card px-3 py-2"
              >
                <div>
                  <div class="text-sm font-medium text-foreground">{{ position.name }}</div>
                  <div class="text-xs text-muted-foreground">{{ position.code }}</div>
                </div>
                <div class="flex gap-2">
                  <UiButton size="sm" variant="ghost" @click="populatePositionForm(position)">编辑</UiButton>
                  <UiButton size="sm" variant="ghost" class="text-destructive" :loading="deletingPositionId === position.id" @click="deletePosition(position.id)">删除</UiButton>
                </div>
              </article>
            </div>
            <p v-else class="text-sm text-muted-foreground">暂无岗位。</p>
          </section>

          <section class="space-y-3 rounded-[12px] border border-border bg-muted/30 p-4">
            <div class="grid gap-3 md:grid-cols-3">
              <UiField label="用户组编码">
                <UiInput v-model="groupForm.code" />
              </UiField>
              <UiField label="用户组名称">
                <UiInput v-model="groupForm.name" />
              </UiField>
              <UiField label="状态">
                <UiSelect v-model="groupForm.status" :options="statusOptions" />
              </UiField>
            </div>
            <div class="flex justify-end gap-2">
              <UiButton variant="ghost" @click="resetGroupForm">重置</UiButton>
              <UiButton :loading="savingGroup" data-testid="access-control-group-save" @click="saveGroup">
                {{ editingGroupId ? '保存用户组' : '创建用户组' }}
              </UiButton>
            </div>
            <div v-if="accessControlStore.userGroups.length" class="space-y-2">
              <article
                v-for="group in accessControlStore.userGroups"
                :key="group.id"
                class="flex items-center justify-between rounded-[8px] border border-border bg-card px-3 py-2"
              >
                <div>
                  <div class="text-sm font-medium text-foreground">{{ group.name }}</div>
                  <div class="text-xs text-muted-foreground">{{ group.code }}</div>
                </div>
                <div class="flex gap-2">
                  <UiButton size="sm" variant="ghost" @click="populateGroupForm(group)">编辑</UiButton>
                  <UiButton size="sm" variant="ghost" class="text-destructive" :loading="deletingGroupId === group.id" @click="deleteGroup(group.id)">删除</UiButton>
                </div>
              </article>
            </div>
            <p v-else class="text-sm text-muted-foreground">暂无用户组。</p>
          </section>
        </div>
      </UiPanelFrame>
    </div>

    <UiPanelFrame variant="panel" padding="md" title="用户组织归属" subtitle="支持主部门、附属部门、岗位、用户组的组合归属。">
      <div class="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,1.2fr)]">
        <div class="space-y-4 rounded-[12px] border border-border bg-muted/30 p-4">
          <div class="grid gap-3 md:grid-cols-2">
            <UiField label="用户">
              <UiSelect v-model="assignmentForm.userId" :options="userOptions" data-testid="access-control-assignment-user" />
            </UiField>
            <UiField label="部门">
              <UiSelect v-model="assignmentForm.orgUnitId" :options="orgUnitParentOptions.filter(option => option.value)" data-testid="access-control-assignment-org-unit" />
            </UiField>
          </div>
          <div class="grid gap-3 md:grid-cols-2">
            <UiField label="岗位">
              <div class="grid gap-2 rounded-[8px] border border-border bg-card p-3">
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
              <div class="grid gap-2 rounded-[8px] border border-border bg-card p-3">
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
          <div class="flex justify-end gap-2">
            <UiButton variant="ghost" @click="resetAssignmentForm">重置</UiButton>
            <UiButton :loading="savingAssignment" data-testid="access-control-assignment-save" @click="saveAssignment">
              {{ editingAssignmentKey ? '保存归属' : '创建归属' }}
            </UiButton>
          </div>
        </div>

        <div v-if="accessControlStore.userOrgAssignments.length" class="space-y-3">
          <article
            v-for="assignment in accessControlStore.userOrgAssignments"
            :key="`${assignment.userId}:${assignment.orgUnitId}`"
            class="rounded-[12px] border border-border bg-card p-4 text-sm"
          >
            <div class="flex flex-wrap items-start justify-between gap-3">
              <div class="space-y-1">
                <div class="font-medium text-foreground">{{ assignmentUserMap.get(assignment.userId) ?? assignment.userId }}</div>
                <div class="text-xs text-muted-foreground">{{ assignmentUnitMap.get(assignment.orgUnitId) ?? assignment.orgUnitId }}</div>
              </div>
              <div class="flex gap-2">
                <UiBadge :label="assignment.isPrimary ? '主归属' : '附属归属'" subtle />
                <UiButton size="sm" variant="ghost" @click="populateAssignmentForm(assignment)">编辑</UiButton>
                <UiButton
                  size="sm"
                  variant="ghost"
                  class="text-destructive"
                  :loading="deletingAssignmentKey === `${assignment.userId}:${assignment.orgUnitId}`"
                  @click="deleteAssignment(assignment)"
                >
                  删除
                </UiButton>
              </div>
            </div>
          </article>
        </div>
        <UiEmptyState v-else title="暂无归属记录" description="用户组织归属还未建立。" />
      </div>
    </UiPanelFrame>
  </div>
</template>
