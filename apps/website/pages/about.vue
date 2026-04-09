<script setup lang="ts">
const { t } = useI18n()

useHead(() => ({
  title: t('pages.about.title'),
  meta: [{ name: 'description', content: t('pages.about.body') }],
}))
</script>

<template>
  <section class="website-placeholder">
    <div class="website-placeholder__inner">
      <div class="website-placeholder__card">
        <p class="website-placeholder__eyebrow">{{ t('site.name') }}</p>
        <h1 class="website-placeholder__title">{{ t('pages.about.title') }}</h1>
        <p class="website-placeholder__body">{{ t('pages.about.body') }}</p>
      </div>
    </div>
  </section>
</template>
