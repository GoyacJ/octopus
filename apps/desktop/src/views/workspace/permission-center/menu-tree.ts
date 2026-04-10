import type { MenuRecord } from '@octopus/schema'

import { getMenuDefinition } from '@/navigation/menuRegistry'

export interface MenuTreeLeaf {
  kind: 'menu'
  id: string
  label: string
  secondary: string
  order: number
  menu: MenuRecord
}

export interface MenuTreeBranch {
  kind: 'group'
  id: string
  label: string
  order: number
  rootMenu?: MenuTreeLeaf
  children: MenuTreeLeaf[]
}

export interface MenuTreeSection {
  id: 'app' | 'workspace' | 'project'
  label: string
  items: Array<MenuTreeLeaf | MenuTreeBranch>
}

export interface MenuTreeGroupLabels {
  app: string
  workspace: string
  console: string
  permissionCenter: string
  project: string
}

export function buildPermissionCenterMenuTreeSections(
  menus: MenuRecord[],
  groupLabels: MenuTreeGroupLabels,
  resolveMenuLabel: (menu: MenuRecord) => string,
): MenuTreeSection[] {
  const appItems: MenuTreeLeaf[] = []
  const workspaceItems: MenuTreeLeaf[] = []
  const projectItems: MenuTreeLeaf[] = []
  const consoleChildren: MenuTreeLeaf[] = []
  const permissionCenterChildren: MenuTreeLeaf[] = []
  let consoleRoot: MenuTreeLeaf | undefined
  let permissionCenterRoot: MenuTreeLeaf | undefined

  for (const menu of menus) {
    const definition = getMenuDefinition(menu.id)
    const leaf = buildMenuTreeLeaf(menu, resolveMenuLabel)

    if (menu.id === 'menu-workspace-console') {
      consoleRoot = leaf
      continue
    }

    if (menu.id === 'menu-workspace-permission-center') {
      permissionCenterRoot = leaf
      continue
    }

    if (definition?.section === 'app') {
      appItems.push(leaf)
      continue
    }

    if (definition?.section === 'project') {
      projectItems.push(leaf)
      continue
    }

    if (definition?.section === 'console' || menu.parentId === 'menu-workspace-console') {
      consoleChildren.push(leaf)
      continue
    }

    if (definition?.section === 'permission-center' || menu.parentId === 'menu-workspace-permission-center') {
      permissionCenterChildren.push(leaf)
      continue
    }

    workspaceItems.push(leaf)
  }

  const workspaceTreeItems: Array<MenuTreeLeaf | MenuTreeBranch> = [...workspaceItems]

  if (consoleRoot || consoleChildren.length) {
    workspaceTreeItems.push({
      kind: 'group',
      id: 'console',
      label: groupLabels.console,
      order: consoleRoot?.order ?? Math.min(...consoleChildren.map(item => item.order)),
      rootMenu: consoleRoot,
      children: sortMenuTreeLeaves(consoleChildren),
    })
  }

  if (permissionCenterRoot || permissionCenterChildren.length) {
    workspaceTreeItems.push({
      kind: 'group',
      id: 'permission-center',
      label: groupLabels.permissionCenter,
      order: permissionCenterRoot?.order ?? Math.min(...permissionCenterChildren.map(item => item.order)),
      rootMenu: permissionCenterRoot,
      children: sortMenuTreeLeaves(permissionCenterChildren),
    })
  }

  return [
    {
      id: 'app',
      label: groupLabels.app,
      items: sortMenuTreeItems(appItems),
    },
    {
      id: 'workspace',
      label: groupLabels.workspace,
      items: sortMenuTreeItems(workspaceTreeItems),
    },
    {
      id: 'project',
      label: groupLabels.project,
      items: sortMenuTreeItems(projectItems),
    },
  ]
}

export function isMenuTreeGroup(item: MenuTreeLeaf | MenuTreeBranch): item is MenuTreeBranch {
  return item.kind === 'group'
}

function buildMenuTreeLeaf(menu: MenuRecord, resolveMenuLabel: (menu: MenuRecord) => string): MenuTreeLeaf {
  const definition = getMenuDefinition(menu.id)
  return {
    kind: 'menu',
    id: menu.id,
    label: resolveMenuLabel(menu),
    secondary: menu.routeName || menu.id,
    order: definition?.order ?? menu.order,
    menu,
  }
}

function sortMenuTreeLeaves(items: MenuTreeLeaf[]) {
  return [...items].sort((left, right) => left.order - right.order)
}

function sortMenuTreeItems(items: Array<MenuTreeLeaf | MenuTreeBranch>) {
  return [...items].sort((left, right) => left.order - right.order)
}
