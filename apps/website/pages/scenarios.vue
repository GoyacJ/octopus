<script setup lang="ts">
import { User, Users, Building2 } from 'lucide-vue-next'
const { t } = useI18n()

useHead({
  title: t('pages.scenarios.title')
})

const icons = [User, Users, Building2]
const segments = (t('pages.scenarios.segments') as any[]).map((seg, i) => ({
  ...seg,
  icon: icons[i]
}))
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
        <div class="grid grid-cols-1 lg:grid-cols-3 gap-10">
          <UiCard v-for="seg in segments" :key="seg.title" variant="default" padding="lg" hover v-reveal class="flex flex-col card-shine border-[var(--website-border-strong)]">
            <div class="w-16 h-16 rounded-2xl bg-[var(--website-accent)]/10 flex items-center justify-center text-[var(--website-accent)] mb-10 transition-transform duration-500 hover:rotate-12">
              <component :is="seg.icon" class="w-8 h-8" />
            </div>
            <h3 class="text-3xl font-bold mb-6 tracking-tight">{{ seg.title }}</h3>
            <p class="text-[var(--website-text-muted)] text-base md:text-lg leading-relaxed mb-12 flex-grow font-medium">
              {{ seg.desc }}
            </p>
            <div class="space-y-5 pt-8 border-t border-[var(--website-border)]">
              <div v-for="feat in seg.features" :key="feat" class="flex items-center gap-4">
                <div class="w-2 h-2 rounded-full bg-[var(--website-accent)] shadow-[0_0_10px_var(--website-accent)]"></div>
                <span class="text-sm font-bold uppercase tracking-widest text-[var(--website-text)]">{{ feat }}</span>
              </div>
            </div>
          </UiCard>
        </div>
      </div>
    </section>
  </div>
</template>
