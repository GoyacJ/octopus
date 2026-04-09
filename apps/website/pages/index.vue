<script setup lang="ts">
import { UiBadge, UiSurface } from '@octopus/ui'

import {
  editorialGraphics,
  homeNarrativeIds,
  interfaceProofs,
  proofMetricIds,
  scenarioIds,
} from '../content-or-copy/site'

const { t, tm } = useI18n()
const localePath = useLocalePath()

usePageSeo('home')

const proofItems = computed(() =>
  proofMetricIds.map((id) => ({
    label: t(`home.proofStrip.items.${id}.label`),
    value: t(`home.proofStrip.items.${id}.value`),
    description: t(`home.proofStrip.items.${id}.description`),
  })),
)

const narrativeSections = computed(() =>
  homeNarrativeIds.map((id) => ({
    id,
    eyebrow: t(`home.narrative.sections.${id}.eyebrow`),
    title: t(`home.narrative.sections.${id}.title`),
    description: t(`home.narrative.sections.${id}.description`),
    bullets: tm(`home.narrative.sections.${id}.bullets`) as string[],
  })),
)

const scenarioCards = computed(() =>
  scenarioIds.map((id) => ({
    id,
    title: t(`scenarios.groups.${id}.title`),
    description: t(`scenarios.groups.${id}.description`),
    bullets: tm(`scenarios.groups.${id}.bullets`) as string[],
  })),
)

const screenshotCards = computed(() =>
  interfaceProofs.map((item) => ({
    ...item,
    eyebrow: t(`media.${item.id}.eyebrow`),
    title: t(`media.${item.id}.title`),
    description: t(`media.${item.id}.description`),
    alt: t(`media.${item.id}.alt`),
  })),
)
</script>

<template>
  <div>
    <BrandHero
      :eyebrow="t('home.hero.eyebrow')"
      :title="t('home.hero.title')"
      :description="t('home.hero.description')"
      :note="t('home.hero.note')"
    >
      <template #actions>
        <SiteButtonLink :href="localePath('/book-demo')" :label="t('home.hero.primary')" size="lg" />
        <SiteButtonLink :href="localePath('/product')" :label="t('home.hero.secondary')" variant="outline" size="lg" />
      </template>

      <template #visual>
        <div class="grid gap-4">
          <UiSurface variant="subtle" padding="md" class="rounded-[24px] border border-border-subtle/70 bg-[color-mix(in_srgb,var(--bg-surface)_84%,transparent)]">
            <div class="flex flex-wrap items-center justify-between gap-3">
              <div>
                <div class="text-[12px] uppercase tracking-[0.14em] text-text-tertiary">{{ t('home.hero.quoteLabel') }}</div>
                <p class="mt-2 max-w-sm font-display text-[1.5rem] leading-tight text-text-primary">
                  {{ t('home.hero.quote') }}
                </p>
              </div>
              <UiBadge :label="t('home.hero.badge')" tone="info" />
            </div>
          </UiSurface>

          <div class="grid gap-4 md:grid-cols-2">
            <img
              src="/screenshots/dashboard.png"
              :alt="t('media.dashboard.alt')"
              class="w-full rounded-[24px] border border-border-subtle/80 shadow-sm"
            >
            <img
              :src="editorialGraphics.valueLoop"
              :alt="t('home.hero.diagramAlt')"
              class="w-full rounded-[24px] border border-border-subtle/80 shadow-sm"
            >
          </div>
        </div>
      </template>
    </BrandHero>

    <section class="brand-section pt-0">
      <div class="brand-container">
        <BrandSectionHeading
          :eyebrow="t('home.belief.eyebrow')"
          :title="t('home.belief.title')"
          :description="t('home.belief.description')"
        />
      </div>
    </section>

    <section class="brand-section pt-0">
      <div class="brand-container">
        <BrandProofStrip :items="proofItems" />
      </div>
    </section>

    <section class="brand-section">
      <div class="brand-container space-y-10">
        <BrandSectionHeading
          :eyebrow="t('home.narrative.eyebrow')"
          :title="t('home.narrative.title')"
          :description="t('home.narrative.description')"
        />

        <div class="grid gap-6 xl:grid-cols-3">
          <UiSurface
            v-for="section in narrativeSections"
            :key="section.id"
            variant="raised"
            padding="lg"
            class="brand-panel h-full"
          >
            <div class="space-y-5">
              <div class="brand-metadata">
                {{ section.eyebrow }}
              </div>
              <div class="space-y-3">
                <h3 class="text-[1.8rem]">
                  {{ section.title }}
                </h3>
                <p class="text-sm leading-7 text-text-secondary">
                  {{ section.description }}
                </p>
              </div>
              <ul class="space-y-3">
                <li
                  v-for="bullet in section.bullets"
                  :key="bullet"
                  class="flex gap-3 text-sm leading-7 text-text-secondary"
                >
                  <span class="mt-2 h-2 w-2 rounded-full bg-primary" />
                  <span>{{ bullet }}</span>
                </li>
              </ul>
            </div>
          </UiSurface>
        </div>
      </div>
    </section>

    <section class="brand-section">
      <div class="brand-container space-y-10">
        <BrandSectionHeading
          :eyebrow="t('home.scenarios.eyebrow')"
          :title="t('home.scenarios.title')"
          :description="t('home.scenarios.description')"
        />

        <div class="grid gap-6 lg:grid-cols-3">
          <UiSurface
            v-for="scenario in scenarioCards"
            :key="scenario.id"
            variant="raised"
            padding="lg"
            class="brand-panel"
          >
            <div class="space-y-4">
              <h3 class="text-[1.7rem]">{{ scenario.title }}</h3>
              <p class="text-sm leading-7 text-text-secondary">{{ scenario.description }}</p>
              <ul class="space-y-2">
                <li
                  v-for="bullet in scenario.bullets"
                  :key="bullet"
                  class="text-sm leading-7 text-text-secondary"
                >
                  {{ bullet }}
                </li>
              </ul>
            </div>
          </UiSurface>
        </div>

        <SiteButtonLink :href="localePath('/scenarios')" :label="t('home.scenarios.cta')" variant="outline" size="lg" />
      </div>
    </section>

    <section class="brand-section">
      <div class="brand-container space-y-10">
        <BrandSectionHeading
          :eyebrow="t('home.interface.eyebrow')"
          :title="t('home.interface.title')"
          :description="t('home.interface.description')"
        />

        <div class="grid gap-6 lg:grid-cols-2">
          <BrandScreenshotFrame
            v-for="item in screenshotCards"
            :key="item.id"
            :eyebrow="item.eyebrow"
            :title="item.title"
            :description="item.description"
            :src="item.src"
            :alt="item.alt"
          />
        </div>
      </div>
    </section>

    <section class="brand-section">
      <div class="brand-container">
        <BrandCallToAction
          :eyebrow="t('home.cta.eyebrow')"
          :title="t('home.cta.title')"
          :description="t('home.cta.description')"
          :primary-href="localePath('/book-demo')"
          :primary-label="t('home.cta.primary')"
          :secondary-href="localePath('/product')"
          :secondary-label="t('home.cta.secondary')"
        />
      </div>
    </section>
  </div>
</template>
