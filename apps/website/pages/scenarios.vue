<script setup lang="ts">
const { t } = useI18n()

useHead(() => ({
  title: t('pages.scenarios.title'),
  meta: [{ name: 'description', content: t('pages.scenarios.body') }],
}))
</script>

<template>
  <section class="website-placeholder">
    <div class="website-placeholder__inner">
      <div class="website-placeholder__card">
        <p class="website-placeholder__eyebrow">{{ t('site.name') }}</p>
        <h1 class="website-placeholder__title">{{ t('pages.scenarios.title') }}</h1>
        <p class="website-placeholder__body">{{ t('pages.scenarios.body') }}</p>
      </div>
    </div>
  </section>
</template>
