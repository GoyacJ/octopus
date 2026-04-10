<script setup lang="ts">
const { t } = useI18n()

useHead({
  title: t('pages.about.title')
})

const principles = [
  {
    title: '本地优先与隐私保护',
    desc: '我们相信 AI 应当属于用户自己。通过本地端侧模型与私有化部署，确保数据隐私与控制权永远掌握在您手中。'
  },
  {
    title: '信创底座与自主可控',
    desc: '深度适配国产 IT 基础设施，支持全国产化替代。在安全合规的前提下，提供最先进的 AI 生产力工具。'
  },
  {
    title: 'Agent 协作与团队赋能',
    desc: 'AI 不应当只是工具，而应当成为团队的成员。我们致力于通过多 Agent 协同架构，让 AI 与人类团队无缝合作。'
  }
]
</script>

<template>
  <div class="relative min-h-screen pb-24">
    <!-- Hero Section -->
    <UiSectionHero
      align="left"
      badge="关于我们 (About Us)"
      :title="t('pages.about.title')"
      :subtitle="t('pages.about.body')"
    />

    <!-- Principles Section -->
    <section class="section-padding relative">
      <div class="container-custom relative z-10">
        <div class="mb-20" v-reveal>
          <h2 class="text-4xl font-bold mb-6 tracking-tight">我们的设计原则</h2>
          <p class="text-xl text-[var(--website-text-muted)] font-medium">驱动 Octopus 演进的核心思想与长期坚持。</p>
        </div>
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-16">
          <div v-for="p in principles" :key="p.title" v-reveal class="group">
            <h3 class="text-2xl font-bold mb-6 tracking-tight group-hover:text-[var(--website-accent)] transition-colors">{{ p.title }}</h3>
            <p class="text-[var(--website-text-muted)] text-base md:text-lg leading-relaxed font-medium">
              {{ p.desc }}
            </p>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>
