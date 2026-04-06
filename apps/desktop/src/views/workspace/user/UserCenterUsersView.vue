<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Plus, UserCheck, UserCog, UserMinus } from 'lucide-vue-next'

import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiField,
  UiInput,
  UiMetricCard,
  UiRadioGroup,
  UiRecordCard,
  UiSelect,
  UiSurface,
  UiTextarea,
  UiToolbarRow,
} from '@octopus/ui'

import { formatDateTime } from '@/i18n/copy'
import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()

const selectedUserId = ref<string>('')
const searchQuery = ref('')
const statusFilter = ref<'all' | 'active' | 'disabled'>('all')
const scopeFilter = ref<'all' | 'all-projects' | 'selected-projects'>('all')

const form = reactive({
  username: '',
  nickname: '',
  gender: 'unknown' as 'male' | 'female' | 'unknown',
  phone: '',
  email: '',
  status: 'active' as 'active' | 'disabled',
  scopeMode: 'all-projects' as 'all-projects' | 'selected-projects',
  scopeProjectIds: [] as string[],
  roleIds: [] as string[],
})

const normalizedSearch = computed(() => searchQuery.value.trim().toLowerCase())
const userItems = computed(() => workbench.workspaceUserListItems)
const canManageUsers = computed(() =>
  workbench.hasPermission('user:manage:update', 'update'),
)

const filteredUsers = computed(() =>
  userItems.value.filter((user) => {
    if (statusFilter.value !== 'all' && user.status !== statusFilter.value) {
      return false
    }
    if (scopeFilter.value !== 'all') {
      const isAllProjects = user.scopeSummary === t('userCenter.scope.allProjects')
      if (scopeFilter.value === 'all-projects' && !isAllProjects) {
        return false
      }
      if (scopeFilter.value === 'selected-projects' && isAllProjects) {
        return false
      }
    }
    if (!normalizedSearch.value) {
      return true
    }
    return [user.nickname, user.username, user.email, user.roleSummary]
      .join(' ')
      .toLowerCase()
      .includes(normalizedSearch.value)
  }),
)

function resolveMetricTone(hasWarning: boolean): 'default' | 'warning' {
  return hasWarning ? 'warning' : 'default'
}

const summaryMetrics = computed(() => {
  const disabledCount = userItems.value.filter((user) => user.status === 'disabled').length
  const selectedScopeCount = userItems.value.filter((user) => user.scopeSummary !== t('userCenter.scope.allProjects')).length
  const noRoleCount = userItems.value.filter((user) => !user.roleNames.length).length
  return [
    {
      id: 'users',
      label: t('userCenter.users.metrics.total'),
      value: String(userItems.value.length),
      helper: t('userCenter.users.metrics.activeHelper', { count: userItems.value.length - disabledCount }),
      tone: 'accent' as const,
    },
    {
      id: 'disabled',
      label: t('userCenter.users.metrics.disabled'),
      value: String(disabledCount),
      helper: t('userCenter.users.metrics.scopeHelper', { count: selectedScopeCount }),
      tone: resolveMetricTone(disabledCount > 0),
    },
    {
      id: 'unassigned',
      label: t('userCenter.users.metrics.unassigned'),
      value: String(noRoleCount),
      helper: t('userCenter.users.metrics.unassignedHelper'),
      tone: resolveMetricTone(noRoleCount > 0),
    },
  ]
})

function applyForm(userId?: string) {
  if (!userId) {
    selectedUserId.value = ''
    form.username = ''
    form.nickname = ''
    form.gender = 'unknown'
    form.phone = ''
    form.email = ''
    form.status = 'active'
    form.scopeMode = 'all-projects'
    form.scopeProjectIds = []
    form.roleIds = []
    return
  }

  const user = workbench.workspaceUsers.find((item) => item.id === userId)
  const membership = workbench.memberships.find((item) =>
    item.workspaceId === workbench.currentWorkspaceId && item.userId === userId,
  )
  if (!user) {
    applyForm()
    return
  }

  selectedUserId.value = user.id
  form.username = user.username
  form.nickname = user.nickname
  form.gender = user.gender
  form.phone = user.phone
  form.email = user.email
  form.status = user.status
  form.scopeMode = membership?.scopeMode ?? 'all-projects'
  form.scopeProjectIds = [...(membership?.scopeProjectIds ?? [])]
  form.roleIds = [...(membership?.roleIds ?? [])]
}

watch(
  () => [workbench.currentWorkspaceId, workbench.workspaceUsers.map((user) => user.id).join('|')],
  () => {
    if (!selectedUserId.value || !workbench.workspaceUsers.some((user) => user.id === selectedUserId.value)) {
      applyForm(workbench.workspaceUsers[0]?.id)
      return
    }
    applyForm(selectedUserId.value)
  },
  { immediate: true },
)

const selectedUserSummary = computed(() =>
  userItems.value.find((user) => user.id === selectedUserId.value),
)

function saveUser() {
  if (selectedUserId.value) {
    workbench.updateUser(selectedUserId.value, {
      username: form.username.trim(),
      nickname: form.nickname.trim(),
      gender: form.gender,
      phone: form.phone.trim(),
      email: form.email.trim(),
      status: form.status,
    })
    workbench.setUserRoles(selectedUserId.value, form.roleIds, workbench.currentWorkspaceId)
    workbench.setMembershipScope(selectedUserId.value, form.scopeMode, form.scopeProjectIds, workbench.currentWorkspaceId)
    return
  }

  const user = workbench.createUser({
    workspaceId: workbench.currentWorkspaceId,
    username: form.username.trim(),
    nickname: form.nickname.trim(),
    gender: form.gender,
    phone: form.phone.trim(),
    email: form.email.trim(),
    status: form.status,
    roleIds: form.roleIds,
    scopeMode: form.scopeMode,
    scopeProjectIds: form.scopeProjectIds,
  })
  applyForm(user.id)
}

function removeUser(userId: string) {
  if (confirm(t('common.confirmDelete'))) {
    workbench.deleteUser(userId)
    applyForm(workbench.workspaceUsers[0]?.id)
  }
}

function switchCurrentUser(userId: string) {
  workbench.switchCurrentUser(userId)
}

const genderOptions = [
  { value: 'unknown', label: t('userCenter.gender.unknown') },
  { value: 'male', label: t('userCenter.gender.male') },
  { value: 'female', label: t('userCenter.gender.female') },
]

const statusOptions = [
  { value: 'active', label: t('userCenter.common.active') },
  { value: 'disabled', label: t('userCenter.common.disabled') },
]

const filterStatusOptions = computed(() => [
  { value: 'all', label: t('userCenter.filters.allStatuses') },
  { value: 'active', label: t('userCenter.common.active') },
  { value: 'disabled', label: t('userCenter.common.disabled') },
])

const filterScopeOptions = computed(() => [
  { value: 'all', label: t('userCenter.filters.allScopes') },
  { value: 'all-projects', label: t('userCenter.scope.allProjects') },
  { value: 'selected-projects', label: t('userCenter.scope.selectedProjects') },
])
</script>

<template>
  <section class="section-stack">
    <div class="grid gap-4 md:grid-cols-3">
      <UiMetricCard
        v-for="metric in summaryMetrics"
        :key="metric.id"
        :data-testid="metric.id === 'users' ? 'user-center-metric-users' : undefined"
        :label="metric.label"
        :value="metric.value"
        :helper="metric.helper"
        :tone="metric.tone"
      />
    </div>

    <UiSurface :title="t('userCenter.users.title')" :subtitle="t('userCenter.users.subtitle')">
      <UiToolbarRow class="mb-4">
        <template #search>
          <UiField :label="t('common.search')">
            <UiInput v-model="searchQuery" :placeholder="t('userCenter.users.searchPlaceholder')" />
          </UiField>
        </template>
        <template #filters>
          <UiField :label="t('userCenter.filters.status')">
            <UiSelect v-model="statusFilter" :options="filterStatusOptions" />
          </UiField>
          <UiField :label="t('userCenter.filters.scope')">
            <UiSelect v-model="scopeFilter" :options="filterScopeOptions" />
          </UiField>
        </template>
        <template #actions>
          <UiButton v-if="canManageUsers" size="sm" @click="applyForm()">
            <Plus :size="16" />
            {{ t('userCenter.users.create') }}
          </UiButton>
        </template>
      </UiToolbarRow>

      <div class="grid gap-4 xl:grid-cols-[minmax(22rem,30rem)_minmax(0,1fr)]">
        <div class="space-y-3">
          <UiRecordCard
            v-for="user in filteredUsers"
            :key="user.id"
            :title="user.nickname"
            :description="user.email"
            :active="selectedUserId === user.id"
            interactive
            @click="applyForm(user.id)"
          >
            <template #eyebrow>{{ user.username }}</template>
            <template #badges>
              <UiBadge v-if="user.isCurrentUser" :label="t('userCenter.users.currentSessionUser')" tone="info" />
              <UiBadge :label="user.status" :tone="user.status === 'active' ? 'success' : 'warning'" />
            </template>
            <template #meta>
              <UiBadge :label="user.roleSummary" subtle />
              <UiBadge :label="user.scopeSummary" subtle />
              <span>{{ t('userCenter.users.effectivePermissions', { count: user.effectivePermissionCount }) }}</span>
              <span>{{ t('userCenter.users.effectiveMenus', { count: user.effectiveMenuCount }) }}</span>
              <span>{{ formatDateTime(user.lastActivityAt) }}</span>
            </template>
            <template #actions>
              <UiButton v-if="canManageUsers" variant="ghost" size="sm" :title="t('userCenter.users.switchCurrentUser')" :data-testid="`user-switch-current-user-${user.id}`" @click.stop="switchCurrentUser(user.id)">
                <UserCheck :size="16" />
              </UiButton>
              <UiButton v-if="canManageUsers" variant="ghost" size="sm" :title="user.status === 'active' ? t('userCenter.users.disable') : t('userCenter.users.enable')" @click.stop="workbench.toggleUserStatus(user.id)">
                <UserCog :size="16" />
              </UiButton>
              <UiButton v-if="canManageUsers" variant="ghost" size="sm" @click.stop="removeUser(user.id)">
                <UserMinus :size="16" />
              </UiButton>
            </template>
          </UiRecordCard>
        </div>

        <UiSurface
          variant="subtle"
          :title="t(selectedUserId ? 'userCenter.users.editTitle' : 'userCenter.users.createTitle')"
          :subtitle="t('userCenter.users.formSubtitle')"
        >
          <div v-if="selectedUserSummary" class="mb-4 flex flex-wrap items-center gap-2">
            <UiBadge :label="selectedUserSummary.roleSummary" subtle />
            <UiBadge :label="t('userCenter.users.effectivePermissions', { count: selectedUserSummary.effectivePermissionCount })" subtle />
            <UiBadge :label="t('userCenter.users.effectiveMenus', { count: selectedUserSummary.effectiveMenuCount })" subtle />
          </div>

          <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
            <UiField :label="t('userCenter.profile.usernameLabel')">
              <UiInput v-model="form.username" :disabled="!canManageUsers" />
            </UiField>
            <UiField :label="t('userCenter.profile.nicknameLabel')">
              <UiInput v-model="form.nickname" :disabled="!canManageUsers" />
            </UiField>
            <UiField :label="t('userCenter.profile.phoneLabel')">
              <UiInput v-model="form.phone" :disabled="!canManageUsers" />
            </UiField>
            <UiField :label="t('userCenter.profile.emailLabel')">
              <UiInput v-model="form.email" :disabled="!canManageUsers" />
            </UiField>
            <UiField :label="t('userCenter.profile.genderLabel')">
              <UiSelect v-model="form.gender" :options="genderOptions" :disabled="!canManageUsers" />
            </UiField>
            <UiField :label="t('userCenter.common.status')">
              <UiSelect v-model="form.status" :options="statusOptions" :disabled="!canManageUsers" />
            </UiField>
          </div>

          <div class="mt-4 grid gap-4 xl:grid-cols-2">
            <UiSurface variant="subtle" padding="sm" :title="t('userCenter.users.roleBindingTitle')">
              <div class="mb-3 flex items-center justify-between">
                <UiBadge :label="String(form.roleIds.length)" subtle />
              </div>
              <div class="space-y-2">
                <UiCheckbox
                  v-for="role in workbench.workspaceRoles"
                  :key="role.id"
                  v-model="form.roleIds"
                  :value="role.id"
                  :label="role.name"
                  :disabled="!canManageUsers"
                />
              </div>
            </UiSurface>

            <UiSurface variant="subtle" padding="sm" :title="t('userCenter.users.scopeTitle')">
              <UiRadioGroup
                v-model="form.scopeMode"
                direction="horizontal"
                :options="[
                  { value: 'all-projects', label: t('userCenter.scope.allProjects') },
                  { value: 'selected-projects', label: t('userCenter.scope.selectedProjects') }
                ]"
                :disabled="!canManageUsers"
              />
              <div v-if="form.scopeMode === 'selected-projects'" class="mt-4 space-y-2">
                <UiCheckbox
                  v-for="project in workbench.workspaceProjects"
                  :key="project.id"
                  v-model="form.scopeProjectIds"
                  :value="project.id"
                  :label="project.name"
                  :disabled="!canManageUsers"
                />
              </div>
            </UiSurface>
          </div>

          <div class="mt-4 flex flex-wrap justify-end gap-3">
            <UiButton variant="ghost" @click="applyForm(workbench.workspaceUsers[0]?.id)">
              {{ t('common.cancel') }}
            </UiButton>
            <UiButton v-if="canManageUsers" @click="saveUser">
              {{ t('common.save') }}
            </UiButton>
          </div>
        </UiSurface>
      </div>
    </UiSurface>
  </section>
</template>
