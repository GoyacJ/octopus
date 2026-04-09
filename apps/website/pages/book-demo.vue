<script setup lang="ts">
import { UiSurface } from '@octopus/ui'

import { bookDemoOptionIds } from '../content-or-copy/site'

const { t, tm } = useI18n()
const localePath = useLocalePath()
const config = useRuntimeConfig()

usePageSeo('bookDemo')

const options = computed(() =>
  bookDemoOptionIds.map((id) => ({
    id,
    title: t(`bookDemo.options.${id}.title`),
    description: t(`bookDemo.options.${id}.description`),
    cta: t(`bookDemo.options.${id}.cta`),
    href: id === 'call'
      ? config.public.demoUrl
      : id === 'email'
        ? 'mailto:hello@octopus.run?subject=Octopus%20Website'
        : localePath('/product'),
    external: id !== 'product',
  })),
)

const checklist = computed(() => tm('bookDemo.checklist.items') as string[])
</script>

<template>
  <div>
    <section class="brand-section pt-16">
      <div class="brand-container max-w-4xl space-y-6">
        <div class="brand-kicker">{{ t('bookDemo.hero.eyebrow') }}</div>
        <h1 class="brand-display">{{ t('bookDemo.hero.title') }}</h1>
        <p class="brand-lead">{{ t('bookDemo.hero.description') }}</p>
      </div>
    </section>

    <section class="brand-section pt-0">
      <div class="brand-container grid gap-6 xl:grid-cols-[0.9fr_1.1fr]">
        <UiSurface variant="raised" padding="lg" class="brand-panel">
          <div class="space-y-4">
            <div class="brand-metadata">{{ t('bookDemo.summary.eyebrow') }}</div>
            <h2 class="text-[2rem]">{{ t('bookDemo.summary.title') }}</h2>
            <p class="text-sm leading-7 text-text-secondary">{{ t('bookDemo.summary.description') }}</p>
            <ul class="space-y-3">
              <li
                v-for="item in checklist"
                :key="item"
                class="flex gap-3 text-sm leading-7 text-text-secondary"
              >
                <span class="mt-2 h-2 w-2 rounded-full bg-primary" />
                <span>{{ item }}</span>
              </li>
            </ul>
          </div>
        </UiSurface>

        <div class="grid gap-6 md:grid-cols-3">
          <UiSurface
            v-for="option in options"
            :key="option.id"
            variant="raised"
            padding="lg"
            class="brand-panel h-full"
          >
            <div class="flex h-full flex-col gap-4">
              <div class="space-y-3">
                <div class="brand-metadata">{{ t(`bookDemo.options.${option.id}.eyebrow`) }}</div>
                <h2 class="text-[1.7rem]">{{ option.title }}</h2>
                <p class="text-sm leading-7 text-text-secondary">{{ option.description }}</p>
              </div>
              <div class="mt-auto pt-4">
                <SiteButtonLink
                  :href="option.href"
                  :label="option.cta"
                  :variant="option.id === 'call' ? 'primary' : 'outline'"
                  class="w-full justify-center"
                  :new-tab="option.external"
                />
              </div>
            </div>
          </UiSurface>
        </div>
      </div>
    </section>
  </div>
</template>
