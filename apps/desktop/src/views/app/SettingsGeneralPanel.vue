<script setup lang="ts">
import { computed } from 'vue'
import { RotateCcw } from 'lucide-vue-next'
import { UiButton, UiField, UiListRow, UiSelect, UiSwitch } from '@octopus/ui'

const props = defineProps<{
  locale: string
  localeOptions: Array<{ value: string, label: string }>
  leftSidebarCollapsed: boolean
  rightSidebarCollapsed: boolean
}>()

const emit = defineEmits<{
  reset: []
  'update:locale': [value: string]
  'update:left-sidebar-collapsed': [value: boolean]
  'update:right-sidebar-collapsed': [value: boolean]
}>()

const localeModel = computed({
  get: () => props.locale,
  set: value => emit('update:locale', value),
})

const leftSidebarCollapsedModel = computed({
  get: () => props.leftSidebarCollapsed,
  set: value => emit('update:left-sidebar-collapsed', value),
})

const rightSidebarCollapsedModel = computed({
  get: () => props.rightSidebarCollapsed,
  set: value => emit('update:right-sidebar-collapsed', value),
})
</script>

<template>
  <section class="space-y-8">
    <div class="flex items-center justify-between">
      <div class="space-y-1">
        <h3 class="text-xl font-bold text-text-primary">{{ $t('settings.general.title') }}</h3>
        <p class="text-[14px] text-text-secondary">{{ $t('settings.header.subtitle') }}</p>
      </div>
      <UiButton variant="ghost" size="sm" class="flex items-center gap-2 text-text-secondary hover:text-text-primary transition-colors" @click="emit('reset')">
        <RotateCcw :size="14" />
        <span>{{ $t('common.resetToDefault') }}</span>
      </UiButton>
    </div>

    <div class="space-y-6">
      <div class="space-y-3">
        <h4 class="px-1 text-[14px] font-bold text-text-primary">{{ $t('settings.general.layoutTitle') }}</h4>
        <div class="space-y-2 rounded-[var(--radius-l)] border border-border bg-surface p-2">
          <div data-testid="settings-layout-row-leftSidebarCollapsed">
            <UiListRow
              :title="$t('settings.preferences.leftSidebarCollapsed')"
              :subtitle="$t('settings.general.leftSidebarHint')"
            >
              <template #actions>
                <UiSwitch v-model="leftSidebarCollapsedModel" />
              </template>
            </UiListRow>
          </div>

          <div data-testid="settings-layout-row-rightSidebarCollapsed">
            <UiListRow
              :title="$t('settings.preferences.rightSidebarCollapsed')"
              :subtitle="$t('settings.general.rightSidebarHint')"
            >
              <template #actions>
                <UiSwitch v-model="rightSidebarCollapsedModel" />
              </template>
            </UiListRow>
          </div>
        </div>
      </div>

      <div class="space-y-3">
        <h4 class="px-1 text-[14px] font-bold text-text-primary">{{ $t('settings.general.i18nTitle') }}</h4>
        <div class="max-w-md px-1">
          <UiField :label="$t('settings.preferences.locale')">
            <UiSelect v-model="localeModel" :options="localeOptions" />
          </UiField>
        </div>
      </div>
    </div>
  </section>
</template>
