<script setup lang="ts">
import { ref, watch } from 'vue'
import {
  ComboboxAnchor,
  ComboboxContent,
  ComboboxEmpty,
  ComboboxInput,
  ComboboxItem,
  ComboboxRoot,
  ComboboxViewport,
} from 'reka-ui'

export interface UiComboboxOption {
  value: string
  label: string
  keywords?: string[]
}

const props = withDefaults(defineProps<{
  modelValue?: string
  options: UiComboboxOption[]
  placeholder?: string
  emptyLabel?: string
}>(), {
  modelValue: '',
  placeholder: '',
  emptyLabel: 'No results',
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
  select: [value: string]
}>()

const open = ref(false)
const query = ref('')

function findLabel(value?: string) {
  return props.options.find((option) => option.value === value)?.label ?? ''
}

function syncQuery(value = props.modelValue) {
  if (!open.value) {
    query.value = findLabel(value)
  }
}

watch(() => props.modelValue, value => syncQuery(value), { immediate: true })
watch(() => props.options, () => syncQuery(), { deep: true })

function handleModelValueUpdate(value: string | undefined) {
  const nextValue = value ?? ''
  emit('update:modelValue', nextValue)
  emit('select', nextValue)
  query.value = findLabel(nextValue)
}

function handleOpenChange(value: boolean) {
  open.value = value

  if (!value) {
    query.value = findLabel(props.modelValue)
  }
}
</script>

<template>
  <ComboboxRoot
    :model-value="props.modelValue || undefined"
    :open="open"
    :open-on-focus="true"
    :open-on-click="true"
    @update:model-value="handleModelValueUpdate"
    @update:open="handleOpenChange"
  >
    <div class="flex flex-col w-full relative">
      <ComboboxAnchor as-child>
        <ComboboxInput
          :model-value="query"
          :display-value="findLabel"
          :placeholder="props.placeholder"
          class="flex h-8 w-full rounded-[var(--radius-xs)] border border-input bg-background px-3 py-1.5 text-label text-text-primary placeholder:text-text-tertiary transition-colors duration-fast focus-visible:outline-none focus-visible:border-primary focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background disabled:cursor-not-allowed disabled:opacity-50"
          data-testid="ui-combobox-input"
          @update:model-value="query = String($event)"
        />
      </ComboboxAnchor>

      <ComboboxContent
        data-testid="ui-combobox-content"
        class="mt-1 z-40 w-full overflow-hidden rounded-[var(--radius-l)] border border-[color-mix(in_srgb,var(--border)_84%,transparent)] bg-popover shadow-md outline-none"
        :side-offset="4"
        position="popper"
      >
        <ComboboxViewport class="max-h-64 overflow-auto p-1">
          <ComboboxItem
            v-for="option in props.options"
            :key="option.value"
            :value="option.value"
            :text-value="`${option.label} ${(option.keywords ?? []).join(' ')}`"
            :data-testid="`ui-combobox-option-${option.value}`"
            class="flex cursor-default items-center rounded-[var(--radius-xs)] px-2 py-1.5 text-left text-label text-text-primary outline-none transition-colors data-[highlighted]:bg-subtle"
          >
            {{ option.label }}
          </ComboboxItem>

          <ComboboxEmpty class="px-3 py-2 text-sm text-text-tertiary text-center">
            {{ props.emptyLabel }}
          </ComboboxEmpty>
        </ComboboxViewport>
      </ComboboxContent>
    </div>
  </ComboboxRoot>
</template>
