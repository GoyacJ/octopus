<script setup lang="ts">
import { computed } from 'vue'
import { RotateCcw } from 'lucide-vue-next'
import { UiButton, UiField, UiSelect } from '@octopus/ui'

const props = defineProps<{
  theme: string
  fontSize: string
  themeOptions: Array<{ value: string, label: string }>
  fontSizeOptions: Array<{ value: string, label: string }>
}>()

const emit = defineEmits<{
  reset: []
  'update:theme': [value: string]
  'update:font-size': [value: string]
}>()

const themeModel = computed({
  get: () => props.theme,
  set: value => emit('update:theme', value),
})

const fontSizeModel = computed({
  get: () => props.fontSize,
  set: value => emit('update:font-size', value),
})
</script>

<template>
  <section class="space-y-8">
    <div class="flex items-center justify-between">
      <div class="space-y-1">
        <h3 class="text-xl font-bold text-text-primary">{{ $t('settings.preferences.title') }}</h3>
        <p class="text-[14px] text-text-secondary">{{ $t('settings.header.subtitle') }}</p>
      </div>
      <UiButton variant="ghost" size="sm" class="flex items-center gap-2 text-text-secondary hover:text-text-primary transition-colors" @click="emit('reset')">
        <RotateCcw :size="14" />
        <span>{{ $t('common.resetToDefault') }}</span>
      </UiButton>
    </div>

    <div class="space-y-6">
      <div class="grid max-w-2xl gap-6 md:grid-cols-2">
        <UiField :label="$t('settings.preferences.theme')">
          <UiSelect v-model="themeModel" :options="themeOptions" />
        </UiField>

        <UiField :label="$t('settings.preferences.fontSize')">
          <UiSelect v-model="fontSizeModel" :options="fontSizeOptions" />
        </UiField>
      </div>
    </div>
  </section>
</template>
