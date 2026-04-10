<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { Check, ChevronRight } from 'lucide-vue-next'

import { UiButton } from '@octopus/ui'

import type { MenuTreeBranch, MenuTreeLeaf, MenuTreeSection } from './menu-tree'
import { isMenuTreeGroup } from './menu-tree'

const props = withDefaults(defineProps<{
  sections: MenuTreeSection[]
  testIdPrefix: string
  selectionMode?: 'single' | 'multiple'
  modelValue?: string[]
  activeId?: string
}>(), {
  selectionMode: 'single',
  modelValue: () => [],
  activeId: '',
})

const emit = defineEmits<{
  'update:modelValue': [value: string[]]
  select: [menuId: string]
}>()

const openSections = ref<string[]>([])
const openBranches = ref<string[]>([])
const knownSectionIds = ref<string[]>([])
const knownBranchIds = ref<string[]>([])
const sectionsInitialized = ref(false)
const branchesInitialized = ref(false)

const branchIds = computed(() => props.sections
  .flatMap(section => section.items.filter(isMenuTreeGroup))
  .map(branch => branch.id))

watch(() => props.sections.map(section => section.id), (ids) => {
  syncOpenValues(
    openSections,
    knownSectionIds,
    sectionsInitialized,
    ids,
  )
}, { immediate: true })

watch(branchIds, (ids) => {
  syncOpenValues(
    openBranches,
    knownBranchIds,
    branchesInitialized,
    ids,
  )
}, { immediate: true })

function sectionValue(sectionId: MenuTreeSection['id']) {
  return `${props.testIdPrefix}-section-${sectionId}`
}

function branchValue(branchId: string) {
  return `${props.testIdPrefix}-branch-${branchId}`
}

function leafTestId(leafId: string) {
  return props.testIdPrefix.endsWith('-menu')
    ? `${props.testIdPrefix}-${leafId}`
    : `${props.testIdPrefix}-menu-${leafId}`
}

function isSectionOpen(sectionId: MenuTreeSection['id']) {
  return openSections.value.includes(sectionId)
}

function toggleSection(sectionId: MenuTreeSection['id']) {
  openSections.value = openSections.value.includes(sectionId)
    ? openSections.value.filter(value => value !== sectionId)
    : [...openSections.value, sectionId]
}

function isBranchOpen(branchId: string) {
  return openBranches.value.includes(branchId)
}

function toggleBranch(branchId: string) {
  openBranches.value = openBranches.value.includes(branchId)
    ? openBranches.value.filter(value => value !== branchId)
    : [...openBranches.value, branchId]
}

function isLeafChecked(leafId: string) {
  return props.modelValue.includes(leafId)
}

function isLeafActive(leafId: string) {
  return props.activeId === leafId
}

function toggleLeafSelection(leafId: string) {
  if (props.selectionMode !== 'multiple') {
    return
  }

  const next = new Set(props.modelValue)
  if (next.has(leafId)) {
    next.delete(leafId)
  } else {
    next.add(leafId)
  }

  emit('update:modelValue', Array.from(next))
}

function handleLeafClick(leaf: MenuTreeLeaf) {
  if (props.selectionMode === 'single') {
    emit('select', leaf.id)
  }
}

function selectionIndicatorClass(checked: boolean) {
  return checked
    ? 'border-border bg-accent text-text-primary shadow-xs'
    : 'border-border/60 bg-transparent text-text-tertiary'
}

function selectableLeafClass(active: boolean) {
  return active
    ? 'border-border bg-surface text-text-primary shadow-xs'
    : 'border-transparent text-text-secondary hover:bg-accent/40'
}

function syncOpenValues(
  current: { value: string[] },
  known: { value: string[] },
  initialized: { value: boolean },
  available: string[],
) {
  if (!initialized.value) {
    current.value = [...available]
    known.value = [...available]
    initialized.value = true
    return
  }

  const availableSet = new Set(available)
  const next = current.value.filter(value => availableSet.has(value))
  const knownSet = new Set(known.value)
  for (const value of available) {
    if (!knownSet.has(value)) {
      next.push(value)
    }
  }

  current.value = next
  known.value = [...available]
}
</script>

<template>
  <div class="space-y-2">
    <section
      v-for="section in props.sections"
      :key="section.id"
      :data-testid="`${props.testIdPrefix}-group-${section.id}`"
      class="rounded-[var(--radius-l)] bg-subtle/60 px-2 py-1.5"
    >
      <UiButton
        variant="ghost"
        size="sm"
        :data-testid="`ui-accordion-trigger-${sectionValue(section.id)}`"
        class="group flex h-auto w-full justify-start gap-2 rounded-[var(--radius-m)] px-2 py-1 text-left text-sm font-semibold uppercase tracking-[0.24em] text-text-tertiary hover:bg-surface hover:text-text-secondary"
        @click="toggleSection(section.id)"
      >
        <ChevronRight
          :size="16"
          class="transition-transform duration-normal"
          :class="isSectionOpen(section.id) ? 'rotate-90' : ''"
        />
        <span>{{ section.label }}</span>
      </UiButton>

      <div v-if="isSectionOpen(section.id)" class="mt-1 space-y-1 px-1 pb-1">
        <template v-for="entry in section.items" :key="entry.id">
          <div
            v-if="isMenuTreeGroup(entry)"
            :data-testid="`${props.testIdPrefix}-group-${entry.id}`"
            class="rounded-[var(--radius-m)] bg-surface/80 px-2 py-1.5"
          >
            <UiButton
              variant="ghost"
              size="sm"
              :data-testid="`ui-accordion-trigger-${branchValue(entry.id)}`"
              class="group flex h-auto w-full justify-start gap-2 rounded-[var(--radius-s)] px-2 py-1 text-left text-sm font-semibold uppercase tracking-[0.2em] text-text-secondary hover:bg-accent/30 hover:text-text-primary"
              @click="toggleBranch(entry.id)"
            >
              <ChevronRight
                :size="16"
                class="transition-transform duration-normal"
                :class="isBranchOpen(entry.id) ? 'rotate-90' : ''"
              />
              <span>{{ entry.label }}</span>
            </UiButton>

            <div v-if="isBranchOpen(entry.id)" class="mt-1 space-y-1 pl-4">
              <template v-if="entry.rootMenu">
                <UiButton
                  v-if="props.selectionMode === 'multiple'"
                  variant="ghost"
                  size="sm"
                  :data-testid="leafTestId(entry.rootMenu.id)"
                  class="flex h-auto w-full min-w-0 justify-start gap-2 rounded-[var(--radius-m)] border px-2 py-2 text-left"
                  :class="selectableLeafClass(isLeafChecked(entry.rootMenu.id))"
                  @click="toggleLeafSelection(entry.rootMenu.id)"
                >
                  <span
                    class="mt-0.5 flex size-[14px] shrink-0 items-center justify-center rounded-sm border transition-colors"
                    :class="selectionIndicatorClass(isLeafChecked(entry.rootMenu.id))"
                  >
                    <Check v-if="isLeafChecked(entry.rootMenu.id)" :size="10" stroke-width="3" />
                  </span>
                  <span class="inline-flex min-w-0 flex-col">
                    <span class="font-medium text-text-primary">{{ entry.rootMenu.label }}</span>
                    <span class="text-xs text-text-tertiary">{{ entry.rootMenu.secondary }}</span>
                  </span>
                </UiButton>
                <UiButton
                  v-else
                  variant="ghost"
                  size="sm"
                  :data-testid="leafTestId(entry.rootMenu.id)"
                  class="flex h-auto w-full min-w-0 justify-start rounded-[var(--radius-m)] border px-2 py-2 text-left"
                  :class="selectableLeafClass(isLeafActive(entry.rootMenu.id))"
                  @click="handleLeafClick(entry.rootMenu)"
                >
                  <span class="inline-flex min-w-0 flex-col">
                    <span class="font-medium text-text-primary">{{ entry.rootMenu.label }}</span>
                    <span class="text-xs text-text-tertiary">{{ entry.rootMenu.secondary }}</span>
                  </span>
                </UiButton>
              </template>

              <template v-for="child in entry.children" :key="child.id">
                <UiButton
                  v-if="props.selectionMode === 'multiple'"
                  variant="ghost"
                  size="sm"
                  :data-testid="leafTestId(child.id)"
                  class="flex h-auto w-full min-w-0 justify-start gap-2 rounded-[var(--radius-m)] border px-2 py-2 text-left"
                  :class="selectableLeafClass(isLeafChecked(child.id))"
                  @click="toggleLeafSelection(child.id)"
                >
                  <span
                    class="mt-0.5 flex size-[14px] shrink-0 items-center justify-center rounded-sm border transition-colors"
                    :class="selectionIndicatorClass(isLeafChecked(child.id))"
                  >
                    <Check v-if="isLeafChecked(child.id)" :size="10" stroke-width="3" />
                  </span>
                  <span class="inline-flex min-w-0 flex-col">
                    <span class="font-medium text-text-primary">{{ child.label }}</span>
                    <span class="text-xs text-text-tertiary">{{ child.secondary }}</span>
                  </span>
                </UiButton>
                <UiButton
                  v-else
                  variant="ghost"
                  size="sm"
                  :data-testid="leafTestId(child.id)"
                  class="flex h-auto w-full min-w-0 justify-start rounded-[var(--radius-m)] border px-2 py-2 text-left"
                  :class="selectableLeafClass(isLeafActive(child.id))"
                  @click="handleLeafClick(child)"
                >
                  <span class="inline-flex min-w-0 flex-col">
                    <span class="font-medium text-text-primary">{{ child.label }}</span>
                    <span class="text-xs text-text-tertiary">{{ child.secondary }}</span>
                  </span>
                </UiButton>
              </template>
            </div>
          </div>

          <UiButton
            v-else-if="props.selectionMode === 'multiple'"
            variant="ghost"
            size="sm"
            :data-testid="leafTestId(entry.id)"
            class="flex h-auto w-full min-w-0 justify-start gap-2 rounded-[var(--radius-m)] border px-2 py-2 text-left"
            :class="selectableLeafClass(isLeafChecked(entry.id))"
            @click="toggleLeafSelection(entry.id)"
          >
            <span
              class="mt-0.5 flex size-[14px] shrink-0 items-center justify-center rounded-sm border transition-colors"
              :class="selectionIndicatorClass(isLeafChecked(entry.id))"
            >
              <Check v-if="isLeafChecked(entry.id)" :size="10" stroke-width="3" />
            </span>
            <span class="inline-flex min-w-0 flex-col">
              <span class="font-medium text-text-primary">{{ entry.label }}</span>
              <span class="text-xs text-text-tertiary">{{ entry.secondary }}</span>
            </span>
          </UiButton>

          <UiButton
            v-else
            variant="ghost"
            size="sm"
            :data-testid="leafTestId(entry.id)"
            class="flex h-auto w-full min-w-0 justify-start rounded-[var(--radius-m)] border px-2 py-2 text-left"
            :class="selectableLeafClass(isLeafActive(entry.id))"
            @click="handleLeafClick(entry)"
          >
            <span class="inline-flex min-w-0 flex-col">
              <span class="font-medium text-text-primary">{{ entry.label }}</span>
              <span class="text-xs text-text-tertiary">{{ entry.secondary }}</span>
            </span>
          </UiButton>
        </template>
      </div>
    </section>
  </div>
</template>
