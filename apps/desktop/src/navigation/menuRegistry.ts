import type { MenuNode, MenuSource } from '@octopus/schema'

export type MenuIconKey =
  | 'dashboard'
  | 'console'
  | 'conversations'
  | 'agents'
  | 'resources'
  | 'knowledge'
  | 'trace'
  | 'runtime'
  | 'projects'
  | 'models'
  | 'tools'
  | 'automations'
  | 'permission-center'
  | 'profile'
  | 'pet'
  | 'users'
  | 'roles'
  | 'permissions'
  | 'menus'
  | 'settings'
  | 'connections'
  | 'teams'
  | 'bell'

export type MenuSection = 'app' | 'project' | 'workspace' | 'console' | 'permission-center'

export interface MenuDefinition {
  id: string
  parentId?: string
  source: MenuSource
  section: MenuSection
  routeName?: string
  routeNames: string[]
  defaultLabel: string
  labelKey: string
  icon: MenuIconKey
  order: number
}

export const MENU_DEFINITIONS: MenuDefinition[] = [
  {
    id: 'menu-app-connections',
    source: 'main-sidebar',
    section: 'app',
    routeName: 'app-connections',
    routeNames: ['app-connections'],
    defaultLabel: '连接管理',
    labelKey: 'connections.header.title',
    icon: 'connections',
    order: 5,
  },
  {
    id: 'menu-app-settings',
    source: 'main-sidebar',
    section: 'app',
    routeName: 'app-settings',
    routeNames: ['app-settings'],
    defaultLabel: '设置',
    labelKey: 'sidebar.navigation.settings',
    icon: 'settings',
    order: 6,
  },
  {
    id: 'menu-workspace-overview',
    source: 'main-sidebar',
    section: 'workspace',
    routeName: 'workspace-overview',
    routeNames: ['workspace-overview'],
    defaultLabel: '概览',
    labelKey: 'sidebar.navigation.overview',
    icon: 'dashboard',
    order: 10,
  },
  {
    id: 'menu-workspace-console',
    source: 'main-sidebar',
    section: 'workspace',
    routeName: 'workspace-console',
    routeNames: [
      'workspace-console',
      'workspace-console-projects',
      'workspace-console-knowledge',
      'workspace-console-resources',
      'workspace-console-agents',
      'workspace-console-models',
      'workspace-console-tools',
    ],
    defaultLabel: '控制台',
    labelKey: 'sidebar.navigation.console',
    icon: 'console',
    order: 12,
  },
  {
    id: 'menu-project-dashboard',
    source: 'main-sidebar',
    section: 'project',
    routeName: 'project-dashboard',
    routeNames: ['project-dashboard'],
    defaultLabel: '控制台',
    labelKey: 'sidebar.navigation.dashboard',
    icon: 'dashboard',
    order: 20,
  },
  {
    id: 'menu-project-conversations',
    source: 'main-sidebar',
    section: 'project',
    routeName: 'project-conversations',
    routeNames: ['project-conversations', 'project-conversation'],
    defaultLabel: '会话',
    labelKey: 'sidebar.projectModules.conversations',
    icon: 'conversations',
    order: 30,
  },
  {
    id: 'menu-project-agents',
    source: 'main-sidebar',
    section: 'project',
    routeName: 'project-agents',
    routeNames: ['project-agents'],
    defaultLabel: '项目数字员工',
    labelKey: 'sidebar.navigation.agents',
    icon: 'agents',
    order: 40,
  },
  {
    id: 'menu-project-resources',
    source: 'main-sidebar',
    section: 'project',
    routeName: 'project-resources',
    routeNames: ['project-resources'],
    defaultLabel: '项目资源',
    labelKey: 'sidebar.navigation.resources',
    icon: 'resources',
    order: 50,
  },
  {
    id: 'menu-project-knowledge',
    source: 'main-sidebar',
    section: 'project',
    routeName: 'project-knowledge',
    routeNames: ['project-knowledge'],
    defaultLabel: '项目知识',
    labelKey: 'sidebar.navigation.knowledge',
    icon: 'knowledge',
    order: 60,
  },
  {
    id: 'menu-project-trace',
    source: 'main-sidebar',
    section: 'project',
    routeName: 'project-trace',
    routeNames: ['project-trace'],
    defaultLabel: 'Trace',
    labelKey: 'sidebar.navigation.trace',
    icon: 'trace',
    order: 70,
  },
  {
    id: 'menu-project-settings',
    source: 'main-sidebar',
    section: 'project',
    routeName: 'project-settings',
    routeNames: ['project-settings'],
    defaultLabel: '项目配置',
    labelKey: 'sidebar.navigation.projectSettings',
    icon: 'settings',
    order: 74,
  },
  {
    id: 'menu-project-runtime',
    source: 'main-sidebar',
    section: 'project',
    routeName: 'project-runtime',
    routeNames: ['project-runtime'],
    defaultLabel: 'Runtime',
    labelKey: 'sidebar.navigation.runtime',
    icon: 'runtime',
    order: 75,
  },
  {
    id: 'menu-workspace-automations',
    source: 'main-sidebar',
    section: 'workspace',
    routeName: 'workspace-automations',
    routeNames: ['workspace-automations'],
    defaultLabel: '自动化',
    labelKey: 'sidebar.navigation.automations',
    icon: 'automations',
    order: 90,
  },
  {
    id: 'menu-workspace-permission-center',
    source: 'main-sidebar',
    section: 'workspace',
    routeName: 'workspace-permission-center',
    routeNames: [
      'workspace-permission-center',
      'workspace-permission-center-users',
      'workspace-permission-center-roles',
      'workspace-permission-center-permissions',
      'workspace-permission-center-menus',
    ],
    defaultLabel: '权限中心',
    labelKey: 'sidebar.navigation.permissionCenter',
    icon: 'permission-center',
    order: 100,
  },
  {
    id: 'menu-workspace-console-projects',
    parentId: 'menu-workspace-console',
    source: 'console',
    section: 'console',
    routeName: 'workspace-console-projects',
    routeNames: ['workspace-console-projects'],
    defaultLabel: '项目管理',
    labelKey: 'sidebar.navigation.projects',
    icon: 'projects',
    order: 110,
  },
  {
    id: 'menu-workspace-console-knowledge',
    parentId: 'menu-workspace-console',
    source: 'console',
    section: 'console',
    routeName: 'workspace-console-knowledge',
    routeNames: ['workspace-console-knowledge'],
    defaultLabel: '知识库',
    labelKey: 'sidebar.navigation.knowledge',
    icon: 'knowledge',
    order: 120,
  },
  {
    id: 'menu-workspace-console-resources',
    parentId: 'menu-workspace-console',
    source: 'console',
    section: 'console',
    routeName: 'workspace-console-resources',
    routeNames: ['workspace-console-resources'],
    defaultLabel: '资源库',
    labelKey: 'sidebar.navigation.resources',
    icon: 'resources',
    order: 130,
  },
  {
    id: 'menu-workspace-console-agents',
    parentId: 'menu-workspace-console',
    source: 'console',
    section: 'console',
    routeName: 'workspace-console-agents',
    routeNames: ['workspace-console-agents'],
    defaultLabel: '数字员工中心',
    labelKey: 'sidebar.navigation.agents',
    icon: 'agents',
    order: 140,
  },
  {
    id: 'menu-workspace-console-models',
    parentId: 'menu-workspace-console',
    source: 'console',
    section: 'console',
    routeName: 'workspace-console-models',
    routeNames: ['workspace-console-models'],
    defaultLabel: '模型',
    labelKey: 'sidebar.navigation.models',
    icon: 'models',
    order: 150,
  },
  {
    id: 'menu-workspace-console-tools',
    parentId: 'menu-workspace-console',
    source: 'console',
    section: 'console',
    routeName: 'workspace-console-tools',
    routeNames: ['workspace-console-tools'],
    defaultLabel: '工具',
    labelKey: 'sidebar.navigation.tools',
    icon: 'tools',
    order: 160,
  },
  {
    id: 'menu-workspace-permission-center-users',
    parentId: 'menu-workspace-permission-center',
    source: 'permission-center',
    section: 'permission-center',
    routeName: 'workspace-permission-center-users',
    routeNames: ['workspace-permission-center-users'],
    defaultLabel: '成员管理',
    labelKey: 'permissionCenter.nav.users',
    icon: 'users',
    order: 170,
  },
  {
    id: 'menu-workspace-permission-center-roles',
    parentId: 'menu-workspace-permission-center',
    source: 'permission-center',
    section: 'permission-center',
    routeName: 'workspace-permission-center-roles',
    routeNames: ['workspace-permission-center-roles'],
    defaultLabel: '角色管理',
    labelKey: 'permissionCenter.nav.roles',
    icon: 'roles',
    order: 180,
  },
  {
    id: 'menu-workspace-permission-center-permissions',
    parentId: 'menu-workspace-permission-center',
    source: 'permission-center',
    section: 'permission-center',
    routeName: 'workspace-permission-center-permissions',
    routeNames: ['workspace-permission-center-permissions'],
    defaultLabel: '权限管理',
    labelKey: 'permissionCenter.nav.permissions',
    icon: 'permissions',
    order: 190,
  },
  {
    id: 'menu-workspace-permission-center-menus',
    parentId: 'menu-workspace-permission-center',
    source: 'permission-center',
    section: 'permission-center',
    routeName: 'workspace-permission-center-menus',
    routeNames: ['workspace-permission-center-menus'],
    defaultLabel: '导航管理',
    labelKey: 'permissionCenter.nav.menus',
    icon: 'menus',
    order: 200,
  },
]

const MENU_DEFINITION_MAP = new Map(MENU_DEFINITIONS.map((item) => [item.id, item]))

export const MAIN_MENU_IDS = MENU_DEFINITIONS
  .filter((item) => item.source === 'main-sidebar' && !item.parentId)
  .map((item) => item.id)

export const CONSOLE_MENU_IDS = MENU_DEFINITIONS
  .filter((item) => item.section === 'console')
  .map((item) => item.id)

export const PERMISSION_CENTER_MENU_IDS = MENU_DEFINITIONS
  .filter((item) => item.section === 'permission-center')
  .map((item) => item.id)

export function getMenuDefinition(menuId: string): MenuDefinition | undefined {
  return MENU_DEFINITION_MAP.get(menuId)
}

export function getRouteMenuId(routeName?: string | null): string | undefined {
  if (!routeName) {
    return undefined
  }

  return MENU_DEFINITIONS.find((item) => item.routeName === routeName)?.id
    ?? MENU_DEFINITIONS.find((item) => item.routeNames.includes(routeName))?.id
}

export function buildWorkspaceMenuNodes(workspaceId: string): MenuNode[] {
  return MENU_DEFINITIONS.map((item) => ({
    id: item.id,
    workspaceId,
    parentId: item.parentId,
    source: item.source,
    label: item.defaultLabel,
    routeName: item.routeName,
    status: 'active',
    order: item.order,
  }))
}

export function getAncestorMenuIds(menuId: string): string[] {
  const ancestors: string[] = []
  let pointer = getMenuDefinition(menuId)

  while (pointer?.parentId) {
    ancestors.unshift(pointer.parentId)
    pointer = getMenuDefinition(pointer.parentId)
  }

  return ancestors
}
