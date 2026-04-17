<script setup lang="ts">
import { computed } from 'vue'
import { ChevronRight } from 'lucide-vue-next'
import {
  AccordionContent,
  AccordionHeader,
  AccordionItem,
  AccordionRoot,
  AccordionTrigger,
} from 'reka-ui'

import { cn } from '../lib/utils'

export interface UiAccordionItem {
  value: string
  title: string
  content?: string
}

const props = withDefaults(defineProps<{
  modelValue?: string[]
  items: UiAccordionItem[]
  multiple?: boolean
  class?: string
}>(), {
  modelValue: () => [],
  multiple: true,
  class: '',
})

const emit = defineEmits<{
  'update:modelValue': [value: string[]]
}>()

const rootValue = computed(() => (
  props.multiple
    ? props.modelValue
    : props.modelValue[0]
))

function handleUpdate(value: string | string[] | undefined) {
  if (Array.isArray(value)) {
    emit('update:modelValue', value)
    return
  }

  emit('update:modelValue', value ? [value] : [])
}
</script>

<template>
  <AccordionRoot
    :model-value="rootValue"
    :type="props.multiple ? 'multiple' : 'single'"
    :collapsible="true"
    :class="cn('flex flex-col gap-1', props.class)"
    @update:model-value="handleUpdate"
  >
    <AccordionItem
      v-for="item in props.items"
      :key="item.value"
      :value="item.value"
      class="flex flex-col"
    >
      <AccordionHeader>
        <AccordionTrigger
          class="ui-focus-ring group flex w-full items-center gap-2 rounded px-1 py-1.5 text-left text-sm font-medium text-text-primary transition-colors hover:bg-subtle"
          :data-testid="`ui-accordion-trigger-${item.value}`"
        >
          <ChevronRight
            :size="16"
            class="text-text-tertiary transition-transform duration-normal group-data-[state=open]:rotate-90"
          />
          <span>{{ item.title }}</span>
        </AccordionTrigger>
      </AccordionHeader>

      <AccordionContent class="overflow-hidden data-[state=closed]:animate-accordion-up data-[state=open]:animate-accordion-down">
        <div class="pl-7 pr-4 pb-2 pt-1 text-[13px] leading-relaxed text-text-secondary">
          <slot name="content" :item="item">
            {{ item.content }}
          </slot>
        </div>
      </AccordionContent>
    </AccordionItem>
  </AccordionRoot>
</template>
