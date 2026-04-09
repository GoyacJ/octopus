<script setup lang="ts">
import { navigationItems } from '../../content-or-copy/site'

const { t } = useI18n()
const localePath = useLocalePath()
</script>

<template>
  <footer class="border-t border-border-subtle/80 py-12">
    <div class="brand-container grid gap-10 md:grid-cols-[1.2fr_0.8fr]">
      <div class="space-y-4">
        <div class="flex items-center gap-3">
          <img src="/brand/logo.png" :alt="t('site.name')" class="h-10 w-10 rounded-2xl border border-border-subtle/80 bg-surface p-1.5 shadow-xs">
          <div>
            <div class="font-display text-[1.25rem]">{{ t('site.name') }}</div>
            <div class="text-[12px] uppercase tracking-[0.12em] text-text-tertiary">{{ t('site.tagline') }}</div>
          </div>
        </div>
        <p class="max-w-xl text-sm leading-7 text-text-secondary">
          {{ t('footer.tagline') }}
        </p>
        <p class="text-xs uppercase tracking-[0.14em] text-text-tertiary">
          {{ t('footer.note') }}
        </p>
      </div>

      <div class="grid gap-8 sm:grid-cols-2">
        <div class="space-y-3">
          <div class="text-[12px] font-semibold uppercase tracking-[0.14em] text-text-tertiary">
            Navigation
          </div>
          <div class="grid gap-2">
            <NuxtLink
              v-for="item in navigationItems"
              :key="item.key"
              :to="localePath(item.path)"
              class="brand-link text-sm text-text-secondary"
            >
              {{ t(`nav.${item.key}`) }}
            </NuxtLink>
            <NuxtLink :to="localePath('/book-demo')" class="brand-link text-sm text-text-secondary">
              {{ t('nav.bookDemo') }}
            </NuxtLink>
          </div>
        </div>

        <div class="space-y-3">
          <div class="text-[12px] font-semibold uppercase tracking-[0.14em] text-text-tertiary">
            Signals
          </div>
          <div class="grid gap-2 text-sm text-text-secondary">
            <p>{{ t('footer.signals.collaboration') }}</p>
            <p>{{ t('footer.signals.runtime') }}</p>
            <p>{{ t('footer.signals.memory') }}</p>
          </div>
        </div>
      </div>
    </div>
  </footer>
</template>
