<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import type { RoleRecord } from '@octopus/schema'
import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiDialog,
  UiField,
  UiInput,
  UiPagination,
  UiRecordCard,
  UiSelect,
  UiTextarea,
} from '@octopus/ui'

import { enumLabel } from '@/i18n/copy'
import { getMenuDefinition } from '@/navigation/menuRegistry'
import { useUserCenterStore } from '@/stores/user-center'
import { useWorkspaceStore } from '@/stores/workspace'
import UserCenterMenuTree from './UserCenterMenuTree.vue'
import { buildUserCenterMenuTreeSections } from './menu-tree'

const PAGE_SIZE = 6

const { t, locale } = useI18n()
const userCenterStore = useUserCenterStore()
const workspaceStore = useWorkspaceStore()

const selectedRoleId = ref('')
const currentPage = ref(1)
const saveMessage = ref('')
const deleteDialogOpen = ref(false)
const pendingDeleteRoleId = ref('')
const form = reactive({
  name: '',
  code: '',
  description: '',
  status: 'active',
  permissionIds: [] as string[],
  menuIds: [] as string[],
})

const statusOptions = computed(() => {
  locale.value
  return [
    { value: 'active', label: enumLabel('recordStatus', 'active') },
    { value: 'disabled', label: enumLabel('recordStatus', 'disabled') },
  ]
})

const metrics = computed(() => [
  { id: 'total', label: t('userCenter.roles.metrics.total'), value: String(userCenterStore.roles.length) },
  { id: 'disabled', label: t('userCenter.roles.metrics.disabled'), value: String(userCenterStore.roles.filter(role => role.status === 'disabled').length) },
])

const permissionOptions = computed(() => userCenterStore.permissions)
const menuOptions = computed(() => userCenterStore.menus.filter(menu => menu.status === 'active'))
const menuTreeSections = computed(() => buildUserCenterMenuTreeSections(
  menuOptions.value,
  {
    app: t('userCenter.roles.menuGroups.app'),
    workspace: t('userCenter.roles.menuGroups.workspace'),
    userCenter: t('userCenter.roles.menuGroups.userCenter'),
    project: t('userCenter.roles.menuGroups.project'),
  },
  menu => menuLabel(menu.id, menu.label),
))
const pageCount = computed(() => Math.max(1, Math.ceil(userCenterStore.roles.length / PAGE_SIZE)))
const pagedRoles = computed(() => {
  const start = (currentPage.value - 1) * PAGE_SIZE
  return userCenterStore.roles.slice(start, start + PAGE_SIZE)
})

watch(
  () => userCenterStore.roles.map(role => role.id).join('|'),
  () => {
    if (currentPage.value > pageCount.value) {
      currentPage.value = pageCount.value
    }
    if (!selectedRoleId.value || !userCenterStore.roles.some(role => role.id === selectedRoleId.value)) {
      applyRole(userCenterStore.roles[0]?.id)
      return
    }
    applyRole(selectedRoleId.value)
  },
  { immediate: true },
)

function resetFormState() {
  form.name = ''
  form.code = ''
  form.description = ''
  form.status = 'active'
  form.permissionIds = []
  form.menuIds = []
}

function applyRole(roleId?: string) {
  const role = userCenterStore.roles.find(item => item.id === roleId)
  if (!role) {
    selectedRoleId.value = ''
    resetFormState()
    return
  }

  selectedRoleId.value = role.id
  form.name = role.name
  form.code = role.code
  form.description = role.description
  form.status = role.status
  form.permissionIds = [...role.permissionIds]
  form.menuIds = [...role.menuIds]
}

function createRoleDraft() {
  selectedRoleId.value = ''
  resetFormState()
}

async function saveRole() {
  if (!workspaceStore.currentWorkspaceId || !form.name.trim() || !form.code.trim()) {
    return
  }

  const record: RoleRecord = {
    id: selectedRoleId.value || `role-${Date.now()}`,
    workspaceId: workspaceStore.currentWorkspaceId,
    name: form.name.trim(),
    code: form.code.trim(),
    description: form.description.trim(),
    status: form.status as RoleRecord['status'],
    permissionIds: [...form.permissionIds],
    menuIds: [...form.menuIds],
  }

  if (selectedRoleId.value) {
    const updated = await userCenterStore.updateRole(selectedRoleId.value, record)
    applyRole(updated.id)
    saveMessage.value = t('userCenter.roles.feedback.saved')
    return
  }

  const created = await userCenterStore.createRole(record)
  selectedRoleId.value = created.id
  applyRole(created.id)
  saveMessage.value = t('userCenter.roles.feedback.saved')
}

function promptDeleteRole(roleId: string) {
  pendingDeleteRoleId.value = roleId
  deleteDialogOpen.value = true
}

async function confirmDeleteRole() {
  if (!pendingDeleteRoleId.value) {
    return
  }
  await userCenterStore.deleteRole(pendingDeleteRoleId.value)
  deleteDialogOpen.value = false
  pendingDeleteRoleId.value = ''
  applyRole(userCenterStore.roles[0]?.id)
  saveMessage.value = t('userCenter.roles.feedback.deleted')
}

function menuLabel(menuId: string, fallback: string) {
  const definition = getMenuDefinition(menuId)
  return definition ? t(definition.labelKey) : fallback
}
</script>

<template>
  <div class="grid gap-6 xl:grid-cols-[minmax(0,1fr)_360px]">
    <section class="space-y-3">
      <div class="grid gap-3 md:grid-cols-2">
        <div v-for="metric in metrics" :key="metric.id" class="rounded-xl border border-border-subtle p-4 dark:border-white/[0.05]">
          <div class="text-xs text-text-secondary">{{ metric.label }}</div>
          <div class="mt-2 text-2xl font-semibold text-text-primary">{{ metric.value }}</div>
        </div>
      </div>

      <div class="flex items-center justify-between gap-3 rounded-xl border border-border-subtle p-4 dark:border-white/[0.05]">
        <div>
          <div class="text-sm font-semibold text-text-primary">{{ t('userCenter.roles.title') }}</div>
          <div class="text-xs text-text-secondary">{{ t('userCenter.roles.listSubtitle') }}</div>
        </div>
        <UiButton data-testid="roles-create-button" @click="createRoleDraft">
          {{ t('userCenter.roles.actions.create') }}
        </UiButton>
      </div>

      <UiRecordCard
        v-for="role in pagedRoles"
        :key="role.id"
        :title="role.name"
        :description="role.description"
        interactive
        class="cursor-pointer"
        :class="selectedRoleId === role.id ? 'ring-1 ring-primary' : ''"
        @click="applyRole(role.id)"
      >
        <template #badges>
          <UiBadge :label="enumLabel('recordStatus', role.status)" subtle />
          <UiBadge :label="role.code" subtle />
          <UiButton
            variant="destructive"
            size="sm"
            :data-testid="`roles-delete-button-${role.code}`"
            @click.stop="promptDeleteRole(role.id)"
          >
            {{ t('userCenter.roles.actions.delete') }}
          </UiButton>
        </template>
      </UiRecordCard>

      <UiPagination
        v-model:page="currentPage"
        :page-count="pageCount"
        :summary-label="`${userCenterStore.roles.length}`"
        root-test-id="roles-list-pagination"
      />
    </section>

    <section class="space-y-4 rounded-xl border border-border-subtle p-5 dark:border-white/[0.05]">
      <div v-if="saveMessage" class="rounded-md border border-primary/20 bg-primary/5 px-3 py-2 text-[12px] text-primary">
        {{ saveMessage }}
      </div>

      <UiField :label="t('userCenter.roles.fields.name')">
        <UiInput v-model="form.name" data-testid="roles-name-input" />
      </UiField>
      <UiField :label="t('userCenter.roles.fields.code')">
        <UiInput v-model="form.code" data-testid="roles-code-input" />
      </UiField>
      <UiField :label="t('common.status')">
        <UiSelect v-model="form.status" :options="statusOptions" />
      </UiField>
      <UiField :label="t('userCenter.roles.fields.permissions')">
        <div class="space-y-2">
          <label
            v-for="permission in permissionOptions"
            :key="permission.id"
            class="block rounded-md border border-border/40 p-3"
          >
            <UiCheckbox
              v-model="form.permissionIds"
              :value="permission.id"
              :data-testid="`roles-permission-${permission.id}`"
            >
              <span class="font-medium text-text-primary">{{ permission.name }}</span>
              <span class="ml-2 text-xs text-text-tertiary">{{ permission.code }}</span>
            </UiCheckbox>
            <div class="mt-1 text-xs text-text-secondary">
              {{ permission.description }}
            </div>
          </label>
        </div>
      </UiField>
      <UiField :label="t('userCenter.roles.fields.menus')" :hint="t('userCenter.roles.menuHint')">
        <UserCenterMenuTree
          v-model="form.menuIds"
          selection-mode="multiple"
          test-id-prefix="roles-menu"
          :sections="menuTreeSections"
        />
      </UiField>
      <UiField :label="t('userCenter.roles.fields.description')">
        <UiTextarea v-model="form.description" :rows="5" />
      </UiField>
      <div class="flex gap-3">
        <UiButton data-testid="roles-save-button" @click="saveRole">{{ t('userCenter.roles.actions.save') }}</UiButton>
        <UiButton variant="ghost" @click="selectedRoleId ? applyRole(selectedRoleId) : createRoleDraft()">{{ t('userCenter.roles.actions.reset') }}</UiButton>
      </div>
    </section>
  </div>

  <UiDialog
    v-model:open="deleteDialogOpen"
    :title="t('userCenter.roles.deleteTitle')"
    :description="t('userCenter.roles.deleteDescription')"
  >
    <template #footer>
      <UiButton variant="ghost" @click="deleteDialogOpen = false">
        {{ t('common.cancel') }}
      </UiButton>
      <UiButton data-testid="roles-delete-confirm-button" @click="confirmDeleteRole">
        {{ t('common.confirm') }}
      </UiButton>
    </template>
  </UiDialog>
</template>
