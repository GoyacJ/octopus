<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type {
  AvatarUploadPayload,
  CreateWorkspaceUserRequest,
  UpdateWorkspaceUserRequest,
} from '@octopus/schema'
import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiDialog,
  UiField,
  UiInspectorPanel,
  UiInput,
  UiListDetailShell,
  UiMetricCard,
  UiPanelFrame,
  UiPagination,
  UiRecordCard,
  UiSelect,
  UiStatusCallout,
} from '@octopus/ui'

import { enumLabel } from '@/i18n/copy'
import { useUserCenterStore } from '@/stores/user-center'
import { useWorkspaceStore } from '@/stores/workspace'
import * as tauriClient from '@/tauri/client'

const PAGE_SIZE = 6

const { t, locale } = useI18n()
const userCenterStore = useUserCenterStore()
const workspaceStore = useWorkspaceStore()

const selectedUserId = ref('')
const currentPage = ref(1)
const saveMessage = ref('')
const deleteDialogOpen = ref(false)
const pendingDeleteUserId = ref('')
const form = reactive({
  username: '',
  displayName: '',
  status: 'active',
  roleId: '',
  scopeProjectIds: [] as string[],
  password: '',
  confirmPassword: '',
})
const avatarMode = ref<'keep' | 'default' | 'upload'>('default')
const passwordMode = ref<'keep' | 'default' | 'custom'>('default')
const pendingAvatarUpload = ref<AvatarUploadPayload | null>(null)
const pendingAvatarFileName = ref('')

const statusOptions = computed(() => {
  locale.value
  return [
    { value: 'active', label: enumLabel('recordStatus', 'active') },
    { value: 'disabled', label: enumLabel('recordStatus', 'disabled') },
  ]
})

const roleOptions = computed(() => {
  locale.value
  return [
    { value: '', label: t('userCenter.users.metrics.unassigned') },
    ...userCenterStore.roles.map(role => ({
      value: role.id,
      label: role.name,
    })),
  ]
})

const projectOptions = computed(() => workspaceStore.projects.filter(project => project.status === 'active'))

const metrics = computed(() => [
  { id: 'total', label: t('userCenter.users.metrics.total'), value: String(userCenterStore.users.length) },
  { id: 'disabled', label: t('userCenter.users.metrics.disabled'), value: String(userCenterStore.users.filter(user => user.status === 'disabled').length) },
])

const pageCount = computed(() => Math.max(1, Math.ceil(userCenterStore.users.length / PAGE_SIZE)))
const pagedUsers = computed(() => {
  const start = (currentPage.value - 1) * PAGE_SIZE
  return userCenterStore.users.slice(start, start + PAGE_SIZE)
})
const selectedUser = computed(() => userCenterStore.users.find(user => user.id === selectedUserId.value) ?? null)
const avatarPreview = computed(() => {
  if (avatarMode.value === 'upload' && pendingAvatarUpload.value) {
    return `data:${pendingAvatarUpload.value.contentType};base64,${pendingAvatarUpload.value.dataBase64}`
  }
  if (avatarMode.value === 'default') {
    return ''
  }
  return selectedUser.value?.avatar ?? ''
})
const avatarFallback = computed(() => (form.displayName || form.username || '?').slice(0, 1).toUpperCase())
const avatarFileLabel = computed(() => {
  if (pendingAvatarFileName.value) {
    return pendingAvatarFileName.value
  }
  if (avatarMode.value === 'upload' && !pendingAvatarUpload.value) {
    return t('userCenter.users.avatar.pendingEmpty')
  }
  if (avatarMode.value === 'keep' && selectedUser.value?.avatar) {
    return t('userCenter.users.avatar.current')
  }
  return t('userCenter.users.avatar.pendingEmpty')
})
const isCurrentUserSelected = computed(() => selectedUser.value?.id === userCenterStore.currentUser?.id)

watch(
  () => userCenterStore.users.map(user => user.id).join('|'),
  () => {
    if (currentPage.value > pageCount.value) {
      currentPage.value = pageCount.value
    }
    if (!selectedUserId.value || !userCenterStore.users.some(user => user.id === selectedUserId.value)) {
      applyUser(userCenterStore.users[0]?.id)
      return
    }
    applyUser(selectedUserId.value)
  },
  { immediate: true },
)

function resetFormState() {
  form.username = ''
  form.displayName = ''
  form.status = 'active'
  form.roleId = ''
  form.scopeProjectIds = []
  form.password = ''
  form.confirmPassword = ''
  avatarMode.value = 'default'
  passwordMode.value = 'default'
  pendingAvatarUpload.value = null
  pendingAvatarFileName.value = ''
}

function applyUser(userId?: string) {
  saveMessage.value = ''
  const user = userCenterStore.users.find(item => item.id === userId)
  if (!user) {
    selectedUserId.value = ''
    resetFormState()
    return
  }

  selectedUserId.value = user.id
  form.username = user.username
  form.displayName = user.displayName
  form.status = user.status
  form.roleId = user.roleIds[0] ?? ''
  form.scopeProjectIds = [...user.scopeProjectIds]
  form.password = ''
  form.confirmPassword = ''
  avatarMode.value = user.avatar ? 'keep' : 'default'
  passwordMode.value = 'keep'
  pendingAvatarUpload.value = null
  pendingAvatarFileName.value = ''
}

function createUserDraft() {
  selectedUserId.value = ''
  resetFormState()
}

async function pickAvatar() {
  const picked = await tauriClient.pickAvatarImage()
  if (!picked) {
    return
  }
  pendingAvatarUpload.value = picked
  pendingAvatarFileName.value = picked.fileName
  avatarMode.value = 'upload'
}

async function saveUser() {
  if (!form.username.trim() || !form.displayName.trim()) {
    return
  }

  const baseInput = {
    username: form.username.trim(),
    displayName: form.displayName.trim(),
    status: form.status as 'active' | 'disabled',
    roleIds: form.roleId ? [form.roleId] : [],
    scopeProjectIds: [...form.scopeProjectIds],
  }

  if (selectedUserId.value) {
    const request: UpdateWorkspaceUserRequest = {
      ...baseInput,
      avatar: avatarMode.value === 'upload' ? pendingAvatarUpload.value ?? undefined : undefined,
      removeAvatar: avatarMode.value === 'default' ? true : undefined,
      password: passwordMode.value === 'custom' ? form.password : undefined,
      confirmPassword: passwordMode.value === 'custom' ? form.confirmPassword : undefined,
      resetPasswordToDefault: passwordMode.value === 'default' ? true : undefined,
    }
    const updated = await userCenterStore.updateUser(selectedUserId.value, request)
    saveMessage.value = t('userCenter.users.feedback.saved')
    applyUser(updated.id)
    return
  }

  const request: CreateWorkspaceUserRequest = {
    ...baseInput,
    avatar: avatarMode.value === 'upload' ? pendingAvatarUpload.value ?? undefined : undefined,
    useDefaultAvatar: avatarMode.value !== 'upload' ? true : undefined,
    password: passwordMode.value === 'custom' ? form.password : undefined,
    confirmPassword: passwordMode.value === 'custom' ? form.confirmPassword : undefined,
    useDefaultPassword: passwordMode.value !== 'custom' ? true : undefined,
  }
  const created = await userCenterStore.createUser(request)
  saveMessage.value = t('userCenter.users.feedback.saved')
  selectedUserId.value = created.id
  applyUser(created.id)
}

function promptDeleteUser(userId: string) {
  pendingDeleteUserId.value = userId
  deleteDialogOpen.value = true
}

async function confirmDeleteUser() {
  if (!pendingDeleteUserId.value) {
    return
  }
  await userCenterStore.deleteUser(pendingDeleteUserId.value)
  deleteDialogOpen.value = false
  pendingDeleteUserId.value = ''
  saveMessage.value = t('userCenter.users.feedback.deleted')
  applyUser(userCenterStore.users[0]?.id)
}
</script>

<template>
  <div data-testid="user-center-users-shell">
    <UiListDetailShell>
      <template #list>
        <section class="space-y-3">
      <div class="grid gap-3 md:grid-cols-2">
        <UiMetricCard v-for="metric in metrics" :key="metric.id" :label="metric.label" :value="metric.value" />
      </div>

          <UiPanelFrame variant="subtle" padding="md" :title="t('userCenter.users.title')" :subtitle="t('userCenter.users.subtitle')">
            <template #actions>
              <UiButton data-testid="users-create-button" @click="createUserDraft">
                {{ t('userCenter.users.actions.create') }}
              </UiButton>
            </template>
          </UiPanelFrame>

      <UiRecordCard
        v-for="user in pagedUsers"
        :key="user.id"
        :title="user.displayName"
        :description="user.username"
        interactive
        :active="selectedUserId === user.id"
        @click="applyUser(user.id)"
      >
        <template #leading>
          <div class="flex h-10 w-10 items-center justify-center overflow-hidden rounded-full border border-border/60 bg-accent text-xs font-semibold uppercase text-text-secondary">
            <img v-if="user.avatar" :src="user.avatar" alt="" class="h-full w-full object-cover">
            <span v-else>{{ user.displayName.slice(0, 1) }}</span>
          </div>
        </template>
        <template #badges>
          <UiBadge :label="enumLabel('recordStatus', user.status)" subtle />
          <UiBadge :label="enumLabel('passwordState', user.passwordState)" subtle />
          <UiButton
            v-if="user.id !== userCenterStore.currentUser?.id"
            variant="destructive"
            size="sm"
            :data-testid="`users-delete-button-${user.username}`"
            @click.stop="promptDeleteUser(user.id)"
          >
            {{ t('userCenter.users.actions.delete') }}
          </UiButton>
        </template>
      </UiRecordCard>

      <UiPagination
        v-model:page="currentPage"
        :page-count="pageCount"
        :summary-label="`${userCenterStore.users.length}`"
        root-test-id="users-list-pagination"
      />
        </section>
      </template>

      <template #default>
        <div data-testid="user-center-users-inspector">
          <UiInspectorPanel :title="selectedUserId ? t('userCenter.users.editTitle') : t('userCenter.users.createTitle')">
            <div class="space-y-4">

      <UiStatusCallout v-if="saveMessage" tone="success" :description="saveMessage" />

      <UiField :label="t('userCenter.users.fields.avatar')" :hint="t('userCenter.users.avatar.description')">
        <div class="space-y-3">
          <div class="flex items-center gap-3">
            <div class="flex h-12 w-12 items-center justify-center overflow-hidden rounded-full border border-border/60 bg-accent text-sm font-semibold uppercase text-text-secondary">
              <img v-if="avatarPreview" :src="avatarPreview" alt="" class="h-full w-full object-cover" data-testid="users-avatar-image">
              <span v-else data-testid="users-avatar-fallback">{{ avatarFallback }}</span>
            </div>
            <div class="text-xs text-text-secondary">{{ avatarFileLabel }}</div>
          </div>
          <div class="flex flex-wrap gap-2">
            <UiButton variant="ghost" data-testid="users-avatar-pick-button" @click="pickAvatar">
              {{ t('userCenter.users.actions.pickAvatar') }}
            </UiButton>
            <UiButton
              variant="ghost"
              @click="avatarMode = 'default'; pendingAvatarUpload = null; pendingAvatarFileName = ''"
            >
              {{ t('userCenter.users.actions.clearAvatar') }}
            </UiButton>
          </div>
          <UiCheckbox
            :model-value="avatarMode === 'default'"
            data-testid="users-avatar-mode-default"
            @update:model-value="avatarMode = $event ? 'default' : (selectedUser?.avatar ? 'keep' : 'upload')"
          >
            {{ t('userCenter.users.avatar.actions.useDefault') }}
          </UiCheckbox>
        </div>
      </UiField>

      <UiField :label="t('userCenter.users.fields.username')">
        <UiInput v-model="form.username" data-testid="users-username-input" />
      </UiField>
      <UiField :label="t('userCenter.users.fields.nickname')">
        <UiInput v-model="form.displayName" data-testid="users-display-name-input" />
      </UiField>
      <UiField :label="t('common.status')">
        <UiSelect v-model="form.status" :options="statusOptions" />
      </UiField>
      <UiField :label="t('userCenter.users.fields.role')">
        <UiSelect v-model="form.roleId" :options="roleOptions" data-testid="users-role-select" />
      </UiField>
      <UiField :label="t('userCenter.users.fields.scopeProjects')">
        <div class="space-y-2">
          <UiCheckbox
            v-for="project in projectOptions"
            :key="project.id"
            v-model="form.scopeProjectIds"
            :value="project.id"
            :data-testid="`users-project-scope-${project.id}`"
          >
            {{ project.name }}
          </UiCheckbox>
        </div>
      </UiField>

      <UiField :label="t('userCenter.users.fields.password')" :hint="t('userCenter.users.password.description')">
        <div class="space-y-3">
          <UiCheckbox
            :model-value="passwordMode === 'default'"
            data-testid="users-password-mode-default"
            @update:model-value="passwordMode = $event ? 'default' : 'keep'"
          >
            {{ t('userCenter.users.password.actions.useDefault') }}
          </UiCheckbox>
          <UiCheckbox
            :model-value="passwordMode === 'custom'"
            data-testid="users-password-mode-custom"
            @update:model-value="passwordMode = $event ? 'custom' : (selectedUserId ? 'keep' : 'default')"
          >
            {{ t('userCenter.users.password.actions.custom') }}
          </UiCheckbox>
          <div v-if="passwordMode === 'custom'" class="space-y-3">
            <UiInput v-model="form.password" type="password" :placeholder="t('userCenter.users.password.fields.password')" data-testid="users-password-input" />
            <UiInput v-model="form.confirmPassword" type="password" :placeholder="t('userCenter.users.password.fields.confirmPassword')" data-testid="users-password-confirm-input" />
          </div>
        </div>
      </UiField>

      <div class="flex gap-3">
        <UiButton data-testid="users-save-button" @click="saveUser">{{ t('userCenter.users.actions.save') }}</UiButton>
        <UiButton variant="ghost" @click="selectedUserId ? applyUser(selectedUserId) : createUserDraft()">{{ t('userCenter.users.actions.reset') }}</UiButton>
      </div>

      <div v-if="isCurrentUserSelected" class="text-xs text-text-secondary">
        {{ t('userCenter.users.currentSessionUser') }}
      </div>
            </div>
          </UiInspectorPanel>
        </div>
      </template>
    </UiListDetailShell>
  </div>

  <UiDialog
    v-model:open="deleteDialogOpen"
    :title="t('userCenter.users.deleteTitle')"
    :description="t('userCenter.users.deleteDescription')"
  >
    <template #footer>
      <UiButton variant="ghost" @click="deleteDialogOpen = false">
        {{ t('common.cancel') }}
      </UiButton>
      <UiButton data-testid="users-delete-confirm-button" @click="confirmDeleteUser">
        {{ t('common.confirm') }}
      </UiButton>
    </template>
  </UiDialog>
</template>
