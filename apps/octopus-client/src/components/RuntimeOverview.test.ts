import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'

import RuntimeOverview from './RuntimeOverview.vue'

describe('RuntimeOverview', () => {
  it('renders the runtime shell headline and core contract sections', () => {
    const wrapper = mount(RuntimeOverview)

    expect(wrapper.text()).toContain('Unified Agent Runtime Platform')
    expect(wrapper.text()).toContain('Run')
    expect(wrapper.text()).toContain('Chat')
  })
})

