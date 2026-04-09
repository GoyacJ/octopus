<script setup lang="ts">
import { UiSurface } from '@octopus/ui'

import {
  editorialGraphics,
  interfaceProofs,
  productFeatureCards,
} from '../content-or-copy/site'

const { t, tm } = useI18n()
const localePath = useLocalePath()

usePageSeo('product')

const screenshotMap = new Map(interfaceProofs.map((item) => [item.id, item]))

const features = computed(() =>
  productFeatureCards.map((item) => ({
    id: item.id,
    title: t(`product.features.${item.id}.title`),
    description: t(`product.features.${item.id}.description`),
    bullets: tm(`product.features.${item.id}.bullets`) as string[],
    media: screenshotMap.get(item.mediaId)!,
  })),
)
</script>

<template>
  <div>
    <section class="brand-section pt-16">
      <div class="brand-container grid gap-10 lg:grid-cols-[1fr_0.9fr] lg:items-start">
        <div class="space-y-6">
          <div class="brand-kicker">{{ t('product.hero.eyebrow') }}</div>
          <h1 class="brand-display">{{ t('product.hero.title') }}</h1>
          <p class="brand-lead">{{ t('product.hero.description') }}</p>
          <div class="flex flex-wrap gap-3">
            <SiteButtonLink :href="localePath('/book-demo')" :label="t('product.hero.primary')" size="lg" />
            <SiteButtonLink :href="localePath('/scenarios')" :label="t('product.hero.secondary')" variant="outline" size="lg" />
          </div>
        </div>

        <div class="brand-panel overflow-hidden p-4">
          <img :src="editorialGraphics.platformLayers" :alt="t('product.hero.diagramAlt')" class="w-full rounded-[22px] border border-border-subtle/80 shadow-sm">
        </div>
      </div>
    </section>

    <section class="brand-section pt-0">
      <div class="brand-container space-y-10">
        <BrandSectionHeading
          :eyebrow="t('product.structure.eyebrow')"
          :title="t('product.structure.title')"
          :description="t('product.structure.description')"
        />

        <div class="grid gap-6">
          <div
            v-for="feature in features"
            :key="feature.id"
            class="grid gap-5 xl:grid-cols-[1.05fr_0.95fr]"
          >
            <UiSurface variant="raised" padding="lg" class="brand-panel">
              <div class="space-y-5">
                <div class="brand-metadata">
                  {{ t(`product.features.${feature.id}.eyebrow`) }}
                </div>
                <div class="space-y-3">
                  <h2 class="text-[2rem]">{{ feature.title }}</h2>
                  <p class="text-sm leading-7 text-text-secondary">{{ feature.description }}</p>
                </div>
                <ul class="space-y-3">
                  <li
                    v-for="bullet in feature.bullets"
                    :key="bullet"
                    class="flex gap-3 text-sm leading-7 text-text-secondary"
                  >
                    <span class="mt-2 h-2 w-2 rounded-full bg-primary" />
                    <span>{{ bullet }}</span>
                  </li>
                </ul>
              </div>
            </UiSurface>

            <BrandScreenshotFrame
              :eyebrow="t(`media.${feature.media.id}.eyebrow`)"
              :title="t(`media.${feature.media.id}.title`)"
              :description="t(`media.${feature.media.id}.description`)"
              :src="feature.media.src"
              :alt="t(`media.${feature.media.id}.alt`)"
            />
          </div>
        </div>
      </div>
    </section>

    <section class="brand-section">
      <div class="brand-container">
        <BrandCallToAction
          :eyebrow="t('product.cta.eyebrow')"
          :title="t('product.cta.title')"
          :description="t('product.cta.description')"
          :primary-href="localePath('/book-demo')"
          :primary-label="t('product.cta.primary')"
          :secondary-href="localePath('/about')"
          :secondary-label="t('product.cta.secondary')"
        />
      </div>
    </section>
  </div>
</template>
