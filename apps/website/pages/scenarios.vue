<script setup lang="ts">
import { UiSurface } from '@octopus/ui'

import { scenarioIds } from '../content-or-copy/site'

const { t, tm } = useI18n()
const localePath = useLocalePath()

usePageSeo('scenarios')

const scenarios = computed(() =>
  scenarioIds.map((id) => ({
    id,
    title: t(`scenarios.groups.${id}.title`),
    description: t(`scenarios.groups.${id}.description`),
    bullets: tm(`scenarios.groups.${id}.bullets`) as string[],
  })),
)
</script>

<template>
  <div>
    <section class="brand-section pt-16">
      <div class="brand-container space-y-6">
        <div class="brand-kicker">{{ t('scenarios.hero.eyebrow') }}</div>
        <h1 class="brand-display">{{ t('scenarios.hero.title') }}</h1>
        <p class="brand-lead">{{ t('scenarios.hero.description') }}</p>
      </div>
    </section>

    <section class="brand-section pt-0">
      <div class="brand-container space-y-10">
        <BrandSectionHeading
          :eyebrow="t('scenarios.matrix.eyebrow')"
          :title="t('scenarios.matrix.title')"
          :description="t('scenarios.matrix.description')"
        />

        <div class="grid gap-6 lg:grid-cols-3">
          <UiSurface
            v-for="scenario in scenarios"
            :key="scenario.id"
            variant="raised"
            padding="lg"
            class="brand-panel"
          >
            <div class="space-y-4">
              <div class="brand-metadata">
                {{ t(`scenarios.groups.${scenario.id}.eyebrow`) }}
              </div>
              <h2 class="text-[1.9rem]">{{ scenario.title }}</h2>
              <p class="text-sm leading-7 text-text-secondary">{{ scenario.description }}</p>
              <ul class="space-y-3">
                <li
                  v-for="bullet in scenario.bullets"
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
      <div class="brand-container">
        <BrandCallToAction
          :eyebrow="t('scenarios.cta.eyebrow')"
          :title="t('scenarios.cta.title')"
          :description="t('scenarios.cta.description')"
          :primary-href="localePath('/book-demo')"
          :primary-label="t('scenarios.cta.primary')"
          :secondary-href="localePath('/product')"
          :secondary-label="t('scenarios.cta.secondary')"
        />
      </div>
    </section>
  </div>
</template>
