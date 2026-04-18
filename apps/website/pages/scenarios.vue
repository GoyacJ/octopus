<script setup lang="ts">
import { User, Users, Building2 } from 'lucide-vue-next'

interface ScenarioSegment {
  title: unknown
  desc: unknown
  features: unknown[]
}

const { t, tm, rt } = useI18n()

useHead({
  title: t('pages.scenarios.title')
})

const icons = [User, Users, Building2]
const segments = computed(() =>
  (tm('pages.scenarios.segments') as ScenarioSegment[]).map((segment, index) => ({
    title: rt(segment.title as Parameters<typeof rt>[0]),
    desc: rt(segment.desc as Parameters<typeof rt>[0]),
    features: segment.features.map((feature) => rt(feature as Parameters<typeof rt>[0])),
    icon: icons[index] ?? Building2,
  })),
)
</script>

<template>
  <div class="relative min-h-screen pb-24">
    <!-- Hero Section -->
    <UiSectionHero
      align="left"
      :badge="t('nav.scenarios')"
      :title="t('pages.scenarios.title')"
      :subtitle="t('pages.scenarios.body')"
    />

    <!-- Scenarios List -->
    <section class="section-padding relative">
      <div class="container-custom relative z-10">
        <div class="grid grid-cols-1 gap-8 lg:grid-cols-3">
          <UiCard
            v-for="seg in segments"
            :key="seg.title"
            variant="default"
            padding="lg"
            hover
            v-reveal
            class="flex h-full flex-col card-shine border-[var(--website-border-strong)] bg-[var(--website-surface)]"
          >
            <div class="mb-8 flex h-14 w-14 items-center justify-center rounded-2xl bg-[var(--website-accent)]/10 text-[var(--website-accent)] transition-transform duration-500 hover:rotate-12">
              <component :is="seg.icon" class="w-8 h-8" />
            </div>
            <h3 class="mb-4 text-2xl font-bold tracking-tight">{{ seg.title }}</h3>
            <p class="mb-8 flex-grow text-base font-medium leading-relaxed text-[var(--website-text-muted)] md:text-lg">
              {{ seg.desc }}
            </p>
            <div class="space-y-4 border-t border-[var(--website-border)] pt-6">
              <div v-for="feat in seg.features" :key="feat" class="flex items-center gap-3">
                <div class="h-2 w-2 rounded-full bg-[var(--website-accent)]"></div>
                <span class="text-sm font-semibold tracking-[0.12em] text-[var(--website-text)]/80">
                  {{ feat }}
                </span>
              </div>
            </div>
          </UiCard>
        </div>
      </div>
    </section>
  </div>
</template>
