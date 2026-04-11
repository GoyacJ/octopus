<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiButton, UiField, UiInput } from '@octopus/ui'

import * as tauriClient from '@/tauri/client'
import { useAuthStore } from '@/stores/auth'

const props = withDefaults(defineProps<{
  inline?: boolean
}>(), {
  inline: false,
})

const { t } = useI18n()
const auth = useAuthStore()

const username = ref('')
const displayName = ref('')
const password = ref('')
const confirmPassword = ref('')
const captchaCode = ref('')
const avatarFileName = ref('')
const avatarPreview = ref('')
const avatarPayload = ref<Awaited<ReturnType<typeof tauriClient.pickAvatarImage>>>(null)
const localError = ref('')

const isRegister = computed(() => auth.mode === 'register')
const activeError = computed(() => localError.value || auth.error)
const captchaImageSrc = computed(() => {
  const svgData = auth.captchaChallenge?.svgData
  if (!svgData) {
    return ''
  }

  return `data:image/svg+xml;base64,${window.btoa(unescape(encodeURIComponent(svgData)))}`
})

watch(
  () => auth.mode,
  () => {
    localError.value = ''
    password.value = ''
    confirmPassword.value = ''
    captchaCode.value = ''
    if (auth.mode === 'login') {
      displayName.value = ''
      avatarFileName.value = ''
      avatarPreview.value = ''
      avatarPayload.value = null
    }
  },
  { immediate: true },
)

watch(
  () => [auth.dialogOpen, auth.isReady, auth.captchaChallenge?.challengeId, props.inline] as const,
  async ([open, ready, challengeId, inline]) => {
    if (!ready) {
      return
    }
    if (!open && !inline) {
      return
    }
    captchaCode.value = ''
    if (!challengeId) {
      await auth.prepareCaptchaChallenge()
    }
  },
  { immediate: true },
)

function avatarDataUrl(): string {
  if (!avatarPayload.value) {
    return ''
  }

  return `data:${avatarPayload.value.contentType};base64,${avatarPayload.value.dataBase64}`
}

async function pickAvatar() {
  const picked = await tauriClient.pickAvatarImage()
  if (!picked) {
    return
  }

  avatarPayload.value = picked
  avatarFileName.value = picked.fileName
  avatarPreview.value = avatarDataUrl()
  localError.value = ''
}

async function refreshCaptcha() {
  await auth.prepareCaptchaChallenge()
  captchaCode.value = ''
}

function validate(): boolean {
  if (!username.value.trim()) {
    localError.value = t('authGate.validation.usernameRequired')
    return false
  }
  if (!password.value) {
    localError.value = t('authGate.validation.passwordRequired')
    return false
  }
  if (!captchaCode.value.trim()) {
    localError.value = t('authGate.validation.captchaRequired')
    return false
  }
  if (isRegister.value) {
    if (!displayName.value.trim()) {
      localError.value = t('authGate.validation.displayNameRequired')
      return false
    }
    if (!avatarPayload.value) {
      localError.value = t('authGate.validation.avatarRequired')
      return false
    }
    if (password.value.length < 8) {
      localError.value = t('authGate.validation.passwordMinLength')
      return false
    }
    if (password.value !== confirmPassword.value) {
      localError.value = t('authGate.validation.passwordMismatch')
      return false
    }
  }

  localError.value = ''
  return true
}

async function submit() {
  if (!validate()) {
    return
  }

  if (isRegister.value && avatarPayload.value) {
    await auth.registerOwner({
      username: username.value.trim(),
      displayName: displayName.value.trim(),
      password: password.value,
      confirmPassword: confirmPassword.value,
      avatar: avatarPayload.value,
      captchaCode: captchaCode.value,
    })
    return
  }

  await auth.login({
    username: username.value.trim(),
    password: password.value,
    captchaCode: captchaCode.value,
  })
}
</script>

<template>
  <div class="space-y-5" data-testid="auth-gate-panel">
    <div class="rounded-[var(--radius-xl)] border border-border bg-subtle px-4 py-3">
      <p class="text-[11px] font-semibold uppercase tracking-[0.28em] text-text-tertiary">
        {{ t('authGate.eyebrow') }}
      </p>
    </div>

    <div class="grid gap-4">
      <UiField :label="t('authGate.fields.username')">
        <UiInput v-model="username" autocomplete="username" />
      </UiField>

      <UiField v-if="isRegister" :label="t('authGate.fields.displayName')">
        <UiInput v-model="displayName" autocomplete="nickname" />
      </UiField>

      <UiField v-if="isRegister" :label="t('authGate.fields.avatar')" :hint="t('authGate.fields.avatarHint')">
        <div class="flex items-center gap-3">
          <div class="flex h-12 w-12 shrink-0 items-center justify-center overflow-hidden rounded-full border border-border/60 bg-accent text-xs font-semibold uppercase text-text-secondary">
            <img v-if="avatarPreview" :src="avatarPreview" alt="" class="h-full w-full object-cover">
            <span v-else>{{ displayName.slice(0, 1) || username.slice(0, 1) || 'A' }}</span>
          </div>
          <div class="min-w-0 flex-1">
            <UiButton type="button" variant="ghost" class="w-full justify-center" @click="pickAvatar">
              {{ avatarFileName || t('authGate.actions.pickAvatar') }}
            </UiButton>
          </div>
        </div>
      </UiField>

      <UiField :label="t('authGate.fields.password')">
        <UiInput v-model="password" type="password" autocomplete="current-password" />
      </UiField>

      <UiField v-if="isRegister" :label="t('authGate.fields.confirmPassword')">
        <UiInput v-model="confirmPassword" type="password" autocomplete="new-password" />
      </UiField>

      <UiField :label="t('authGate.fields.captcha')" :hint="t('authGate.fields.captchaHint')">
        <div class="grid gap-3 sm:grid-cols-[minmax(0,1fr)_176px] sm:items-start">
          <UiInput v-model="captchaCode" autocomplete="one-time-code" />
          <div class="rounded-[var(--radius-lg)] border border-border bg-subtle p-2">
            <img
              v-if="captchaImageSrc"
              :src="captchaImageSrc"
              :alt="t('authGate.fields.captcha')"
              class="h-[52px] w-full rounded-[4px] border border-border/60 bg-white object-contain"
            >
            <div v-else class="flex h-[52px] items-center justify-center text-xs text-text-tertiary">
              {{ t('authGate.actions.refreshCaptcha') }}
            </div>
            <UiButton type="button" variant="ghost" class="mt-2 w-full justify-center" @click="refreshCaptcha">
              {{ t('authGate.actions.refreshCaptcha') }}
            </UiButton>
          </div>
        </div>
      </UiField>
    </div>

    <p v-if="activeError" class="rounded-[var(--radius-l)] border border-destructive/20 bg-destructive/5 px-3 py-2 text-sm text-destructive">
      {{ activeError }}
    </p>

    <div class="flex items-center justify-end gap-3">
      <UiButton data-testid="auth-gate-submit" :loading="auth.submitting" @click="submit">
        {{ isRegister ? t('authGate.actions.register') : t('authGate.actions.login') }}
      </UiButton>
    </div>
  </div>
</template>
