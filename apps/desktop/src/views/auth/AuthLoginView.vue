<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import AuthGateForm from '@/components/auth/AuthGateForm.vue'
import { useAuthStore } from '@/stores/auth'
import { useShellStore } from '@/stores/shell'

const { t } = useI18n()
const auth = useAuthStore()
const shell = useShellStore()

const connection = computed(() => shell.activeWorkspaceConnection)
const isRegister = computed(() => auth.mode === 'register')
const title = computed(() =>
  isRegister.value ? t('authGate.register.title') : t('authGate.login.title'),
)
const description = computed(() =>
  isRegister.value ? t('authGate.register.description') : t('authGate.login.description'),
)
const workspaceName = computed(() =>
  auth.bootstrapStatus?.workspace?.name
  || connection.value?.label
  || 'Octopus Workspace',
)
const workspaceAddress = computed(() => connection.value?.baseUrl || '')
const isLoading = computed(() => shell.loading || auth.bootstrapping || !auth.isReady)
</script>

<template>
  <div class="flex min-h-screen items-center justify-center bg-background px-6 py-10">
    <div class="grid w-full max-w-5xl overflow-hidden rounded-[var(--radius-xl)] border border-border bg-surface shadow-sm lg:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
      <section class="border-b border-border bg-muted/35 px-8 py-10 lg:border-b-0 lg:border-r">
        <p class="text-[11px] font-semibold uppercase tracking-[0.28em] text-text-tertiary">
          {{ t('authGate.eyebrow') }}
        </p>
        <div class="mt-5 space-y-3">
          <h1 class="text-[30px] font-bold tracking-[-0.03em] text-text-primary">
            {{ workspaceName }}
          </h1>
          <p class="text-sm leading-6 text-text-secondary">
            {{ description }}
          </p>
          <p v-if="workspaceAddress" class="text-xs text-text-tertiary">
            {{ workspaceAddress }}
          </p>
        </div>
      </section>

      <section class="px-8 py-10">
        <div class="mx-auto max-w-xl">
          <div class="space-y-2">
            <h2 class="text-[22px] font-bold tracking-[-0.02em] text-text-primary">
              {{ title }}
            </h2>
            <p class="text-sm leading-6 text-text-secondary">
              {{ isLoading ? '正在校验当前工作区登录状态。' : description }}
            </p>
          </div>

          <div class="mt-6">
            <div
              v-if="isLoading"
              data-testid="browser-auth-loading"
              class="rounded-[var(--radius-l)] border border-border bg-subtle px-4 py-6 text-sm text-text-secondary"
            >
              正在加载登录上下文…
            </div>
            <div
              v-else
              data-testid="browser-auth-login-view"
              class="rounded-[var(--radius-l)] border border-border bg-card p-6"
            >
              <AuthGateForm inline />
            </div>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>
