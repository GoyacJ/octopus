<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import { UiDialog } from '@octopus/ui'

import AuthGateForm from './AuthGateForm.vue'
import { useAuthStore } from '@/stores/auth'

const { t } = useI18n()
const auth = useAuthStore()

const isRegister = computed(() => auth.mode === 'register')
const title = computed(() =>
  isRegister.value ? t('authGate.register.title') : t('authGate.login.title'),
)
const description = computed(() =>
  isRegister.value ? t('authGate.register.description') : t('authGate.login.description'),
)
</script>

<template>
  <UiDialog
    :open="auth.dialogOpen"
    :title="title"
    :description="description"
    :close-label="t('authGate.closeLabel')"
    content-test-id="auth-gate-dialog"
    content-class="max-w-lg"
    @update:open="() => undefined"
  >
    <AuthGateForm />
  </UiDialog>
</template>
