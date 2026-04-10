<script setup lang="ts">
import { 
  Apple, 
  Monitor as Windows, 
  Terminal as Linux, 
  ExternalLink, 
  ChevronRight, 
  Download 
} from 'lucide-vue-next'

const { t } = useI18n()
const version = '1.0.2' // Mock version, should come from config or fetch

useHead({
  title: t('pages.download.title')
})

const platforms = [
  {
    id: 'macos',
    name: t('pages.download.platforms.macos'),
    icon: Apple,
    file: 'Octopus_1.0.2_x64.dmg',
    hint: t('pages.download.hints.macos'),
    primary: true
  },
  {
    id: 'windows',
    name: t('pages.download.platforms.windows'),
    icon: Windows,
    file: 'Octopus_1.0.2_x64_en-US.msi',
    hint: t('pages.download.hints.windows'),
    primary: false
  },
  {
    id: 'linux',
    name: t('pages.download.platforms.linux'),
    icon: Linux,
    file: 'Octopus_1.0.2_amd64.AppImage',
    hint: t('pages.download.hints.linux'),
    primary: false
  }
]
</script>

<template>
  <div class="relative min-h-screen pb-24">
    <!-- Hero Download -->
    <UiSectionHero
      :badge="t('pages.download.version', { version })"
      :title="t('pages.download.title')"
      :subtitle="t('pages.download.subtitle')"
    >
      <template #visual>
        <div class="max-w-6xl mx-auto -mt-8">
          <!-- Platform Grid -->
          <div class="grid grid-cols-1 md:grid-cols-3 gap-8 mb-16">
            <UiCard 
              v-for="platform in platforms" 
              :key="platform.id" 
              :variant="platform.primary ? 'glass' : 'default'" 
              padding="lg" 
              hover
              class="group flex flex-col items-center text-center card-shine border-[var(--website-border-strong)]"
            >
              <div 
                class="w-16 h-16 rounded-2xl flex items-center justify-center mb-8 transition-all duration-300"
                :class="platform.primary ? 'bg-[var(--website-accent)] text-white' : 'bg-[var(--website-surface-soft)] text-[var(--website-text-muted)] group-hover:bg-[var(--website-accent)]/10 group-hover:text-[var(--website-accent)]'"
              >
                <component :is="platform.icon" class="w-8 h-8" />
              </div>
              
              <h3 class="text-xl font-bold mb-2">{{ platform.name }}</h3>
              <p class="text-sm text-[var(--website-text-muted)] mb-8">{{ platform.hint }}</p>
              
              <UiButton 
                :variant="platform.primary ? 'primary' : 'outline'" 
                class="w-full"
              >
                <Download class="w-4 h-4 mr-2" />
                {{ t('pages.download.cta') }}
              </UiButton>
              
              <p class="mt-4 text-[10px] text-[var(--website-text-muted)] font-mono opacity-50">
                {{ platform.file }}
              </p>
            </UiCard>
          </div>

          <!-- Footer Help -->
          <div class="text-center text-[var(--website-text-muted)]" v-reveal>
            <p class="text-sm mb-4">{{ t('pages.download.requirements') }}</p>
            <div class="flex items-center justify-center gap-6">
              <a href="#" class="text-xs font-semibold hover:text-[var(--website-accent)] flex items-center">
                <ExternalLink class="w-3 h-3 mr-1" />
                {{ t('pages.download.changelog') }}
              </a>
              <a href="#" class="text-xs font-semibold hover:text-[var(--website-accent)] flex items-center">
                {{ t('pages.download.support') }}
                <ChevronRight class="w-3 h-3 ml-1" />
              </a>
            </div>
          </div>
        </div>
      </template>
    </UiSectionHero>

    <!-- Visual Hint -->
    <section class="section-padding relative">
      <div class="container-custom relative z-10">
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-16 items-center">
          <div class="relative" v-reveal>
            <img src="/images/dashboard.png" class="rounded-[var(--radius-xl)] shadow-2xl border border-[var(--website-border)]" alt="Octopus Running" />
            <div class="absolute -top-4 -right-4 glass p-4 rounded-xl border border-[var(--website-border-strong)] animate-bounce-slow">
              <span class="text-xs font-bold">{{ t('pages.download.newVersion') }}</span>
            </div>
          </div>
          <div class="max-w-md" v-reveal>
            <h2 class="text-3xl font-bold mb-6 tracking-tight">{{ t('pages.download.experience.title') }}</h2>
            <p class="text-[var(--website-text-muted)] leading-relaxed mb-8">
              {{ t('pages.download.experience.desc') }}
            </p>
            <ul class="space-y-4">
              <li v-for="i in ['fastBoot', 'lowMemory', 'privacy']" :key="i" class="flex items-center gap-3">
                <div class="w-5 h-5 rounded-full bg-[var(--website-accent)]/10 flex items-center justify-center text-[var(--website-accent)]">
                  <ChevronRight class="w-4 h-4" />
                </div>
                <span class="text-sm font-semibold tracking-tight">{{ t(`pages.download.experience.bullets.${i}`) }}</span>
              </li>
            </ul>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.animate-bounce-slow {
  animation: bounce 3s infinite;
}
@keyframes bounce {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-10px); }
}
</style>
