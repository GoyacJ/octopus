<script setup lang="ts">
const { t, locale } = useI18n()
useHead({ title: 'Documentation - Octopus' })
</script>

<template>
  <div class="relative min-h-screen pb-24">
    <UiSectionHero
      align="center"
      title="Documentation"
      :subtitle="locale === 'zh-CN' ? '关于部署和使用 Octopus 您需要了解的一切。' : 'Everything you need to know about deploying and using Octopus.'"
    >
      <template #visual>
        <div class="max-w-2xl mx-auto py-20 text-center" v-reveal>
          <div class="w-20 h-20 bg-[var(--website-accent)]/10 rounded-full flex items-center justify-center mx-auto mb-8">
            <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"/><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"/></svg>
          </div>
          <h2 class="text-3xl font-bold mb-4">{{ locale === 'zh-CN' ? '文档编写中' : 'Under Construction' }}</h2>
          <p class="text-[var(--website-text-muted)]">
            {{ locale === 'zh-CN' ? '我们的技术文档正在最后完善中。请稍后查看指南和 API 参考。' : 'Our technical documentation is being finalized. Check back soon for guides and API references.' }}
          </p>
          <UiButton to="/" variant="outline" class="mt-10 glass px-8">{{ locale === 'zh-CN' ? '返回首页' : 'Back to Home' }}</UiButton>
        </div>
      </template>
    </UiSectionHero>
  </div>
</template>
