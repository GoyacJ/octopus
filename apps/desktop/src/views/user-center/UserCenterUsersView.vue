<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiSurface } from '@octopus/ui'

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

const selectedMembership = computed(() =>
  selectedUserId.value
    ? workbench.memberships.find((membership) =>
        membership.workspaceId === workbench.currentWorkspaceId && membership.userId === selectedUserId.value,
      )
    : undefined,
)

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

function scopeLabel(userId: string) {
  const membership = workbench.memberships.find((item) =>
    item.workspaceId === workbench.currentWorkspaceId && item.userId === userId,
  )
  if (!membership) {
    return t('common.na')
  }

  if (membership.scopeMode === 'all-projects') {
    return t('userCenter.scope.allProjects')
  }

  const projectNames = membership.scopeProjectIds
    .map((projectId) => workbench.projects.find((project) => project.id === projectId)?.name)
    .filter((name): name is string => Boolean(name))

  return projectNames.length
    ? `${t('userCenter.scope.selectedProjects')}: ${projectNames.join(', ')}`
    : t('userCenter.scope.selectedProjects')
}

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
  workbench.deleteUser(userId)
  applyForm(workbench.workspaceUsers[0]?.id)
}

function switchCurrentUser(userId: string) {
  workbench.switchCurrentUser(userId)
}
</script>

<template>
  <div class="page-layout">
    <UiSurface :title="t('userCenter.users.title')" :subtitle="t('userCenter.users.subtitle')">
      <div class="toolbar">
        <div>
          <strong>{{ t('userCenter.users.listTitle') }}</strong>
          <small>{{ t('userCenter.users.listSubtitle') }}</small>
        </div>
        <button type="button" class="primary-button" @click="applyForm()">
          {{ t('userCenter.users.create') }}
        </button>
      </div>

      <div class="user-list">
        <article
          v-for="user in workbench.workspaceUsers"
          :key="user.id"
          class="user-item"
          :class="{ active: selectedUserId === user.id }"
        >
          <button type="button" class="user-summary" @click="applyForm(user.id)">
            <div class="user-avatar">{{ user.avatar }}</div>
            <div class="user-copy">
              <strong>{{ user.nickname }}</strong>
              <small>{{ user.email }}</small>
              <span>{{ roleSummary(user.id) }}</span>
            </div>
          </button>

          <div class="user-meta">
            <UiBadge :label="user.status" :tone="user.status === 'active' ? 'success' : 'warning'" />
            <UiBadge :label="scopeLabel(user.id)" subtle />
          </div>

          <div class="user-actions">
            <button
              type="button"
              class="ghost-button"
              :data-testid="`user-switch-current-user-${user.id}`"
              @click="switchCurrentUser(user.id)"
            >
              {{ t('userCenter.users.switchCurrentUser') }}
            </button>
            <button type="button" class="ghost-button" @click="workbench.toggleUserStatus(user.id)">
              {{ user.status === 'active' ? t('userCenter.users.disable') : t('userCenter.users.enable') }}
            </button>
            <button type="button" class="ghost-button" @click="removeUser(user.id)">
              {{ t('userCenter.users.delete') }}
            </button>
          </div>
        </article>
      </div>
    </UiSurface>

    <UiSurface :title="t(selectedUserId ? 'userCenter.users.editTitle' : 'userCenter.users.createTitle')" :subtitle="t('userCenter.users.formSubtitle')">
      <div class="form-grid">
        <label>
          <span>{{ t('userCenter.profile.usernameLabel') }}</span>
          <input v-model="form.username" />
        </label>
        <label>
          <span>{{ t('userCenter.profile.nicknameLabel') }}</span>
          <input v-model="form.nickname" />
        </label>
        <label>
          <span>{{ t('userCenter.profile.phoneLabel') }}</span>
          <input v-model="form.phone" />
        </label>
        <label>
          <span>{{ t('userCenter.profile.emailLabel') }}</span>
          <input v-model="form.email" />
        </label>
        <label>
          <span>{{ t('userCenter.profile.genderLabel') }}</span>
          <select v-model="form.gender">
            <option value="unknown">{{ t('userCenter.gender.unknown') }}</option>
            <option value="male">{{ t('userCenter.gender.male') }}</option>
            <option value="female">{{ t('userCenter.gender.female') }}</option>
          </select>
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
          <strong>{{ t('userCenter.users.roleBindingTitle') }}</strong>
          <label v-for="role in workbench.workspaceRoles" :key="role.id" class="check-row">
            <input v-model="form.roleIds" type="checkbox" :value="role.id">
            <span>{{ role.name }}</span>
          </label>
        </section>

        <section>
          <strong>{{ t('userCenter.users.scopeTitle') }}</strong>
          <label class="check-row">
            <input v-model="form.scopeMode" type="radio" value="all-projects">
            <span>{{ t('userCenter.scope.allProjects') }}</span>
          </label>
          <label class="check-row">
            <input v-model="form.scopeMode" type="radio" value="selected-projects">
            <span>{{ t('userCenter.scope.selectedProjects') }}</span>
          </label>

          <div v-if="form.scopeMode === 'selected-projects'" class="project-scope">
            <label v-for="project in workbench.workspaceProjects" :key="project.id" class="check-row">
              <input v-model="form.scopeProjectIds" type="checkbox" :value="project.id">
              <span>{{ project.name }}</span>
            </label>
          </div>
        </section>
      </div>

      <div class="form-actions">
        <button type="button" class="ghost-button" @click="applyForm(workbench.workspaceUsers[0]?.id)">
          {{ t('common.cancel') }}
        </button>
        <button type="button" class="primary-button" @click="saveUser">
          {{ t('common.save') }}
        </button>
      </div>

      <p v-if="selectedMembership" class="helper-text">
        {{ scopeLabel(selectedMembership.userId) }}
      </p>
    </UiSurface>
  </div>
</template>

<style scoped>
.page-layout,
.toolbar,
.user-summary,
.user-copy,
.user-actions,
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
.user-copy small,
.user-copy span,
.helper-text,
.form-grid label span {
  color: var(--text-secondary);
}

.user-list {
  display: grid;
  gap: 0.85rem;
}

.user-item {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.75rem;
  padding: 0.9rem;
  border: 1px solid var(--border-subtle);
  border-radius: var(--radius-l);
}

.user-item.active {
  border-color: color-mix(in srgb, var(--brand-primary) 34%, var(--border-subtle));
  background: color-mix(in srgb, var(--brand-primary) 6%, transparent);
}

.user-summary {
  align-items: center;
  gap: 0.85rem;
  min-width: 0;
  text-align: left;
}

.user-avatar {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2.8rem;
  height: 2.8rem;
  border-radius: 1rem;
  background: color-mix(in srgb, var(--brand-primary) 16%, transparent);
  font-weight: 700;
}

.user-copy {
  min-width: 0;
  flex-direction: column;
  gap: 0.15rem;
}

.user-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
  align-self: center;
}

.user-actions {
  grid-column: 1 / -1;
  flex-wrap: wrap;
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

.form-grid input,
.form-grid select {
  min-height: 2.6rem;
  padding: 0 0.75rem;
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

.project-scope {
  margin-top: 0.5rem;
}

.form-actions {
  justify-content: flex-end;
  gap: 0.65rem;
}

.helper-text {
  margin: 0.75rem 0 0;
}
</style>
