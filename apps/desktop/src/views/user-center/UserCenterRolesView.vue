<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiSurface } from '@octopus/ui'

import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()

const selectedRoleId = ref<string>('')

const form = reactive({
  name: '',
  code: '',
  description: '',
  status: 'active' as 'active' | 'disabled',
  permissionIds: [] as string[],
  menuIds: [] as string[],
})

function memberCount(roleId: string) {
  return workbench.memberships.filter((membership) =>
    membership.workspaceId === workbench.currentWorkspaceId && membership.roleIds.includes(roleId),
  ).length
}

function applyRole(roleId?: string) {
  if (!roleId) {
    selectedRoleId.value = ''
    form.name = ''
    form.code = ''
    form.description = ''
    form.status = 'active'
    form.permissionIds = []
    form.menuIds = []
    return
  }

  const role = workbench.workspaceRoles.find((item) => item.id === roleId)
  if (!role) {
    applyRole()
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

watch(
  () => [workbench.currentWorkspaceId, workbench.workspaceRoles.map((role) => role.id).join('|')],
  () => {
    if (!selectedRoleId.value || !workbench.workspaceRoles.some((role) => role.id === selectedRoleId.value)) {
      applyRole(workbench.workspaceRoles[0]?.id)
      return
    }

    applyRole(selectedRoleId.value)
  },
  { immediate: true },
)

const availableMenus = computed(() =>
  workbench.workspaceMenus.filter((menu) => menu.source === 'user-center' || !menu.parentId),
)

function saveRole() {
  if (selectedRoleId.value) {
    workbench.updateRole(selectedRoleId.value, {
      name: form.name,
      code: form.code,
      description: form.description,
      status: form.status,
      permissionIds: form.permissionIds,
      menuIds: form.menuIds,
    })
    return
  }

  const role = workbench.createRole({
    name: form.name,
    code: form.code,
    description: form.description,
    status: form.status,
    permissionIds: form.permissionIds,
    menuIds: form.menuIds,
  })
  applyRole(role.id)
}

function removeRole(roleId: string) {
  const removed = workbench.deleteRole(roleId)
  if (!removed) {
    return
  }

  applyRole(workbench.workspaceRoles[0]?.id)
}
</script>

<template>
  <div class="page-layout">
    <UiSurface :title="t('userCenter.roles.title')" :subtitle="t('userCenter.roles.subtitle')">
      <div class="toolbar">
        <div>
          <strong>{{ t('userCenter.roles.listTitle') }}</strong>
          <small>{{ t('userCenter.roles.listSubtitle') }}</small>
        </div>
        <button type="button" class="primary-button" @click="applyRole()">
          {{ t('userCenter.roles.create') }}
        </button>
      </div>

      <div class="role-list">
        <article
          v-for="role in workbench.workspaceRoles"
          :key="role.id"
          class="role-item"
          :class="{ active: selectedRoleId === role.id }"
          @click="applyRole(role.id)"
        >
          <div class="role-header">
            <div>
              <strong>{{ role.name }}</strong>
              <small>{{ role.code }}</small>
            </div>
            <UiBadge :label="role.status" :tone="role.status === 'active' ? 'success' : 'warning'" />
          </div>
          <p>{{ role.description }}</p>
          <div class="role-stats">
            <span>{{ t('userCenter.roles.permissionCount', { count: role.permissionIds.length }) }}</span>
            <span>{{ t('userCenter.roles.menuCount', { count: role.menuIds.length }) }}</span>
            <span>{{ t('userCenter.roles.memberCount', { count: memberCount(role.id) }) }}</span>
          </div>
          <div class="role-actions">
            <button type="button" class="ghost-button" @click.stop="workbench.toggleRoleStatus(role.id)">
              {{ role.status === 'active' ? t('userCenter.roles.disable') : t('userCenter.roles.enable') }}
            </button>
            <button type="button" class="ghost-button" @click.stop="removeRole(role.id)">
              {{ t('userCenter.roles.delete') }}
            </button>
          </div>
        </article>
      </div>
    </UiSurface>

    <UiSurface :title="t(selectedRoleId ? 'userCenter.roles.editTitle' : 'userCenter.roles.createTitle')" :subtitle="t('userCenter.roles.formSubtitle')">
      <div class="form-grid">
        <label>
          <span>{{ t('userCenter.roles.nameLabel') }}</span>
          <input v-model="form.name" />
        </label>
        <label>
          <span>{{ t('userCenter.roles.codeLabel') }}</span>
          <input v-model="form.code" />
        </label>
        <label class="full-width">
          <span>{{ t('userCenter.roles.descriptionLabel') }}</span>
          <textarea v-model="form.description" rows="3" />
        </label>
        <label>
          <span>{{ t('userCenter.common.status') }}</span>
          <select v-model="form.status">
            <option value="active">{{ t('userCenter.common.active') }}</option>
            <option value="disabled">{{ t('userCenter.common.disabled') }}</option>
          </select>
        </label>
      </div>

      <div class="binding-grid">
        <section>
          <strong>{{ t('userCenter.roles.permissionBindingTitle') }}</strong>
          <label v-for="permission in workbench.workspacePermissions" :key="permission.id" class="check-row">
            <input v-model="form.permissionIds" type="checkbox" :value="permission.id">
            <span>{{ permission.name }}</span>
          </label>
        </section>

        <section>
          <strong>{{ t('userCenter.roles.menuBindingTitle') }}</strong>
          <label v-for="menu in availableMenus" :key="menu.id" class="check-row">
            <input v-model="form.menuIds" type="checkbox" :value="menu.id">
            <span>{{ menu.label }}</span>
          </label>
          <p class="helper-text">{{ t('userCenter.roles.menuHint') }}</p>
        </section>
      </div>

      <div class="form-actions">
        <button type="button" class="ghost-button" @click="applyRole(workbench.workspaceRoles[0]?.id)">
          {{ t('common.cancel') }}
        </button>
        <button type="button" class="primary-button" @click="saveRole">
          {{ t('common.save') }}
        </button>
      </div>
    </UiSurface>
  </div>
</template>

<style scoped>
.page-layout,
.toolbar,
.role-actions,
.form-actions {
  display: flex;
}

.page-layout {
  flex-direction: column;
  gap: 1rem;
}

.toolbar {
  align-items: flex-end;
  justify-content: space-between;
  gap: 1rem;
  margin-bottom: 1rem;
}

.toolbar small,
.role-header small,
.role-item p,
.helper-text,
.form-grid label span {
  color: var(--text-secondary);
}

.role-list {
  display: grid;
  gap: 0.85rem;
}

.role-item {
  display: grid;
  gap: 0.7rem;
  padding: 0.9rem;
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-l);
  cursor: pointer;
}

.role-item.active {
  border-color: color-mix(in srgb, var(--brand-primary) 34%, var(--border-subtle));
  background: color-mix(in srgb, var(--brand-primary) 6%, transparent);
}

.role-header,
.role-stats {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.role-item p {
  margin: 0;
}

.role-stats {
  flex-wrap: wrap;
  font-size: 0.92rem;
}

.role-actions {
  gap: 0.5rem;
}

.form-grid,
.binding-grid {
  display: grid;
  gap: 0.85rem 1rem;
}

.form-grid {
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  margin-bottom: 1rem;
}

.form-grid label,
.binding-grid section {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}

.full-width {
  grid-column: 1 / -1;
}

.form-grid input,
.form-grid select,
.form-grid textarea {
  padding: 0.75rem;
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-m);
  background: var(--bg-input);
}

.binding-grid {
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  margin-bottom: 1rem;
}

.check-row {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  margin-top: 0.55rem;
}

.form-actions {
  justify-content: flex-end;
  gap: 0.65rem;
}

.helper-text {
  margin: 0.75rem 0 0;
}
</style>
