<script setup lang="ts">
import type { Component } from 'vue'
import {
  ArrowRight,
  BarChart3,
  Check,
  CheckCircle2,
  Clock3,
  Code2,
  Eye,
  Landmark,
  Layers,
  Link2,
  PenLine,
  Shield,
  X,
  Zap,
} from 'lucide-vue-next'

const { t, tm, rt } = useI18n()

useHead({
  title: t('pages.home.title')
})

type FeatureVariant = 'default' | 'soft' | 'outline' | 'glass'

interface HomeFeature {
  key: string
  icon: Component
  cols: number
  variant: FeatureVariant
}

interface WorkflowStep {
  title: unknown
  heading: unknown
  desc: unknown
}

interface ComparisonItem {
  chat: unknown
  octopus: unknown
}

interface UseCaseItem {
  category: unknown
  title: unknown
  task: unknown
  tags: unknown[]
}

interface FaqItem {
  question: unknown
  answer: unknown
}

const features: HomeFeature[] = [
  { key: 'taskAgent', icon: Link2, cols: 7, variant: 'soft' },
  { key: 'local', icon: Shield, cols: 5, variant: 'soft' },
  { key: 'parallel', icon: Zap, cols: 4, variant: 'outline' },
  { key: 'context', icon: Layers, cols: 4, variant: 'default' },
  { key: 'automation', icon: Clock3, cols: 4, variant: 'soft' },
  { key: 'visibility', icon: Eye, cols: 12, variant: 'outline' }
]

const workflowIcons = [PenLine, Zap, CheckCircle2]
const useCaseIcons = [BarChart3, Code2, Landmark]

const workflowSteps = computed(() =>
  (tm('pages.home.workflow.steps') as WorkflowStep[]).map((step, index) => ({
    title: rt(step.title as Parameters<typeof rt>[0]),
    heading: rt(step.heading as Parameters<typeof rt>[0]),
    desc: rt(step.desc as Parameters<typeof rt>[0]),
    icon: workflowIcons[index] ?? CheckCircle2,
  })),
)

const comparisonItems = computed(() =>
  (tm('pages.home.comparison.items') as ComparisonItem[]).map((item) => ({
    chat: rt(item.chat as Parameters<typeof rt>[0]),
    octopus: rt(item.octopus as Parameters<typeof rt>[0]),
  })),
)

const useCases = computed(() =>
  (tm('pages.home.useCases.items') as UseCaseItem[]).map((item, index) => ({
    category: rt(item.category as Parameters<typeof rt>[0]),
    title: rt(item.title as Parameters<typeof rt>[0]),
    task: rt(item.task as Parameters<typeof rt>[0]),
    tags: item.tags.map((tag) => rt(tag as Parameters<typeof rt>[0])),
    icon: useCaseIcons[index] ?? Landmark,
  })),
)

const faqs = computed(() =>
  (tm('pages.home.faq.items') as FaqItem[]).map((item) => ({
    question: rt(item.question as Parameters<typeof rt>[0]),
    answer: rt(item.answer as Parameters<typeof rt>[0]),
  })),
)
</script>

<template>
  <div class="relative min-h-screen">
    <div class="glow-orb w-[600px] h-[600px] bg-orange-500 top-[-100px] left-[-300px]"></div>
    <div class="glow-orb w-[500px] h-[500px] bg-amber-500 bottom-[100px] right-[-200px]"></div>

    <UiSectionHero
      align="split"
      :badge="t('pages.home.badge')"
      :title="t('pages.home.title')"
      :highlight="t('pages.home.highlight')"
      :subtitle="t('pages.home.subtitle')"
    >
      <template #actions>
        <UiButton to="/download" size="lg" class="px-10 h-16 text-lg font-black group shadow-2xl shadow-[var(--website-accent)]/20">
          {{ t('pages.home.cta.primary') }}
          <ArrowRight class="ml-2 w-6 h-6 transition-transform group-hover:translate-x-1" />
        </UiButton>
        <UiButton to="/product" variant="outline" size="lg" class="px-10 h-16 text-lg glass font-bold border-[var(--website-border-strong)]">
          {{ t('pages.home.cta.secondary') }}
        </UiButton>
      </template>

      <template #visual>
        <div class="relative group perspective-1000">
          <div class="absolute -inset-4 bg-gradient-to-tr from-[var(--website-accent)] to-amber-500 rounded-[3rem] blur-3xl opacity-10 group-hover:opacity-20 transition duration-1000"></div>
          <UiCard variant="glass" padding="none" class="shadow-[0_32px_64px_-16px_rgba(0,0,0,0.2)] relative overflow-hidden rounded-[2.5rem] border border-[var(--website-border-strong)] transform-gpu transition-all duration-700 group-hover:rotate-x-1 group-hover:rotate-y-1">
            <img src="/screenshots/dashboard.png" alt="Octopus Dashboard" class="w-full h-auto" />
            <div class="absolute inset-0 bg-gradient-to-t from-[var(--website-bg)]/30 via-transparent to-transparent pointer-events-none"></div>
          </UiCard>
        </div>
      </template>
    </UiSectionHero>

    <section class="section-padding relative">
      <div class="container-custom relative z-10">
        <div class="mb-20 text-center" v-reveal>
          <h2 class="text-4xl md:text-5xl font-bold mb-6 tracking-tight">{{ t('pages.home.foundation.title') }}</h2>
          <p class="text-xl text-[var(--website-text-muted)] max-w-3xl mx-auto leading-relaxed">
            {{ t('pages.home.foundation.body') }}
          </p>
        </div>

        <UiBentoGrid>
          <UiBentoItem
            v-for="feature in features"
            :key="feature.key"
            :cols="feature.cols"
            :variant="feature.variant"
            v-reveal
            class="card-shine"
          >
            <div class="flex h-full flex-col p-8 md:p-10 group">
              <div class="mb-6 flex h-14 w-14 items-center justify-center rounded-2xl bg-[var(--website-accent)]/10 text-[var(--website-accent)] transition-all duration-500 group-hover:bg-[var(--website-accent)] group-hover:text-white group-hover:rotate-[10deg]">
                <component :is="feature.icon" class="w-7 h-7" />
              </div>
              <h3 class="mb-4 text-2xl font-bold tracking-tight">{{ t(`pages.home.features.${feature.key}.title`) }}</h3>
              <p class="text-base leading-relaxed text-[var(--website-text-muted)] md:text-lg">
                {{ t(`pages.home.features.${feature.key}.desc`) }}
              </p>
            </div>
          </UiBentoItem>
        </UiBentoGrid>
      </div>
    </section>

    <section class="section-padding bg-[var(--website-surface-soft)]/50">
      <div class="container-custom">
        <div class="max-w-2xl mb-14" v-reveal>
          <h2 class="text-4xl md:text-5xl font-bold tracking-tight mb-5">{{ t('pages.home.workflow.title') }}</h2>
          <p class="text-xl text-[var(--website-text-muted)] leading-relaxed">{{ t('pages.home.workflow.body') }}</p>
        </div>
        <div class="grid grid-cols-1 gap-8 md:grid-cols-3">
          <UiCard
            v-for="(step, index) in workflowSteps"
            :key="step.heading"
            variant="default"
            padding="lg"
            v-reveal
            class="relative border-[var(--website-border-strong)] bg-[var(--website-surface)]"
          >
            <div class="absolute -top-4 left-6 flex h-10 w-10 items-center justify-center rounded-full bg-[var(--website-accent)] text-white text-sm font-bold shadow-lg">
              {{ index + 1 }}
            </div>
            <div class="pt-4">
              <div class="mb-5 flex h-14 w-14 items-center justify-center rounded-2xl bg-[var(--website-accent)]/10 text-[var(--website-accent)]">
                <component :is="step.icon" class="w-7 h-7" />
              </div>
              <p class="mb-2 text-sm font-bold uppercase tracking-[0.18em] text-[var(--website-accent)]/80">{{ step.title }}</p>
              <h3 class="mb-3 text-2xl font-bold tracking-tight">{{ step.heading }}</h3>
              <p class="text-base leading-relaxed text-[var(--website-text-muted)]">{{ step.desc }}</p>
            </div>
          </UiCard>
        </div>
      </div>
    </section>

    <section class="section-padding">
      <div class="container-custom">
        <div class="max-w-3xl mb-14" v-reveal>
          <h2 class="text-4xl md:text-5xl font-bold tracking-tight mb-5">{{ t('pages.home.comparison.title') }}</h2>
          <p class="text-xl text-[var(--website-text-muted)] leading-relaxed">{{ t('pages.home.comparison.body') }}</p>
        </div>
        <div class="grid grid-cols-1 gap-8 lg:grid-cols-2">
          <UiCard padding="lg" v-reveal class="border-[var(--website-border-strong)] bg-[var(--website-surface)]">
            <p class="mb-6 text-sm font-bold uppercase tracking-[0.18em] text-[var(--website-text-muted)]">{{ t('pages.home.comparison.chatLabel') }}</p>
            <div class="space-y-4">
              <div v-for="item in comparisonItems" :key="item.chat" class="flex items-start gap-3">
                <div class="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-rose-500/10 text-rose-500">
                  <X class="w-4 h-4" />
                </div>
                <p class="text-base leading-relaxed text-[var(--website-text-muted)]">{{ item.chat }}</p>
              </div>
            </div>
          </UiCard>
          <UiCard padding="lg" v-reveal class="border-[var(--website-border-strong)] bg-[var(--website-accent)]/[0.06]">
            <p class="mb-6 text-sm font-bold uppercase tracking-[0.18em] text-[var(--website-accent)]">{{ t('pages.home.comparison.octopusLabel') }}</p>
            <div class="space-y-4">
              <div v-for="item in comparisonItems" :key="item.octopus" class="flex items-start gap-3">
                <div class="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-[var(--website-accent)]/10 text-[var(--website-accent)]">
                  <Check class="w-4 h-4" />
                </div>
                <p class="text-base leading-relaxed text-[var(--website-text)]">{{ item.octopus }}</p>
              </div>
            </div>
          </UiCard>
        </div>
      </div>
    </section>

    <section class="section-padding bg-[var(--website-surface-soft)]/60">
      <div class="container-custom">
        <div class="max-w-3xl mb-14" v-reveal>
          <h2 class="text-4xl md:text-5xl font-bold tracking-tight mb-5">{{ t('pages.home.useCases.title') }}</h2>
          <p class="text-xl text-[var(--website-text-muted)] leading-relaxed">{{ t('pages.home.useCases.body') }}</p>
        </div>
        <div class="grid grid-cols-1 gap-8 lg:grid-cols-3">
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
            <div class="flex flex-wrap gap-2">
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

    <section class="section-padding">
      <div class="container-custom">
        <div class="max-w-3xl mb-14" v-reveal>
          <h2 class="text-4xl md:text-5xl font-bold tracking-tight mb-5">{{ t('pages.home.faq.title') }}</h2>
        </div>
        <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
          <details
            v-for="faq in faqs"
            :key="faq.question"
            v-reveal
            class="group rounded-[var(--radius-xl)] border border-[var(--website-border-strong)] bg-[var(--website-surface)] p-6"
          >
            <summary class="cursor-pointer list-none text-lg font-bold tracking-tight marker:hidden">
              <div class="flex items-start justify-between gap-6">
                <span>{{ faq.question }}</span>
                <span class="text-[var(--website-accent)] transition-transform duration-300 group-open:rotate-45">+</span>
              </div>
            </summary>
            <p class="mt-4 pr-8 text-base leading-relaxed text-[var(--website-text-muted)]">{{ faq.answer }}</p>
          </details>
        </div>
      </div>
    </section>

    <section class="section-padding pt-0">
      <div class="container-custom">
        <div class="relative overflow-hidden rounded-[calc(var(--radius-xl)+8px)] bg-[var(--website-accent)] px-6 py-16 text-center text-white shadow-[0_24px_70px_-28px_rgba(249,115,22,0.55)] md:px-10 md:py-20">
          <div class="absolute inset-0 opacity-10 pointer-events-none">
            <div class="absolute top-0 left-0 w-full h-full bg-[radial-gradient(circle_at_center,_white_1px,_transparent_1px)] bg-[size:24px_24px]"></div>
          </div>

          <div class="relative z-10 max-w-2xl mx-auto px-6">
            <h2 class="text-4xl md:text-5xl font-bold mb-6">{{ t('pages.home.consulting.title') }}</h2>
            <p class="text-white/80 text-lg mb-10 leading-relaxed">
              {{ t('pages.home.consulting.body') }}
            </p>
            <UiButton to="/book-demo" variant="secondary" size="lg" class="bg-white text-[var(--website-accent)] hover:bg-white/90 whitespace-nowrap">
              {{ t('pages.home.consulting.button') }}
            </UiButton>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>
