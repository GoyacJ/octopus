<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiBadge, UiSurface } from '@octopus/ui'

import { useWorkbenchStore } from '@/stores/workbench'

const { t, locale } = useI18n()
const workbench = useWorkbenchStore()

const membership = computed(() => workbench.currentMembership)
const roleNames = computed(() =>
  workbench.currentUserRoles.length
    ? workbench.currentUserRoles.map((role) => role.name)
    : [t('userCenter.common.noRoles')],
)

const scopeSummary = computed(() => {
  if (!membership.value) {
    return t('common.na')
  }

  if (membership.value.scopeMode === 'all-projects') {
    return t('userCenter.scope.allProjects')
  }

  const projectNames = membership.value.scopeProjectIds
    .map((projectId) => workbench.projects.find((project) => project.id === projectId)?.name)
    .filter((name): name is string => Boolean(name))

  return projectNames.length
    ? `${t('userCenter.scope.selectedProjects')}: ${projectNames.join(', ')}`
    : t('userCenter.scope.selectedProjects')
})

const permissionSummary = computed(() =>
  workbench.currentEffectivePermissionIds
    .map((permissionId) => workbench.workspacePermissions.find((permission) => permission.id === permissionId)?.name)
    .filter((name): name is string => Boolean(name)),
)

const maskedPassword = computed(() => {
  if (!workbench.currentUser) {
    return '********'
  }

  if (workbench.currentUser.passwordState === 'reset-required') {
    return t('userCenter.profile.passwordResetRequired')
  }

  if (workbench.currentUser.passwordState === 'temporary') {
    return t('userCenter.profile.passwordTemporary')
  }

  return '********'
})

const passwordUpdatedLabel = computed(() => {
  if (!workbench.currentUser?.passwordUpdatedAt) {
    return t('common.na')
  }

  return new Intl.DateTimeFormat(locale.value, {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(workbench.currentUser.passwordUpdatedAt)
})

function resetPassword() {
  if (!workbench.currentUser) {
    return
  }

  workbench.resetUserPassword(workbench.currentUser.id)
}
</script>

<template>
  <div class="profile-layout">
    <UiSurface :title="t('userCenter.profile.title')" :subtitle="t('userCenter.profile.subtitle')">
      <div class="profile-card">
        <div class="profile-avatar">
          {{ workbench.currentUser?.avatar ?? '--' }}
        </div>

        <div class="profile-copy">
          <h3>{{ workbench.currentUser?.nickname ?? t('common.na') }}</h3>
          <p>{{ workbench.currentUser?.username ?? t('common.na') }}</p>
          <div class="profile-badges">
            <UiBadge :label="workbench.currentUser?.status ?? t('common.na')" :tone="workbench.currentUser?.status === 'active' ? 'success' : 'warning'" />
            <UiBadge :label="scopeSummary" subtle />
          </div>
        </div>
      </div>

      <dl class="detail-grid">
        <div>
          <dt>{{ t('userCenter.profile.usernameLabel') }}</dt>
          <dd>{{ workbench.currentUser?.username ?? t('common.na') }}</dd>
        </div>
        <div>
          <dt>{{ t('userCenter.profile.nicknameLabel') }}</dt>
          <dd>{{ workbench.currentUser?.nickname ?? t('common.na') }}</dd>
        </div>
        <div>
          <dt>{{ t('userCenter.profile.genderLabel') }}</dt>
          <dd>{{ workbench.currentUser?.gender ?? t('common.na') }}</dd>
        </div>
        <div>
          <dt>{{ t('userCenter.profile.phoneLabel') }}</dt>
          <dd>{{ workbench.currentUser?.phone || t('common.na') }}</dd>
        </div>
        <div>
          <dt>{{ t('userCenter.profile.emailLabel') }}</dt>
          <dd>{{ workbench.currentUser?.email ?? t('common.na') }}</dd>
        </div>
        <div>
          <dt>{{ t('userCenter.profile.roleLabel') }}</dt>
          <dd>{{ roleNames.join(', ') }}</dd>
        </div>
      </dl>
    </UiSurface>

    <UiSurface :title="t('userCenter.profile.securityTitle')" :subtitle="t('userCenter.profile.securitySubtitle')">
      <dl class="detail-grid">
        <div>
          <dt>{{ t('userCenter.profile.passwordLabel') }}</dt>
          <dd>{{ maskedPassword }}</dd>
        </div>
        <div>
          <dt>{{ t('userCenter.profile.passwordUpdatedAtLabel') }}</dt>
          <dd>{{ passwordUpdatedLabel }}</dd>
        </div>
        <div>
          <dt>{{ t('userCenter.profile.permissionScopeLabel') }}</dt>
          <dd>{{ scopeSummary }}</dd>
        </div>
      </dl>

      <button type="button" class="primary-button" @click="resetPassword">
        {{ t('userCenter.profile.resetPassword') }}
      </button>
    </UiSurface>

    <UiSurface :title="t('userCenter.profile.permissionTitle')" :subtitle="t('userCenter.profile.permissionSubtitle')">
      <div class="list-grid">
        <div>
          <h4>{{ t('userCenter.profile.roleListTitle') }}</h4>
          <ul>
            <li v-for="roleName in roleNames" :key="roleName">{{ roleName }}</li>
          </ul>
        </div>
        <div>
          <h4>{{ t('userCenter.profile.atomicPermissionTitle') }}</h4>
          <ul>
            <li v-for="permissionName in permissionSummary" :key="permissionName">{{ permissionName }}</li>
            <li v-if="!permissionSummary.length">{{ t('userCenter.common.empty') }}</li>
          </ul>
        </div>
      </div>
    </UiSurface>
  </div>
</template>

<style scoped>
.profile-layout,
.profile-card,
.profile-copy,
.profile-badges {
  display: flex;
}

.profile-layout {
  flex-direction: column;
  gap: 1rem;
}

.profile-card {
  align-items: center;
  gap: 1rem;
  margin-bottom: 1rem;
}

.profile-avatar {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 4rem;
  height: 4rem;
  border-radius: 1.25rem;
  background: linear-gradient(135deg, color-mix(in srgb, var(--brand-primary) 28%, transparent), color-mix(in srgb, var(--brand-primary) 8%, transparent));
  color: var(--text-primary);
  font-weight: 700;
}

.profile-copy {
  flex-direction: column;
  gap: 0.35rem;
}

.profile-copy h3,
.list-grid h4 {
  margin: 0;
}

.profile-copy p,
.detail-grid dt {
  color: var(--text-secondary);
}

.profile-badges {
  flex-wrap: wrap;
  gap: 0.5rem;
}

.detail-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 0.85rem 1rem;
  margin: 0 0 1rem;
}

.detail-grid dd {
  margin: 0.2rem 0 0;
  color: var(--text-primary);
  font-weight: 600;
}

.list-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 1rem;
}

.list-grid ul {
  margin: 0.6rem 0 0;
  padding-left: 1rem;
  color: var(--text-secondary);
  line-height: 1.6;
}
</style>
