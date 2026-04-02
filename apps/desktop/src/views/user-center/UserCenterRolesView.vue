<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Shield, Users, PanelLeftOpen, Plus, Trash2, Power } from 'lucide-vue-next'

import { UiBadge, UiButton, UiCheckbox, UiField, UiInput, UiSectionHeading, UiSelect, UiSurface, UiTextarea } from '@octopus/ui'

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

const statusOptions = computed(() => [
  { value: 'active', label: t('userCenter.common.active') },
  { value: 'disabled', label: t('userCenter.common.disabled') },
])

const selectedRole = computed(() =>
  workbench.workspaceRoles.find((item) => item.id === selectedRoleId.value),
)

const availableMenus = computed(() =>
  workbench.workspaceMenus.filter((menu) => menu.source === 'user-center' || !menu.parentId),
)

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
  <section class="role-page section-stack">
    <UiSectionHeading
      :eyebrow="t('userCenter.roles.title')"
      :title="t('userCenter.roles.listTitle')"
      :subtitle="t('userCenter.roles.subtitle')"
    />

    <div class="role-layout">
      <UiSurface class="role-list-surface" :title="t('userCenter.roles.listTitle')" :subtitle="t('userCenter.roles.listSubtitle')">
        <div class="role-toolbar">
          <UiButton size="sm" @click="applyRole()">
            <Plus :size="16" />
            {{ t('userCenter.roles.create') }}
          </UiButton>
        </div>

        <div class="role-list">
          <article
            v-for="role in workbench.workspaceRoles"
            :key="role.id"
            class="role-card"
            :class="{ active: selectedRoleId === role.id }"
            @click="applyRole(role.id)"
          >
            <div class="role-card-header">
              <div class="role-card-copy">
                <strong>{{ role.name }}</strong>
                <small>{{ role.code }}</small>
              </div>
              <UiBadge :label="role.status" :tone="role.status === 'active' ? 'success' : 'warning'" />
            </div>
            <p class="role-card-description">{{ role.description }}</p>
            <div class="role-card-metrics">
              <span><Shield :size="14" /> {{ t('userCenter.roles.permissionCount', { count: role.permissionIds.length }) }}</span>
              <span><PanelLeftOpen :size="14" /> {{ t('userCenter.roles.menuCount', { count: role.menuIds.length }) }}</span>
              <span><Users :size="14" /> {{ t('userCenter.roles.memberCount', { count: memberCount(role.id) }) }}</span>
            </div>
            <div class="role-card-actions">
              <UiButton variant="ghost" size="sm" @click.stop="workbench.toggleRoleStatus(role.id)">
                <Power :size="14" />
                {{ role.status === 'active' ? t('userCenter.roles.disable') : t('userCenter.roles.enable') }}
              </UiButton>
              <UiButton variant="ghost" size="sm" class="role-danger" @click.stop="removeRole(role.id)">
                <Trash2 :size="14" />
                {{ t('userCenter.roles.delete') }}
              </UiButton>
            </div>
          </article>
        </div>
      </UiSurface>

      <UiSurface
        class="role-editor-surface"
        :title="t(selectedRoleId ? 'userCenter.roles.editTitle' : 'userCenter.roles.createTitle')"
        :subtitle="t('userCenter.roles.formSubtitle')"
      >
        <div class="role-editor-shell">
          <div class="role-form-grid">
            <UiField :label="t('userCenter.roles.nameLabel')">
              <UiInput v-model="form.name" />
            </UiField>
            <UiField :label="t('userCenter.roles.codeLabel')">
              <UiInput v-model="form.code" />
            </UiField>
            <UiField :label="t('userCenter.common.status')">
              <UiSelect v-model="form.status" :options="statusOptions" />
            </UiField>
            <UiField class="role-form-full" :label="t('userCenter.roles.descriptionLabel')">
              <UiTextarea v-model="form.description" :rows="4" />
            </UiField>
          </div>

          <div class="role-binding-grid">
            <section class="binding-panel">
              <div class="binding-header">
                <strong>{{ t('userCenter.roles.permissionBindingTitle') }}</strong>
                <UiBadge :label="String(form.permissionIds.length)" subtle />
              </div>
              <div class="binding-list">
                <UiCheckbox
                  v-for="permission in workbench.workspacePermissions"
                  :key="permission.id"
                  v-model="form.permissionIds"
                  :value="permission.id"
                  :label="permission.name"
                />
              </div>
            </section>

            <section class="binding-panel">
              <div class="binding-header">
                <strong>{{ t('userCenter.roles.menuBindingTitle') }}</strong>
                <UiBadge :label="String(form.menuIds.length)" subtle />
              </div>
              <div class="binding-list">
                <UiCheckbox
                  v-for="menu in availableMenus"
                  :key="menu.id"
                  v-model="form.menuIds"
                  :value="menu.id"
                  :label="menu.label"
                />
              </div>
              <p class="binding-hint">{{ t('userCenter.roles.menuHint') }}</p>
            </section>
          </div>

          <div class="role-editor-actions">
            <UiButton variant="ghost" @click="applyRole(workbench.workspaceRoles[0]?.id)">
              {{ t('common.cancel') }}
            </UiButton>
            <UiButton @click="saveRole">
              {{ t('common.save') }}
            </UiButton>
          </div>
        </div>
      </UiSurface>
    </div>
  </section>
</template>

<style scoped>
.role-page,
.role-editor-shell,
.role-card-copy,
.binding-panel,
.binding-list {
  display: flex;
  flex-direction: column;
}

.role-layout {
  display: grid;
  grid-template-columns: minmax(18rem, 26rem) minmax(0, 1fr);
  gap: 1.1rem;
}

.role-toolbar,
.role-card-header,
.role-card-metrics,
.role-card-actions,
.binding-header,
.role-editor-actions {
  display: flex;
  align-items: center;
}

.role-toolbar,
.role-card-header,
.binding-header,
.role-editor-actions {
  justify-content: space-between;
}

.role-list-surface,
.role-editor-surface {
  min-height: 0;
}

.role-list {
  display: grid;
  gap: 0.85rem;
}

.role-card {
  display: grid;
  gap: 0.8rem;
  padding: 1rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  border-radius: calc(var(--radius-lg) + 2px);
  background: color-mix(in srgb, var(--bg-subtle) 68%, transparent);
  cursor: pointer;
  transition: border-color var(--duration-fast) var(--ease-apple), transform var(--duration-fast) var(--ease-apple), box-shadow var(--duration-fast) var(--ease-apple);
}

.role-card:hover,
.role-card.active {
  border-color: color-mix(in srgb, var(--brand-primary) 26%, var(--border-strong));
  transform: translateY(-1px);
  box-shadow: var(--shadow-sm);
}

.role-card-copy {
  gap: 0.2rem;
}

.role-card-copy small,
.role-card-description,
.binding-hint {
  color: var(--text-secondary);
}

.role-card-description {
  line-height: 1.6;
}

.role-card-metrics {
  gap: 0.85rem;
  flex-wrap: wrap;
  color: var(--text-secondary);
  font-size: 0.8rem;
}

.role-card-metrics span {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
}

.role-card-actions,
.role-editor-actions {
  gap: 0.6rem;
  flex-wrap: wrap;
}

.role-danger {
  color: var(--status-error);
}

.role-form-grid,
.role-binding-grid {
  display: grid;
  gap: 1rem;
}

.role-form-grid {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.role-form-full {
  grid-column: 1 / -1;
}

.role-binding-grid {
  grid-template-columns: repeat(2, minmax(0, 1fr));
}

.binding-panel {
  gap: 0.9rem;
  min-height: 0;
  padding: 1rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 90%, transparent);
  border-radius: calc(var(--radius-lg) + 1px);
  background: color-mix(in srgb, var(--bg-subtle) 62%, transparent);
}

.binding-list {
  gap: 0.7rem;
}

.binding-hint {
  font-size: 0.82rem;
  line-height: 1.55;
}

@media (max-width: 1180px) {
  .role-layout {
    grid-template-columns: minmax(0, 1fr);
  }
}

@media (max-width: 860px) {
  .role-form-grid,
  .role-binding-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
