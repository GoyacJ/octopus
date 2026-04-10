<script setup lang="ts">
const { t, locale } = useI18n()
useHead({ title: t('footer.privacy') + ' - Octopus' })
</script>

<template>
  <div class="relative min-h-screen pb-24">
    <UiSectionHero
      align="left"
      :title="t('footer.privacy')"
      :subtitle="locale === 'zh-CN' ? '最近更新：2026年4月10日' : 'Last updated: April 10, 2026'"
    />
    <section class="section-padding">
      <div class="container-custom max-w-4xl prose dark:prose-invert prose-orange font-medium">
        <template v-if="locale === 'zh-CN'">
          <p class="text-xl text-[var(--website-text-muted)] mb-8">
            您的隐私对我们至关重要。Octopus 的政策是尊重您的隐私，并处理我们在网站上收集的有关您的任何信息。
          </p>
          <h2 class="text-2xl font-bold mt-12 mb-4">1. 我们收集的信息</h2>
          <p>我们仅在真正需要为您提供服务时才要求提供个人信息。我们通过公平合法的手段收集信息，并告知您并征得您的同意。</p>

          <h2 class="text-2xl font-bold mt-12 mb-4">2. 信息的使用</h2>
          <p>我们仅在为您提供所请求服务的时间内保留收集的信息。我们存储的数据将通过商业上可接受的手段进行保护，以防止丢失和盗窃，以及未经授权的访问、披露、复制、使用或修改。</p>

          <h2 class="text-2xl font-bold mt-12 mb-4">3. 数据主权</h2>
          <p>由于 Octopus 是一个私有化优先的 AI 平台，您的大部分数据都保留在您自己的域或私有部署中。我们无权访问您私有 Octopus 实例中处理的数据。</p>
        </template>
        <template v-else>
          <p class="text-xl text-[var(--website-text-muted)] mb-8">
            Your privacy is important to us. It is Octopus's policy to respect your privacy regarding any information we may collect from you across our website.
          </p>
          <h2 class="text-2xl font-bold mt-12 mb-4">1. Information We Collect</h2>
          <p>We only ask for personal information when we truly need it to provide a service to you. We collect it by fair and lawful means, with your knowledge and consent.</p>

          <h2 class="text-2xl font-bold mt-12 mb-4">2. Use of Information</h2>
          <p>We only retain collected information for as long as necessary to provide you with your requested service. What data we store, we’ll protect within commercially acceptable means to prevent loss and theft, as well as unauthorized access, disclosure, copying, use or modification.</p>

          <h2 class="text-2xl font-bold mt-12 mb-4">3. Data Sovereignty</h2>
          <p>As Octopus is a private-first AI platform, most of your data stays within your own domain or private deployment. We do not have access to the data processed within your private Octopus instances.</p>
        </template>
      </div>
    </section>
  </div>
</template>
