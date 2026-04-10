<script setup lang="ts">
const { t, locale } = useI18n()
useHead({ title: t('footer.terms') + ' - Octopus' })
</script>

<template>
  <div class="relative min-h-screen pb-24">
    <UiSectionHero
      align="left"
      :title="t('footer.terms')"
      :subtitle="locale === 'zh-CN' ? '最近更新：2026年4月10日' : 'Last updated: April 10, 2026'"
    />
    <section class="section-padding">
      <div class="container-custom max-w-4xl prose dark:prose-invert prose-orange font-medium">
        <template v-if="locale === 'zh-CN'">
          <h2 class="text-2xl font-bold mt-12 mb-4">1. 条款</h2>
          <p>访问 https://octopus.run 网站，即表示您同意受这些服务条款、所有适用法律和法规的约束，并同意您有责任遵守任何适用的当地法律。</p>

          <h2 class="text-2xl font-bold mt-12 mb-4">2. 使用许可</h2>
          <p>允许临时下载 Octopus 网站上的材料（信息或软件）的一份副本，仅用于个人、非商业性的暂时查看。</p>

          <h2 class="text-2xl font-bold mt-12 mb-4">3. 免责声明</h2>
          <p>Octopus 网站上的材料按“原样”提供。Octopus 不作任何明示或暗示的保证，并特此声明并否认所有其他保证，包括但不限于适销性、特定用途的适用性或不侵犯知识产权或其他侵权行为的暗示保证或条件。</p>
        </template>
        <template v-else>
          <h2 class="text-2xl font-bold mt-12 mb-4">1. Terms</h2>
          <p>By accessing the website at https://octopus.run, you are agreeing to be bound by these terms of service, all applicable laws and regulations, and agree that you are responsible for compliance with any applicable local laws.</p>

          <h2 class="text-2xl font-bold mt-12 mb-4">2. Use License</h2>
          <p>Permission is granted to temporarily download one copy of the materials (information or software) on Octopus's website for personal, non-commercial transitory viewing only.</p>

          <h2 class="text-2xl font-bold mt-12 mb-4">3. Disclaimer</h2>
          <p>The materials on Octopus's website are provided on an 'as is' basis. Octopus makes no warranties, expressed or implied, and hereby disclaims and negates all other warranties including, without limitation, implied warranties or conditions of merchantability, fitness for a particular purpose, or non-infringement of intellectual property or other violation of rights.</p>
        </template>
      </div>
    </section>
  </div>
</template>
