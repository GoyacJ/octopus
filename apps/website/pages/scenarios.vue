<script setup lang="ts">
import {
  BarChart3,
  Banknote,
  Building2,
  Code2,
  Palette,
  Rocket,
  User,
  Users,
} from 'lucide-vue-next'

interface ScenarioSegment {
  title: unknown
  desc: unknown
  features: unknown[]
}

interface ScenarioUseCase {
  category: unknown
  title: unknown
  task: unknown
  workflow: unknown[]
  tags: unknown[]
}

const { t, tm, rt } = useI18n()

useHead({
  title: t('pages.scenarios.title')
})

const segmentIcons = [User, Users, Building2]
const useCaseIcons = [BarChart3, Rocket, Users, Palette, Code2, Banknote]

const segments = computed(() =>
  (tm('pages.scenarios.segments') as ScenarioSegment[]).map((segment, index) => ({
    title: rt(segment.title as Parameters<typeof rt>[0]),
    desc: rt(segment.desc as Parameters<typeof rt>[0]),
    features: segment.features.map((feature) => rt(feature as Parameters<typeof rt>[0])),
    icon: segmentIcons[index] ?? Building2,
  })),
)

const useCases = computed(() =>
  (tm('pages.scenarios.useCases.items') as ScenarioUseCase[]).map((item, index) => ({
    category: rt(item.category as Parameters<typeof rt>[0]),
    title: rt(item.title as Parameters<typeof rt>[0]),
    task: rt(item.task as Parameters<typeof rt>[0]),
    workflow: item.workflow.map((step) => rt(step as Parameters<typeof rt>[0])),
    tags: item.tags.map((tag) => rt(tag as Parameters<typeof rt>[0])),
    icon: useCaseIcons[index] ?? BarChart3,
  })),
)
</script>

<template>
  <div class="relative min-h-screen pb-24">
    <UiSectionHero
      align="left"
      :badge="t('nav.scenarios')"
      :title="t('pages.scenarios.title')"
      :subtitle="t('pages.scenarios.body')"
    />

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

    <section class="section-padding bg-[var(--website-surface-soft)]/50">
      <div class="container-custom">
        <div class="max-w-3xl mb-14" v-reveal>
          <h2 class="text-4xl md:text-5xl font-bold tracking-tight mb-5">{{ t('pages.scenarios.useCases.title') }}</h2>
          <p class="text-xl text-[var(--website-text-muted)] leading-relaxed">{{ t('pages.scenarios.useCases.body') }}</p>
        </div>

        <div class="grid grid-cols-1 gap-8 lg:grid-cols-2 xl:grid-cols-3">
          <UiCard
            v-for="item in useCases"
            :key="item.title"
            variant="default"
            padding="lg"
            hover
            v-reveal
            class="card-shine border-[var(--website-border-strong)] bg-[var(--website-surface)]"
          >
            <div class="mb-5 flex h-14 w-14 items-center justify-center rounded-2xl bg-[var(--website-accent)]/10 text-[var(--website-accent)]">
              <component :is="item.icon" class="w-7 h-7" />
            </div>
            <p class="mb-2 text-sm font-bold uppercase tracking-[0.18em] text-[var(--website-accent)]/80">{{ item.category }}</p>
            <h3 class="mb-4 text-2xl font-bold tracking-tight">{{ item.title }}</h3>
            <p class="mb-6 text-base leading-relaxed text-[var(--website-text-muted)]">{{ item.task }}</p>

            <ol class="mb-6 space-y-3">
              <li v-for="(step, index) in item.workflow" :key="step" class="flex items-start gap-3">
                <span class="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-[var(--website-surface-soft)] text-xs font-bold text-[var(--website-accent)]">
                  {{ index + 1 }}
                </span>
                <span class="text-sm leading-relaxed text-[var(--website-text-muted)]">{{ step }}</span>
              </li>
            </ol>

            <div class="flex flex-wrap gap-2 border-t border-[var(--website-border)] pt-5">
              <span
                v-for="tag in item.tags"
                :key="tag"
                class="rounded-full border border-[var(--website-border)] bg-[var(--website-surface-soft)] px-3 py-1 text-xs font-semibold tracking-[0.12em] text-[var(--website-text-muted)]"
              >
                {{ tag }}
              </span>
            </div>
          </UiCard>
        </div>
      </div>
    </section>
  </div>
</template>
