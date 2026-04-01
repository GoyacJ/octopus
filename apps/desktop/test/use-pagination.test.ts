// @vitest-environment jsdom

import { computed, nextTick, ref } from 'vue'
import { describe, expect, it } from 'vitest'

import { usePagination } from '@/composables/usePagination'

describe('usePagination', () => {
  it('computes initial pagination state and slices items for the current page', () => {
    const items = ref([1, 2, 3, 4, 5, 6])
    const pagination = usePagination(items, { pageSize: 2 })

    expect(pagination.currentPage.value).toBe(1)
    expect(pagination.pageCount.value).toBe(3)
    expect(pagination.totalItems.value).toBe(6)
    expect(pagination.pagedItems.value).toEqual([1, 2])
    expect(pagination.isFirstPage.value).toBe(true)
    expect(pagination.isLastPage.value).toBe(false)
  })

  it('moves between pages and clamps page changes to the valid range', async () => {
    const items = ref([1, 2, 3, 4, 5, 6])
    const pagination = usePagination(items, { pageSize: 2 })

    pagination.nextPage()
    await nextTick()

    expect(pagination.currentPage.value).toBe(2)
    expect(pagination.pagedItems.value).toEqual([3, 4])

    pagination.setPage(99)
    await nextTick()

    expect(pagination.currentPage.value).toBe(3)
    expect(pagination.pagedItems.value).toEqual([5, 6])
    expect(pagination.isLastPage.value).toBe(true)

    pagination.previousPage()
    pagination.previousPage()
    pagination.previousPage()
    await nextTick()

    expect(pagination.currentPage.value).toBe(1)
    expect(pagination.pagedItems.value).toEqual([1, 2])
  })

  it('resets to the first page when reset dependencies change', async () => {
    const items = ref([1, 2, 3, 4, 5, 6])
    const query = ref('')
    const pagination = usePagination(computed(() => items.value), {
      pageSize: 2,
      resetOn: [query],
    })

    pagination.setPage(3)
    await nextTick()
    expect(pagination.currentPage.value).toBe(3)

    query.value = 'abc'
    await nextTick()

    expect(pagination.currentPage.value).toBe(1)
    expect(pagination.pagedItems.value).toEqual([1, 2])
  })

  it('clamps the current page when the item count shrinks', async () => {
    const items = ref([1, 2, 3, 4, 5, 6])
    const pagination = usePagination(items, { pageSize: 2, initialPage: 3 })

    await nextTick()
    expect(pagination.currentPage.value).toBe(3)

    items.value = [1, 2, 3]
    await nextTick()

    expect(pagination.pageCount.value).toBe(2)
    expect(pagination.currentPage.value).toBe(2)
    expect(pagination.pagedItems.value).toEqual([3])
  })

  it('keeps multiple pagination instances isolated from each other', async () => {
    const agents = ref([1, 2, 3, 4, 5, 6])
    const teams = ref(['a', 'b', 'c', 'd'])

    const agentPagination = usePagination(agents, { pageSize: 2 })
    const teamPagination = usePagination(teams, { pageSize: 2 })

    agentPagination.setPage(3)
    teamPagination.setPage(2)
    await nextTick()

    expect(agentPagination.currentPage.value).toBe(3)
    expect(agentPagination.pagedItems.value).toEqual([5, 6])
    expect(teamPagination.currentPage.value).toBe(2)
    expect(teamPagination.pagedItems.value).toEqual(['c', 'd'])
  })
})
