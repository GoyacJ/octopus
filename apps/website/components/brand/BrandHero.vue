<script setup lang="ts">
interface Props {
  eyebrow: string
  title: string
  description: string
  note?: string
}

defineProps<Props>()
</script>

<template>
  <section class="brand-section pt-10 md:pt-16">
    <div class="brand-container grid gap-8 lg:grid-cols-[1.15fr_0.85fr] lg:items-end">
      <div class="space-y-6">
        <div class="brand-kicker">
          {{ eyebrow }}
        </div>
        <h1 class="brand-display">
          {{ title }}
        </h1>
        <p class="brand-lead">
          {{ description }}
        </p>
        <div class="flex flex-wrap gap-3">
          <slot name="actions" />
        </div>
      </div>

      <div class="brand-panel p-5 md:p-7">
        <div class="space-y-5">
          <div class="text-[12px] font-semibold uppercase tracking-[0.16em] text-text-tertiary">
            Octopus / Narrative
          </div>
          <div class="space-y-3">
            <p class="font-display text-[clamp(1.8rem,4vw,3rem)] leading-[1.02] text-text-primary">
              <span class="text-text-secondary">People direct the work.</span>
              <br>
              AI carries it forward.
            </p>
            <p class="max-w-md text-[15px] leading-7 text-text-secondary">
              {{ note }}
            </p>
          </div>
          <slot name="visual" />
        </div>
      </div>
    </div>
  </section>
</template>
