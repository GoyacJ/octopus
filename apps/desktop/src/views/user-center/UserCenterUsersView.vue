<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiField,
  UiInput,
  UiRadioGroup,
  UiSelect,
  UiSurface,
} from '@octopus/ui'
import { Plus, UserCheck, UserCog, UserMinus } from 'lucide-vue-next'

import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()

const selectedUserId = ref<string>('')

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

function roleSummary(userId: string) {
  const membership = workbench.memberships.find((item) =>
    item.workspaceId === workbench.currentWorkspaceId && item.userId === userId,
  )
  if (!membership?.roleIds.length) {
    return t('userCenter.common.noRoles')
  }

  return membership.roleIds
    .map((roleId) => workbench.workspaceRoles.find((role) => role.id === roleId)?.name)
    .filter((name): name is string => Boolean(name))
    .join(', ')
}

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
</script>

<template>
  <div class="flex flex-col gap-6">
    <UiSurface :title="t('userCenter.users.title')" :subtitle="t('userCenter.users.subtitle')">
      <div class="flex flex-col gap-4">
        <div class="flex items-center justify-between gap-4">
          <div class="flex flex-col">
            <h3 class="text-base font-bold text-text-primary">{{ t('userCenter.users.listTitle') }}</h3>
            <p class="text-sm text-text-secondary">{{ t('userCenter.users.listSubtitle') }}</p>
          </div>
          <UiButton size="sm" @click="applyForm()">
            <Plus :size="16" />
            {{ t('userCenter.users.create') }}
          </UiButton>
        </div>

        <div class="flex flex-col gap-2">
          <article
            v-for="user in workbench.workspaceUsers"
            :key="user.id"
            class="group flex flex-col justify-between gap-4 rounded-[calc(var(--radius-lg)+1px)] border p-4 transition-all duration-fast ease-apple sm:flex-row sm:items-center"
            :class="[
              selectedUserId === user.id
                ? 'border-primary/20 bg-surface shadow-sm'
                : 'border-border bg-subtle/30 hover:border-border-strong hover:bg-subtle/50'
            ]"
          >
            <div class="flex items-center gap-4 min-w-0 flex-1 cursor-pointer" @click="applyForm(user.id)">
              <div class="flex size-11 shrink-0 items-center justify-center rounded-[calc(var(--radius-m)+2px)] bg-primary/10 text-primary font-bold text-lg">
                {{ user.avatar }}
              </div>
              <div class="flex flex-col min-w-0">
                <div class="flex items-center gap-2">
                  <strong class="text-sm font-semibold text-text-primary truncate">{{ user.nickname }}</strong>
                  <UiBadge :label="user.status" :tone="user.status === 'active' ? 'success' : 'warning'" />
                </div>
                <span class="text-xs text-text-tertiary truncate">{{ user.email }}</span>
                <span class="text-xs text-text-secondary mt-0.5 truncate">{{ roleSummary(user.id) }}</span>
              </div>
            </div>

            <div class="flex items-center gap-2 opacity-0 group-hover:opacity-100 transition-opacity focus-within:opacity-100">
              <UiButton variant="ghost" size="sm" :title="t('userCenter.users.switchCurrentUser')" @click="switchCurrentUser(user.id)">
                <UserCheck :size="16" />
              </UiButton>
              <UiButton variant="ghost" size="sm" :title="user.status === 'active' ? t('userCenter.users.disable') : t('userCenter.users.enable')" @click="workbench.toggleUserStatus(user.id)">
                <UserCog :size="16" />
              </UiButton>
              <UiButton variant="ghost" size="sm" class="text-status-error hover:bg-status-error/10" @click="removeUser(user.id)">
                <UserMinus :size="16" />
              </UiButton>
            </div>
          </article>
        </div>
      </div>
    </UiSurface>

    <UiSurface 
      :title="t(selectedUserId ? 'userCenter.users.editTitle' : 'userCenter.users.createTitle')" 
      :subtitle="t('userCenter.users.formSubtitle')"
    >
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
        <UiField :label="t('userCenter.profile.usernameLabel')">
          <UiInput v-model="form.username" />
        </UiField>
        <UiField :label="t('userCenter.profile.nicknameLabel')">
          <UiInput v-model="form.nickname" />
        </UiField>
        <UiField :label="t('userCenter.profile.phoneLabel')">
          <UiInput v-model="form.phone" />
        </UiField>
        <UiField :label="t('userCenter.profile.emailLabel')">
          <UiInput v-model="form.email" />
        </UiField>
        <UiField :label="t('userCenter.profile.genderLabel')">
          <UiSelect v-model="form.gender" :options="genderOptions" />
        </UiField>
        <UiField :label="t('userCenter.common.status')">
          <UiSelect v-model="form.status" :options="statusOptions" />
        </UiField>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-8 py-4 border-t border-border/60 mt-4">
        <div class="flex flex-col gap-3">
          <h4 class="text-sm font-bold text-text-primary">{{ t('userCenter.users.roleBindingTitle') }}</h4>
          <div class="flex flex-wrap gap-x-6 gap-y-2">
            <UiCheckbox 
              v-for="role in workbench.workspaceRoles" 
              :key="role.id" 
              v-model="form.roleIds" 
              :value="role.id" 
              :label="role.name"
            />
          </div>
        </div>

        <div class="flex flex-col gap-3">
          <h4 class="text-sm font-bold text-text-primary">{{ t('userCenter.users.scopeTitle') }}</h4>
          <UiRadioGroup 
            v-model="form.scopeMode"
            direction="horizontal"
            :options="[
              { value: 'all-projects', label: t('userCenter.scope.allProjects') },
              { value: 'selected-projects', label: t('userCenter.scope.selectedProjects') }
            ]"
          />

          <div v-if="form.scopeMode === 'selected-projects'" class="flex flex-wrap gap-x-6 gap-y-2 pt-2 animate-in fade-in slide-in-from-top-1 duration-fast">
            <UiCheckbox 
              v-for="project in workbench.workspaceProjects" 
              :key="project.id" 
              v-model="form.scopeProjectIds" 
              :value="project.id" 
              :label="project.name"
            />
          </div>
        </div>
      </div>

      <div class="flex justify-end gap-3 pt-6 border-t border-border/60 mt-2">
        <UiButton variant="ghost" @click="applyForm(workbench.workspaceUsers[0]?.id)">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton @click="saveUser">
          {{ t('common.save') }}
        </UiButton>
      </div>
    </UiSurface>
  </div>
</template>