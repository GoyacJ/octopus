<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiSurface } from '@octopus/ui'

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
  <div class="page-layout">
    <UiSurface :title="t('userCenter.permissions.title')" :subtitle="t('userCenter.permissions.subtitle')">
      <div class="toolbar">
        <div>
          <strong>{{ t('userCenter.permissions.listTitle') }}</strong>
          <small>{{ t('userCenter.permissions.listSubtitle') }}</small>
        </div>
        <div class="toolbar-actions">
          <button
            type="button"
            class="ghost-button"
            :class="{ active: viewKind === 'atomic' }"
            @click="viewKind = 'atomic'"
          >
            {{ t('userCenter.permissions.atomicTab') }}
          </button>
          <button
            type="button"
            class="ghost-button"
            :class="{ active: viewKind === 'bundle' }"
            @click="viewKind = 'bundle'"
          >
            {{ t('userCenter.permissions.bundleTab') }}
          </button>
          <button type="button" class="primary-button" @click="applyPermission()">
            {{ t('userCenter.permissions.create') }}
          </button>
        </div>
      </div>

      <div class="permission-list">
        <article
          v-for="permission in visiblePermissions"
          :key="permission.id"
          class="permission-item"
          :class="{ active: selectedPermissionId === permission.id }"
          @click="applyPermission(permission.id)"
        >
          <div class="permission-header">
            <div>
              <strong>{{ permission.name }}</strong>
              <small>{{ permission.code }}</small>
            </div>
            <div class="permission-badges">
              <UiBadge :label="permission.kind" subtle />
              <UiBadge :label="permission.status" :tone="permission.status === 'active' ? 'success' : 'warning'" />
            </div>
          </div>
          <p>{{ permission.description }}</p>
          <div class="permission-targets">
            <span v-if="permission.kind === 'atomic'">{{ permission.targetType }} / {{ (permission.targetIds ?? []).join(', ') || t('userCenter.common.empty') }}</span>
            <span v-else>{{ t('userCenter.permissions.bundleMembers', { count: permission.memberPermissionIds?.length ?? 0 }) }}</span>
          </div>
          <div class="permission-actions">
            <button type="button" class="ghost-button" @click.stop="workbench.togglePermissionStatus(permission.id)">
              {{ permission.status === 'active' ? t('userCenter.permissions.disable') : t('userCenter.permissions.enable') }}
            </button>
            <button type="button" class="ghost-button" @click.stop="workbench.deletePermission(permission.id)">
              {{ t('userCenter.permissions.delete') }}
            </button>
          </div>
        </article>
      </div>
    </UiSurface>

    <UiSurface :title="t(selectedPermissionId ? 'userCenter.permissions.editTitle' : 'userCenter.permissions.createTitle')" :subtitle="t('userCenter.permissions.formSubtitle')">
      <div class="form-grid">
        <label>
          <span>{{ t('userCenter.permissions.nameLabel') }}</span>
          <input v-model="form.name" />
        </label>
        <label>
          <span>{{ t('userCenter.permissions.codeLabel') }}</span>
          <input v-model="form.code" />
        </label>
        <label>
          <span>{{ t('userCenter.common.status') }}</span>
          <select v-model="form.status">
            <option value="active">{{ t('userCenter.common.active') }}</option>
            <option value="disabled">{{ t('userCenter.common.disabled') }}</option>
          </select>
        </label>
        <label>
          <span>{{ t('userCenter.permissions.kindLabel') }}</span>
          <select v-model="form.kind">
            <option value="atomic">{{ t('userCenter.permissions.atomicTab') }}</option>
            <option value="bundle">{{ t('userCenter.permissions.bundleTab') }}</option>
          </select>
        </label>
        <label class="full-width">
          <span>{{ t('userCenter.permissions.descriptionLabel') }}</span>
          <textarea v-model="form.description" rows="3" />
        </label>
      </div>

      <div v-if="form.kind === 'atomic'" class="binding-grid">
        <section>
          <strong>{{ t('userCenter.permissions.targetTypeTitle') }}</strong>
          <label v-for="target in ['project', 'agent', 'tool', 'skill', 'mcp']" :key="target" class="check-row">
            <input v-model="form.targetType" type="radio" :value="target">
            <span>{{ target }}</span>
          </label>
        </section>

        <section>
          <strong>{{ t('userCenter.permissions.targetBindingTitle') }}</strong>
          <label v-for="item in targetOptions" :key="item.id" class="check-row">
            <input v-model="form.targetIds" type="checkbox" :value="item.id">
            <span>{{ item.label }}</span>
          </label>
        </section>

        <section>
          <strong>{{ t('userCenter.permissions.actionTitle') }}</strong>
          <input v-model="form.action">
        </section>
      </div>

      <div v-else class="binding-grid">
        <section>
          <strong>{{ t('userCenter.permissions.bundleComposeTitle') }}</strong>
          <label
            v-for="permission in workbench.workspacePermissions.filter((item) => item.kind === 'atomic')"
            :key="permission.id"
            class="check-row"
          >
            <input v-model="form.memberPermissionIds" type="checkbox" :value="permission.id">
            <span>{{ permission.name }}</span>
          </label>
        </section>
      </div>

      <div class="form-actions">
        <button type="button" class="ghost-button" @click="applyPermission(visiblePermissions[0]?.id)">
          {{ t('common.cancel') }}
        </button>
        <button type="button" class="primary-button" @click="savePermission">
          {{ t('common.save') }}
        </button>
      </div>
    </UiSurface>
  </div>
</template>

<style scoped>
.page-layout,
.toolbar,
.toolbar-actions,
.permission-actions,
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
.permission-header small,
.permission-item p,
.permission-targets,
.form-grid label span {
  color: var(--text-secondary);
}

.toolbar-actions,
.permission-actions,
.form-actions {
  gap: 0.5rem;
}

.permission-list {
  display: grid;
  gap: 0.85rem;
}

.permission-item {
  display: grid;
  gap: 0.7rem;
  padding: 0.9rem;
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-l);
  cursor: pointer;
}

.permission-item.active {
  border-color: color-mix(in srgb, var(--brand-primary) 34%, var(--border-subtle));
  background: color-mix(in srgb, var(--brand-primary) 6%, transparent);
}

.permission-header,
.permission-badges {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.permission-item p {
  margin: 0;
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
.form-grid textarea,
.binding-grid input[type='text'],
.binding-grid section > input {
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
</style>
