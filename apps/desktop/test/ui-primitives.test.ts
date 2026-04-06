// @vitest-environment jsdom

import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import { defineComponent, h, ref } from 'vue'

import {
  UiActionCard,
  UiButton,
  UiCodeEditor,
  UiCombobox,
  UiContextMenu,
  UiDataTable,
  UiDialog,
  UiDropdownMenu,
  UiFilterChipGroup,
  UiInfoCard,
  UiMetricCard,
  UiNavCardList,
  UiPageHero,
  UiPanelFrame,
  UiPopover,
  UiRecordCard,
  UiRankingList,
  UiSelectionMenu,
  UiTimelineList,
  UiToolbarRow,
  UiAccordion,
  UiTabs,
} from '@octopus/ui'

Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', {
  configurable: true,
  value: () => {},
})

describe('Shared UI primitives', () => {
  it('renders UiButton variants through a single component API', () => {
    const wrapper = mount(UiButton, {
      props: {
        variant: 'secondary',
      },
      slots: {
        default: 'Save',
      },
    })

    expect(wrapper.text()).toContain('Save')
    expect(wrapper.classes().join(' ')).toContain('bg-secondary')
  })

  it('renders a loading UiButton with disabled semantics and loading label', () => {
    const wrapper = mount(UiButton, {
      props: {
        loading: true,
        loadingLabel: 'Saving',
      },
      slots: {
        default: 'Save',
      },
    })

    expect(wrapper.attributes('disabled')).toBeDefined()
    expect(wrapper.attributes('aria-busy')).toBe('true')
    expect(wrapper.text()).toContain('Saving')
    expect(wrapper.text()).not.toContain('Save')
    expect(wrapper.find('[data-testid="ui-button-spinner"]').exists()).toBe(true)
  })

  it('renders UiDialog content when open and emits visibility updates', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const wrapper = mount(UiDialog, {
      attachTo: document.body,
      props: {
        open: true,
        title: 'Create project',
        description: 'Provide the project name.',
      },
      slots: {
        default: '<div data-testid="dialog-body">Dialog content</div>',
      },
    })

    await wrapper.vm.$nextTick()

    expect(document.body.textContent).toContain('Create project')
    expect(document.body.querySelector('[data-testid="ui-dialog-content"]')).not.toBeNull()
    expect(document.body.querySelector('[data-testid="ui-dialog-body"]')).not.toBeNull()
    expect(document.body.querySelector('[data-testid="dialog-body"]')).not.toBeNull()
    expect(warnSpy).not.toHaveBeenCalled()

    ;(document.body.querySelector('[data-testid="ui-dialog-close"]') as HTMLButtonElement)?.click()

    expect(wrapper.emitted('update:open')).toEqual([[false]])
    wrapper.unmount()
    warnSpy.mockRestore()
  })

  it('renders UiDialog header, footer, and danger slots through the shared shell', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const wrapper = mount(UiDialog, {
      attachTo: document.body,
      props: {
        open: true,
        title: 'Edit agent',
      },
      slots: {
        header: '<div data-testid="dialog-header">Custom header</div>',
        default: '<div data-testid="dialog-body-custom">Dialog body</div>',
        footer: '<button data-testid="dialog-footer" type="button">Save</button>',
        danger: '<div data-testid="dialog-danger">Danger zone</div>',
      },
    })

    await wrapper.vm.$nextTick()

    expect(document.body.querySelector('[data-testid="dialog-header"]')).not.toBeNull()
    expect(document.body.querySelector('[data-testid="dialog-body-custom"]')).not.toBeNull()
    expect(document.body.querySelector('[data-testid="dialog-footer"]')).not.toBeNull()
    expect(document.body.querySelector('[data-testid="dialog-danger"]')).not.toBeNull()
    expect(warnSpy).not.toHaveBeenCalled()

    wrapper.unmount()
    warnSpy.mockRestore()
  })

  it('keeps UiPopover controlled through v-model', async () => {
    const Demo = defineComponent({
      components: { UiPopover },
      setup() {
        const open = ref(false)

        return { open }
      },
      template: `
        <UiPopover v-model:open="open">
          <template #trigger>
            <button type="button" data-testid="popover-trigger">Toggle</button>
          </template>
          <div data-testid="popover-body">Popover content</div>
        </UiPopover>
      `,
    })

    const wrapper = mount(Demo, {
      attachTo: document.body,
    })

    await wrapper.get('[data-testid="popover-trigger"]').trigger('click')
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.open).toBe(true)
    expect(document.body.textContent).toContain('Popover content')
    wrapper.unmount()
  })
  it('renders UiDropdownMenu items and emits selection', async () => {
    const wrapper = mount(UiDropdownMenu, {
      attachTo: document.body,
      props: {
        open: true,
        items: [
          { key: 'rename', label: 'Rename' },
          { key: 'delete', label: 'Delete', tone: 'danger' },
        ],
      },
      slots: {
        trigger: '<button type="button">Open</button>',
      },
    })

    await wrapper.get('[data-testid="ui-dropdown-item-delete"]').trigger('click')

    expect(wrapper.emitted('select')).toEqual([['delete']])
    expect(wrapper.emitted('update:open')).toEqual([[false]])
    wrapper.unmount()
  })

  it('renders UiTabs pill variant and switches controlled values', async () => {
    const Demo = defineComponent({
      components: { UiTabs },
      setup() {
        const value = ref('agent')

        return { value }
      },
      template: `
        <UiTabs
          v-model="value"
          variant="pill"
          :tabs="[
            { value: 'agent', label: 'Agents' },
            { value: 'team', label: 'Teams' },
          ]"
        />
      `,
    })

    const wrapper = mount(Demo)

    expect(wrapper.text()).toContain('Agents')
    expect(wrapper.text()).toContain('Teams')

    await wrapper.get('[data-testid="ui-tabs-trigger-team"]').trigger('click')

    expect(wrapper.vm.value).toBe('team')
  })

  it('renders UiAccordion and updates controlled values', async () => {
    const Demo = defineComponent({
      components: { UiAccordion },
      setup() {
        const value = ref<string[]>(['overview'])

        return { value }
      },
      template: `
        <UiAccordion
          v-model="value"
          :multiple="false"
          :items="[
            { value: 'overview', title: 'Overview', content: 'Overview content' },
            { value: 'activity', title: 'Activity', content: 'Activity content' },
          ]"
        />
      `,
    })

    const wrapper = mount(Demo)

    expect(wrapper.text()).toContain('Overview content')

    await wrapper.get('[data-testid="ui-accordion-trigger-activity"]').trigger('click')

    expect(wrapper.vm.value).toEqual(['activity'])
    expect(wrapper.text()).toContain('Activity content')
  })

  it('filters UiCombobox options and emits selected values', async () => {
    const wrapper = mount(UiCombobox, {
      attachTo: document.body,
      props: {
        options: [
          { value: 'architect', label: 'Architect', keywords: ['system'] },
          { value: 'analyst', label: 'Analyst', keywords: ['research'] },
        ],
      },
    })

    await wrapper.get('[data-testid="ui-combobox-input"]').setValue('sys')

    expect(document.body.textContent).toContain('Architect')
    expect(document.body.textContent).not.toContain('Analyst')

    await wrapper.get('[data-testid="ui-combobox-option-architect"]').trigger('click')

    expect(wrapper.emitted('update:modelValue')).toEqual([['architect']])
    expect(wrapper.emitted('select')).toEqual([['architect']])
    wrapper.unmount()
  })

  it('opens UiContextMenu from right click and emits item selection', async () => {
    const wrapper = mount(UiContextMenu, {
      attachTo: document.body,
      props: {
        items: [
          { key: 'open', label: 'Open' },
          { key: 'archive', label: 'Archive' },
        ],
      },
      slots: {
        default: '<div data-testid="context-target">Context target</div>',
      },
    })

    await wrapper.get('[data-testid="context-target"]').trigger('contextmenu', {
      clientX: 80,
      clientY: 120,
    })

    await wrapper.get('[data-testid="ui-context-item-archive"]').trigger('click')

    expect(wrapper.emitted('select')).toEqual([['archive']])
    wrapper.unmount()
  })

  it('renders UiDataTable with declarative columns', () => {
    const wrapper = mount(UiDataTable, {
      props: {
        data: [
          { id: 'agent-1', name: 'Architect', role: 'Lead' },
          { id: 'agent-2', name: 'Analyst', role: 'Support' },
        ],
        columns: [
          {
            id: 'name',
            header: 'Name',
            accessorKey: 'name',
          },
          {
            id: 'role',
            header: 'Role',
            accessorKey: 'role',
          },
        ],
      },
    })

    expect(wrapper.text()).toContain('Name')
    expect(wrapper.text()).toContain('Architect')
    expect(wrapper.text()).toContain('Support')
  })

  it('keeps UiCodeEditor controlled via modelValue', async () => {
    const wrapper = mount(UiCodeEditor, {
      props: {
        modelValue: '# Title',
        language: 'markdown',
      },
    })

    expect(wrapper.text()).toContain('# Title')

    await wrapper.get('[data-testid="ui-code-editor-textarea"]').setValue('## Changed')

    expect(wrapper.emitted('update:modelValue')).toEqual([['## Changed']])
  })

  it('renders UiPageHero with meta, action, and aside slots', () => {
    const wrapper = mount(UiPageHero, {
      props: {
        title: 'Workspace overview',
        description: 'Shared hero shell',
      },
      slots: {
        meta: '<span>Meta</span>',
        default: '<p>Body</p>',
        actions: '<button type="button">Action</button>',
        aside: '<div>Aside</div>',
      },
    })

    expect(wrapper.text()).toContain('Workspace overview')
    expect(wrapper.text()).toContain('Shared hero shell')
    expect(wrapper.text()).toContain('Meta')
    expect(wrapper.text()).toContain('Action')
    expect(wrapper.text()).toContain('Aside')
  })

  it('renders UiActionCard and UiInfoCard through shared page abstractions', () => {
    const actionCard = mount(UiActionCard, {
      props: {
        title: 'Open knowledge',
        description: 'Jump to project context',
      },
      slots: {
        icon: '<span>K</span>',
      },
    })
    const infoCard = mount(UiInfoCard, {
      props: {
        label: 'Current phase',
        title: 'Delivery',
        description: 'Unified styling',
      },
    })

    expect(actionCard.text()).toContain('Open knowledge')
    expect(actionCard.text()).toContain('Jump to project context')
    expect(infoCard.text()).toContain('Current phase')
    expect(infoCard.text()).toContain('Delivery')
    expect(infoCard.text()).toContain('Unified styling')
  })

  it('renders UiPanelFrame variants through shared shell abstractions', () => {
    const wrapper = mount(UiPanelFrame, {
      props: {
        variant: 'hero',
        padding: 'lg',
        title: 'Panel title',
      },
      slots: {
        actions: '<button type="button" data-testid="panel-frame-action">Edit</button>',
        default: '<div data-testid="panel-frame-body">Hero shell</div>',
      },
    })

    expect(wrapper.find('[data-testid="panel-frame-body"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="panel-frame-action"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('Panel title')
    expect(wrapper.classes().join(' ')).toContain('w-full')
    expect(wrapper.classes().join(' ')).toContain('transition-all')
  })

  it('renders UiMetricCard with helper text and progress state', () => {
    const wrapper = mount(UiMetricCard, {
      props: {
        label: 'Coverage',
        value: '84%',
        helper: 'Shared UI adoption',
        progress: 84,
      },
    })

    expect(wrapper.text()).toContain('Coverage')
    expect(wrapper.text()).toContain('84%')
    expect(wrapper.text()).toContain('Shared UI adoption')
    expect(wrapper.find('[data-testid="ui-metric-progress"]').attributes('style')).toContain('84%')
  })

  it('renders UiRankingList and UiTimelineList through shared data-display abstractions', () => {
    const ranking = mount(UiRankingList, {
      props: {
        items: [
          { id: 'a', label: 'Architect', value: '240', ratio: 0.8, helper: 'Primary owner' },
          { id: 'b', label: 'Analyst', value: '120', ratio: 0.4 },
        ],
      },
    })
    const timeline = mount(UiTimelineList, {
      props: {
        items: [
          { id: 'step-1', title: 'Review shell', description: 'Collapsed custom surfaces', timestamp: '2026-04-03 10:00', helper: 'conversation' },
        ],
        density: 'compact',
      },
    })

    expect(ranking.text()).toContain('Architect')
    expect(ranking.text()).toContain('240')
    expect(ranking.find('[data-testid="ui-ranking-bar-a"]').attributes('style')).toContain('80%')
    expect(timeline.text()).toContain('Review shell')
    expect(timeline.text()).toContain('Collapsed custom surfaces')
    expect(timeline.text()).toContain('2026-04-03 10:00')
    expect(timeline.text()).toContain('conversation')
  })

  it('renders UiToolbarRow and UiNavCardList with shared composition slots', async () => {
    const toolbar = mount(UiToolbarRow, {
      props: {
        testId: 'toolbar-root',
      },
      slots: {
        search: '<input data-testid="toolbar-search" />',
        filters: '<div data-testid="toolbar-filters">filters</div>',
        tabs: '<div data-testid="toolbar-tabs">tabs</div>',
        chips: '<div data-testid="toolbar-chips">chips</div>',
        views: '<div data-testid="toolbar-views">views</div>',
        actions: '<button data-testid="toolbar-action">Create</button>',
      },
    })

    const nav = mount(UiNavCardList, {
      props: {
        density: 'compact',
        items: [
          { id: 'profile', label: 'Profile', helper: 'Current user', active: true, badge: 'live' },
          { id: 'roles', label: 'Roles', helper: 'Manage bindings' },
        ],
      },
    })

    expect(toolbar.find('[data-testid="toolbar-root"]').exists()).toBe(true)
    expect(toolbar.find('[data-testid="toolbar-search"]').exists()).toBe(true)
    expect(toolbar.find('[data-testid="toolbar-filters"]').exists()).toBe(true)
    expect(toolbar.find('[data-testid="toolbar-tabs"]').exists()).toBe(true)
    expect(toolbar.find('[data-testid="toolbar-chips"]').exists()).toBe(true)
    expect(toolbar.find('[data-testid="toolbar-views"]').exists()).toBe(true)
    expect(toolbar.find('[data-testid="toolbar-action"]').exists()).toBe(true)
    expect(nav.find('[data-testid="ui-nav-card-profile"]').classes()).toContain('is-active')

    await nav.get('[data-testid="ui-nav-card-action-roles"]').trigger('click')

    expect(nav.emitted('select')).toEqual([['roles']])
  })

  it('renders UiSelectionMenu with grouped items', () => {
    const wrapper = mount(UiSelectionMenu, {
      attachTo: document.body,
      props: {
        open: true,
        title: 'Select actor',
        testId: 'selection-menu',
        sections: [
          {
            label: 'Agents',
            items: [
              { id: 'agent:architect', label: 'Architect', helper: 'Primary owner' },
              { id: 'agent:analyst', label: 'Analyst' },
            ],
          },
          {
            label: 'Teams',
            items: [
              { id: 'team:redesign', label: 'Redesign team', active: true },
            ],
          },
        ],
      },
      slots: {
        trigger: '<button type="button" data-testid="selection-trigger">Open</button>',
      },
    })

    expect(wrapper.props('open')).toBe(true)
    expect(wrapper.props('title')).toBe('Select actor')
    expect(wrapper.props('sections')).toHaveLength(2)
    expect(wrapper.find('[data-testid="selection-trigger"]').exists()).toBe(true)
    wrapper.unmount()
  })

  it('renders UiSelectionMenu custom item slots without breaking mount behavior', () => {
    const wrapper = mount(UiSelectionMenu, {
      attachTo: document.body,
      props: {
        open: true,
        sections: [
          {
            id: 'agents',
            items: [
              { id: 'agent:architect', label: 'Architect' },
            ],
          },
        ],
      },
      slots: {
        trigger: '<button type="button">Open</button>',
        item: ({ item }: { item: { id: string, label: string } }) => h('button', { 'data-testid': `custom-selection-${item.id}` }, item.label),
      },
    })

    expect(wrapper.props('sections')).toHaveLength(1)
    expect(wrapper.html()).toContain('Open')
    wrapper.unmount()
  })

  it('renders UiFilterChipGroup and toggles the active option', async () => {
    const Demo = defineComponent({
      components: { UiFilterChipGroup },
      setup() {
        const value = ref('')

        return { value }
      },
      template: `
        <UiFilterChipGroup
          v-model="value"
          test-id="filter-chip-group"
          :items="[
            { value: 'frontend', label: '前端开发' },
            { value: 'testing', label: '自动测试' },
          ]"
        />
      `,
    })
    const wrapper = mount(Demo)

    expect(wrapper.get('[data-testid="filter-chip-group"]').exists()).toBe(true)

    await wrapper.get('[data-testid="ui-filter-chip-frontend"]').trigger('click')
    await wrapper.get('[data-testid="ui-filter-chip-frontend"]').trigger('click')

    expect(wrapper.vm.value).toBe('')
  })

  it('renders UiRecordCard with leading, metrics, meta, and actions', async () => {
    const wrapper = mount(UiRecordCard, {
      props: {
        title: 'Workspace Admin',
        description: 'Can manage all user-center settings',
        active: true,
        interactive: true,
        layout: 'tile',
        testId: 'record-card',
      },
      slots: {
        eyebrow: '<span>role.admin</span>',
        leading: '<span data-testid="record-leading">WA</span>',
        badges: '<span data-testid="record-badge">live</span>',
        metrics: '<span data-testid="record-metrics">84%</span>',
        secondary: '<span data-testid="record-secondary">default scope</span>',
        meta: '<span data-testid="record-meta">members: 3</span>',
        actions: '<button data-testid="record-action" type="button">Edit</button>',
      },
    })

    await wrapper.get('[data-testid="record-card"]').trigger('click')

    expect(wrapper.get('[data-testid="record-card"]').classes()).toContain('is-active')
    expect(wrapper.get('[data-testid="record-card"]').attributes('data-ui-record-layout')).toBe('tile')
    expect(wrapper.text()).toContain('Workspace Admin')
    expect(wrapper.text()).toContain('Can manage all user-center settings')
    expect(wrapper.find('[data-testid="record-leading"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="record-badge"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="record-metrics"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="record-secondary"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="record-meta"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="record-action"]').exists()).toBe(true)
    expect(wrapper.emitted('click')).toHaveLength(1)
  })
})
