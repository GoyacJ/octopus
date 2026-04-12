<script lang="ts" setup>
import { computed } from 'vue'
import { ChevronLeft, ChevronRight } from 'lucide-vue-next'
import UiButton from './UiButton.vue'

const props = withDefaults(defineProps<{
  page: number
  pageCount: number
  previousLabel?: string
  nextLabel?: string
  summaryLabel?: string
  metaLabel?: string
  pageInfoLabel?: string
  rootTestId?: string
  hidePageInfo?: boolean
}>(), {
  previousLabel: 'Prev',
  nextLabel: 'Next',
  summaryLabel: '',
  metaLabel: '',
  pageInfoLabel: '',
  rootTestId: 'ui-pagination',
  hidePageInfo: false,
})

const emit = defineEmits<{
  'update:page': [value: number]
}>()

const safePageCount = computed(() => Math.max(1, props.pageCount))
const safePage = computed(() => Math.min(safePageCount.value, Math.max(1, props.page)))
const canGoPrevious = computed(() => safePage.value > 1)
const canGoNext = computed(() => safePage.value < safePageCount.value)
const resolvedPageInfoLabel = computed(() => props.pageInfoLabel || `${safePage.value} / ${safePageCount.value}`)

function goPrevious() {
  if (canGoPrevious.value) emit('update:page', safePage.value - 1)
}

function goNext() {
  if (canGoNext.value) emit('update:page', safePage.value + 1)
}
</script>

<template>
  <div class="flex items-center justify-between w-full text-sm" :data-testid="rootTestId">
    <div class="flex items-center gap-2 text-text-tertiary">
      <span v-if="metaLabel">{{ metaLabel }}</span>
      <span v-if="summaryLabel">{{ summaryLabel }}</span>
    </div>

    <div class="flex items-center gap-1">
      <UiButton
        variant="ghost"
        size="sm"
        class="h-8 gap-1 text-text-secondary"
        :data-testid="`${rootTestId}-previous`"
        :disabled="!canGoPrevious"
        @click="goPrevious"
      >
        <slot name="previousIcon"><ChevronLeft :size="14" /></slot>
        <span class="hidden sm:inline">{{ previousLabel }}</span>
      </UiButton>

      <span v-if="!hidePageInfo" class="px-2 text-text-secondary font-medium">
        {{ resolvedPageInfoLabel }}
      </span>

      <UiButton
        variant="ghost"
        size="sm"
        class="h-8 gap-1 text-text-secondary"
        :data-testid="`${rootTestId}-next`"
        :disabled="!canGoNext"
        @click="goNext"
      >
        <span class="hidden sm:inline">{{ nextLabel }}</span>
        <slot name="nextIcon"><ChevronRight :size="14" /></slot>
      </UiButton>
    </div>
  </div>
</template>
