import { computed, ref, toValue, watch } from 'vue'
import type { MaybeRefOrGetter, WatchSource } from 'vue'

interface UseAccessControlSelectionOptions<T> {
  getId: (item: T) => string
  resetOn?: WatchSource[]
}

export function useAccessControlSelection<T>(
  items: MaybeRefOrGetter<T[]>,
  options: UseAccessControlSelectionOptions<T>,
) {
  const selectedIds = ref<string[]>([])
  const resolvedItems = computed(() => toValue(items))
  const selectedIdSet = computed(() => new Set(selectedIds.value))
  const selectedCount = computed(() => selectedIds.value.length)
  const hasSelection = computed(() => selectedIds.value.length > 0)

  function clearSelection() {
    selectedIds.value = []
  }

  function setSelection(nextIds: Iterable<string>) {
    selectedIds.value = Array.from(new Set(nextIds))
  }

  function isSelected(id: string) {
    return selectedIdSet.value.has(id)
  }

  function toggleSelection(id: string, force?: boolean) {
    const next = new Set(selectedIds.value)
    const shouldSelect = typeof force === 'boolean' ? force : !next.has(id)
    if (shouldSelect) {
      next.add(id)
    } else {
      next.delete(id)
    }
    setSelection(next)
  }

  function selectPage(itemsOnPage: T[], force?: boolean) {
    const pageIds = itemsOnPage.map(options.getId)
    const next = new Set(selectedIds.value)
    const shouldSelect = typeof force === 'boolean'
      ? force
      : pageIds.some(id => !next.has(id))

    for (const id of pageIds) {
      if (shouldSelect) {
        next.add(id)
      } else {
        next.delete(id)
      }
    }

    setSelection(next)
  }

  function isPageSelected(itemsOnPage: T[]) {
    const pageIds = itemsOnPage.map(options.getId)
    return pageIds.length > 0 && pageIds.every(id => selectedIdSet.value.has(id))
  }

  watch(resolvedItems, (nextItems) => {
    const validIds = new Set(nextItems.map(options.getId))
    if (selectedIds.value.some(id => !validIds.has(id))) {
      selectedIds.value = selectedIds.value.filter(id => validIds.has(id))
    }
  }, { immediate: true })

  if (options.resetOn?.length) {
    watch(options.resetOn, () => {
      clearSelection()
    })
  }

  return {
    selectedIds,
    selectedCount,
    hasSelection,
    clearSelection,
    isSelected,
    isPageSelected,
    selectPage,
    setSelection,
    toggleSelection,
  }
}
