<script setup lang="ts">
interface Props {
  badge?: string
  title: string
  highlight?: string
  subtitle?: string
  align?: 'left' | 'center' | 'split'
  showGlow?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  align: 'split',
  showGlow: true
})
</script>

<template>
  <section class="relative overflow-hidden pt-32 pb-20 md:pt-40 md:pb-32 lg:pt-52 lg:pb-40">
    <!-- Premium Background -->
    <div v-if="showGlow" class="absolute inset-0 z-0 pointer-events-none">
      <div class="absolute top-[-10%] left-[-10%] w-[60%] h-[60%] bg-[var(--website-accent)] opacity-[0.07] blur-[120px] rounded-full animate-pulse"></div>
      <div class="absolute bottom-[-10%] right-[-10%] w-[50%] h-[50%] bg-amber-500 opacity-[0.05] blur-[100px] rounded-full"></div>
    </div>
    <div class="bg-grid opacity-40"></div>
    <div class="bg-noise"></div>

    <div class="container-custom relative z-10">
      <!-- Split Layout (Left Content, Right Visual) -->
      <div v-if="align === 'split'" class="grid grid-cols-1 lg:grid-cols-12 gap-16 items-center">
        <div class="lg:col-span-6 flex flex-col items-start text-left" v-reveal>
          <UiBadge v-if="badge" variant="glass" class="mb-8 px-4 py-1.5 border-[var(--website-border-strong)] shadow-lg text-[10px] font-black tracking-[0.3em] uppercase">
            {{ badge }}
          </UiBadge>
          
          <h1 class="text-hero-title mb-8">
            <span class="block">{{ title }}</span>
            <span v-if="highlight" class="text-highlight">{{ highlight }}</span>
          </h1>
          
          <p v-if="subtitle" class="text-hero-description max-w-xl mb-12">
            {{ subtitle }}
          </p>

          <div class="flex flex-wrap gap-5">
            <slot name="actions" />
          </div>
        </div>

        <div class="lg:col-span-6 relative w-full" v-reveal>
          <!-- Decorative Floating Elements -->
          <div class="absolute -top-10 -right-10 w-32 h-32 glass rounded-3xl rotate-12 z-20 hidden md:flex items-center justify-center animate-float shadow-2xl border-[var(--website-border-strong)]">
            <div class="text-center">
              <p class="text-[10px] font-bold opacity-50 uppercase tracking-tighter">Security</p>
              <p class="text-xl font-black text-green-500">100%</p>
            </div>
          </div>
          <div class="absolute -bottom-10 -left-10 w-48 h-20 glass rounded-2xl -rotate-6 z-20 hidden md:flex items-center px-6 gap-4 animate-float shadow-2xl border-[var(--website-border-strong)]" style="animation-delay: -2s">
            <div class="w-8 h-8 rounded-full bg-[var(--website-accent)]/20 flex items-center justify-center">
              <div class="w-2 h-2 rounded-full bg-[var(--website-accent)] animate-ping"></div>
            </div>
            <p class="text-xs font-bold tracking-tight">Agent Syncing...</p>
          </div>

          <div class="relative z-10">
            <slot name="visual" />
          </div>
        </div>
      </div>

      <!-- Classic Centered Layout -->
      <div v-else :class="[
        align === 'center' ? 'mx-auto text-center items-center' : 'text-left items-start',
        'flex flex-col max-w-5xl mx-auto'
      ]" v-reveal>
        <UiBadge v-if="badge" variant="glass" class="mb-8 px-4 py-1.5 shadow-xl text-xs font-bold tracking-[0.2em] uppercase">
          {{ badge }}
        </UiBadge>
        <h1 class="text-hero-title mb-8">{{ title }}</h1>
        <p v-if="subtitle" class="text-hero-description max-w-2xl mb-12" :class="align === 'center' ? 'mx-auto' : ''">
          {{ subtitle }}
        </p>
        <div class="flex flex-wrap gap-4 mb-20">
          <slot name="actions" />
        </div>
        <div class="w-full">
          <slot name="visual" />
        </div>
      </div>
    </div>
  </section>
</template>
