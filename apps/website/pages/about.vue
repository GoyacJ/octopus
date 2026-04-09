<script setup lang="ts">
import { UiSurface } from '@octopus/ui'

import { aboutPrincipleIds, editorialGraphics } from '../content-or-copy/site'

const { t, tm } = useI18n()
const localePath = useLocalePath()

usePageSeo('about')

const principles = computed(() =>
  aboutPrincipleIds.map((id) => ({
    id,
    title: t(`about.principles.${id}.title`),
    description: t(`about.principles.${id}.description`),
  })),
)

const journeyPoints = computed(() => tm('about.journey.points') as string[])
</script>

<template>
  <div>
    <section class="brand-section pt-16">
      <div class="brand-container grid gap-8 lg:grid-cols-[1fr_0.95fr] lg:items-start">
        <div class="space-y-6">
          <div class="brand-kicker">{{ t('about.hero.eyebrow') }}</div>
          <h1 class="brand-display">{{ t('about.hero.title') }}</h1>
          <p class="brand-lead">{{ t('about.hero.description') }}</p>
        </div>
        <div class="brand-panel overflow-hidden p-4">
          <img :src="editorialGraphics.governanceFlow" :alt="t('about.hero.diagramAlt')" class="w-full rounded-[22px] border border-border-subtle/80 shadow-sm">
        </div>
      </div>
    </section>

    <section class="brand-section pt-0">
      <div class="brand-container space-y-10">
        <BrandSectionHeading
          :eyebrow="t('about.principlesEyebrow')"
          :title="t('about.principlesTitle')"
          :description="t('about.principlesDescription')"
        />

        <div class="grid gap-6 md:grid-cols-2">
          <UiSurface
            v-for="principle in principles"
            :key="principle.id"
            variant="raised"
            padding="lg"
            class="brand-panel"
          >
            <div class="space-y-3">
              <h2 class="text-[1.8rem]">{{ principle.title }}</h2>
              <p class="text-sm leading-7 text-text-secondary">{{ principle.description }}</p>
            </div>
          </UiSurface>
        </div>
      </div>
    </section>

    <section class="brand-section">
      <div class="brand-container grid gap-6 xl:grid-cols-[0.95fr_1.05fr]">
        <UiSurface variant="raised" padding="lg" class="brand-panel">
          <div class="space-y-4">
            <div class="brand-metadata">{{ t('about.journey.eyebrow') }}</div>
            <h2 class="text-[2rem]">{{ t('about.journey.title') }}</h2>
            <p class="text-sm leading-7 text-text-secondary">{{ t('about.journey.lead') }}</p>
            <ul class="space-y-3">
              <li
                v-for="point in journeyPoints"
                :key="point"
                class="flex gap-3 text-sm leading-7 text-text-secondary"
              >
                <span class="mt-2 h-2 w-2 rounded-full bg-primary" />
                <span>{{ point }}</span>
              </li>
            </ul>
          </div>
        </UiSurface>

        <div class="brand-panel overflow-hidden p-4">
          <img :src="editorialGraphics.platformLayers" :alt="t('about.journey.diagramAlt')" class="w-full rounded-[22px] border border-border-subtle/80 shadow-sm">
        </div>
      </div>
    </section>

    <section class="brand-section">
      <div class="brand-container">
        <BrandCallToAction
          :eyebrow="t('about.cta.eyebrow')"
          :title="t('about.cta.title')"
          :description="t('about.cta.description')"
          :primary-href="localePath('/book-demo')"
          :primary-label="t('about.cta.primary')"
          :secondary-href="localePath('/product')"
          :secondary-label="t('about.cta.secondary')"
        />
      </div>
    </section>
  </div>
</template>
