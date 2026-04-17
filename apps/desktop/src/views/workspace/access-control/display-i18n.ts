import type {
  AccessCapabilityBundle,
  AccessMemberSummary,
  AccessRolePreset,
  AccessRoleRecord,
  AccessRoleTemplate,
  MenuDefinition,
  PermissionDefinition,
} from '@octopus/schema'

import { getMenuDefinition } from '@/navigation/menuRegistry'
import i18n from '@/plugins/i18n'

type RoleLike = Pick<AccessRoleRecord, 'code' | 'name'> & Partial<Pick<AccessRoleRecord, 'description' | 'source'>>

const PRESET_KEY_BY_CODE = {
  owner: 'owner',
  admin: 'admin',
  member: 'member',
  viewer: 'viewer',
  auditor: 'auditor',
  custom: 'custom',
  mixed: 'mixed',
} as const

const TEMPLATE_KEY_BY_CODE = {
  owner: 'owner',
  admin: 'admin',
  member: 'member',
  viewer: 'viewer',
  auditor: 'auditor',
} as const

const BUNDLE_KEY_BY_CODE = {
  workspace_governance: 'workspaceGovernance',
  member_management: 'memberManagement',
  project_and_resource_access: 'projectAndResourceAccess',
  automation_and_tools: 'automationAndTools',
  security_and_audit: 'securityAndAudit',
} as const

const SYSTEM_ROLE_KEY_BY_CODE = {
  'system.owner': 'owner',
  'system.admin': 'admin',
  'system.member': 'member',
  'system.viewer': 'viewer',
  'system.auditor': 'auditor',
} as const

const DISPLAY_ACTION_KEY_BY_CODE = {
  read: 'read',
  view: 'view',
  manage: 'manage',
  create: 'create',
  update: 'update',
  delete: 'delete',
  revoke: 'revoke',
  run: 'run',
  debug: 'debug',
  edit: 'edit',
  publish: 'publish',
  grant: 'grant',
  import: 'import',
  export: 'export',
  upload: 'upload',
  retrieve: 'retrieve',
  enable: 'enable',
  configure: 'configure',
  invoke: 'invoke',
  'bind-credential': 'bindCredential',
  submit_turn: 'submitTurn',
  resolve: 'resolve',
  cancel: 'cancel',
  success: 'success',
  denied: 'denied',
  failure: 'failure',
  locked: 'locked',
} as const

const PERMISSION_RESOURCE_LABEL_KEY_BY_CODE = {
  'access.users': 'members',
  'access.org': 'organization',
  'access.roles': 'roles',
  'access.policies': 'policies',
  'access.menus': 'menus',
  'access.sessions': 'sessions',
  'tool.catalog': 'toolCatalog',
  'runtime': 'runtime',
  'runtime.auth': 'runtimeAuth',
  'runtime.subrun': 'runtimeSubrun',
  audit: 'audit',
} as const

const AUDIT_ENTITY_KEY_BY_CODE = {
  'access.users': 'members',
  'access.users.preset': 'memberPreset',
  'access.user': 'member',
  'access.user-preset': 'memberPreset',
  'access.session': 'session',
  'access.user-sessions': 'userSessions',
  'access.role-binding': 'roleBinding',
  'access.data-policy': 'dataPolicy',
  'access.resource-policy': 'resourcePolicy',
  'access.menu-policy': 'menuPolicy',
  'system.auth': 'systemAuth',
  'system.auth.login': 'systemLogin',
} as const

function isZhLocale() {
  return i18n.global.locale.value.toLowerCase().startsWith('zh')
}

function translateOrFallback(key: string, fallback: string, params?: Record<string, unknown>) {
  return i18n.global.te(key)
    ? String(i18n.global.t(key, params ?? {}))
    : fallback
}

function translateIfExists(key: string, params?: Record<string, unknown>) {
  if (!i18n.global.te(key)) {
    return undefined
  }
  return String(i18n.global.t(key, params ?? {}))
}

function lowerCaseFirst(value: string) {
  if (!value) {
    return value
  }
  return value[0]!.toLowerCase() + value.slice(1)
}

function presetFieldPath(
  code: string | null | undefined,
  field: 'name' | 'description' | 'recommendedFor',
) {
  const presetKey = code ? PRESET_KEY_BY_CODE[code as keyof typeof PRESET_KEY_BY_CODE] : undefined
  if (!presetKey) {
    return undefined
  }
  return `accessControl.display.presets.${presetKey}.${field}`
}

function templateFieldPath(
  code: string | null | undefined,
  field: 'name' | 'description',
) {
  const templateKey = code ? TEMPLATE_KEY_BY_CODE[code as keyof typeof TEMPLATE_KEY_BY_CODE] : undefined
  if (!templateKey) {
    return undefined
  }
  return `accessControl.display.templates.${templateKey}.${field}`
}

function bundleFieldPath(
  code: string | null | undefined,
  field: 'name' | 'description',
) {
  const bundleKey = code ? BUNDLE_KEY_BY_CODE[code as keyof typeof BUNDLE_KEY_BY_CODE] : undefined
  if (!bundleKey) {
    return undefined
  }
  return `accessControl.display.bundles.${bundleKey}.${field}`
}

function roleFieldPath(
  code: string | null | undefined,
  field: 'name' | 'description',
) {
  const roleKey = code ? SYSTEM_ROLE_KEY_BY_CODE[code as keyof typeof SYSTEM_ROLE_KEY_BY_CODE] : undefined
  if (!roleKey) {
    return undefined
  }
  return `accessControl.display.roles.system.${roleKey}.${field}`
}

function displayActionPath(action: string) {
  const actionKey = DISPLAY_ACTION_KEY_BY_CODE[action as keyof typeof DISPLAY_ACTION_KEY_BY_CODE]
  if (!actionKey) {
    return undefined
  }
  return `accessControl.display.actions.${actionKey}`
}

function permissionResourceLabel(resourceType: string) {
  const existingCommonKey = {
    workspace: 'accessControl.common.options.resourceType.workspace',
    project: 'accessControl.common.options.resourceType.project',
    team: 'accessControl.common.options.resourceType.team',
    agent: 'accessControl.common.options.resourceType.agent',
    resource: 'accessControl.common.options.resourceType.resource',
    knowledge: 'accessControl.common.options.resourceType.knowledge',
    automation: 'accessControl.common.options.resourceType.automation',
    pet: 'accessControl.common.options.resourceType.pet',
    artifact: 'accessControl.common.options.resourceType.artifact',
    inbox: 'accessControl.common.options.resourceType.inbox',
    'provider-credential': 'accessControl.common.options.resourceType.provider-credential',
    'tool.builtin': 'accessControl.common.options.resourceType.toolBuiltin',
    'tool.skill': 'accessControl.common.options.resourceType.toolSkill',
    'tool.mcp': 'accessControl.common.options.resourceType.toolMcp',
    'runtime.config.workspace': 'accessControl.common.options.resourceType.runtime.config.workspace',
    'runtime.config.project': 'accessControl.common.options.resourceType.runtime.config.project',
    'runtime.config.user': 'accessControl.common.options.resourceType.runtime.config.user',
    'runtime.session': 'accessControl.common.options.resourceType.runtime.session',
    'runtime.approval': 'accessControl.common.options.resourceType.runtime.approval',
  } as const

  const commonKey = existingCommonKey[resourceType as keyof typeof existingCommonKey]
  if (commonKey && i18n.global.te(commonKey)) {
    return String(i18n.global.t(commonKey))
  }

  const displayKey = PERMISSION_RESOURCE_LABEL_KEY_BY_CODE[resourceType as keyof typeof PERMISSION_RESOURCE_LABEL_KEY_BY_CODE]
  if (displayKey) {
    return translateOrFallback(
      `accessControl.display.permissionResources.${displayKey}`,
      resourceType,
    )
  }

  return resourceType
}

function buildZhPermissionName(permission: PermissionDefinition) {
  const action = permission.actions[0] ?? ''
  const resourceLabel = permissionResourceLabel(permission.resourceType)
  switch (action) {
    case 'bind-credential':
      return `${resourceLabel}绑定凭据`
    case 'grant':
      return `管理${resourceLabel}授权`
    case 'submit_turn':
      return `向${resourceLabel}提交轮次`
    case 'read':
    case 'view':
      return `查看${resourceLabel}`
    case 'manage':
      return `管理${resourceLabel}`
    case 'create':
      return `创建${resourceLabel}`
    case 'update':
    case 'edit':
    case 'configure':
      return `更新${resourceLabel}`
    case 'delete':
      return `删除${resourceLabel}`
    case 'publish':
      return `发布${resourceLabel}`
    case 'import':
      return `导入${resourceLabel}`
    case 'export':
      return `导出${resourceLabel}`
    case 'upload':
      return `上传${resourceLabel}`
    case 'retrieve':
      return `检索${resourceLabel}`
    case 'enable':
      return `启用${resourceLabel}`
    case 'invoke':
      return `调用${resourceLabel}`
    case 'run':
      return `运行${resourceLabel}`
    case 'debug':
      return `调试${resourceLabel}`
    case 'resolve':
      return `处理${resourceLabel}`
    case 'cancel':
      return `取消${resourceLabel}`
    default:
      return permission.name
  }
}

function buildZhPermissionDescription(permission: PermissionDefinition) {
  const action = permission.actions[0] ?? ''
  const resourceLabel = permissionResourceLabel(permission.resourceType)
  switch (action) {
    case 'bind-credential':
      return `允许为${resourceLabel}绑定凭据。`
    case 'grant':
      return `允许管理${resourceLabel}授权。`
    case 'submit_turn':
      return `允许向${resourceLabel}提交轮次。`
    case 'read':
    case 'view':
      return `允许查看${resourceLabel}。`
    case 'manage':
      return `允许管理${resourceLabel}。`
    case 'create':
      return `允许创建${resourceLabel}。`
    case 'update':
    case 'edit':
    case 'configure':
      return `允许更新${resourceLabel}。`
    case 'delete':
      return `允许删除${resourceLabel}。`
    case 'publish':
      return `允许发布${resourceLabel}。`
    case 'import':
      return `允许导入${resourceLabel}。`
    case 'export':
      return `允许导出${resourceLabel}。`
    case 'upload':
      return `允许上传${resourceLabel}。`
    case 'retrieve':
      return `允许检索${resourceLabel}。`
    case 'enable':
      return `允许启用${resourceLabel}。`
    case 'invoke':
      return `允许调用${resourceLabel}。`
    case 'run':
      return `允许运行${resourceLabel}。`
    case 'debug':
      return `允许调试${resourceLabel}。`
    case 'resolve':
      return `允许处理${resourceLabel}。`
    case 'cancel':
      return `允许取消${resourceLabel}。`
    default:
      return permission.description
  }
}

function auditEntityLabel(code: string) {
  const entityKey = AUDIT_ENTITY_KEY_BY_CODE[code as keyof typeof AUDIT_ENTITY_KEY_BY_CODE]
  if (!entityKey) {
    return undefined
  }
  return translateIfExists(`accessControl.display.audit.entities.${entityKey}`)
}

function composeAuditActionLabel(action: string, resourceLabel: string) {
  const actionLabel = getResourceActionLabel(action)
  if (actionLabel === action) {
    return undefined
  }

  if (isZhLocale()) {
    if (action === 'success' || action === 'failure' || action === 'locked' || action === 'denied') {
      return `${resourceLabel}${actionLabel}`
    }
    return `${actionLabel}${resourceLabel}`
  }

  if (action === 'success' || action === 'failure' || action === 'locked' || action === 'denied') {
    return `${resourceLabel} ${lowerCaseFirst(actionLabel)}`
  }
  return `${actionLabel} ${lowerCaseFirst(resourceLabel)}`
}

export function getAccessPresetName(code: string | null | undefined, fallback: string) {
  const key = presetFieldPath(code, 'name')
  return key ? translateOrFallback(key, fallback) : fallback
}

export function getAccessPresetDescription(
  preset: Pick<AccessRolePreset, 'code' | 'description'>,
) {
  const key = presetFieldPath(preset.code, 'description')
  return key ? translateOrFallback(key, preset.description) : preset.description
}

export function getAccessPresetRecommendedFor(
  preset: Pick<AccessRolePreset, 'code' | 'recommendedFor'>,
) {
  const key = presetFieldPath(preset.code, 'recommendedFor')
  return key ? translateOrFallback(key, preset.recommendedFor) : preset.recommendedFor
}

export function getAccessTemplateName(code: string | null | undefined, fallback: string) {
  const key = templateFieldPath(code, 'name')
  return key ? translateOrFallback(key, fallback) : fallback
}

export function getAccessTemplateDescription(
  template: Pick<AccessRoleTemplate, 'code' | 'description'>,
) {
  const key = templateFieldPath(template.code, 'description')
  return key ? translateOrFallback(key, template.description) : template.description
}

export function getAccessCapabilityBundleName(code: string | null | undefined, fallback: string) {
  const key = bundleFieldPath(code, 'name')
  return key ? translateOrFallback(key, fallback) : fallback
}

export function getAccessCapabilityBundleDescription(
  bundle: Pick<AccessCapabilityBundle, 'code' | 'description'>,
) {
  const key = bundleFieldPath(bundle.code, 'description')
  return key ? translateOrFallback(key, bundle.description) : bundle.description
}

export function getAccessRoleName(role: RoleLike) {
  const key = roleFieldPath(role.code, 'name')
  if (!key) {
    return role.name
  }
  if (role.source && role.source !== 'system') {
    return role.name
  }
  return translateOrFallback(key, role.name)
}

export function getAccessRoleDescription(role: RoleLike) {
  const key = roleFieldPath(role.code, 'description')
  if (!key || !role.description) {
    return role.description ?? ''
  }
  if (role.source && role.source !== 'system') {
    return role.description
  }
  return translateOrFallback(key, role.description)
}

export function getAccessMemberPresetName(member: Pick<AccessMemberSummary, 'primaryPresetCode' | 'primaryPresetName'>) {
  const fallback = member.primaryPresetName || translateOrFallback(
    'accessControl.display.presets.unassigned.name',
    'No preset assigned',
  )
  return getAccessPresetName(member.primaryPresetCode, fallback)
}

export function getAccessMemberEffectiveRoleNames(member: Pick<AccessMemberSummary, 'effectiveRoleNames' | 'effectiveRoles'>) {
  if (!member.effectiveRoles?.length) {
    return [...member.effectiveRoleNames]
  }
  return member.effectiveRoles.map(role => getAccessRoleName(role))
}

export function getAccessMenuLabel(menu: Pick<MenuDefinition, 'id' | 'label'>) {
  const definition = getMenuDefinition(menu.id)
  if (definition?.labelKey && i18n.global.te(definition.labelKey)) {
    return String(i18n.global.t(definition.labelKey))
  }
  return menu.label || definition?.defaultLabel || menu.id
}

export function getPermissionDisplayName(permission: PermissionDefinition) {
  const directKey = translateIfExists(
    `accessControl.display.permissions.${permission.code.replaceAll('.', '_').replaceAll('-', '_')}.name`,
  )
  if (directKey) {
    return directKey
  }
  if (!isZhLocale()) {
    return permission.name
  }
  return buildZhPermissionName(permission)
}

export function getPermissionDisplayDescription(permission: PermissionDefinition) {
  const directKey = translateIfExists(
    `accessControl.display.permissions.${permission.code.replaceAll('.', '_').replaceAll('-', '_')}.description`,
  )
  if (directKey) {
    return directKey
  }
  if (!isZhLocale()) {
    return permission.description
  }
  return buildZhPermissionDescription(permission)
}

export function getResourceActionLabel(action: string) {
  const key = displayActionPath(action)
  return key ? translateOrFallback(key, action) : action
}

export function getAuditActionLabel(action: string) {
  const directKey = translateIfExists(
    `accessControl.display.audit.actions.${action.replaceAll('.', '_').replaceAll('-', '_')}`,
  )
  if (directKey) {
    return directKey
  }

  const segments = action.split('.')
  if (segments.length < 2) {
    return action
  }

  const actionCode = segments[segments.length - 1] ?? ''
  const resourceCode = segments.slice(0, -1).join('.')
  const resourceLabel = auditEntityLabel(resourceCode)
  if (!resourceLabel) {
    return action
  }

  return composeAuditActionLabel(actionCode, resourceLabel) ?? action
}

export function getAuditResourceLabel(resource: string) {
  const [resourceCode, resourceId] = resource.split(':')
  if (!resourceCode) {
    return resource
  }

  const resourceLabel = auditEntityLabel(resourceCode)
  if (!resourceLabel) {
    return resource
  }
  if (!resourceId) {
    return resourceLabel
  }

  const separator = isZhLocale() ? '：' : ': '
  return `${resourceLabel}${separator}${resourceId}`
}

export function createProjectAccessPolicyName(name: string) {
  return translateOrFallback(
    'accessControl.display.generated.projectAccessPolicy',
    `${name} project access`,
    { name },
  )
}
