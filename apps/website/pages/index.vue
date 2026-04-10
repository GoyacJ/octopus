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
  rows: number
  variant: FeatureVariant
}

const features: HomeFeature[] = [
  {
    key: 'private',
    icon: ShieldCheck,
    cols: 6,
    rows: 2,
    variant: 'glass'
  },
  {
    key: 'localization',
    icon: Cpu,
    cols: 6,
    rows: 1,
    variant: 'soft'
  },
  {
    key: 'team',
    icon: Users,
    cols: 3,
    rows: 1,
    variant: 'outline'
  },
  {
    key: 'pet',
    icon: Heart,
    cols: 3,
    rows: 1,
    variant: 'soft'
  },
  {
    key: 'knowledge',
    icon: Brain,
    cols: 6,
    rows: 1,
    variant: 'default'
  },
  {
    key: 'security',
    icon: Lock,
    cols: 6,
    rows: 1,
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
      title="构建您的私有化"
      highlight="数字员工团队"
      subtitle="在信创环境或私有云中，部署安全、可控、极速的 AI 专家集群与组织知识大脑。"
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
            v-for="(feature, index) in features"
            :key="feature.key"
            :cols="feature.cols"
            :rows="feature.rows"
            :variant="feature.variant"
            v-reveal
            class="card-shine"
          >
            <div class="p-10 h-full flex flex-col group">
              <div class="mb-auto">
                <div class="w-14 h-14 rounded-2xl bg-[var(--website-accent)]/10 flex items-center justify-center text-[var(--website-accent)] mb-8 transition-all duration-500 group-hover:bg-[var(--website-accent)] group-hover:text-white group-hover:rotate-[10deg]">
                  <component :is="feature.icon" class="w-7 h-7" />
                </div>
                <h3 class="text-2xl font-bold mb-4 tracking-tight">{{ t(`pages.home.features.${feature.key}.title`) }}</h3>
                <p class="text-[var(--website-text-muted)] leading-relaxed text-base md:text-lg opacity-80 group-hover:opacity-100">
                  {{ t(`pages.home.features.${feature.key}.desc`) }}
                </p>
              </div>
              <div class="mt-10 flex items-center text-[var(--website-accent)] text-sm font-bold tracking-widest uppercase opacity-0 group-hover:opacity-100 transition-all transform translate-x-[-10px] group-hover:translate-x-0">
                DISCOVER MORE <ArrowRight class="ml-2 w-4 h-4" />
              </div>
            </div>
          </UiBentoItem>
        </UiBentoGrid>
      </div>
    </section>

    <!-- Final CTA -->
    <section class="section-padding">
      <div class="container-custom">
        <UiCard variant="glass" class="bg-[var(--website-accent)] text-white text-center py-20 overflow-hidden relative">
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
        </UiCard>
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
