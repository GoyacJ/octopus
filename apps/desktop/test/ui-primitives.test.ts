// @vitest-environment jsdom

import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import { defineComponent, h, nextTick, ref } from 'vue'

import {
  UiArtifactBlock,
  UiActionCard,
  UiBadge,
  UiButton,
  UiCodeEditor,
  UiCombobox,
  UiContextMenu,
  UiDataTable,
  UiDialog,
  UiDropdownMenu,
  UiEmptyState,
  UiErrorState,
  UiFilterChipGroup,
  UiInfoCard,
  UiInboxBlock,
  UiMetricCard,
  UiNavCardList,
  UiNotificationBadge,
  UiPageHeader,
  UiPageHero,
  UiPageShell,
  UiPanelFrame,
  UiPopover,
  UiRecordCard,
  UiRankingList,
  UiRestrictedState,
  UiSearchableMultiSelect,
  UiSelectionMenu,
  UiSkeleton,
  UiSurface,
  UiConversationComposerShell,
  UiDotLottie,
  UiInspectorPanel,
  UiKbd,
  UiHierarchyList,
  UiListRow,
  UiListDetailShell,
  UiListDetailWorkspace,
  UiMessageCenter,
  UiNotificationCenter,
  UiNotificationRow,
  UiStatusCallout,
  UiStatTile,
  UiSwitch,
  UiRiveCanvas,
  UiToastItem,
  UiToastViewport,
  UiTraceBlock,
  UiTimelineList,
  UiToolbarRow,
  UiAccordion,
  UiTabs,
} from '@octopus/ui'
import type { InboxItemRecord, NotificationRecord } from '@octopus/schema'

Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', {
  configurable: true,
  value: () => {},
})

function createNotification(overrides: Partial<NotificationRecord> = {}): NotificationRecord {
  return {
    id: 'notif-1',
    scopeKind: 'app',
    level: 'info',
    title: 'Saved',
    body: 'Preferences updated.',
    source: 'settings',
    createdAt: 1,
    readAt: undefined,
    toastVisibleUntil: undefined,
    scopeOwnerId: undefined,
    routeTo: undefined,
    actionLabel: undefined,
    ...overrides,
  }
}

function createInboxItem(overrides: Partial<InboxItemRecord> = {}): InboxItemRecord {
  return {
    id: 'inbox-1',
    workspaceId: 'ws-local',
    projectId: 'proj-redesign',
    itemType: 'approval',
    title: 'Need approval',
    description: 'Runtime needs approval.',
    status: 'pending',
    priority: 'high',
    actionable: true,
    routeTo: '/workspaces/ws-local/projects/proj-redesign/settings',
    actionLabel: 'Review approval',
    createdAt: 1,
    ...overrides,
  }
}

function formatUiTimestamp(timestamp: number): string {
  return new Intl.DateTimeFormat(undefined, {
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  }).format(new Date(timestamp))
}

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
    expect(wrapper.classes().join(' ')).toContain('bg-surface')
    expect(wrapper.classes().join(' ')).toContain('border-border-subtle')
  })

  it('renders UiNotificationBadge as a brand-soft pill and caps large counts', () => {
    const wrapper = mount(UiNotificationBadge, {
      props: {
        count: 124,
      },
    })

    const classes = wrapper.get('[data-testid="ui-notification-badge"]').classes().join(' ')

    expect(wrapper.text()).toBe('99+')
    expect(classes).toContain('rounded-full')
    expect(classes).toContain('bg-accent')
    expect(classes).toContain('border-border-strong')
    expect(classes).toContain('text-accent-foreground')
    expect(classes).not.toContain('bg-foreground')
  })

  it('keeps UiButton outline and ghost variants on neutral hover states', () => {
    const outlineButton = mount(UiButton, {
      props: {
        variant: 'outline',
      },
      slots: {
        default: 'Open',
      },
    })
    const ghostButton = mount(UiButton, {
      props: {
        variant: 'ghost',
      },
      slots: {
        default: 'Later',
      },
    })

    const outlineClasses = outlineButton.classes().join(' ')
    const ghostClasses = ghostButton.classes().join(' ')

    expect(outlineClasses).toContain('bg-surface')
    expect(outlineClasses).toContain('hover:bg-subtle')
    expect(outlineClasses).toContain('enabled:active:scale-[0.99]')
    expect(outlineClasses).not.toContain('hover:bg-accent')
    expect(ghostClasses).toContain('hover:bg-subtle')
    expect(ghostClasses).toContain('enabled:active:scale-[0.99]')
    expect(ghostClasses).not.toContain('hover:bg-accent')
  })

  it('keeps UiBadge info tone on a soft semantic fill instead of the raw accent fill', () => {
    const wrapper = mount(UiBadge, {
      props: {
        label: 'Live',
        tone: 'info',
      },
    })

    const classes = wrapper.classes().join(' ')

    expect(classes).toContain('bg-[var(--color-status-info-soft)]')
    expect(classes).toContain('text-status-info')
    expect(classes).toContain('text-micro')
    expect(classes).not.toContain('bg-accent')
  })

  it('renders UiHierarchyList with expandable branches and selectable leaf nodes', async () => {
    const wrapper = mount(UiHierarchyList, {
      props: {
        items: [
          {
            id: 'workspace',
            label: 'Workspace',
            depth: 0,
            expandable: true,
            expanded: true,
          },
          {
            id: 'workspace.overview.read',
            label: 'workspace.overview.read',
            depth: 1,
          },
        ],
        selectedId: 'workspace.overview.read',
      },
      slots: {
        default: ({ item }: { item: { label: string } }) => h('span', { 'data-testid': 'hierarchy-label' }, item.label),
      },
    })

    expect(wrapper.get('[data-testid="ui-hierarchy-list"]').exists()).toBe(true)
    expect(wrapper.get('[data-testid="ui-hierarchy-item-workspace"]').attributes('data-depth')).toBe('0')
    expect(wrapper.get('[data-testid="ui-hierarchy-item-workspace.overview.read"]').attributes('data-depth')).toBe('1')

    await wrapper.get('[data-testid="ui-hierarchy-toggle-workspace"]').trigger('click')
    expect(wrapper.emitted('toggle')).toEqual([['workspace']])

    await wrapper.get('[data-testid="ui-hierarchy-item-workspace.overview.read"]').trigger('click')
    expect(wrapper.emitted('select')).toEqual([['workspace.overview.read']])
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

  it('constrains tall UiDialog content to the viewport and scrolls inside the body region', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const wrapper = mount(UiDialog, {
      attachTo: document.body,
      props: {
        open: true,
        title: 'Grant models',
      },
      slots: {
        default: '<div data-testid="dialog-scroll-content">Long dialog content</div>',
        footer: '<button data-testid="dialog-footer-scroll" type="button">Save</button>',
      },
    })

    await wrapper.vm.$nextTick()

    const content = document.body.querySelector<HTMLElement>('[data-testid="ui-dialog-content"]')
    const body = document.body.querySelector<HTMLElement>('[data-testid="ui-dialog-body"]')

    expect(content).not.toBeNull()
    expect(content?.className).toContain('max-h-[calc(100dvh-2rem)]')
    expect(content?.className).toContain('overflow-hidden')

    expect(body).not.toBeNull()
    expect(body?.className).toContain('flex-1')
    expect(body?.className).toContain('min-h-0')
    expect(body?.className).toContain('overflow-y-auto')
    expect(warnSpy).not.toHaveBeenCalled()

    wrapper.unmount()
    warnSpy.mockRestore()
  })

  it('keeps the UiDialog close control on the neutral hover grammar', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    const wrapper = mount(UiDialog, {
      attachTo: document.body,
      props: {
        open: true,
        title: 'Close project',
      },
    })

    await wrapper.vm.$nextTick()

    const closeButton = document.body.querySelector('[data-testid="ui-dialog-close"]')

    expect(closeButton).not.toBeNull()
    expect(closeButton?.className).toContain('hover:bg-subtle')
    expect(closeButton?.className).not.toContain('hover:bg-accent')
    expect(closeButton?.className).toContain('ui-focus-ring')
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

    const popoverContent = document.body.querySelector('[data-testid="ui-popover-content"]')

    expect(wrapper.vm.open).toBe(true)
    expect(popoverContent).not.toBeNull()
    expect(popoverContent?.className).toContain('border-[color-mix(in_srgb,var(--border)_84%,transparent)]')
    expect(popoverContent?.className).not.toContain('border-border')
    expect(document.body.textContent).toContain('Popover content')
    wrapper.unmount()
  })
  it('renders UiDropdownMenu items and emits selection', async () => {
    const Demo = defineComponent({
      components: { UiDropdownMenu },
      setup() {
        const open = ref(false)
        const selected = ref('')
        return { open, selected }
      },
      template: `
        <UiDropdownMenu
          v-model:open="open"
          :items="[
            { key: 'rename', label: 'Rename', shortcut: ['⌘', 'R'] },
            { key: 'delete', label: 'Delete', tone: 'danger' },
          ]"
          @select="selected = $event"
        >
          <template #trigger>
            <button type="button" data-testid="dropdown-trigger">Open</button>
          </template>
        </UiDropdownMenu>
      `,
    })

    const wrapper = mount(Demo, {
      attachTo: document.body,
    })

    await wrapper.get('[data-testid="dropdown-trigger"]').trigger('click')
    await wrapper.vm.$nextTick()

    const renameItem = document.body.querySelector<HTMLElement>('[data-testid="ui-dropdown-item-rename"]')
    const deleteItem = document.body.querySelector('[data-testid="ui-dropdown-item-delete"]') as HTMLElement | null
    const dropdownContent = document.body.querySelector<HTMLElement>('[data-testid="ui-dropdown-content"]')

    expect(renameItem).not.toBeNull()
    expect(dropdownContent).not.toBeNull()
    expect(dropdownContent?.className).toContain('border-[color-mix(in_srgb,var(--border)_84%,transparent)]')
    expect(dropdownContent?.className).not.toContain('border-border')
    expect(renameItem?.querySelector('[data-testid="ui-kbd"]')?.textContent).toContain('⌘+R')
    expect(renameItem?.className).toContain('data-[highlighted]:bg-subtle')
    expect(renameItem?.className).not.toContain('data-[highlighted]:bg-accent')
    expect(deleteItem).not.toBeNull()
    deleteItem?.click()

    expect(wrapper.vm.selected).toBe('delete')
    wrapper.unmount()
  })

  it('opens UiDropdownMenu from its trigger in controlled mode', async () => {
    const Demo = defineComponent({
      components: { UiDropdownMenu },
      setup() {
        const open = ref(false)
        return { open }
      },
      template: `
        <UiDropdownMenu
          v-model:open="open"
          :items="[{ key: 'export-folder', label: 'Export folder' }]"
        >
          <template #trigger>
            <button type="button" data-testid="dropdown-trigger">Open menu</button>
          </template>
        </UiDropdownMenu>
      `,
    })

    const wrapper = mount(Demo, {
      attachTo: document.body,
    })

    await wrapper.get('[data-testid="dropdown-trigger"]').trigger('click')
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.open).toBe(true)
    expect(document.body.textContent).toContain('Export folder')

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

  it('keeps shared focus-ring utility on tabs, accordion triggers, hierarchy toggles, and switches', () => {
    const tabs = mount(UiTabs, {
      props: {
        modelValue: 'agent',
        variant: 'pill',
        tabs: [
          { value: 'agent', label: 'Agents' },
          { value: 'team', label: 'Teams' },
        ],
      },
    })
    const accordion = mount(UiAccordion, {
      props: {
        items: [
          { value: 'overview', title: 'Overview', content: 'Details' },
        ],
      },
    })
    const hierarchy = mount(UiHierarchyList, {
      props: {
        items: [
          {
            id: 'workspace',
            label: 'Workspace',
            depth: 0,
            expandable: true,
            expanded: false,
          },
        ],
      },
    })
    const uiSwitch = mount(UiSwitch, {
      props: {
        modelValue: false,
      },
    })

    expect(tabs.get('[data-testid="ui-tabs-trigger-agent"]').classes().join(' ')).toContain('ui-focus-ring')
    expect(accordion.get('[data-testid="ui-accordion-trigger-overview"]').classes().join(' ')).toContain('ui-focus-ring')
    expect(hierarchy.get('[data-testid="ui-hierarchy-toggle-workspace"]').classes().join(' ')).toContain('ui-focus-ring')

    const switchClasses = uiSwitch.get('[role="switch"]').classes().join(' ')
    expect(switchClasses).toContain('ui-focus-ring')
    expect(switchClasses).not.toContain('ring-offset-1')
  })

  it('renders UiMessageCenter with notification and inbox tabs and emits inbox selection', async () => {
    const notificationCreatedAt = Date.UTC(2026, 3, 12, 10, 5)
    const inboxCreatedAt = Date.UTC(2026, 3, 11, 8, 15)
    const wrapper = mount(UiMessageCenter, {
      attachTo: document.body,
      props: {
        open: true,
        activeTab: 'notifications',
        notificationTabLabel: 'Notifications',
        inboxTabLabel: 'Inbox',
        notificationTitle: 'Notifications',
        notificationUnreadLabel: '2 unread',
        notificationEmptyTitle: 'No notifications',
        notificationEmptyDescription: 'Everything is up to date.',
        notificationMarkAllLabel: 'Mark all read',
        notifications: [
          createNotification({ createdAt: notificationCreatedAt }),
          createNotification({ id: 'notif-2', scopeKind: 'workspace', createdAt: notificationCreatedAt + 60_000 }),
        ],
        unreadCount: 2,
        activeFilter: 'all',
        filterLabels: {
          all: 'All',
          app: 'App',
          workspace: 'Workspace',
          user: 'User',
        },
        scopeLabels: {
          app: 'App',
          workspace: 'Workspace',
          user: 'User',
        },
        inboxTitle: 'Inbox',
        inboxSubtitle: '1 actionable item',
        inboxLoading: false,
        inboxError: '',
        inboxItems: [createInboxItem({ createdAt: inboxCreatedAt })],
        inboxEmptyTitle: 'No inbox items',
        inboxEmptyDescription: 'Nothing requires attention.',
        inboxOpenLabel: 'Open',
        inboxStatusHeading: 'Status',
        inboxTypeHeading: 'Type',
        inboxLoadingLabel: 'Loading inbox…',
        inboxErrorTitle: 'Inbox unavailable',
        inboxErrorDescription: 'Try again later.',
      },
    })

    expect(document.body.textContent).toContain('Notifications')
    expect(document.body.textContent).toContain('Inbox')
    expect(document.body.textContent).toContain(formatUiTimestamp(notificationCreatedAt))
    expect(wrapper.findComponent(UiSurface).attributes('class') ?? '').toContain('border-[color-mix(in_srgb,var(--border)_84%,transparent)]')
    expect(wrapper.findComponent(UiSurface).attributes('class') ?? '').not.toContain('border-border')
    expect(wrapper.get('[data-testid="ui-notification-filter-all"]').classes().join(' ')).toContain('bg-subtle')
    expect(wrapper.get('[data-testid="ui-notification-filter-all"]').classes().join(' ')).not.toContain('bg-accent')
    expect(wrapper.get('[data-testid="ui-notification-filter-all"]').classes().join(' ')).not.toContain('shadow-xs')
    expect(wrapper.get('[data-testid="ui-notification-filter-workspace"]').classes().join(' ')).toContain('hover:bg-subtle')
    expect(wrapper.get('[data-testid="ui-notification-filter-workspace"]').classes().join(' ')).not.toContain('hover:bg-accent')

    await wrapper.get('[data-testid="ui-tabs-trigger-inbox"]').trigger('click')

    expect(wrapper.emitted('update:activeTab')).toEqual([['inbox']])

    await wrapper.setProps({ activeTab: 'inbox' })

    expect(document.body.textContent).toContain('Need approval')
    expect(document.body.textContent).toContain('Review approval')
    expect(document.body.textContent).toContain(formatUiTimestamp(inboxCreatedAt))

    await wrapper.get('[data-testid="ui-message-center-inbox-action-inbox-1"]').trigger('click')

    expect(wrapper.emitted('select-inbox')).toEqual([[expect.objectContaining({ id: 'inbox-1' })]])
    wrapper.unmount()
  })

  it('renders UiMessageCenter inbox loading, error, and empty states', async () => {
    const wrapper = mount(UiMessageCenter, {
      attachTo: document.body,
      props: {
        open: true,
        activeTab: 'inbox',
        notificationTabLabel: 'Notifications',
        inboxTabLabel: 'Inbox',
        notificationTitle: 'Notifications',
        notificationUnreadLabel: '0 unread',
        notificationEmptyTitle: 'No notifications',
        notificationEmptyDescription: 'Everything is up to date.',
        notificationMarkAllLabel: 'Mark all read',
        notifications: [],
        unreadCount: 0,
        activeFilter: 'all',
        filterLabels: {
          all: 'All',
          app: 'App',
          workspace: 'Workspace',
          user: 'User',
        },
        scopeLabels: {
          app: 'App',
          workspace: 'Workspace',
          user: 'User',
        },
        inboxTitle: 'Inbox',
        inboxSubtitle: '0 actionable items',
        inboxLoading: true,
        inboxError: '',
        inboxItems: [],
        inboxEmptyTitle: 'No inbox items',
        inboxEmptyDescription: 'Nothing requires attention.',
        inboxOpenLabel: 'Open',
        inboxStatusHeading: 'Status',
        inboxTypeHeading: 'Type',
        inboxLoadingLabel: 'Loading inbox…',
        inboxErrorTitle: 'Inbox unavailable',
        inboxErrorDescription: 'Try again later.',
      },
    })

    expect(document.body.textContent).toContain('Loading inbox…')

    await wrapper.setProps({ inboxLoading: false, inboxError: 'network down' })
    expect(document.body.textContent).toContain('Inbox unavailable')
    expect(document.body.textContent).toContain('Try again later.')

    await wrapper.setProps({ inboxError: '' })
    expect(document.body.textContent).toContain('No inbox items')

    wrapper.unmount()
  })

  it('renders UiListDetailWorkspace with toolbar, list, and detail states', () => {
    const wrapper = mount(UiListDetailWorkspace, {
      props: {
        detailTitle: '用户详情',
        detailSubtitle: '查看并编辑当前用户',
        emptyDetailTitle: '请选择用户',
        emptyDetailDescription: '从左侧列表中选择一个用户后即可查看详情。',
      },
      slots: {
        toolbar: '<div data-testid="workspace-toolbar-slot">Toolbar</div>',
        list: '<div data-testid="workspace-list-slot">List</div>',
      },
    })

    expect(wrapper.find('[data-testid="ui-list-detail-workspace"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="ui-list-detail-workspace-toolbar"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="workspace-toolbar-slot"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="workspace-list-slot"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('请选择用户')
    expect(wrapper.text()).toContain('从左侧列表中选择一个用户后即可查看详情。')
  })

  it('renders UiListDetailShell and UiInspectorPanel with one integrated list-detail shell grammar', () => {
    const wrapper = mount(defineComponent({
      components: {
        UiInspectorPanel,
        UiListDetailShell,
      },
      template: `
        <UiListDetailShell>
          <template #list>
            <div data-testid="ui-list-detail-list-slot">List</div>
          </template>
          <UiInspectorPanel title="Inspector" subtitle="Shared detail shell">
            <div data-testid="ui-list-detail-detail-slot">Detail</div>
          </UiInspectorPanel>
        </UiListDetailShell>
      `,
    }))

    const shell = wrapper.find('[data-testid="ui-list-detail-shell"]')
    const listPane = wrapper.find('[data-testid="ui-list-detail-shell-list"]')
    const detailPane = wrapper.find('[data-testid="ui-list-detail-shell-detail"]')
    const inspector = wrapper.find('[data-testid="ui-inspector-panel"]')
    const inspectorHeader = wrapper.find('[data-testid="ui-inspector-panel-header"]')
    const inspectorBody = wrapper.find('[data-testid="ui-inspector-panel-body"]')

    expect(shell.exists()).toBe(true)
    expect(shell.classes().join(' ')).toContain('gap-px')
    expect(shell.classes().join(' ')).toContain('overflow-hidden')
    expect(shell.classes().join(' ')).toContain('bg-border')

    expect(listPane.exists()).toBe(true)
    expect(listPane.classes().join(' ')).toContain('bg-surface')

    expect(detailPane.exists()).toBe(true)
    expect(detailPane.classes().join(' ')).toContain('bg-[color-mix(in_srgb,var(--surface)_72%,var(--subtle)_28%)]')

    expect(inspector.exists()).toBe(true)
    expect(inspector.classes().join(' ')).toContain('bg-subtle')
    expect(inspector.classes().join(' ')).not.toContain('shadow-xs')

    expect(inspectorHeader.exists()).toBe(true)
    expect(inspectorHeader.classes().join(' ')).toContain('border-b')
    expect(inspectorBody.exists()).toBe(true)
    expect(inspectorBody.classes().join(' ')).toContain('px-5')
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
    const activityTriggerClasses = wrapper.get('[data-testid="ui-accordion-trigger-activity"]').classes().join(' ')

    expect(wrapper.text()).toContain('Overview content')
    expect(activityTriggerClasses).toContain('hover:bg-subtle')
    expect(activityTriggerClasses).not.toContain('hover:bg-accent')

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

    const comboboxContent = document.body.querySelector<HTMLElement>('[data-testid="ui-combobox-content"]')

    expect(comboboxContent).not.toBeNull()
    expect(comboboxContent?.className).toContain('border-[color-mix(in_srgb,var(--border)_84%,transparent)]')
    expect(comboboxContent?.className).not.toContain('border-border')
    expect(wrapper.get('[data-testid="ui-combobox-option-architect"]').classes().join(' ')).toContain('data-[highlighted]:bg-subtle')
    expect(wrapper.get('[data-testid="ui-combobox-option-architect"]').classes().join(' ')).not.toContain('data-[highlighted]:bg-accent')
    expect(document.body.textContent).toContain('Architect')
    expect(document.body.textContent).not.toContain('Analyst')

    await wrapper.get('[data-testid="ui-combobox-option-architect"]').trigger('click')

    expect(wrapper.emitted('update:modelValue')).toEqual([['architect']])
    expect(wrapper.emitted('select')).toEqual([['architect']])
    wrapper.unmount()
  })

  it('renders the selected UiCombobox label instead of the raw value', async () => {
    const wrapper = mount(UiCombobox, {
      attachTo: document.body,
      props: {
        modelValue: 'agent-7849d6eq',
        options: [
          { value: 'agent-7849d6eq', label: '产品负责人', keywords: ['product'] },
          { value: 'agent-analyst', label: '市场研究员', keywords: ['research'] },
        ],
      },
    })

    const input = wrapper.get('[data-testid="ui-combobox-input"]')
    expect((input.element as HTMLInputElement).value).toBe('产品负责人')

    await input.trigger('focus')
    await wrapper.vm.$nextTick()

    expect((input.element as HTMLInputElement).value).toBe('产品负责人')
    wrapper.unmount()
  })

  it('opens UiContextMenu from right click and emits item selection', async () => {
    const wrapper = mount(UiContextMenu, {
      attachTo: document.body,
      props: {
        items: [
          { key: 'open', label: 'Open', shortcut: ['Enter'] },
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

    const archiveItem = wrapper.get('[data-testid="ui-context-item-archive"]')
    const contextContent = document.body.querySelector<HTMLElement>('[data-testid="ui-context-content"]')

    expect(contextContent).not.toBeNull()
    expect(contextContent?.className).toContain('border-[color-mix(in_srgb,var(--border)_84%,transparent)]')
    expect(contextContent?.className).not.toContain('border-border')
    expect(document.body.querySelector('[data-testid="ui-context-item-open"] [data-testid="ui-kbd"]')?.textContent).toContain('Enter')
    expect(archiveItem.classes().join(' ')).toContain('data-[highlighted]:bg-subtle')
    expect(archiveItem.classes().join(' ')).not.toContain('data-[highlighted]:bg-accent')
    await wrapper.get('[data-testid="ui-context-item-archive"]').trigger('click')

    expect(wrapper.emitted('select')).toEqual([['archive']])
    wrapper.unmount()
  })

  it('renders UiListRow with shared active semantics and slot regions', () => {
    const wrapper = mount(UiListRow, {
      props: {
        title: 'Workspace API',
        subtitle: 'Shared adapter contract',
        eyebrow: 'Runtime',
        active: true,
        interactive: true,
      },
      slots: {
        meta: '<span data-testid="list-row-meta">Meta</span>',
        actions: '<button data-testid="list-row-action" type="button">Open</button>',
      },
    })

    expect(wrapper.attributes('data-ui-state')).toBe('active')
    expect(wrapper.classes().join(' ')).toContain('active:scale-[0.99]')
    expect(wrapper.find('[data-testid="list-row-meta"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="list-row-action"]').exists()).toBe(true)
  })

  it('renders UiStatTile, UiTraceBlock, and UiArtifactBlock with canonical tone metadata', () => {
    const statTile = mount(UiStatTile, {
      props: {
        label: 'Agents',
        value: '12',
        helper: 'Active workforce',
        tone: 'warning',
      },
    })
    const traceBlock = mount(UiTraceBlock, {
      props: {
        title: 'Workspace sync',
        detail: 'Updated runtime snapshot.',
        actor: 'Runtime',
        timestampLabel: '09:41',
        tone: 'info',
      },
    })
    const artifactBlock = mount(UiArtifactBlock, {
      props: {
        title: 'Runtime Delivery Summary',
        excerpt: 'Latest artifact emitted by the workspace runtime.',
        typeLabel: 'Report',
        versionLabel: 'v3',
        statusLabel: 'Published',
      },
    })

    expect(statTile.attributes('data-ui-tone')).toBe('warning')
    expect(traceBlock.attributes('data-ui-tone')).toBe('info')
    expect(artifactBlock.attributes('data-ui-artifact-block')).toBe('true')
  })

  it('renders UiTraceBlock metadata chips without turning it into a nested timeline', () => {
    const traceBlock = mount(UiTraceBlock, {
      props: {
        title: 'Workspace sync',
        detail: 'Updated runtime snapshot.',
        actor: 'Runtime',
        timestampLabel: '09:41',
        tone: 'warning',
        metaItems: ['Tool', 'workspace-api'],
      },
    })

    expect(traceBlock.find('[data-testid="ui-trace-block-meta"]').exists()).toBe(true)
    expect(traceBlock.findAll('[data-testid="ui-trace-block-meta-item"]')).toHaveLength(2)
    expect(traceBlock.text()).toContain('Tool')
    expect(traceBlock.text()).toContain('workspace-api')
  })

  it('renders AI-native block primitives with integrated bands instead of floating cards', () => {
    const composer = mount(UiConversationComposerShell, {
      slots: {
        default: '<div data-testid="composer-slot">Composer</div>',
      },
    })
    const artifactBlock = mount(UiArtifactBlock, {
      props: {
        title: 'Runtime Delivery Summary',
        excerpt: 'Latest artifact emitted by the workspace runtime.',
        typeLabel: 'Report',
        versionLabel: 'v3',
        statusLabel: 'Published',
      },
    })
    const inboxBlock = mount(UiInboxBlock, {
      props: {
        title: 'Need approval',
        description: 'Runtime needs approval.',
        priorityLabel: 'High',
        timestampLabel: '09:41',
        statusLabel: 'Pending',
      },
    })
    const traceBlock = mount(UiTraceBlock, {
      props: {
        title: 'Workspace sync',
        detail: 'Updated runtime snapshot.',
        actor: 'Runtime',
        timestampLabel: '09:41',
        tone: 'info',
      },
    })

    expect(composer.classes().join(' ')).toContain('border-[color-mix(in_srgb,var(--border)_76%,transparent)]')
    expect(composer.classes().join(' ')).not.toContain('shadow-sm')

    expect(artifactBlock.classes().join(' ')).toContain('overflow-hidden')
    expect(artifactBlock.classes().join(' ')).not.toContain('shadow-xs')
    expect(artifactBlock.get('[data-testid="ui-artifact-block-header"]').classes().join(' ')).toContain('border-b')
    expect(artifactBlock.get('[data-testid="ui-artifact-block-footer"]').classes().join(' ')).toContain('border-t')

    expect(inboxBlock.classes().join(' ')).toContain('overflow-hidden')
    expect(inboxBlock.get('[data-testid="ui-inbox-block-priority"]').classes().join(' ')).toContain('border-border-strong')
    expect(inboxBlock.get('[data-testid="ui-inbox-block-priority"]').classes().join(' ')).not.toContain('text-text-secondary')

    expect(traceBlock.classes().join(' ')).toContain('overflow-hidden')
    expect(traceBlock.classes().join(' ')).not.toContain('border-transparent')
    expect(traceBlock.get('[data-testid="ui-trace-block-header"]').classes().join(' ')).toContain('border-b')
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
    expect(wrapper.html()).toContain('border-[color-mix(in_srgb,var(--border)_42%,transparent)]')
    expect(wrapper.html()).toContain('border-[color-mix(in_srgb,var(--border)_28%,transparent)]')
  })

  it('keeps UiDataTable row hover neutral while preserving row affordance', () => {
    const wrapper = mount(UiDataTable, {
      props: {
        rowTestId: 'ui-data-table-row',
        data: [
          { id: 'agent-1', name: 'Architect', role: 'Lead' },
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

    const classes = wrapper.get('[data-testid="ui-data-table-row-0"]').classes().join(' ')

    expect(classes).toContain('hover:bg-subtle')
    expect(classes).not.toContain('hover:bg-accent')
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

  it('renders the new shared page shell components through their public exports', () => {
    const wrapper = mount(defineComponent({
      components: {
        UiConversationComposerShell,
        UiInspectorPanel,
        UiListDetailShell,
        UiPageHeader,
        UiPageShell,
        UiStatusCallout,
      },
      template: `
        <UiPageShell test-id="page-shell">
          <UiPageHeader eyebrow="Workspace" title="Tools" description="Shared workbench shell">
            <template #meta><span data-testid="page-header-meta">Meta</span></template>
            <template #actions><button type="button" data-testid="page-header-action">Action</button></template>
          </UiPageHeader>
          <UiListDetailShell>
            <template #list>
              <div data-testid="list-slot">List</div>
            </template>
            <UiInspectorPanel title="Inspector" subtitle="Detail column">
              <UiStatusCallout tone="warning" title="Heads up" description="Shared state" />
              <UiConversationComposerShell>
                <div data-testid="composer-shell">Composer</div>
              </UiConversationComposerShell>
            </UiInspectorPanel>
          </UiListDetailShell>
        </UiPageShell>
      `,
    }))

    const pageShell = wrapper.get('[data-testid="page-shell"]')

    expect(pageShell.exists()).toBe(true)
    expect(pageShell.attributes('data-density')).toBe('regular')
    expect(wrapper.find('[data-testid="list-slot"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="page-header-meta"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="page-header-action"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('Shared workbench shell')
    expect(wrapper.text()).toContain('Inspector')
    expect(wrapper.text()).toContain('Heads up')
    expect(wrapper.find('[data-testid="composer-shell"]').exists()).toBe(true)
  })

  it('supports explicit page-shell density presets through a shared prop', () => {
    const wrapper = mount(UiPageShell, {
      props: {
        density: 'comfortable',
        testId: 'comfortable-page-shell',
      },
      slots: {
        default: '<div data-testid="comfortable-page-shell-body">Body</div>',
      },
    })

    expect(wrapper.get('[data-testid="comfortable-page-shell"]').attributes('data-density')).toBe('comfortable')
    expect(wrapper.get('[data-testid="comfortable-page-shell-body"]').exists()).toBe(true)
  })

  it('renders UiKbd through the shared export surface and ignores empty key entries', () => {
    const wrapper = mount(UiKbd, {
      props: {
        keys: ['⌘', 'K', ''],
      },
    })

    const kbd = wrapper.get('[data-testid="ui-kbd"]')

    expect(kbd.text()).toBe('⌘+K')

    const emptyWrapper = mount(UiKbd, {
      props: {
        keys: [],
      },
    })

    expect(emptyWrapper.find('[data-testid="ui-kbd"]').exists()).toBe(false)
  })

  it('keeps UiStatusCallout restrained instead of using saturated fills', () => {
    const info = mount(UiStatusCallout, {
      props: {
        tone: 'info',
        title: 'Heads up',
        description: 'Shared state',
      },
    })

    const warning = mount(UiStatusCallout, {
      props: {
        tone: 'warning',
        title: 'Review required',
      },
    })

    expect(info.classes().join(' ')).toContain('border-[color-mix(in_srgb,var(--color-status-info)_18%,var(--border))]')
    expect(info.classes().join(' ')).toContain('bg-[color-mix(in_srgb,var(--color-status-info-soft)_72%,var(--surface)_28%)]')
    expect(info.classes().join(' ')).not.toContain('bg-accent')
    expect(info.classes().join(' ')).not.toContain('border-transparent')
    expect(info.html()).toContain('text-status-info')
    expect(warning.html()).toContain('text-status-warning')
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

  it('renders UiSkeleton across line, card, and table-row variants while respecting reduced motion', () => {
    const line = mount(UiSkeleton, {
      props: {
        variant: 'line',
        count: 3,
      },
    })
    const card = mount(UiSkeleton, {
      props: {
        variant: 'card',
        count: 2,
      },
    })
    const tableRow = mount(UiSkeleton, {
      props: {
        variant: 'table-row',
        count: 4,
        reducedMotion: true,
      },
    })

    expect(line.get('[data-testid="ui-skeleton"]').attributes('data-ui-skeleton-variant')).toBe('line')
    expect(line.findAll('[data-testid="ui-skeleton-item"]')).toHaveLength(3)
    expect(line.html()).toContain('ui-skeleton-block--animated')

    expect(card.get('[data-testid="ui-skeleton"]').attributes('data-ui-skeleton-variant')).toBe('card')
    expect(card.findAll('[data-testid="ui-skeleton-item"]')).toHaveLength(2)
    expect(card.html()).toContain('rounded-[var(--radius-l)]')

    expect(tableRow.get('[data-testid="ui-skeleton"]').attributes('data-ui-skeleton-variant')).toBe('table-row')
    expect(tableRow.get('[data-testid="ui-skeleton"]').attributes('data-ui-skeleton-animated')).toBe('false')
    expect(tableRow.findAll('[data-testid="ui-skeleton-item"]')).toHaveLength(4)
    expect(tableRow.html()).not.toContain('ui-skeleton-block--animated')
  })

  it('renders UiErrorState with shared intro, actions, and details regions', () => {
    const wrapper = mount(UiErrorState, {
      props: {
        eyebrow: 'Runtime error',
        title: 'Something broke',
        description: 'Try again or return to safety.',
      },
      slots: {
        icon: '<span data-testid="ui-error-state-icon">!</span>',
        summary: '<div data-testid="ui-error-state-summary">Summary</div>',
        actions: '<button data-testid="ui-error-state-action" type="button">Retry</button>',
        details: '<pre data-testid="ui-error-state-detail-block">Stack</pre>',
      },
    })

    expect(wrapper.find('[data-testid="ui-error-state"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="ui-error-state-intro"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="ui-error-state-icon"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="ui-error-state-summary"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="ui-error-state-actions"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="ui-error-state-details"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('Something broke')
  })

  it('renders UiRestrictedState with tone-aware intro, meta, body, and actions regions', () => {
    const wrapper = mount(UiRestrictedState, {
      props: {
        tone: 'accent',
        eyebrow: 'Upgrade required',
        title: 'Unlock shared providers',
        description: 'Your current plan does not include this capability.',
      },
      slots: {
        icon: '<span data-testid="ui-restricted-state-icon">#</span>',
        meta: '<span data-testid="ui-restricted-state-meta">Plan gate</span>',
        default: '<p data-testid="ui-restricted-state-copy">Ask an owner to grant access.</p>',
        actions: '<button data-testid="ui-restricted-state-action" type="button">Manage plan</button>',
      },
    })

    const intro = wrapper.get('[data-testid="ui-restricted-state-intro"]')
    const body = wrapper.get('[data-testid="ui-restricted-state-body"]')
    const actions = wrapper.get('[data-testid="ui-restricted-state-actions"]')

    expect(intro.attributes('data-ui-restricted-tone')).toBe('accent')
    expect(intro.classes().join(' ')).toContain('bg-[color-mix(in_srgb,var(--accent)_12%,var(--surface)_88%)]')
    expect(wrapper.find('[data-testid="ui-restricted-state-icon"]').exists()).toBe(true)
    expect(body.text()).toContain('Plan gate')
    expect(body.text()).toContain('Ask an owner to grant access.')
    expect(actions.text()).toContain('Manage plan')
  })

  it('keeps UiActionCard hover neutral while strengthening the border affordance', () => {
    const wrapper = mount(UiActionCard, {
      props: {
        title: 'Open knowledge',
        description: 'Jump to project context',
      },
    })

    const classes = wrapper.get('article').classes().join(' ')

    expect(classes).toContain('hover:bg-subtle')
    expect(classes).toContain('hover:border-border-strong')
    expect(classes).toContain('active:scale-[0.99]')
    expect(classes).not.toContain('hover:bg-accent')
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
    expect(wrapper.html()).toContain('border-[color-mix(in_srgb,var(--border)_84%,transparent)]')
  })

  it('maps UiPanelFrame panel and interactive variants onto their matching shared surface semantics', () => {
    const panel = mount(UiPanelFrame, {
      props: {
        variant: 'panel',
        title: 'Panel variant',
      },
    })
    const interactive = mount(UiPanelFrame, {
      props: {
        variant: 'interactive',
        title: 'Interactive variant',
      },
    })

    expect(panel.html()).toContain('border-[color-mix(in_srgb,var(--border)_68%,transparent)]')
    expect(panel.html()).toContain('bg-subtle')
    expect(panel.html()).toContain('shadow-none')

    expect(interactive.html()).toContain('hover:bg-subtle')
    expect(interactive.html()).toContain('hover:border-border-strong')
    expect(interactive.html()).toContain('shadow-xs')
  })

  it('keeps UiSurface interactive hover neutral while strengthening the border affordance', () => {
    const wrapper = mount(UiSurface, {
      props: {
        variant: 'interactive',
        title: 'Interactive surface',
      },
    })

    const classes = wrapper.get('section').classes().join(' ')

    expect(classes).toContain('hover:bg-subtle')
    expect(classes).toContain('hover:border-border-strong')
    expect(classes).not.toContain('hover:bg-accent')
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
    expect(wrapper.get('[data-ui-performance-contained="true"]').attributes('class') ?? '').toContain('[content-visibility:auto]')
  })

  it('renders UiMetricCard accent tone with a brand-soft fill and stronger border', () => {
    const wrapper = mount(UiMetricCard, {
      props: {
        label: 'Priority',
        value: '12',
        tone: 'accent',
      },
    })

    const classes = wrapper.get('article').classes().join(' ')

    expect(classes).toContain('bg-accent')
    expect(classes).toContain('border-border-strong')
    expect(classes).not.toContain('border-transparent')
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
    expect(timeline.get('[data-ui-performance-contained="true"]').attributes('class') ?? '').toContain('[content-visibility:auto]')
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

  it('renders UiToolbarRow inline layout without dropping shared slots', () => {
    const toolbar = mount(UiToolbarRow, {
      props: {
        layout: 'inline',
        testId: 'toolbar-inline',
      },
      slots: {
        search: '<input data-testid="toolbar-inline-search" />',
        filters: '<div data-testid="toolbar-inline-filters">filters</div>',
        actions: '<button data-testid="toolbar-inline-action">Upload</button>',
      },
    })

    expect(toolbar.find('[data-testid="toolbar-inline"]').exists()).toBe(true)
    expect(toolbar.find('[data-testid="toolbar-inline-search"]').exists()).toBe(true)
    expect(toolbar.find('[data-testid="toolbar-inline-filters"]').exists()).toBe(true)
    expect(toolbar.find('[data-testid="toolbar-inline-action"]').exists()).toBe(true)
    expect(toolbar.html()).toContain('xl:flex-row')
  })

  it('keeps UiToolbarRow on the shared compact toolbar radius', () => {
    const toolbar = mount(UiToolbarRow, {
      props: {
        testId: 'toolbar-radius',
      },
      slots: {
        actions: '<button data-testid="toolbar-radius-action">Run</button>',
      },
    })

    const classes = toolbar.get('[data-testid="toolbar-radius"]').classes().join(' ')

    expect(classes).toContain('rounded-[var(--radius-m)]')
    expect(classes).not.toContain('rounded-[var(--radius-l)]')
  })

  it('renders UiSelectionMenu with grouped items', async () => {
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
              { id: 'agent:architect', label: 'Architect', helper: 'Primary owner', shortcut: ['⌘', '1'] },
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
    await wrapper.vm.$nextTick()

    const menu = document.body.querySelector<HTMLElement>('[data-testid="selection-menu"]')
    const menuHeader = menu?.firstElementChild as HTMLElement | null
    const architectItem = document.body.querySelector<HTMLElement>('[data-testid="ui-selection-item-agent:architect"]')
    const redesignTeamItem = document.body.querySelector<HTMLElement>('[data-testid="ui-selection-item-team:redesign"]')

    expect(menu).not.toBeNull()
    expect(menuHeader?.className).toContain('border-b')
    expect(menuHeader?.className).toContain('bg-subtle')
    expect(architectItem).not.toBeNull()
    expect(redesignTeamItem).not.toBeNull()
    expect(architectItem?.className).toContain('hover:bg-subtle')
    expect(architectItem?.className).toContain('hover:border-border')
    expect(architectItem?.className).not.toContain('hover:bg-accent')
    expect(architectItem?.querySelector('[data-testid="ui-kbd"]')?.textContent).toContain('⌘+1')
    expect(redesignTeamItem?.className).toContain('border-border-strong')
    expect(redesignTeamItem?.className).toContain('bg-accent')
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

  it('filters and toggles UiSearchableMultiSelect options', async () => {
    const Demo = defineComponent({
      components: { UiSearchableMultiSelect },
      setup() {
        const modelValue = ref<string[]>(['skill:analysis'])

        return { modelValue }
      },
      template: `
        <UiSearchableMultiSelect
          v-model="modelValue"
          trigger-label="Skills"
          search-placeholder="Search skills"
          empty-label="No matches"
          :options="[
            { value: 'skill:analysis', label: 'Analysis', keywords: ['research'] },
            { value: 'skill:builder', label: 'Builder', keywords: ['implementation'] },
            { value: 'skill:review', label: 'Review', keywords: ['qa'] }
          ]"
        />
      `,
    })

    const wrapper = mount(Demo, {
      attachTo: document.body,
    })

    await wrapper.get('[data-testid="ui-searchable-multi-select-trigger"]').trigger('click')
    await wrapper.vm.$nextTick()

    const searchInput = document.body.querySelector<HTMLInputElement>('[data-testid="ui-searchable-multi-select-input"]')
    const builderOption = document.body.querySelector<HTMLElement>('[data-testid="ui-searchable-multi-select-option-skill:builder"]')
    expect(searchInput).not.toBeNull()
    expect(builderOption?.className).toContain('hover:bg-subtle')
    expect(builderOption?.className).not.toContain('hover:bg-accent')
    searchInput!.value = 'impl'
    searchInput!.dispatchEvent(new Event('input', { bubbles: true }))
    await wrapper.vm.$nextTick()

    expect(document.body.textContent).toContain('Builder')
    expect(document.body.textContent).not.toContain('Review')

    document.body.querySelector<HTMLElement>('[data-testid="ui-searchable-multi-select-option-skill:builder"]')?.dispatchEvent(new MouseEvent('click', { bubbles: true }))
    await wrapper.vm.$nextTick()

    expect(wrapper.vm.modelValue).toEqual(['skill:analysis', 'skill:builder'])
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
        description: 'Can manage all access-control settings',
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
    expect(wrapper.text()).toContain('Can manage all access-control settings')
    expect(wrapper.find('[data-testid="record-leading"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="record-badge"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="record-metrics"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="record-secondary"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="record-meta"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="record-action"]').exists()).toBe(true)
    expect(wrapper.emitted('click')).toHaveLength(1)
  })

  it('renders UiPageHeader compact mode with tightened header rhythm', () => {
    const wrapper = mount(UiPageHeader, {
      props: {
        eyebrow: 'Workspace',
        title: 'Models',
        description: 'Compact header',
        compact: true,
      },
      slots: {
        meta: '<span data-testid="compact-header-meta">09:41</span>',
        actions: '<button data-testid="compact-header-action" type="button">Create</button>',
      },
    })

    const headerClasses = wrapper.get('header').classes().join(' ')
    const metaClasses = wrapper.get('[data-testid="compact-header-meta"]').element.parentElement?.className ?? ''

    expect(headerClasses).toContain('gap-3')
    expect(wrapper.html()).toContain('text-section-title')
    expect(wrapper.html()).toContain('text-label')
    expect(metaClasses).toContain('text-micro')
    expect(metaClasses).toContain('tabular-nums')
    expect(wrapper.find('[data-testid="compact-header-action"]').exists()).toBe(true)
  })

  it('keeps UiRecordCard active state on the brand-soft selection fill instead of a hard accent fill', () => {
    const wrapper = mount(UiRecordCard, {
      props: {
        title: 'Active row',
        active: true,
        testId: 'active-record-card',
      },
    })

    const classes = wrapper.get('[data-testid="active-record-card"]').classes().join(' ')

    expect(classes).toContain('is-active')
    expect(classes).toContain('bg-accent')
    expect(classes).toContain('border-border-strong')
  })

  it('adds restrained press feedback only to interactive UiRecordCard instances', () => {
    const wrapper = mount(UiRecordCard, {
      props: {
        title: 'Interactive row',
        interactive: true,
        testId: 'interactive-record-card',
      },
    })

    expect(wrapper.get('[data-testid="interactive-record-card"]').classes().join(' ')).toContain('active:scale-[0.99]')
  })

  it('renders UiRecordCard compact layout with tightened spacing and low-contrast footer', () => {
    const wrapper = mount(UiRecordCard, {
      props: {
        title: 'Lin Zhou',
        description: 'operator',
        layout: 'compact',
        testId: 'compact-record-card',
      },
      slots: {
        secondary: '<span data-testid="record-secondary">Password set</span>',
        meta: '<span data-testid="record-meta">Operator</span>',
        actions: '<button data-testid="record-action" type="button">Edit</button>',
      },
    })

    const classes = wrapper.get('[data-testid="compact-record-card"]').classes().join(' ')

    expect(wrapper.get('[data-testid="compact-record-card"]').attributes('data-ui-record-layout')).toBe('compact')
    expect(classes).toContain('gap-1')
    expect(classes).toContain('p-2')
    expect(wrapper.html()).not.toContain('border-t border-border/70')
    expect(wrapper.html()).toContain('pt-1.5')
    expect(wrapper.find('[data-testid="record-secondary"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="record-meta"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="record-action"]').exists()).toBe(true)
  })

  it('renders UiEmptyState media support above icon and actions', () => {
    const wrapper = mount(UiEmptyState, {
      props: {
        title: 'Nothing here yet',
        description: 'Connect a workspace to get started.',
      },
      slots: {
        media: '<div data-testid="empty-state-media">Illustration</div>',
        icon: '<span data-testid="empty-state-icon">O</span>',
        actions: '<button data-testid="empty-state-action" type="button">Connect</button>',
      },
    })

    expect(wrapper.find('[data-testid="empty-state-media"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="empty-state-icon"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="empty-state-action"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('Nothing here yet')
  })

  it('renders UiSwitch with clear off-state track contrast', () => {
    const wrapper = mount(UiSwitch, {
      props: {
        modelValue: false,
      },
    })

    const switchButton = wrapper.get('[role="switch"]')
    const switchHtml = wrapper.html()
    const switchClasses = switchButton.classes().join(' ')

    expect(switchButton.attributes('aria-checked')).toBe('false')
    expect(switchClasses).toContain('border-[var(--control-toggle-off-border)]')
    expect(switchClasses).toContain('bg-[var(--control-toggle-off-bg)]')
    expect(switchClasses).toContain('shadow-xs')
    expect(switchClasses).toContain('enabled:active:scale-[0.99]')
    expect(switchClasses).not.toContain('bg-border-strong/60')
    expect(switchClasses).not.toContain('shadow-inner')
    expect(switchHtml).toContain('border-[var(--control-toggle-thumb-border)]')
  })

  it('renders UiNotificationRow and emits read and select actions', async () => {
    const createdAt = Date.UTC(2026, 3, 12, 10, 5)
    const wrapper = mount(UiNotificationRow, {
      props: {
        notification: createNotification({
          id: 'notif-row',
          scopeKind: 'workspace',
          routeTo: '/workspaces/ws-local/overview',
          createdAt,
        }),
        scopeLabel: 'Workspace',
      },
    })

    expect(wrapper.text()).toContain('Saved')
    expect(wrapper.text()).toContain('Workspace')
    expect(wrapper.text()).toContain(formatUiTimestamp(createdAt))
    expect(wrapper.get('[data-testid="ui-notification-row-notif-row"]').classes().join(' ')).toContain('overflow-hidden')
    expect(wrapper.get('[data-testid="ui-notification-row-notif-row"]').classes().join(' ')).not.toContain('border-l-status-info')
    expect(wrapper.get('[data-testid="ui-notification-row-header-notif-row"]').classes().join(' ')).toContain('border-b')
    expect(wrapper.get('[data-testid="ui-notification-row-marker-notif-row"]').classes().join(' ')).toContain('bg-status-info')
    expect(wrapper.get('[data-testid="ui-notification-row-mark-read-notif-row"]').classes().join(' ')).toContain('hover:bg-subtle')
    expect(wrapper.get('[data-testid="ui-notification-row-mark-read-notif-row"]').classes().join(' ')).not.toContain('hover:bg-accent')
    expect(wrapper.get('[data-testid="ui-notification-row-header-notif-row"]').html()).toContain('text-micro')
    expect(wrapper.get('[data-testid="ui-notification-row-notif-row"]').html()).toContain('text-label')
    expect(wrapper.get('[data-testid="ui-notification-row-notif-row"]').html()).toContain('text-caption')
    expect(wrapper.get('[data-testid="ui-notification-row-notif-row"]').html()).toContain('tabular-nums')

    await wrapper.get('[data-testid="ui-notification-row-mark-read-notif-row"]').trigger('click')
    expect(wrapper.emitted('mark-read')).toEqual([['notif-row']])

    await wrapper.get('[data-testid="ui-notification-row-notif-row"]').trigger('click')
    expect(wrapper.emitted('select')).toEqual([[expect.objectContaining({ id: 'notif-row' })]])
  })

  it('renders warning and error notifications with stronger semantic accents while keeping read rows de-emphasized', () => {
    const warningWrapper = mount(UiNotificationRow, {
      props: {
        notification: createNotification({
          id: 'notif-warning',
          level: 'warning',
        }),
        scopeLabel: 'App',
      },
    })

    const errorWrapper = mount(UiNotificationRow, {
      props: {
        notification: createNotification({
          id: 'notif-error',
          level: 'error',
          readAt: 99,
        }),
        scopeLabel: 'App',
      },
    })

    const warningClasses = warningWrapper.get('[data-testid="ui-notification-row-notif-warning"]').classes().join(' ')
    const errorClasses = errorWrapper.get('[data-testid="ui-notification-row-notif-error"]').classes().join(' ')
    const warningHeaderClasses = warningWrapper.get('[data-testid="ui-notification-row-header-notif-warning"]').classes().join(' ')
    const errorHeaderClasses = errorWrapper.get('[data-testid="ui-notification-row-header-notif-error"]').classes().join(' ')

    expect(warningClasses).not.toContain('border-l-status-warning')
    expect(warningHeaderClasses).toContain('bg-[var(--color-status-warning-soft)]')
    expect(errorClasses).not.toContain('border-l-status-error')
    expect(errorHeaderClasses).toContain('bg-[var(--color-status-error-soft)]')
    expect(errorClasses).toContain('opacity-70')
  })

  it('renders UiNotificationCenter filters and list actions', async () => {
    const wrapper = mount(UiNotificationCenter, {
      props: {
        open: true,
        notifications: [
          createNotification({
            id: 'notif-center',
            scopeKind: 'user',
          }),
        ],
        unreadCount: 1,
        activeFilter: 'all',
        filterLabels: {
          all: 'All',
          app: 'App',
          workspace: 'Workspace',
          user: 'User',
        },
        scopeLabels: {
          app: 'App',
          workspace: 'Workspace',
          user: 'User',
        },
        title: 'Notifications',
        emptyTitle: 'No notifications',
        emptyDescription: 'You are all caught up.',
        markAllLabel: 'Mark all read',
      },
      attachTo: document.body,
    })

    expect(wrapper.text()).toContain('Notifications')
    expect(wrapper.text()).toContain('Mark all read')
    expect(wrapper.findComponent(UiSurface).attributes('class') ?? '').toContain('border-[color-mix(in_srgb,var(--border)_84%,transparent)]')
    expect(wrapper.findComponent(UiSurface).attributes('class') ?? '').not.toContain('border-border')
    expect(wrapper.get('[data-testid="ui-notification-filter-workspace"]').classes().join(' ')).toContain('hover:bg-subtle')
    expect(wrapper.get('[data-testid="ui-notification-filter-workspace"]').classes().join(' ')).not.toContain('hover:bg-accent')

    await wrapper.get('[data-testid="ui-notification-filter-workspace"]').trigger('click')
    expect(wrapper.emitted('update:filter')).toEqual([['workspace']])

    await wrapper.get('[data-testid="ui-notification-center-mark-all"]').trigger('click')
    expect(wrapper.emitted('mark-all-read')).toEqual([[]])
  })

  it('renders UiToastItem and UiToastViewport with close actions', async () => {
    const createdAt = Date.UTC(2026, 3, 12, 10, 5)
    const toast = createNotification({
      id: 'notif-toast',
      level: 'success',
      scopeKind: 'app',
      createdAt,
    })

    const item = mount(UiToastItem, {
      props: {
        notification: toast,
        scopeLabel: 'App',
      },
    })

    expect(item.text()).toContain('Saved')
    expect(item.text()).toContain('App')
    expect(item.text()).toContain(formatUiTimestamp(createdAt))
    expect(item.html()).toContain('text-status-success')
    expect(item.html()).toContain('border-[color-mix(in_srgb,var(--color-status-success)_22%,var(--border))]')
    expect(item.get('[data-testid="ui-toast-close-notif-toast"]').classes().join(' ')).toContain('hover:bg-subtle')
    expect(item.get('[data-testid="ui-toast-close-notif-toast"]').classes().join(' ')).not.toContain('hover:bg-accent')

    await item.get('[data-testid="ui-toast-close-notif-toast"]').trigger('click')
    expect(item.emitted('close')).toEqual([['notif-toast']])

    const viewport = mount(UiToastViewport, {
      props: {
        notifications: [toast],
        scopeLabels: {
          app: 'App',
          workspace: 'Workspace',
          user: 'User',
        },
      },
      attachTo: document.body,
    })

    expect(viewport.find('[data-testid="ui-toast-viewport"]').exists()).toBe(true)
    expect(viewport.text()).toContain('Saved')
    expect(viewport.text()).toContain(formatUiTimestamp(createdAt))

    await viewport.get('[data-testid="ui-toast-close-notif-toast"]').trigger('click')
    expect(viewport.emitted('close')).toEqual([['notif-toast']])
  })

  it('renders warning and error toasts with elevated semantic emphasis', () => {
    const warningToast = mount(UiToastItem, {
      props: {
        notification: createNotification({
          id: 'notif-toast-warning',
          level: 'warning',
        }),
        scopeLabel: 'Workspace',
      },
    })

    const errorToast = mount(UiToastItem, {
      props: {
        notification: createNotification({
          id: 'notif-toast-error',
          level: 'error',
        }),
        scopeLabel: 'Workspace',
      },
    })

    expect(warningToast.html()).toContain('text-status-warning')
    expect(warningToast.html()).toContain('bg-[color-mix(in_srgb,var(--color-status-warning-soft)_48%,var(--bg-popover))]')
    expect(errorToast.html()).toContain('text-status-error')
    expect(errorToast.html()).toContain('bg-[color-mix(in_srgb,var(--color-status-error-soft)_48%,var(--bg-popover))]')
  })

  it('keeps UiToastItem lift restrained and disables media autoplay when reduced motion is active', async () => {
    const toast = mount(UiToastItem, {
      props: {
        notification: createNotification({
          id: 'notif-toast-calm',
          level: 'info',
        }),
        scopeLabel: 'Workspace',
      },
    })

    expect(toast.findComponent(UiSurface).attributes('class') ?? '').toContain('shadow-sm')
    expect(toast.findComponent(UiSurface).attributes('class') ?? '').not.toContain('shadow-md')

    let observerCallback: ((entries: Array<{ isIntersecting: boolean, intersectionRatio: number }>) => void) | null = null
    const observe = vi.fn()
    const disconnect = vi.fn()
    vi.stubGlobal('IntersectionObserver', vi.fn((callback: typeof observerCallback) => {
      observerCallback = callback
      return {
        observe,
        disconnect,
        unobserve: vi.fn(),
        takeRecords: vi.fn(() => []),
      }
    }))

    const dotLottie = mount(UiDotLottie, {
      props: {
        src: '/animations/pulse.lottie',
        autoplay: true,
        loop: true,
        reducedMotion: true,
        lazy: true,
      },
    })

    expect(dotLottie.get('[data-testid="ui-dotlottie"]').attributes('data-lazy-ready')).toBe('false')
    observerCallback?.([{ isIntersecting: true, intersectionRatio: 1 }])
    await nextTick()
    expect(dotLottie.get('[data-testid="ui-dotlottie"]').attributes('data-autoplay')).toBe('false')
    expect(dotLottie.get('[data-testid="ui-dotlottie"]').attributes('data-loop')).toBe('false')
    expect(dotLottie.get('[data-testid="ui-dotlottie"]').attributes('data-lazy-ready')).toBe('true')

    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue(null)

    const riveCanvas = mount(UiRiveCanvas, {
      props: {
        src: '/animations/pet.riv',
        autoplay: true,
        reducedMotion: true,
        lazy: true,
      },
    })

    expect(riveCanvas.get('[data-testid="ui-rive-canvas"]').attributes('data-lazy-ready')).toBe('false')
    observerCallback?.([{ isIntersecting: true, intersectionRatio: 1 }])
    await nextTick()

    expect(riveCanvas.get('[data-testid="ui-rive-canvas"]').attributes('data-autoplay')).toBe('false')
    expect(riveCanvas.get('[data-testid="ui-rive-canvas"]').attributes('data-lazy-ready')).toBe('true')
    expect(observe).toHaveBeenCalled()
    expect(disconnect).toHaveBeenCalled()
    vi.unstubAllGlobals()
  })
})
