<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Blocks, Plus, Power, ShieldCheck, Trash2 } from 'lucide-vue-next'

import { UiBadge, UiButton, UiCheckbox, UiField, UiInput, UiRadioGroup, UiSectionHeading, UiSelect, UiSurface, UiTextarea } from '@octopus/ui'

import { useWorkbenchStore } from '@/stores/workbench'

const { t } = useI18n()
const workbench = useWorkbenchStore()

const viewKind = ref<'atomic' | 'bundle'>('atomic')
const selectedPermissionId = ref<string>('')

const form = reactive({
  name: '',
  code: '',
  description: '',
  status: 'active' as 'active' | 'disabled',
  kind: 'atomic' as 'atomic' | 'bundle',
  targetType: 'project' as 'project' | 'agent' | 'tool' | 'skill' | 'mcp',
  targetIds: [] as string[],
  action: 'view',
  memberPermissionIds: [] as string[],
})

const visiblePermissions = computed(() =>
  workbench.workspacePermissions.filter((permission) => permission.kind === viewKind.value),
)

const targetOptions = computed(() => {
  if (form.targetType === 'project') {
    return workbench.workspaceProjects.map((project) => ({ id: project.id, label: project.name }))
  }

  if (form.targetType === 'agent') {
    return workbench.workspaceAgents.map((agent) => ({ id: agent.id, label: agent.name }))
  }

  return workbench.toolCatalogGroups
    .flatMap((group) => group.items)
    .filter((item) =>
      form.targetType === 'tool'
        ? item.kind === 'builtin'
        : item.kind === form.targetType,
    )
    .map((item) => ({ id: item.id, label: item.name }))
})

const selectedPermission = computed(() =>
  workbench.workspacePermissions.find((item) => item.id === selectedPermissionId.value),
)

const statusOptions = computed(() => [
  { value: 'active', label: t('userCenter.common.active') },
  { value: 'disabled', label: t('userCenter.common.disabled') },
])

const kindOptions = computed(() => [
  { value: 'atomic', label: t('userCenter.permissions.atomicTab') },
  { value: 'bundle', label: t('userCenter.permissions.bundleTab') },
])

const targetTypeOptions = computed(() => [
  { value: 'project', label: 'project' },
  { value: 'agent', label: 'agent' },
  { value: 'tool', label: 'tool' },
  { value: 'skill', label: 'skill' },
  { value: 'mcp', label: 'mcp' },
])

function applyPermission(permissionId?: string) {
  if (!permissionId) {
    selectedPermissionId.value = ''
    form.name = ''
    form.code = ''
    form.description = ''
    form.status = 'active'
    form.kind = viewKind.value
    form.targetType = 'project'
    form.targetIds = []
    form.action = 'view'
    form.memberPermissionIds = []
    return
  }

  const permission = workbench.workspacePermissions.find((item) => item.id === permissionId)
  if (!permission) {
    applyPermission()
    return
  }

  selectedPermissionId.value = permission.id
  form.name = permission.name
  form.code = permission.code
  form.description = permission.description
  form.status = permission.status
  form.kind = permission.kind
  form.targetType = permission.targetType ?? 'project'
  form.targetIds = [...(permission.targetIds ?? [])]
  form.action = permission.action ?? 'view'
  form.memberPermissionIds = [...(permission.memberPermissionIds ?? [])]
}

watch(
  () => [viewKind.value, workbench.currentWorkspaceId, workbench.workspacePermissions.map((permission) => permission.id).join('|')],
  () => {
    const currentVisible = visiblePermissions.value
    if (!selectedPermissionId.value || !currentVisible.some((permission) => permission.id === selectedPermissionId.value)) {
      applyPermission(currentVisible[0]?.id)
      return
    }

    applyPermission(selectedPermissionId.value)
  },
  { immediate: true },
)

function savePermission() {
  if (selectedPermissionId.value) {
    workbench.updatePermission(selectedPermissionId.value, {
      name: form.name,
      code: form.code,
      description: form.description,
      status: form.status,
      kind: form.kind,
      targetType: form.kind === 'atomic' ? form.targetType : undefined,
      targetIds: form.kind === 'atomic' ? form.targetIds : undefined,
      action: form.kind === 'atomic' ? form.action : undefined,
      memberPermissionIds: form.kind === 'bundle' ? form.memberPermissionIds : undefined,
    })
    return
  }

  const permission = workbench.createPermission({
    name: form.name,
    code: form.code,
    description: form.description,
    status: form.status,
    kind: form.kind,
    targetType: form.kind === 'atomic' ? form.targetType : undefined,
    targetIds: form.kind === 'atomic' ? form.targetIds : undefined,
    action: form.kind === 'atomic' ? form.action : undefined,
    memberPermissionIds: form.kind === 'bundle' ? form.memberPermissionIds : undefined,
  })
  applyPermission(permission.id)
}
</script>

<template>
  <section class="permission-page section-stack">
    <UiSectionHeading
      :eyebrow="t('userCenter.permissions.title')"
      :title="t('userCenter.permissions.listTitle')"
      :subtitle="t('userCenter.permissions.subtitle')"
    />

    <div class="permission-layout">
      <UiSurface class="permission-list-surface" :title="t('userCenter.permissions.listTitle')" :subtitle="t('userCenter.permissions.listSubtitle')">
        <div class="permission-toolbar">
          <div class="permission-tabs">
            <UiButton variant="ghost" size="sm" :class="viewKind === 'atomic' ? 'is-active' : ''" @click="viewKind = 'atomic'">
              {{ t('userCenter.permissions.atomicTab') }}
            </UiButton>
            <UiButton variant="ghost" size="sm" :class="viewKind === 'bundle' ? 'is-active' : ''" @click="viewKind = 'bundle'">
              {{ t('userCenter.permissions.bundleTab') }}
            </UiButton>
          </div>
          <UiButton size="sm" @click="applyPermission()">
            <Plus :size="16" />
            {{ t('userCenter.permissions.create') }}
          </UiButton>
        </div>

        <div class="permission-list">
          <article
            v-for="permission in visiblePermissions"
            :key="permission.id"
            class="permission-card"
            :class="{ active: selectedPermissionId === permission.id }"
            @click="applyPermission(permission.id)"
          >
            <div class="permission-card-header">
              <div class="permission-card-copy">
                <strong>{{ permission.name }}</strong>
                <small>{{ permission.code }}</small>
              </div>
              <div class="permission-badges">
                <UiBadge :label="permission.kind" subtle />
                <UiBadge :label="permission.status" :tone="permission.status === 'active' ? 'success' : 'warning'" />
              </div>
            </div>
            <p class="permission-card-description">{{ permission.description }}</p>
            <div class="permission-card-meta">
              <span v-if="permission.kind === 'atomic'">
                <ShieldCheck :size="14" /> {{ permission.targetType }} / {{ (permission.targetIds ?? []).join(', ') || t('userCenter.common.empty') }}
              </span>
              <span v-else>
                <Blocks :size="14" /> {{ t('userCenter.permissions.bundleMembers', { count: permission.memberPermissionIds?.length ?? 0 }) }}
              </span>
            </div>
            <div class="permission-card-actions">
              <UiButton variant="ghost" size="sm" @click.stop="workbench.togglePermissionStatus(permission.id)">
                <Power :size="14" />
                {{ permission.status === 'active' ? t('userCenter.permissions.disable') : t('userCenter.permissions.enable') }}
              </UiButton>
              <UiButton variant="ghost" size="sm" class="permission-danger" @click.stop="workbench.deletePermission(permission.id)">
                <Trash2 :size="14" />
                {{ t('userCenter.permissions.delete') }}
              </UiButton>
            </div>
          </article>
        </div>
      </UiSurface>

      <UiSurface
        class="permission-editor-surface"
        :title="t(selectedPermissionId ? 'userCenter.permissions.editTitle' : 'userCenter.permissions.createTitle')"
        :subtitle="t('userCenter.permissions.formSubtitle')"
      >
        <div class="permission-editor-shell">
          <div class="permission-form-grid">
            <UiField :label="t('userCenter.permissions.nameLabel')">
              <UiInput v-model="form.name" />
            </UiField>
            <UiField :label="t('userCenter.permissions.codeLabel')">
              <UiInput v-model="form.code" />
            </UiField>
            <UiField :label="t('userCenter.common.status')">
              <UiSelect v-model="form.status" :options="statusOptions" />
            </UiField>
            <UiField :label="t('userCenter.permissions.kindLabel')">
              <UiSelect v-model="form.kind" :options="kindOptions" />
            </UiField>
            <UiField class="permission-form-full" :label="t('userCenter.permissions.descriptionLabel')">
              <UiTextarea v-model="form.description" :rows="4" />
            </UiField>
          </div>

          <div v-if="form.kind === 'atomic'" class="permission-binding-grid">
            <section class="binding-panel">
              <div class="binding-header">
                <strong>{{ t('userCenter.permissions.targetTypeTitle') }}</strong>
              </div>
              <UiRadioGroup v-model="form.targetType" direction="vertical" :options="targetTypeOptions" />
            </section>

            <section class="binding-panel">
              <div class="binding-header">
                <strong>{{ t('userCenter.permissions.targetBindingTitle') }}</strong>
                <UiBadge :label="String(form.targetIds.length)" subtle />
              </div>
              <div class="binding-list">
                <UiCheckbox
                  v-for="item in targetOptions"
                  :key="item.id"
                  v-model="form.targetIds"
                  :value="item.id"
                  :label="item.label"
                />
              </div>
            </section>

            <section class="binding-panel binding-panel-compact">
              <div class="binding-header">
                <strong>{{ t('userCenter.permissions.actionTitle') }}</strong>
              </div>
              <UiInput v-model="form.action" />
            </section>
          </div>

          <div v-else class="permission-binding-grid permission-binding-grid-single">
            <section class="binding-panel">
              <div class="binding-header">
                <strong>{{ t('userCenter.permissions.bundleComposeTitle') }}</strong>
                <UiBadge :label="String(form.memberPermissionIds.length)" subtle />
              </div>
              <div class="binding-list">
                <UiCheckbox
                  v-for="permission in workbench.workspacePermissions.filter((item) => item.kind === 'atomic')"
                  :key="permission.id"
                  v-model="form.memberPermissionIds"
                  :value="permission.id"
                  :label="permission.name"
                />
              </div>
            </section>
          </div>

          <div class="permission-editor-actions">
            <UiButton variant="ghost" @click="applyPermission(visiblePermissions[0]?.id)">
              {{ t('common.cancel') }}
            </UiButton>
            <UiButton @click="savePermission">
              {{ t('common.save') }}
            </UiButton>
          </div>
        </div>
      </UiSurface>
    </div>
  </section>
</template>

<style scoped>
.permission-page,
.permission-editor-shell,
.permission-card-copy,
.binding-panel,
.binding-list {
  display: flex;
  flex-direction: column;
}

.permission-layout {
  display: grid;
  grid-template-columns: minmax(20rem, 28rem) minmax(0, 1fr);
  gap: 1.1rem;
}

.permission-toolbar,
.permission-tabs,
.permission-card-header,
.permission-badges,
.permission-card-meta,
.permission-card-actions,
.binding-header,
.permission-editor-actions {
  display: flex;
  align-items: center;
}

.permission-toolbar,
.permission-card-header,
.binding-header,
.permission-editor-actions {
  justify-content: space-between;
}

.permission-toolbar,
.permission-card-actions,
.permission-editor-actions {
  gap: 0.6rem;
  flex-wrap: wrap;
}

.permission-tabs .is-active {
  border-color: color-mix(in srgb, var(--brand-primary) 26%, var(--border-strong));
  background: color-mix(in srgb, var(--brand-primary) 10%, transparent);
  color: var(--text-primary);
}

.permission-list {
  display: grid;
  gap: 0.85rem;
}

.permission-card {
  display: grid;
  gap: 0.8rem;
  padding: 1rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 92%, transparent);
  border-radius: calc(var(--radius-lg) + 2px);
  background: color-mix(in srgb, var(--bg-subtle) 68%, transparent);
  cursor: pointer;
  transition: border-color var(--duration-fast) var(--ease-apple), transform var(--duration-fast) var(--ease-apple), box-shadow var(--duration-fast) var(--ease-apple);
}

.permission-card:hover,
.permission-card.active {
  border-color: color-mix(in srgb, var(--brand-primary) 26%, var(--border-strong));
  transform: translateY(-1px);
  box-shadow: var(--shadow-sm);
}

.permission-card-copy {
  gap: 0.2rem;
}

.permission-card-copy small,
.permission-card-description,
.permission-card-meta {
  color: var(--text-secondary);
}

.permission-card-description {
  line-height: 1.6;
}

.permission-card-meta {
  gap: 0.65rem;
  flex-wrap: wrap;
  font-size: 0.8rem;
}

.permission-card-meta span {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
}

.permission-danger {
  color: var(--status-error);
}

.permission-form-grid,
.permission-binding-grid {
  display: grid;
  gap: 1rem;
}

.permission-form-grid {
  grid-template-columns: repeat(4, minmax(0, 1fr));
}

.permission-form-full {
  grid-column: 1 / -1;
}

.permission-binding-grid {
  grid-template-columns: minmax(16rem, 18rem) minmax(0, 1fr) minmax(14rem, 16rem);
}

.permission-binding-grid-single {
  grid-template-columns: minmax(0, 1fr);
}

.binding-panel {
  gap: 0.9rem;
  min-height: 0;
  padding: 1rem;
  border: 1px solid color-mix(in srgb, var(--border-subtle) 90%, transparent);
  border-radius: calc(var(--radius-lg) + 1px);
  background: color-mix(in srgb, var(--bg-subtle) 62%, transparent);
}

.binding-panel-compact {
  justify-content: flex-start;
}

.binding-list {
  gap: 0.7rem;
}

@media (max-width: 1240px) {
  .permission-layout {
    grid-template-columns: minmax(0, 1fr);
  }
}

@media (max-width: 980px) {
  .permission-form-grid,
  .permission-binding-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
