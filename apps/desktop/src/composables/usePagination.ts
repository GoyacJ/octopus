import { computed, ref, toValue, watch } from 'vue'
import type { MaybeRefOrGetter, WatchSource } from 'vue'

interface UsePaginationOptions {
  pageSize: number
  initialPage?: number
  resetOn?: WatchSource[]
}

export function usePagination<T>(
  items: MaybeRefOrGetter<T[]>,
  options: UsePaginationOptions,
) {
  const pageSize = Math.max(1, options.pageSize)
  const currentPage = ref(Math.max(1, options.initialPage ?? 1))

  const resolvedItems = computed(() => toValue(items))
  const totalItems = computed(() => resolvedItems.value.length)
  const pageCount = computed(() => Math.max(1, Math.ceil(totalItems.value / pageSize)))
  const isFirstPage = computed(() => currentPage.value <= 1)
  const isLastPage = computed(() => currentPage.value >= pageCount.value)
  const pagedItems = computed(() => {
    const start = (currentPage.value - 1) * pageSize
    return resolvedItems.value.slice(start, start + pageSize)
  })

  function clampPage(page: number) {
    return Math.min(pageCount.value, Math.max(1, page))
  }

  function setPage(page: number) {
    currentPage.value = clampPage(page)
  }

  function resetPage() {
    currentPage.value = 1
  }

  function nextPage() {
    setPage(currentPage.value + 1)
  }

  function previousPage() {
    setPage(currentPage.value - 1)
  }

  watch(pageCount, () => {
    currentPage.value = clampPage(currentPage.value)
  }, { immediate: true })

  if (options.resetOn?.length) {
    watch(options.resetOn, () => {
      resetPage()
    })
  }

  return {
    currentPage,
    pageCount,
    totalItems,
    pagedItems,
    isFirstPage,
    isLastPage,
    nextPage,
    previousPage,
    setPage,
    resetPage,
  }
}
