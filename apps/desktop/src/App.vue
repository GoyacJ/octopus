<script setup lang="ts">
import { computed, onMounted, onUnmounted, watch } from 'vue'
import { RouterView, useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'

import type { NotificationRecord } from '@octopus/schema'
import { UiButton, UiStatusCallout, UiToastViewport } from '@octopus/ui'

import AuthGateDialog from '@/components/auth/AuthGateDialog.vue'
import i18n from '@/plugins/i18n'
import { useWorkbenchRouteSync } from '@/composables/useWorkbenchRouteSync'
import WorkbenchLayout from '@/layouts/WorkbenchLayout.vue'
import { useAuthStore } from '@/stores/auth'
import { useNotificationStore } from '@/stores/notifications'
import { useAppUpdateStore } from '@/stores/app-update'
import { usePetStore } from '@/stores/pet'
import { useRuntimeStore } from '@/stores/runtime'
import { useShellStore } from '@/stores/shell'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'
import {
  WORKSPACE_AUTH_FAILURE_EVENT,
  WORKSPACE_AUTHORIZATION_FAILURE_EVENT,
  type WorkspaceAuthFailureDetail,
  type WorkspaceAuthorizationFailureDetail,
} from '@/tauri/shared'

const auth = useAuthStore()
const notifications = useNotificationStore()
const appUpdate = useAppUpdateStore()
const router = useRouter()
const route = useRoute()
const shell = useShellStore()
const runtime = useRuntimeStore()
const pet = usePetStore()
const workspaceAccessControl = useWorkspaceAccessControlStore()
const { t } = useI18n()
const isBrowserHostRuntime = import.meta.env.VITE_HOST_RUNTIME === 'browser'

useWorkbenchRouteSync()

function resolveTheme(theme: 'light' | 'dark' | 'system'): 'light' | 'dark' {
  if (theme === 'system') {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
  }

  return theme
}

async function bootstrapShell() {
  await shell.bootstrap(shell.defaultWorkspaceId, shell.defaultProjectId)
  await notifications.bootstrap()
  await auth.bootstrapAuth()
  runtime.syncWorkspaceScopeFromShell()
  void appUpdate.initialize().catch((error) => {
    console.warn('failed to initialize app update status', error)
  })
}

const notificationScopeLabels = computed(() => ({
  app: t('notifications.scopes.app'),
  workspace: t('notifications.scopes.workspace'),
  user: t('notifications.scopes.user'),
}))

const showHostUnavailable = computed(() => {
  if (shell.loading) {
    return false
  }

  if (shell.error) {
    return true
  }

  return shell.backendConnection?.state === 'unavailable'
})

const hostUnavailableDescription = computed(() =>
  shell.error || t('app.hostUnavailable.description'),
)
const isAuthRoute = computed(() => route.name === 'auth-login')
const shouldShowWorkbenchLayout = computed(() => !isAuthRoute.value)
const shouldShowAuthDialog = computed(() => !isBrowserHostRuntime && shouldShowWorkbenchLayout.value)

const handleWorkspaceAuthFailure = (event: Event) => {
  const detail = (event as CustomEvent<WorkspaceAuthFailureDetail>).detail
  auth.handleAuthError(detail.workspaceConnectionId, 'session-expired')
}

const handleWorkspaceAuthorizationFailure = (event: Event) => {
  const detail = (event as CustomEvent<WorkspaceAuthorizationFailureDetail>).detail
  void workspaceAccessControl.reloadAll(detail.workspaceConnectionId).catch(() => {})
}

async function handleNotificationSelect(notification: NotificationRecord) {
  await notifications.markRead(notification.id)
  if (notification.routeTo) {
    await router.push(notification.routeTo)
  }
}

onMounted(async () => {
  window.addEventListener(WORKSPACE_AUTH_FAILURE_EVENT, handleWorkspaceAuthFailure as EventListener)
  window.addEventListener(
    WORKSPACE_AUTHORIZATION_FAILURE_EVENT,
    handleWorkspaceAuthorizationFailure as EventListener,
  )
  await bootstrapShell()
})

onUnmounted(() => {
  window.removeEventListener(WORKSPACE_AUTH_FAILURE_EVENT, handleWorkspaceAuthFailure as EventListener)
  window.removeEventListener(
    WORKSPACE_AUTHORIZATION_FAILURE_EVENT,
    handleWorkspaceAuthorizationFailure as EventListener,
  )
})

watch(
  () => shell.preferences.locale,
  (locale) => {
    i18n.global.locale.value = locale
  },
  { immediate: true },
)

watch(
  () => shell.preferences.theme,
  (theme) => {
    document.documentElement.dataset.theme = resolveTheme(theme)
  },
  { immediate: true },
)

watch(
  () => shell.preferences.fontSize,
  (fontSize) => {
    const root = document.documentElement
    root.style.setProperty('--font-size-base', `${fontSize}px`)
    root.style.fontSize = `${fontSize}px`
    document.body.style.fontFamily = 'var(--font-sans)'
  },
  { immediate: true },
)

watch(
  () => shell.activeWorkspaceConnectionId,
  async (workspaceConnectionId, previousConnectionId) => {
    if (previousConnectionId) {
      pet.clearWorkspaceScope(previousConnectionId)
    }
    if (workspaceConnectionId) {
      await auth.bootstrapAuth(workspaceConnectionId)
    }
    runtime.syncWorkspaceScopeFromShell()
  },
)

watch(
  () => [isBrowserHostRuntime, shell.loading, auth.isReady, auth.isAuthenticated, route.name, route.fullPath] as const,
  async ([browserRuntime, shellLoading, authReady, authenticated, routeName, fullPath]) => {
    if (!browserRuntime || shellLoading || !authReady) {
      return
    }

    if (!authenticated && routeName !== 'auth-login') {
      await router.replace({
        name: 'auth-login',
        query: {
          redirect: fullPath,
        },
      })
      return
    }

    if (authenticated && routeName === 'auth-login') {
      const redirect = typeof route.query.redirect === 'string' && route.query.redirect !== '/login'
        ? route.query.redirect
        : null
      if (redirect) {
        await router.replace(redirect)
        return
      }

      const workspaceId = shell.activeWorkspaceConnection?.workspaceId
        || shell.preferences.defaultWorkspaceId
        || 'ws-local'
      await router.replace({
        name: 'workspace-overview',
        params: { workspaceId },
        query: shell.defaultProjectId ? { project: shell.defaultProjectId } : undefined,
      })
    }
  },
  { immediate: true },
)

</script>

<template>
  <div
    v-if="showHostUnavailable"
    data-testid="desktop-backend-guard"
    class="flex min-h-screen items-center justify-center bg-background px-6"
  >
    <div class="w-full max-w-xl rounded-[var(--radius-xl)] border border-border bg-card p-8 shadow-lg">
      <div class="space-y-2">
        <p class="text-[11px] font-semibold uppercase tracking-[0.24em] text-text-tertiary">
          {{ t('app.hostUnavailable.eyebrow') }}
        </p>
        <h1 class="text-2xl font-semibold tracking-tight text-text-primary">
          {{ t('app.hostUnavailable.title') }}
        </h1>
        <p class="text-sm leading-6 text-text-secondary">
          {{ hostUnavailableDescription }}
        </p>
      </div>

      <UiStatusCallout
        tone="warning"
        class="mt-5"
        :description="t('app.hostUnavailable.description')"
      />

      <div class="mt-6 flex flex-wrap gap-3">
        <UiButton data-testid="desktop-backend-retry" @click="bootstrapShell">
          {{ t('app.hostUnavailable.retry') }}
        </UiButton>
        <UiButton
          v-if="shell.canRestartBackend"
          data-testid="desktop-backend-restart"
          variant="ghost"
          @click="shell.restartBackend"
        >
          {{ t('app.hostUnavailable.restart') }}
        </UiButton>
      </div>
    </div>
  </div>
  <WorkbenchLayout v-else-if="shouldShowWorkbenchLayout">
    <RouterView />
  </WorkbenchLayout>
  <RouterView v-else />
  <AuthGateDialog v-if="shouldShowAuthDialog" />
  <UiToastViewport
    :notifications="notifications.activeToasts"
    :scope-labels="notificationScopeLabels"
    @close="notifications.dismissToast"
    @select="handleNotificationSelect"
  />
</template>
