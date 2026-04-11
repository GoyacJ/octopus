<script setup lang="ts">
import { computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useForm } from 'vee-validate'
import { toTypedSchema } from '@vee-validate/zod'
import { z } from 'zod'
import { useRoute, useRouter } from 'vue-router'

import { UiButton, UiDialog, UiField, UiInput } from '@octopus/ui'

import { createWorkspaceOverviewTarget } from '@/i18n/navigation'
import { useAuthStore } from '@/stores/auth'

const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  'update:open': [open: boolean]
}>()

const { t } = useI18n()
const auth = useAuthStore()
const route = useRoute()
const router = useRouter()

const schema = toTypedSchema(z.object({
  baseUrl: z.string().trim().min(1, t('connectWorkspace.validation.baseUrlRequired')).url(t('connectWorkspace.validation.baseUrlInvalid')),
  username: z.string().trim().min(1, t('connectWorkspace.validation.usernameRequired')),
  password: z.string().min(1, t('connectWorkspace.validation.passwordRequired')),
  captchaCode: z.string().trim().min(1, t('connectWorkspace.validation.captchaRequired')),
}))

const { defineField, errors, handleSubmit, resetForm } = useForm({
  validationSchema: schema,
  initialValues: {
    baseUrl: '',
    username: '',
    password: '',
    captchaCode: '',
  },
})

const [baseUrl] = defineField('baseUrl')
const [username] = defineField('username')
const [password] = defineField('password')
const [captchaCode] = defineField('captchaCode')

const activeError = computed(() => auth.error)
const captchaImageSrc = computed(() => {
  const svgData = auth.connectionCaptcha?.svgData
  if (!svgData) {
    return ''
  }

  return `data:image/svg+xml;base64,${window.btoa(unescape(encodeURIComponent(svgData)))}`
})

watch(
  () => props.open,
  async (open) => {
    if (open) {
      resetForm({
        values: {
          baseUrl: '',
          username: '',
          password: '',
          captchaCode: '',
        },
      })
      auth.error = ''
      auth.clearConnectionCaptcha()
    }
  },
)

watch(
  () => baseUrl.value,
  async (nextBaseUrl) => {
    const normalized = (nextBaseUrl ?? '').trim()
    if (!props.open || !/^https?:\/\//.test(normalized)) {
      auth.clearConnectionCaptcha()
      return
    }

    await auth.prepareConnectionCaptcha(normalized)
  },
)

async function refreshCaptcha() {
  const normalizedBaseUrl = (baseUrl.value ?? '').trim()
  if (!normalizedBaseUrl) {
    return
  }

  await auth.prepareConnectionCaptcha(normalizedBaseUrl)
  captchaCode.value = ''
}

const submit = handleSubmit(async (values) => {
  const connection = await auth.connectWorkspace(values)
  emit('update:open', false)
  resetForm()

  if (String(route.params.workspaceId ?? '') !== connection.workspaceId || route.name !== 'workspace-overview') {
    await router.push(createWorkspaceOverviewTarget(connection.workspaceId))
  }
})
</script>

<template>
  <UiDialog
    :open="open"
    :title="t('connectWorkspace.title')"
    :description="t('connectWorkspace.description')"
    :close-label="t('common.cancel')"
    content-test-id="connect-workspace-dialog"
    content-class="max-w-lg"
    @update:open="(nextOpen) => emit('update:open', nextOpen)"
  >
    <form class="space-y-5" data-testid="connect-workspace-form" @submit.prevent="submit">
      <div class="rounded-[var(--radius-xl)] border border-border bg-subtle px-4 py-3">
        <p class="text-[11px] font-semibold uppercase tracking-[0.28em] text-text-tertiary">
          {{ t('connectWorkspace.eyebrow') }}
        </p>
      </div>

      <div class="grid gap-4">
        <UiField :label="t('connectWorkspace.fields.baseUrl')" :hint="errors.baseUrl">
          <UiInput v-model="baseUrl" data-testid="connect-workspace-base-url" :placeholder="t('connectWorkspace.placeholders.baseUrl')" />
        </UiField>

        <UiField :label="t('connectWorkspace.fields.username')" :hint="errors.username">
          <UiInput v-model="username" data-testid="connect-workspace-username" autocomplete="username" />
        </UiField>

        <UiField :label="t('connectWorkspace.fields.password')" :hint="errors.password">
          <UiInput v-model="password" data-testid="connect-workspace-password" type="password" autocomplete="current-password" />
        </UiField>

        <UiField :label="t('connectWorkspace.fields.captcha')" :hint="errors.captchaCode || t('connectWorkspace.fields.captchaHint')">
          <div class="grid gap-3 sm:grid-cols-[minmax(0,1fr)_176px] sm:items-start">
            <UiInput v-model="captchaCode" data-testid="connect-workspace-captcha" autocomplete="one-time-code" />
            <div class="rounded-[var(--radius-lg)] border border-border bg-subtle p-2">
              <img
                v-if="captchaImageSrc"
                :src="captchaImageSrc"
                :alt="t('connectWorkspace.fields.captcha')"
                class="h-[52px] w-full rounded-[4px] border border-border/60 bg-white object-contain"
              >
              <div v-else class="flex h-[52px] items-center justify-center text-xs text-text-tertiary">
                {{ t('connectWorkspace.actions.refreshCaptcha') }}
              </div>
              <UiButton type="button" variant="ghost" class="mt-2 w-full justify-center" @click="refreshCaptcha">
                {{ t('connectWorkspace.actions.refreshCaptcha') }}
              </UiButton>
            </div>
          </div>
        </UiField>
      </div>

      <p v-if="activeError" class="rounded-[var(--radius-l)] border border-destructive/20 bg-destructive/5 px-3 py-2 text-sm text-destructive">
        {{ activeError }}
      </p>

      <div class="flex items-center justify-end gap-3">
        <UiButton
          type="button"
          variant="ghost"
          :disabled="auth.submitting"
          @click="emit('update:open', false)"
        >
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton data-testid="connect-workspace-submit" type="submit" :loading="auth.submitting">
          {{ t('connectWorkspace.actions.submit') }}
        </UiButton>
      </div>
    </form>
  </UiDialog>
</template>
