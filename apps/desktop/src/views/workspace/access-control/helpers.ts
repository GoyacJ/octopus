export interface UiSelectOption {
  label: string
  value: string
  disabled?: boolean
}

type Translate = (key: string, params?: Record<string, unknown>) => string

function translated(t: Translate, key: string, fallback: string) {
  const label = t(key)
  return label === key ? fallback : label
}

function optionLabel(t: Translate, namespace: string, value: string) {
  return translated(t, `accessControl.common.options.${namespace}.${value}`, value)
}

function resourceTypeOptionKey(value: string) {
  switch (value) {
    case 'tool.builtin':
      return 'toolBuiltin'
    case 'tool.skill':
      return 'toolSkill'
    case 'tool.mcp':
      return 'toolMcp'
    default:
      return value
  }
}

function capabilityModuleKey(value: string) {
  const [rawPrefix = value] = value.split('.')
  if (rawPrefix === 'provider-credential') {
    return 'providerCredential'
  }
  return rawPrefix
}

function menuSourceOptionKey(value: string) {
  switch (value) {
    case 'access-control':
      return 'accessControl'
    case 'main-sidebar':
      return 'mainSidebar'
    default:
      return value
  }
}

export function createStatusOptions(t: Translate): UiSelectOption[] {
  return [
    { label: optionLabel(t, 'status', 'active'), value: 'active' },
    { label: optionLabel(t, 'status', 'disabled'), value: 'disabled' },
  ]
}

export function createPolicyEffectOptions(t: Translate): UiSelectOption[] {
  return [
    { label: optionLabel(t, 'policyEffect', 'allow'), value: 'allow' },
    { label: optionLabel(t, 'policyEffect', 'deny'), value: 'deny' },
  ]
}

export function createSubjectTypeOptions(t: Translate): UiSelectOption[] {
  return [
    { label: optionLabel(t, 'subjectType', 'user'), value: 'user' },
    { label: optionLabel(t, 'subjectType', 'org_unit'), value: 'org_unit' },
    { label: optionLabel(t, 'subjectType', 'position'), value: 'position' },
    { label: optionLabel(t, 'subjectType', 'user_group'), value: 'user_group' },
  ]
}

export function createScopeTypeOptions(t: Translate): UiSelectOption[] {
  return [
    { label: optionLabel(t, 'scopeType', 'all'), value: 'all' },
    { label: optionLabel(t, 'scopeType', 'selected-projects'), value: 'selected-projects' },
    { label: optionLabel(t, 'scopeType', 'org-unit-self'), value: 'org-unit-self' },
    { label: optionLabel(t, 'scopeType', 'org-unit-subtree'), value: 'org-unit-subtree' },
    { label: optionLabel(t, 'scopeType', 'tag-match'), value: 'tag-match' },
  ]
}

export function createMenuVisibilityOptions(t: Translate): UiSelectOption[] {
  return [
    { label: optionLabel(t, 'menuVisibility', 'inherit'), value: 'inherit' },
    { label: optionLabel(t, 'menuVisibility', 'visible'), value: 'visible' },
    { label: optionLabel(t, 'menuVisibility', 'hidden'), value: 'hidden' },
  ]
}

export function createClassificationOptions(t: Translate): UiSelectOption[] {
  return [
    { label: optionLabel(t, 'classification', 'internal'), value: 'internal' },
    { label: optionLabel(t, 'classification', 'confidential'), value: 'confidential' },
    { label: optionLabel(t, 'classification', 'restricted'), value: 'restricted' },
    { label: optionLabel(t, 'classification', 'secret'), value: 'secret' },
  ]
}

export function createDataResourceTypeOptions(t: Translate): UiSelectOption[] {
  return [
    { label: optionLabel(t, 'dataResourceType', 'project'), value: 'project' },
    { label: optionLabel(t, 'dataResourceType', 'agent'), value: 'agent' },
    { label: optionLabel(t, 'dataResourceType', 'resource'), value: 'resource' },
    { label: optionLabel(t, 'dataResourceType', 'knowledge'), value: 'knowledge' },
    { label: optionLabel(t, 'dataResourceType', 'tool'), value: 'tool' },
  ]
}

export function createResourceTypeOptions(t: Translate): UiSelectOption[] {
  return [
    { label: optionLabel(t, 'resourceType', resourceTypeOptionKey('agent')), value: 'agent' },
    { label: optionLabel(t, 'resourceType', resourceTypeOptionKey('resource')), value: 'resource' },
    { label: optionLabel(t, 'resourceType', resourceTypeOptionKey('knowledge')), value: 'knowledge' },
    { label: optionLabel(t, 'resourceType', resourceTypeOptionKey('tool.builtin')), value: 'tool.builtin' },
    { label: optionLabel(t, 'resourceType', resourceTypeOptionKey('tool.skill')), value: 'tool.skill' },
    { label: optionLabel(t, 'resourceType', resourceTypeOptionKey('tool.mcp')), value: 'tool.mcp' },
  ]
}

export function createAuditOutcomeOptions(t: Translate): UiSelectOption[] {
  return [
    { label: optionLabel(t, 'auditOutcome', 'success'), value: 'success' },
    { label: optionLabel(t, 'auditOutcome', 'denied'), value: 'denied' },
    { label: optionLabel(t, 'auditOutcome', 'failure'), value: 'failure' },
    { label: optionLabel(t, 'auditOutcome', 'locked'), value: 'locked' },
  ]
}

export function getStatusLabel(t: Translate, value: string): string {
  return optionLabel(t, 'status', value)
}

export function getPolicyEffectLabel(t: Translate, value: string): string {
  return optionLabel(t, 'policyEffect', value)
}

export function getSubjectTypeLabel(t: Translate, value: string): string {
  return optionLabel(t, 'subjectType', value)
}

export function getScopeTypeLabel(t: Translate, value: string): string {
  return optionLabel(t, 'scopeType', value)
}

export function getMenuVisibilityLabel(t: Translate, value: string): string {
  return optionLabel(t, 'menuVisibility', value)
}

export function getClassificationLabel(t: Translate, value: string): string {
  return optionLabel(t, 'classification', value)
}

export function getDataResourceTypeLabel(t: Translate, value: string): string {
  return optionLabel(t, 'dataResourceType', value)
}

export function getResourceTypeLabel(t: Translate, value: string): string {
  if (value.startsWith('access.')) {
    return value
  }
  return optionLabel(t, 'resourceType', resourceTypeOptionKey(value))
}

export function getCapabilityModuleLabel(t: Translate, value: string): string {
  return translated(t, `accessControl.common.capabilityModules.${capabilityModuleKey(value)}`, value)
}

export function getMenuSourceLabel(t: Translate, value: string): string {
  return translated(t, `accessControl.common.menuSources.${menuSourceOptionKey(value)}`, value)
}

export function getAuditOutcomeLabel(t: Translate, value: string): string {
  return optionLabel(t, 'auditOutcome', value)
}

export function parseListInput(value: string): string[] {
  return Array.from(
    new Set(
      value
        .split(/[\n,]/g)
        .map(item => item.trim())
        .filter(Boolean),
    ),
  )
}

export function stringifyListInput(value: string[] | undefined): string {
  return (value ?? []).join(', ')
}

export function normalizeOrderInput(value: string, fallback = 0): number {
  const parsed = Number.parseInt(value.trim(), 10)
  return Number.isFinite(parsed) ? parsed : fallback
}
