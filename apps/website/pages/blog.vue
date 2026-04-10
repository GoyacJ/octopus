<script setup lang="ts">
const { t, locale } = useI18n()
useHead({ title: 'Blog - Octopus' })
</script>

<template>
  <div class="relative min-h-screen pb-24">
    <UiSectionHero
      align="center"
      title="Octopus Blog"
      :subtitle="locale === 'zh-CN' ? '来自构建私有 AI 未来团队的洞察、更新和故事。' : 'Insights, updates, and stories from the team building the future of private AI.'"
    >
      <template #visual>
        <div class="max-w-2xl mx-auto py-20 text-center" v-reveal>
          <div class="w-20 h-20 bg-[var(--website-accent)]/10 rounded-full flex items-center justify-center mx-auto mb-8">
            <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/></svg>
          </div>
          <h2 class="text-3xl font-bold mb-4">{{ locale === 'zh-CN' ? '即将上线' : 'Coming Soon' }}</h2>
          <p class="text-[var(--website-text-muted)]">
            {{ locale === 'zh-CN' ? '我们正在准备第一批文章。敬请期待！' : 'We are preparing our first set of articles. Stay tuned!' }}
          </p>
          <UiButton to="/" variant="outline" class="mt-10 glass px-8">{{ locale === 'zh-CN' ? '返回首页' : 'Back to Home' }}</UiButton>
        </div>
      </template>
    </UiSectionHero>
  </div>
</template>
