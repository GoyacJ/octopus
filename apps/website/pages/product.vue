<script setup lang="ts">
import { 
  Puzzle, 
  Terminal, 
  Activity, 
  Layers, 
  Monitor, 
  ShieldAlert
} from 'lucide-vue-next'

const { t } = useI18n()

useHead({
  title: t('pages.product.title')
})

const capabilities = [
  { key: 'mcp', icon: Puzzle, img: '/screenshots/mcp.png' },
  { key: 'sandbox', icon: Terminal, img: '/screenshots/builtin.png' },
  { key: 'telemetry', icon: Activity, img: '/screenshots/conversation.png' },
  { key: 'plugin', icon: Layers, img: '/screenshots/skill.png' },
  { key: 'desktop', icon: Monitor, img: '/screenshots/dashboard.png' },
  { key: 'enterprise', icon: ShieldAlert, img: '/screenshots/rbac.png' }
]
</script>

<template>
  <div class="relative min-h-screen pb-24">
    <!-- Global Decorative Elements -->
    <div class="glow-orb w-[600px] h-[600px] bg-orange-500/20 top-[20%] right-[-300px]"></div>

    <!-- Hero Section -->
    <UiSectionHero
      align="split"
      :badge="t('pages.product.heroBadge')"
      :title="t('pages.product.heroTitle')"
      :highlight="t('pages.product.heroHighlight')"
      :subtitle="t('pages.product.body')"
    >
      <template #visual>
        <div class="relative group">
          <div class="absolute -inset-4 bg-gradient-to-tr from-orange-600 to-amber-400 rounded-[2rem] blur-2xl opacity-10"></div>
          <UiCard variant="glass" padding="none" class="shadow-2xl border-[var(--website-border-strong)] rounded-3xl overflow-hidden">
            <img src="/screenshots/agent.png" :alt="t('pages.product.title')" class="w-full h-auto" />
          </UiCard>
        </div>
      </template>
    </UiSectionHero>

    <!-- Capabilities Grid -->
    <section class="section-padding relative">
      <div class="container-custom relative z-10">
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-10">
          <div 
            v-for="(cap, index) in capabilities" 
            :key="cap.key"
            class="group"
            v-reveal
          >
            <UiCard padding="none" hover class="h-full flex flex-col overflow-hidden card-shine border-[var(--website-border-strong)]">
              <!-- Visual Preview -->
              <div class="aspect-video bg-[var(--website-surface-soft)] overflow-hidden relative border-b border-[var(--website-border)]">
                <img :src="cap.img" class="w-full h-full object-cover transition-transform duration-700 group-hover:scale-110" :alt="cap.key" />
                <div class="absolute inset-0 bg-gradient-to-t from-[var(--website-surface)]/60 to-transparent"></div>
              </div>
              
              <!-- Content -->
              <div class="p-6 flex-grow">
                <div class="w-10 h-10 rounded-lg bg-[var(--website-accent)]/10 flex items-center justify-center text-[var(--website-accent)] mb-4">
                  <component :is="cap.icon" class="w-5 h-5" />
                </div>
                <h3 class="text-lg font-bold mb-2">{{ t(`pages.product.features.${cap.key}.title`) }}</h3>
                <p class="text-[var(--website-text-muted)] text-sm leading-relaxed mb-6">
                  {{ t(`pages.product.features.${cap.key}.desc`) }}
                </p>
              </div>
            </UiCard>
          </div>
        </div>
      </div>
    </section>

    <!-- Advanced Governance Section -->
    <section class="section-padding bg-[var(--website-surface-soft)]/50">
      <div class="container-custom">
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-16 items-center">
          <div>
            <h2 class="text-3xl font-bold mb-8 tracking-tight">{{ t('pages.product.governance.title') }}</h2>
            <div class="space-y-8">
              <div v-for="i in (t('pages.product.governance.items') as any)" :key="i.title" class="flex gap-4" v-reveal>
                <div class="mt-1 w-5 h-5 rounded-full bg-green-500/20 flex items-center justify-center text-green-500 shrink-0">
                  <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
                </div>
                <div>
                  <h4 class="font-bold mb-1">{{ i.title }}</h4>
                  <p class="text-sm text-[var(--website-text-muted)] leading-relaxed">{{ i.desc }}</p>
                </div>
              </div>
            </div>
          </div>
          <div class="relative" v-reveal>
            <UiCard variant="glass" padding="none" class="shadow-2xl rotate-1 group transition-transform hover:rotate-0 border-[var(--website-border-strong)]">
              <img src="/screenshots/rbac.png" alt="Octopus RBAC" class="rounded-[var(--radius-l)]" />
              <!-- Badge Decoration -->
              <div class="absolute -bottom-6 -left-6 glass px-6 py-4 rounded-2xl border border-[var(--website-border-strong)] shadow-xl">
                <div class="flex items-center gap-3">
                  <div class="w-3 h-3 rounded-full bg-green-500 animate-pulse"></div>
                  <span class="text-sm font-bold tracking-tight">Governance Ready</span>
                </div>
              </div>
            </UiCard>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>
