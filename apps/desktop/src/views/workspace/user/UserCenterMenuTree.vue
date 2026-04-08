<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { Check, ChevronRight } from 'lucide-vue-next'

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
      class="rounded-lg bg-muted/12 px-2 py-1.5"
    >
      <button
        type="button"
        :data-testid="`ui-accordion-trigger-${sectionValue(section.id)}`"
        class="group flex w-full items-center gap-2 rounded-md px-2 py-1 text-left text-sm font-semibold uppercase tracking-[0.24em] text-text-tertiary transition-colors hover:bg-accent/30"
        @click="toggleSection(section.id)"
      >
        <ChevronRight
          :size="16"
          class="transition-transform duration-normal"
          :class="isSectionOpen(section.id) ? 'rotate-90' : ''"
        />
        <span>{{ section.label }}</span>
      </button>

      <div v-if="isSectionOpen(section.id)" class="mt-1 space-y-1 px-1 pb-1">
        <template v-for="entry in section.items" :key="entry.id">
          <div
            v-if="isMenuTreeGroup(entry)"
            :data-testid="`${props.testIdPrefix}-group-${entry.id}`"
            class="rounded-lg bg-muted/20 px-2 py-1.5"
          >
            <button
              type="button"
              :data-testid="`ui-accordion-trigger-${branchValue(entry.id)}`"
              class="group flex w-full items-center gap-2 rounded-md px-2 py-1 text-left text-sm font-semibold uppercase tracking-[0.2em] text-text-secondary transition-colors hover:bg-accent/30"
              @click="toggleBranch(entry.id)"
            >
              <ChevronRight
                :size="16"
                class="transition-transform duration-normal"
                :class="isBranchOpen(entry.id) ? 'rotate-90' : ''"
              />
              <span>{{ entry.label }}</span>
            </button>

            <div v-if="isBranchOpen(entry.id)" class="mt-1 space-y-1 pl-4">
              <template v-if="entry.rootMenu">
                <button
                  v-if="props.selectionMode === 'multiple'"
                  type="button"
                  :data-testid="leafTestId(entry.rootMenu.id)"
                  class="flex w-full min-w-0 items-start gap-2 rounded-md px-2 py-2 text-left transition-colors hover:bg-accent/40"
                  @click="toggleLeafSelection(entry.rootMenu.id)"
                >
                  <span
                    class="mt-0.5 flex size-[14px] shrink-0 items-center justify-center rounded-sm border transition-colors"
                    :class="isLeafChecked(entry.rootMenu.id)
                      ? 'bg-primary border-primary dark:border-white/[0.1] text-primary-foreground'
                      : 'bg-transparent border-border/60 dark:border-white/[0.1]'"
                  >
                    <Check v-if="isLeafChecked(entry.rootMenu.id)" :size="10" stroke-width="3" />
                  </span>
                  <span class="inline-flex min-w-0 flex-col">
                    <span class="font-medium text-text-primary">{{ entry.rootMenu.label }}</span>
                    <span class="text-xs text-text-tertiary">{{ entry.rootMenu.secondary }}</span>
                  </span>
                </button>
                <button
                  v-else
                  type="button"
                  :data-testid="leafTestId(entry.rootMenu.id)"
                  class="flex w-full min-w-0 items-start rounded-md px-2 py-2 text-left transition-colors hover:bg-accent/40"
                  :class="isLeafActive(entry.rootMenu.id) ? 'bg-accent/55' : ''"
                  @click="handleLeafClick(entry.rootMenu)"
                >
                  <span class="inline-flex min-w-0 flex-col">
                    <span class="font-medium text-text-primary">{{ entry.rootMenu.label }}</span>
                    <span class="text-xs text-text-tertiary">{{ entry.rootMenu.secondary }}</span>
                  </span>
                </button>
              </template>

              <template v-for="child in entry.children" :key="child.id">
                <button
                  v-if="props.selectionMode === 'multiple'"
                  type="button"
                  :data-testid="leafTestId(child.id)"
                  class="flex w-full min-w-0 items-start gap-2 rounded-md px-2 py-2 text-left transition-colors hover:bg-accent/40"
                  @click="toggleLeafSelection(child.id)"
                >
                  <span
                    class="mt-0.5 flex size-[14px] shrink-0 items-center justify-center rounded-sm border transition-colors"
                    :class="isLeafChecked(child.id)
                      ? 'bg-primary border-primary dark:border-white/[0.1] text-primary-foreground'
                      : 'bg-transparent border-border/60 dark:border-white/[0.1]'"
                  >
                    <Check v-if="isLeafChecked(child.id)" :size="10" stroke-width="3" />
                  </span>
                  <span class="inline-flex min-w-0 flex-col">
                    <span class="font-medium text-text-primary">{{ child.label }}</span>
                    <span class="text-xs text-text-tertiary">{{ child.secondary }}</span>
                  </span>
                </button>
                <button
                  v-else
                  type="button"
                  :data-testid="leafTestId(child.id)"
                  class="flex w-full min-w-0 items-start rounded-md px-2 py-2 text-left transition-colors hover:bg-accent/40"
                  :class="isLeafActive(child.id) ? 'bg-accent/55' : ''"
                  @click="handleLeafClick(child)"
                >
                  <span class="inline-flex min-w-0 flex-col">
                    <span class="font-medium text-text-primary">{{ child.label }}</span>
                    <span class="text-xs text-text-tertiary">{{ child.secondary }}</span>
                  </span>
                </button>
              </template>
            </div>
          </div>

          <button
            v-else-if="props.selectionMode === 'multiple'"
            type="button"
            :data-testid="leafTestId(entry.id)"
            class="flex w-full min-w-0 items-start gap-2 rounded-md px-2 py-2 text-left transition-colors hover:bg-accent/40"
            @click="toggleLeafSelection(entry.id)"
          >
            <span
              class="mt-0.5 flex size-[14px] shrink-0 items-center justify-center rounded-sm border transition-colors"
              :class="isLeafChecked(entry.id)
                ? 'bg-primary border-primary dark:border-white/[0.1] text-primary-foreground'
                : 'bg-transparent border-border/60 dark:border-white/[0.1]'"
            >
              <Check v-if="isLeafChecked(entry.id)" :size="10" stroke-width="3" />
            </span>
            <span class="inline-flex min-w-0 flex-col">
              <span class="font-medium text-text-primary">{{ entry.label }}</span>
              <span class="text-xs text-text-tertiary">{{ entry.secondary }}</span>
            </span>
          </button>

          <button
            v-else
            type="button"
            :data-testid="leafTestId(entry.id)"
            class="flex w-full min-w-0 items-start rounded-md px-2 py-2 text-left transition-colors hover:bg-accent/40"
            :class="isLeafActive(entry.id) ? 'bg-accent/55' : ''"
            @click="handleLeafClick(entry)"
          >
            <span class="inline-flex min-w-0 flex-col">
              <span class="font-medium text-text-primary">{{ entry.label }}</span>
              <span class="text-xs text-text-tertiary">{{ entry.secondary }}</span>
            </span>
          </button>
        </template>
      </div>
    </section>
  </div>
</template>
