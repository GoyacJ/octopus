<script setup lang="ts">
import { computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter, RouterView } from 'vue-router'
import { navigationIcons } from '@octopus/icons'
import { navigationDefinitions } from '@octopus/shared'
import { UiAppShell, type ShellNavigationItem } from '@octopus/ui'
import { useShellStore } from './stores/shell'

const router = useRouter()
const route = useRoute()
const { locale, t } = useI18n()
const shell = useShellStore()

watch(
  () => shell.resolved,
  (value) => {
    document.documentElement.dataset.theme = value
  },
  { immediate: true, deep: true },
)

watch(
  () => shell.locale,
  (value) => {
    locale.value = value
    document.documentElement.lang = value
  },
  { immediate: true, deep: true },
)

const navigation = computed<ShellNavigationItem[]>(() =>
  navigationDefinitions.map((item) => ({
    id: item.id,
    iconClass: navigationIcons[item.id],
    label: t(item.labelKey),
    to: item.to,
    enabled: item.phaseZeroEnabled,
  })),
)

function handleNavigate(item: ShellNavigationItem) {
  router.push(item.to)
}

const themeLabel = computed(() => `${t('shell.theme')}: ${shell.mode}`)
const localeLabel = computed(() => `${t('shell.language')}: ${shell.locale}`)
</script>

<template>
  <UiAppShell
    :workspace-name="t('shell.workspace')"
    :tagline="t('shell.tagline')"
    :navigation="navigation"
    :current-path="route.path"
    :later-label="t('shell.later')"
    @navigate="handleNavigate"
  >
    <template #actions>
      <button
        type="button"
        class="rounded-full border border-[color:var(--oc-border-subtle)] px-3 py-2 text-sm text-[color:var(--oc-text-secondary)] transition hover:border-[color:var(--oc-accent)] hover:text-[color:var(--oc-text-primary)]"
        @click="shell.cycleTheme()"
      >
        {{ themeLabel }}
      </button>
      <button
        type="button"
        class="rounded-full border border-[color:var(--oc-border-subtle)] px-3 py-2 text-sm text-[color:var(--oc-text-secondary)] transition hover:border-[color:var(--oc-accent)] hover:text-[color:var(--oc-text-primary)]"
        @click="shell.toggleLocale()"
      >
        {{ localeLabel }}
      </button>
    </template>

    <RouterView />
  </UiAppShell>
</template>
