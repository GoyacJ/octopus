<script setup lang="ts">
import { computed, ref } from 'vue'
import { Check, ChevronDown, Search } from 'lucide-vue-next'

import UiBadge from './UiBadge.vue'
import UiButton from './UiButton.vue'
import UiCheckbox from './UiCheckbox.vue'
import UiInput from './UiInput.vue'
import UiPopover from './UiPopover.vue'

export interface UiSearchableMultiSelectOption {
  value: string
  label: string
  keywords?: string[]
  helper?: string
  disabled?: boolean
}

const props = withDefaults(defineProps<{
  modelValue: string[]
  options: UiSearchableMultiSelectOption[]
  triggerLabel?: string
  placeholder?: string
  searchPlaceholder?: string
  emptyLabel?: string
  maxPreview?: number
  disabled?: boolean
}>(), {
  triggerLabel: '',
  placeholder: 'Select',
  searchPlaceholder: 'Search',
  emptyLabel: 'No results',
  maxPreview: 2,
  disabled: false,
})

const emit = defineEmits<{
  'update:modelValue': [value: string[]]
}>()

const open = ref(false)
const query = ref('')

const selectedOptions = computed(() => props.options.filter(option => props.modelValue.includes(option.value)))
const filteredOptions = computed(() => {
  const keyword = query.value.trim().toLowerCase()
  if (!keyword) {
    return props.options
  }
  return props.options.filter((option) => {
    const haystack = [option.label, option.helper ?? '', ...(option.keywords ?? [])]
      .join(' ')
      .toLowerCase()
    return haystack.includes(keyword)
  })
})
const triggerLabel = computed(() => {
  if (!selectedOptions.value.length) {
    return props.placeholder
  }
  const preview = selectedOptions.value.slice(0, props.maxPreview).map(option => option.label).join(', ')
  return selectedOptions.value.length > props.maxPreview
    ? `${preview} +${selectedOptions.value.length - props.maxPreview}`
    : preview
})

function toggleValue(value: string) {
  const current = new Set(props.modelValue)
  if (current.has(value)) {
    current.delete(value)
  } else {
    current.add(value)
  }
  emit('update:modelValue', [...current])
}
</script>

<template>
  <UiPopover
    :open="open"
    align="start"
    class="w-[min(26rem,calc(100vw-2rem))] p-0"
    @update:open="open = $event"
  >
    <template #trigger>
      <UiButton
        type="button"
        variant="secondary"
        class="w-full justify-between"
        :disabled="props.disabled"
        data-testid="ui-searchable-multi-select-trigger"
      >
        <span class="min-w-0 truncate text-left">{{ triggerLabel }}</span>
        <span class="ml-3 inline-flex items-center gap-2">
          <UiBadge v-if="selectedOptions.length" :label="String(selectedOptions.length)" subtle />
          <ChevronDown :size="14" />
        </span>
      </UiButton>
    </template>

    <div class="flex flex-col">
      <div class="border-b border-border/40 px-3 py-3 dark:border-white/[0.08]">
        <div class="relative">
          <Search :size="14" class="pointer-events-none absolute start-3 top-1/2 -translate-y-1/2 text-text-tertiary" />
          <UiInput
            v-model="query"
            :placeholder="props.searchPlaceholder"
            data-testid="ui-searchable-multi-select-input"
            style="padding-left: 2.5rem"
          />
        </div>
      </div>

      <div class="border-b border-border/40 px-2 py-2">
        <div class="flex items-center justify-between gap-2">
          <button
            type="button"
            class="text-xs text-text-tertiary hover:text-text-primary transition-colors"
            @click="emit('update:modelValue', filteredOptions.map(o => o.value))"
          >
            全选
          </button>
          <button
            type="button"
            class="text-xs text-text-tertiary hover:text-text-primary transition-colors"
            @click="emit('update:modelValue', [])"
          >
            反选
          </button>
        </div>
      </div>

      <div class="max-h-72 overflow-y-auto p-2">
        <div v-if="!filteredOptions.length" class="px-2 py-6 text-center text-sm text-text-tertiary">
          {{ props.emptyLabel }}
        </div>

        <button
          v-for="option in filteredOptions"
          :key="option.value"
          type="button"
          :data-testid="`ui-searchable-multi-select-option-${option.value}`"
          class="flex w-full items-start justify-between gap-3 rounded-lg px-2 py-2 text-left transition hover:bg-accent disabled:cursor-not-allowed disabled:opacity-50"
          :disabled="option.disabled"
          @click="toggleValue(option.value)"
        >
          <span class="flex min-w-0 items-start gap-2">
            <UiCheckbox
              :model-value="props.modelValue.includes(option.value)"
              class="pt-0.5"
            />
            <span class="min-w-0">
              <span class="block truncate text-sm font-medium text-text-primary">{{ option.label }}</span>
              <span v-if="option.helper" class="block pt-0.5 text-xs text-text-tertiary">{{ option.helper }}</span>
            </span>
          </span>

          <Check
            v-if="props.modelValue.includes(option.value)"
            :size="14"
            class="mt-0.5 shrink-0 text-primary"
          />
        </button>
      </div>
    </div>
  </UiPopover>
</template>
