import type { MenuNode, MenuSource } from '@octopus/schema'

export type MenuIconKey =
  | 'dashboard'
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
  | 'user-center'
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

export type MenuSection = 'app' | 'project' | 'workspace' | 'user-center'

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
    id: 'menu-workspace-projects',
    source: 'main-sidebar',
    section: 'workspace',
    routeName: 'workspace-projects',
    routeNames: ['workspace-projects'],
    defaultLabel: '项目管理',
    labelKey: 'sidebar.navigation.projects',
    icon: 'projects',
    order: 12,
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
    defaultLabel: '项目智能体',
    labelKey: 'sidebar.navigation.agents',
    icon: 'agents',
    order: 40,
  },
  {
    id: 'menu-workspace-agents',
    source: 'main-sidebar',
    section: 'workspace',
    routeName: 'workspace-agents',
    routeNames: ['workspace-agents'],
    defaultLabel: '智能体库',
    labelKey: 'sidebar.navigation.agents',
    icon: 'agents',
    order: 45,
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
    id: 'menu-workspace-models',
    source: 'main-sidebar',
    section: 'workspace',
    routeName: 'workspace-models',
    routeNames: ['workspace-models'],
    defaultLabel: '模型',
    labelKey: 'sidebar.navigation.models',
    icon: 'models',
    order: 90,
  },
  {
    id: 'menu-workspace-tools',
    source: 'main-sidebar',
    section: 'workspace',
    routeName: 'workspace-tools',
    routeNames: ['workspace-tools'],
    defaultLabel: '工具',
    labelKey: 'sidebar.navigation.tools',
    icon: 'tools',
    order: 100,
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
    order: 110,
  },
  {
    id: 'menu-workspace-user-center',
    source: 'main-sidebar',
    section: 'workspace',
    routeName: 'workspace-user-center',
    routeNames: [
      'workspace-user-center',
      'workspace-user-center-profile',
      'workspace-user-center-pet',
      'workspace-user-center-users',
      'workspace-user-center-roles',
      'workspace-user-center-permissions',
      'workspace-user-center-menus',
    ],
    defaultLabel: '用户中心',
    labelKey: 'sidebar.navigation.userCenter',
    icon: 'user-center',
    order: 120,
  },
  {
    id: 'menu-workspace-user-center-profile',
    parentId: 'menu-workspace-user-center',
    source: 'user-center',
    section: 'user-center',
    routeName: 'workspace-user-center-profile',
    routeNames: ['workspace-user-center-profile'],
    defaultLabel: '基本信息',
    labelKey: 'userCenter.nav.profile',
    icon: 'profile',
    order: 130,
  },
  {
    id: 'menu-workspace-user-center-pet',
    parentId: 'menu-workspace-user-center',
    source: 'user-center',
    section: 'user-center',
    routeName: 'workspace-user-center-pet',
    routeNames: ['workspace-user-center-pet'],
    defaultLabel: '宠物',
    labelKey: 'userCenter.nav.pet',
    icon: 'pet',
    order: 135,
  },
  {
    id: 'menu-workspace-user-center-users',
    parentId: 'menu-workspace-user-center',
    source: 'user-center',
    section: 'user-center',
    routeName: 'workspace-user-center-users',
    routeNames: ['workspace-user-center-users'],
    defaultLabel: '用户列表',
    labelKey: 'userCenter.nav.users',
    icon: 'users',
    order: 140,
  },
  {
    id: 'menu-workspace-user-center-roles',
    parentId: 'menu-workspace-user-center',
    source: 'user-center',
    section: 'user-center',
    routeName: 'workspace-user-center-roles',
    routeNames: ['workspace-user-center-roles'],
    defaultLabel: '角色列表',
    labelKey: 'userCenter.nav.roles',
    icon: 'roles',
    order: 150,
  },
  {
    id: 'menu-workspace-user-center-permissions',
    parentId: 'menu-workspace-user-center',
    source: 'user-center',
    section: 'user-center',
    routeName: 'workspace-user-center-permissions',
    routeNames: ['workspace-user-center-permissions'],
    defaultLabel: '权限列表',
    labelKey: 'userCenter.nav.permissions',
    icon: 'permissions',
    order: 160,
  },
  {
    id: 'menu-workspace-user-center-menus',
    parentId: 'menu-workspace-user-center',
    source: 'user-center',
    section: 'user-center',
    routeName: 'workspace-user-center-menus',
    routeNames: ['workspace-user-center-menus'],
    defaultLabel: '菜单列表',
    labelKey: 'userCenter.nav.menus',
    icon: 'menus',
    order: 170,
  },
]

const MENU_DEFINITION_MAP = new Map(MENU_DEFINITIONS.map((item) => [item.id, item]))

export const MAIN_MENU_IDS = MENU_DEFINITIONS
  .filter((item) => item.source === 'main-sidebar' && !item.parentId)
  .map((item) => item.id)

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
