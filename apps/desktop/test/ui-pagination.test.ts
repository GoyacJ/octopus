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

    const buttons = wrapper.findAll('button')

    expect(wrapper.text()).toContain('2 per page')
    expect(wrapper.text()).toContain('Page 1 / 3, 6 items')
    expect(wrapper.text()).toContain('1 / 3')
    expect(buttons).toHaveLength(2)
    expect(buttons[0]?.attributes('disabled')).toBeDefined()
    expect(buttons[1]?.attributes('disabled')).toBeUndefined()
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

    const [previousButton, nextButton] = wrapper.findAll('button')

    await previousButton?.trigger('click')
    await nextButton?.trigger('click')

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

    const [previousButton, nextButton] = wrapper.findAll('button')

    await previousButton?.trigger('click')
    await nextButton?.trigger('click')

    expect(wrapper.emitted('update:page')).toBeUndefined()
  })

  it('hides the page info label when hidePageInfo is enabled', () => {
    const wrapper = mount(UiPagination, {
      props: {
        page: 2,
        pageCount: 4,
        summaryLabel: '4 total',
        hidePageInfo: true,
      },
    })

    expect(wrapper.text()).toContain('4 total')
    expect(wrapper.text()).not.toContain('2 / 4')
  })
})
