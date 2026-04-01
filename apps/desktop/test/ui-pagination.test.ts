// @vitest-environment jsdom

import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import { UiPagination } from '@octopus/ui'

describe('UiPagination', () => {
  it('renders labels and disabled states from the controlled props', () => {
    const wrapper = mount(UiPagination, {
      props: {
        page: 1,
        pageCount: 3,
        previousLabel: 'Previous',
        nextLabel: 'Next',
        summaryLabel: 'Page 1 / 3, 6 items',
        metaLabel: '2 per page',
        pageInfoLabel: '1 / 3',
      },
    })

    expect(wrapper.text()).toContain('2 per page')
    expect(wrapper.text()).toContain('Page 1 / 3, 6 items')
    expect(wrapper.text()).toContain('1 / 3')
    expect(wrapper.get('[data-testid="ui-pagination-prev"]').attributes('disabled')).toBeDefined()
    expect(wrapper.get('[data-testid="ui-pagination-next"]').attributes('disabled')).toBeUndefined()
  })

  it('emits update:page only for valid previous and next page transitions', async () => {
    const wrapper = mount(UiPagination, {
      props: {
        page: 2,
        pageCount: 3,
        previousLabel: 'Previous',
        nextLabel: 'Next',
        summaryLabel: 'Page 2 / 3',
        pageInfoLabel: '2 / 3',
      },
    })

    await wrapper.get('[data-testid="ui-pagination-prev"]').trigger('click')
    await wrapper.get('[data-testid="ui-pagination-next"]').trigger('click')

    expect(wrapper.emitted('update:page')).toEqual([[1], [3]])
  })

  it('does not emit page updates when already at the bounds', async () => {
    const wrapper = mount(UiPagination, {
      props: {
        page: 1,
        pageCount: 1,
        previousLabel: 'Previous',
        nextLabel: 'Next',
        summaryLabel: 'Page 1 / 1',
      },
    })

    await wrapper.get('[data-testid="ui-pagination-prev"]').trigger('click')
    await wrapper.get('[data-testid="ui-pagination-next"]').trigger('click')

    expect(wrapper.emitted('update:page')).toBeUndefined()
  })
})
