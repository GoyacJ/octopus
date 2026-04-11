export interface UiSelectOption {
  label: string
  value: string
  disabled?: boolean
}

export const statusOptions: UiSelectOption[] = [
  { label: '启用', value: 'active' },
  { label: '禁用', value: 'disabled' },
]

export const policyEffectOptions: UiSelectOption[] = [
  { label: '允许', value: 'allow' },
  { label: '拒绝', value: 'deny' },
]

export const subjectTypeOptions: UiSelectOption[] = [
  { label: '用户', value: 'user' },
  { label: '部门', value: 'org_unit' },
  { label: '岗位', value: 'position' },
  { label: '用户组', value: 'user_group' },
]

export const scopeTypeOptions: UiSelectOption[] = [
  { label: '全部范围', value: 'all' },
  { label: '选定项目', value: 'selected-projects' },
  { label: '直属部门', value: 'org-unit-self' },
  { label: '部门及子部门', value: 'org-unit-subtree' },
  { label: '标签匹配', value: 'tag-match' },
]

export const menuVisibilityOptions: UiSelectOption[] = [
  { label: '跟随授权', value: 'inherit' },
  { label: '始终显示', value: 'visible' },
  { label: '始终隐藏', value: 'hidden' },
]

export const classificationOptions: UiSelectOption[] = [
  { label: '内部', value: 'internal' },
  { label: '机密', value: 'confidential' },
  { label: '限制', value: 'restricted' },
  { label: '绝密', value: 'secret' },
]

export const dataResourceTypeOptions: UiSelectOption[] = [
  { label: '项目', value: 'project' },
  { label: 'Agent', value: 'agent' },
  { label: '资源', value: 'resource' },
  { label: '知识', value: 'knowledge' },
  { label: '工具', value: 'tool' },
]

export const resourceTypeOptions: UiSelectOption[] = [
  { label: 'Agent', value: 'agent' },
  { label: '资源', value: 'resource' },
  { label: '知识', value: 'knowledge' },
  { label: '内置工具', value: 'tool.builtin' },
  { label: 'Skill 工具', value: 'tool.skill' },
  { label: 'MCP 工具', value: 'tool.mcp' },
]

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
