<script setup lang="ts">
interface PrincipleItem {
  title: unknown
  desc: unknown
}

const { t, tm, rt } = useI18n()

useHead({
  title: t('pages.about.title')
})

const highlights = computed(() =>
  (tm('pages.about.highlights') as unknown[]).map((item) =>
    rt(item as Parameters<typeof rt>[0]),
  ),
)

const principles = computed(() =>
  (tm('pages.about.principles.items') as PrincipleItem[]).map((item) => ({
    title: rt(item.title as Parameters<typeof rt>[0]),
    desc: rt(item.desc as Parameters<typeof rt>[0]),
  })),
)
</script>

<template>
  <div class="relative min-h-screen pb-24">
    <UiSectionHero
      align="left"
      :badge="t('nav.about')"
      :title="t('pages.about.title')"
      :subtitle="t('pages.about.body')"
    />

    <section class="section-padding relative">
      <div class="container-custom relative z-10">
        <div class="grid grid-cols-1 gap-12 lg:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)] lg:items-start">
          <div v-reveal>
            <p class="mb-3 text-sm font-bold uppercase tracking-[0.18em] text-[var(--website-accent)]/80">{{ t('pages.about.story.title') }}</p>
            <h2 class="mb-6 text-4xl font-bold tracking-tight md:text-5xl">{{ t('pages.about.body') }}</h2>
          </div>
          <UiCard
            variant="default"
            padding="lg"
            v-reveal
            class="border-[var(--website-border-strong)] bg-[var(--website-surface)] shadow-[0_28px_70px_-36px_rgba(0,0,0,0.28)]"
          >
            <p class="mb-5 text-lg leading-relaxed text-[var(--website-text)]">{{ t('pages.about.story.lead') }}</p>
            <p class="text-base leading-relaxed text-[var(--website-text-muted)]">{{ t('pages.about.story.detail') }}</p>
            <div class="mt-8 flex flex-wrap gap-3">
              <span
                v-for="item in highlights"
                :key="item"
                class="rounded-full border border-[var(--website-border)] bg-[var(--website-surface-soft)] px-4 py-2 text-sm font-semibold tracking-[0.08em] text-[var(--website-text-muted)]"
              >
                {{ item }}
              </span>
            </div>
          </UiCard>
        </div>
      </div>
    </section>

    <section class="section-padding bg-[var(--website-surface-soft)]/50">
      <div class="container-custom">
        <div class="mb-16 max-w-3xl" v-reveal>
          <h2 class="mb-5 text-4xl font-bold tracking-tight md:text-5xl">{{ t('pages.about.principles.title') }}</h2>
          <p class="text-xl leading-relaxed text-[var(--website-text-muted)]">{{ t('pages.about.principles.body') }}</p>
        </div>
        <div class="grid grid-cols-1 gap-8 md:grid-cols-2">
          <UiCard
            v-for="principle in principles"
            :key="principle.title"
            variant="default"
            padding="lg"
            hover
            v-reveal
            class="card-shine border-[var(--website-border-strong)] bg-[var(--website-surface)]"
          >
            <h3 class="mb-4 text-2xl font-bold tracking-tight">{{ principle.title }}</h3>
            <p class="text-base leading-relaxed text-[var(--website-text-muted)]">{{ principle.desc }}</p>
          </UiCard>
        </div>
      </div>
    </section>
  </div>
</template>
