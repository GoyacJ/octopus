<script setup lang="ts">
import type { Component } from 'vue'
import { 
  ShieldCheck, 
  Cpu, 
  Users, 
  Heart, 
  Brain, 
  Lock,
  ArrowRight
} from 'lucide-vue-next'

const { t } = useI18n()

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

const features: HomeFeature[] = [
  {
    key: 'private',
    icon: ShieldCheck,
    cols: 7,
    variant: 'soft'
  },
  {
    key: 'localization',
    icon: Cpu,
    cols: 5,
    variant: 'soft'
  },
  {
    key: 'team',
    icon: Users,
    cols: 4,
    variant: 'outline'
  },
  {
    key: 'pet',
    icon: Heart,
    cols: 4,
    variant: 'soft'
  },
  {
    key: 'knowledge',
    icon: Brain,
    cols: 4,
    variant: 'default'
  },
  {
    key: 'security',
    icon: Lock,
    cols: 12,
    variant: 'outline'
  }
]
</script>

<template>
  <div class="relative min-h-screen">
    <!-- Global Decorative Elements -->
    <div class="glow-orb w-[600px] h-[600px] bg-orange-500 top-[-100px] left-[-300px]"></div>
    <div class="glow-orb w-[500px] h-[500px] bg-amber-500 bottom-[100px] right-[-200px]"></div>

    <!-- Hero Section -->
    <UiSectionHero
      align="split"
      :badge="t('site.status')"
      :title="t('pages.home.title')"
      :highlight="t('pages.home.highlight')"
      :subtitle="t('pages.home.subtitle')"
    >
      <template #actions>
        <UiButton to="/book-demo" size="lg" class="px-10 h-16 text-lg font-black group shadow-2xl shadow-[var(--website-accent)]/20">
          {{ t('pages.home.cta.primary') }}
          <ArrowRight class="ml-2 w-6 h-6 transition-transform group-hover:translate-x-1" />
        </UiButton>
        <UiButton to="/product" variant="outline" size="lg" class="px-10 h-16 text-lg glass font-bold border-[var(--website-border-strong)]">
          {{ t('pages.home.cta.secondary') }}
        </UiButton>
      </template>

      <template #visual>
        <div class="relative group perspective-1000">
          <!-- Main Image with complex effects -->
          <div class="absolute -inset-4 bg-gradient-to-tr from-[var(--website-accent)] to-amber-500 rounded-[3rem] blur-3xl opacity-10 group-hover:opacity-20 transition duration-1000"></div>
          <UiCard variant="glass" padding="none" class="shadow-[0_32px_64px_-16px_rgba(0,0,0,0.2)] relative overflow-hidden rounded-[2.5rem] border border-[var(--website-border-strong)] transform-gpu transition-all duration-700 group-hover:rotate-x-1 group-hover:rotate-y-1">
            <img src="/screenshots/dashboard.png" alt="Octopus Dashboard" class="w-full h-auto" />
            <div class="absolute inset-0 bg-gradient-to-t from-[var(--website-bg)]/30 via-transparent to-transparent pointer-events-none"></div>
          </UiCard>
        </div>
      </template>
    </UiSectionHero>

    <!-- Bento Grid Section -->
    <section class="section-padding relative">
      <div class="container-custom relative z-10">
        <div class="mb-20 text-center" v-reveal>
          <h2 class="text-4xl md:text-5xl font-bold mb-6 tracking-tight">{{ t('nav.product') }}</h2>
          <p class="text-xl text-[var(--website-text-muted)] max-w-2xl mx-auto leading-relaxed">
            {{ t('pages.product.body') }}
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

    <!-- Final CTA -->
    <section class="section-padding pt-0">
      <div class="container-custom">
        <div class="relative overflow-hidden rounded-[calc(var(--radius-xl)+8px)] bg-[var(--website-accent)] px-6 py-16 text-center text-white shadow-[0_24px_70px_-28px_rgba(249,115,22,0.55)] md:px-10 md:py-20">
          <!-- Background Pattern -->
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

<style scoped>
@keyframes fade-in {
  from { opacity: 0; transform: translateY(10px); }
  to { opacity: 1; transform: translateY(0); }
}
.animate-fade-in {
  animation: fade-in 0.8s ease-out forwards;
}
</style>
