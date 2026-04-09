<script setup lang="ts">
import { UiButton } from '@octopus/ui'
import { Menu, X } from 'lucide-vue-next'

import { navigationItems } from '../../content-or-copy/site'

const route = useRoute()
const { t } = useI18n()
const localePath = useLocalePath()

const menuOpen = ref(false)

function resolvePath(path: string) {
  return localePath(path)
}

function isActive(path: string) {
  const localizedPath = resolvePath(path)
  if (path === '/') {
    return route.path === localizedPath
  }

  return route.path === localizedPath || route.path.startsWith(`${localizedPath}/`)
}

watch(() => route.path, () => {
  menuOpen.value = false
})
</script>

<template>
  <header class="sticky top-0 z-40 border-b border-border-subtle/80 bg-[color-mix(in_srgb,var(--bg-main)_88%,transparent)] backdrop-blur-sm">
    <div class="brand-container flex min-h-[4.75rem] items-center justify-between gap-4 py-4">
      <NuxtLink :to="resolvePath('/')" class="flex items-center gap-3 text-text-primary">
        <img src="/brand/logo.png" :alt="t('site.name')" class="h-10 w-10 rounded-2xl border border-border-subtle/80 bg-surface p-1.5 shadow-xs">
        <div class="space-y-0.5">
          <div class="font-display text-[1.3rem] leading-none">{{ t('site.name') }}</div>
          <div class="text-[12px] tracking-[0.12em] text-text-tertiary uppercase">{{ t('site.tagline') }}</div>
        </div>
      </NuxtLink>

      <nav class="hidden items-center gap-6 lg:flex">
        <NuxtLink
          v-for="item in navigationItems"
          :key="item.key"
          :to="resolvePath(item.path)"
          class="brand-link text-sm font-medium"
          :class="isActive(item.path) ? 'text-text-primary' : 'text-text-secondary'"
        >
          {{ t(`nav.${item.key}`) }}
        </NuxtLink>
      </nav>

      <div class="hidden items-center gap-3 lg:flex">
        <SiteLanguageSwitcher />
        <SiteThemeToggle />
        <SiteButtonLink :href="resolvePath('/book-demo')" :label="t('nav.bookDemo')" />
      </div>

      <UiButton
        variant="ghost"
        size="icon"
        class="lg:hidden"
        :aria-label="menuOpen ? 'Close navigation' : 'Open navigation'"
        @click="menuOpen = !menuOpen"
      >
        <X v-if="menuOpen" class="h-4 w-4" />
        <Menu v-else class="h-4 w-4" />
      </UiButton>
    </div>

    <div v-if="menuOpen" class="border-t border-border-subtle/80 lg:hidden">
      <div class="brand-container grid gap-4 py-4">
        <NuxtLink
          v-for="item in navigationItems"
          :key="item.key"
          :to="resolvePath(item.path)"
          class="rounded-[var(--radius-l)] border border-border-subtle/80 bg-surface px-4 py-3 text-sm font-medium"
          :class="isActive(item.path) ? 'text-text-primary' : 'text-text-secondary'"
        >
          {{ t(`nav.${item.key}`) }}
        </NuxtLink>
        <div class="flex flex-wrap gap-3">
          <SiteLanguageSwitcher />
          <SiteThemeToggle />
        </div>
        <SiteButtonLink :href="resolvePath('/book-demo')" :label="t('nav.bookDemo')" size="lg" />
      </div>
    </div>
  </header>
</template>
