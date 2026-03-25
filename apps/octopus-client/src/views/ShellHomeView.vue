<script setup lang="ts">
import { onMounted } from 'vue'
import { useI18n } from 'vue-i18n'

import AutomationControlPanel from '@/components/AutomationControlPanel.vue'
import RuntimeActivityPanel from '@/components/RuntimeActivityPanel.vue'
import RuntimeOverview from '@/components/RuntimeOverview.vue'
import TaskControlPanel from '@/components/TaskControlPanel.vue'
import { useRuntimeControlStore } from '@/stores/useRuntimeControlStore'
import { useShellStore } from '@/stores/useShellStore'

const shellStore = useShellStore()
const runtimeStore = useRuntimeControlStore()
const { locale, t } = useI18n()

const switchLocale = () => {
  const nextLocale = shellStore.locale === 'zh-CN' ? 'en-US' : 'zh-CN'

  shellStore.setLocale(nextLocale)
  locale.value = nextLocale
}

onMounted(() => {
  void runtimeStore.loadAutomations()
})
</script>

<template>
  <section class="space-y-6">
    <div class="rounded-[28px] border border-[var(--border-muted)] bg-[var(--surface-panel)] p-6 shadow-sm">
      <div class="flex flex-wrap items-center justify-between gap-4">
        <div class="max-w-3xl">
          <p class="text-sm uppercase tracking-[0.24em] text-[var(--text-muted)]">Phase 1</p>
          <p class="mt-3 text-sm text-[var(--text-muted)]">{{ t('app.subtitle') }}</p>
        </div>
        <div class="flex gap-3">
          <button
            class="rounded-full border border-[var(--border-muted)] px-4 py-2 text-sm transition hover:bg-[var(--surface-elevated)]"
            type="button"
            @click="switchLocale"
          >
            {{ t('app.locale') }}
          </button>
          <button
            class="rounded-full bg-[var(--accent-primary)] px-4 py-2 text-sm font-medium text-white transition hover:opacity-90"
            type="button"
            @click="shellStore.toggleTheme"
          >
            {{ t('app.theme') }}: {{ shellStore.isDark ? t('app.dark') : t('app.light') }}
          </button>
        </div>
      </div>
    </div>
    <section class="grid gap-6 xl:grid-cols-[0.95fr_1.05fr]">
      <div class="space-y-6">
        <TaskControlPanel />
        <AutomationControlPanel />
      </div>
      <RuntimeActivityPanel />
    </section>
    <RuntimeOverview />
  </section>
</template>
