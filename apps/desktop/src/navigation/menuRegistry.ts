import type { MenuNode, MenuSource } from '@octopus/schema'

export type MenuIconKey =
  | 'dashboard'
  | 'conversations'
  | 'agents'
  | 'resources'
  | 'knowledge'
  | 'trace'
  | 'models'
  | 'tools'
  | 'automations'
  | 'user-center'
  | 'profile'
  | 'users'
  | 'roles'
  | 'permissions'
  | 'menus'

export type MenuSection = 'project' | 'workspace' | 'user-center'

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
    id: 'menu-workspace-knowledge',
    source: 'main-sidebar',
    section: 'workspace',
    routeName: 'workspace-knowledge',
    routeNames: ['workspace-knowledge'],
    defaultLabel: '知识库',
    labelKey: 'sidebar.navigation.knowledge',
    icon: 'knowledge',
    order: 15,
  },
  {
    id: 'menu-workspace-resources',
    source: 'main-sidebar',
    section: 'workspace',
    routeName: 'workspace-resources',
    routeNames: ['workspace-resources'],
    defaultLabel: '资源库',
    labelKey: 'sidebar.navigation.resources',
    icon: 'resources',
    order: 18,
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
    id: 'menu-conversations',
    source: 'main-sidebar',
    section: 'project',
    routeName: 'project-conversations',
    routeNames: ['project-conversations', 'conversation'],
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
    defaultLabel: '项目智能体',
    labelKey: 'sidebar.navigation.agents',
    icon: 'agents',
    order: 40,
  },
  {
    id: 'menu-project-resources',
    source: 'main-sidebar',
    section: 'project',
    routeName: 'resources',
    routeNames: ['resources'],
    defaultLabel: '项目资源',
    labelKey: 'sidebar.navigation.resources',
    icon: 'resources',
    order: 50,
  },
  {
    id: 'menu-knowledge',
    source: 'main-sidebar',
    section: 'project',
    routeName: 'knowledge',
    routeNames: ['knowledge'],
    defaultLabel: '项目知识',
    labelKey: 'sidebar.navigation.knowledge',
    icon: 'knowledge',
    order: 60,
  },
  {
    id: 'menu-trace',
    source: 'main-sidebar',
    section: 'project',
    routeName: 'trace',
    routeNames: ['trace'],
    defaultLabel: 'Trace',
    labelKey: 'sidebar.navigation.trace',
    icon: 'trace',
    order: 70,
  },
  {
    id: 'menu-agents',
    source: 'main-sidebar',
    section: 'workspace',
    routeName: 'agents',
    routeNames: ['agents'],
    defaultLabel: '智能体库',
    labelKey: 'sidebar.navigation.agents',
    icon: 'agents',
    order: 45,
  },
  {
    id: 'menu-models',
    source: 'main-sidebar',
    section: 'workspace',
    routeName: 'models',
    routeNames: ['models'],
    defaultLabel: '模型',
    labelKey: 'sidebar.navigation.models',
    icon: 'models',
    order: 80,
  },
  {
    id: 'menu-tools',
    source: 'main-sidebar',
    section: 'workspace',
    routeName: 'tools',
    routeNames: ['tools'],
    defaultLabel: '工具',
    labelKey: 'sidebar.navigation.tools',
    icon: 'tools',
    order: 90,
  },
  {
    id: 'menu-automations',
    source: 'main-sidebar',
    section: 'workspace',
    routeName: 'automations',
    routeNames: ['automations'],
    defaultLabel: '自动化',
    labelKey: 'sidebar.navigation.automations',
    icon: 'automations',
    order: 100,
  },
  {
    id: 'menu-user-center',
    source: 'main-sidebar',
    section: 'workspace',
    routeName: 'user-center',
    routeNames: ['user-center', 'user-center-profile', 'user-center-users', 'user-center-roles', 'user-center-permissions', 'user-center-menus'],
    defaultLabel: '用户中心',
    labelKey: 'sidebar.navigation.userCenter',
    icon: 'user-center',
    order: 110,
  },
  {
    id: 'menu-user-center-profile',
    parentId: 'menu-user-center',
    source: 'user-center',
    section: 'user-center',
    routeName: 'user-center-profile',
    routeNames: ['user-center-profile'],
    defaultLabel: '基本信息',
    labelKey: 'userCenter.nav.profile',
    icon: 'profile',
    order: 120,
  },
  {
    id: 'menu-user-center-users',
    parentId: 'menu-user-center',
    source: 'user-center',
    section: 'user-center',
    routeName: 'user-center-users',
    routeNames: ['user-center-users'],
    defaultLabel: '用户列表',
    labelKey: 'userCenter.nav.users',
    icon: 'users',
    order: 130,
  },
  {
    id: 'menu-user-center-roles',
    parentId: 'menu-user-center',
    source: 'user-center',
    section: 'user-center',
    routeName: 'user-center-roles',
    routeNames: ['user-center-roles'],
    defaultLabel: '角色列表',
    labelKey: 'userCenter.nav.roles',
    icon: 'roles',
    order: 140,
  },
  {
    id: 'menu-user-center-permissions',
    parentId: 'menu-user-center',
    source: 'user-center',
    section: 'user-center',
    routeName: 'user-center-permissions',
    routeNames: ['user-center-permissions'],
    defaultLabel: '权限列表',
    labelKey: 'userCenter.nav.permissions',
    icon: 'permissions',
    order: 150,
  },
  {
    id: 'menu-user-center-menus',
    parentId: 'menu-user-center',
    source: 'user-center',
    section: 'user-center',
    routeName: 'user-center-menus',
    routeNames: ['user-center-menus'],
    defaultLabel: '菜单列表',
    labelKey: 'userCenter.nav.menus',
    icon: 'menus',
    order: 160,
  },
]

const MENU_DEFINITION_MAP = new Map(MENU_DEFINITIONS.map((item) => [item.id, item]))

export const USER_CENTER_MENU_IDS = MENU_DEFINITIONS
  .filter((item) => item.section === 'user-center')
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
