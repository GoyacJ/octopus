<script setup lang="ts">
import { computed, nextTick, reactive, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import {
  UiBadge,
  UiButton,
  UiCheckbox,
  UiDialog,
  UiEmptyState,
  UiField,
  UiHierarchyList,
  UiInput,
  UiListDetailWorkspace,
  UiPagination,
  UiPanelFrame,
  UiRecordCard,
  UiSelect,
  UiSurface,
  UiSwitch,
  UiStatusCallout,
  UiTabs,
  UiToolbarRow,
} from '@octopus/ui'

import type {
  OrgUnitRecord,
  OrgUnitUpsertRequest,
  PositionRecord,
  PositionUpsertRequest,
  UserGroupRecord,
  UserGroupUpsertRequest,
  UserOrgAssignmentRecord,
  UserOrgAssignmentUpsertRequest,
} from '@octopus/schema'

import { usePagination } from '@/composables/usePagination'
import { formatList } from '@/i18n/copy'
import { useWorkspaceAccessControlStore } from '@/stores/workspace-access-control'

import { createStatusOptions, getStatusLabel } from './helpers'
import { useAccessControlNotifications } from './useAccessControlNotifications'
import { useAccessControlSelection } from './useAccessControlSelection'

interface UnitFormState {
  code: string
  name: string
  parentId: string
  status: string
}

interface PositionFormState {
  code: string
  name: string
  status: string
}

interface GroupFormState {
  code: string
  name: string
  status: string
}

interface AssignmentFormState {
  userId: string
  orgUnitId: string
  isPrimary: boolean
  positionIds: string[]
  userGroupIds: string[]
}

interface OrgHierarchyItem {
  id: string
  label: string
  description?: string
  depth: number
  expandable?: boolean
  expanded?: boolean
  selectable?: boolean
  testId: string
  contentTestId?: string
  record: OrgUnitRecord
}

const { t } = useI18n()
const accessControlStore = useWorkspaceAccessControlStore()
const { notifyError, notifySuccess, notifyWarning } = useAccessControlNotifications('access-control.org')

const activeSection = ref('units')
const roleSubsection = ref('positions')
const submitError = ref('')

const unitsQuery = ref('')
const positionsQuery = ref('')
const groupsQuery = ref('')
const assignmentsQuery = ref('')

const selectedOrgUnitId = ref('')
const selectedPositionId = ref('')
const selectedGroupId = ref('')
const selectedAssignmentKey = ref('')

const createUnitDialogOpen = ref(false)
const createPositionDialogOpen = ref(false)
const createGroupDialogOpen = ref(false)
const createAssignmentDialogOpen = ref(false)
const bulkDeleteUnitsDialogOpen = ref(false)
const bulkDeletePositionsDialogOpen = ref(false)
const bulkDeleteGroupsDialogOpen = ref(false)
const bulkDeleteAssignmentsDialogOpen = ref(false)

const savingUnit = ref(false)
const savingPosition = ref(false)
const savingGroup = ref(false)
const savingAssignment = ref(false)
const deletingSelectedUnits = ref(false)
const deletingSelectedPositions = ref(false)
const deletingSelectedGroups = ref(false)
const deletingSelectedAssignments = ref(false)
const hiddenAssignmentKeys = ref<string[]>([])
const togglingUnitIds = ref<string[]>([])
const togglingPositionIds = ref<string[]>([])
const togglingGroupIds = ref<string[]>([])
const unitStatusOverrides = ref<Record<string, string>>({})
const positionStatusOverrides = ref<Record<string, string>>({})
const groupStatusOverrides = ref<Record<string, string>>({})

const unitCreateForm = reactive<UnitFormState>(createEmptyUnitForm())
const unitEditForm = reactive<UnitFormState>(createEmptyUnitForm())
const positionCreateForm = reactive<PositionFormState>(createEmptyPositionForm())
const positionEditForm = reactive<PositionFormState>(createEmptyPositionForm())
const groupCreateForm = reactive<GroupFormState>(createEmptyGroupForm())
const groupEditForm = reactive<GroupFormState>(createEmptyGroupForm())
const assignmentCreateForm = reactive<AssignmentFormState>(createEmptyAssignmentForm())
const assignmentEditForm = reactive<AssignmentFormState>(createEmptyAssignmentForm())

const sectionTabs = computed(() => [
  { value: 'units', label: t('accessControl.org.sections.units') },
  { value: 'roles', label: t('accessControl.org.sections.roles') },
  { value: 'assignments', label: t('accessControl.org.sections.assignments') },
])
const roleTabs = computed(() => [
  { value: 'positions', label: t('accessControl.org.sections.positions') },
  { value: 'groups', label: t('accessControl.org.sections.groups') },
])

const statusOptions = computed(() => createStatusOptions(t))
const orgUnitMap = computed(() => new Map(accessControlStore.orgUnits.map(unit => [unit.id, unit.name])))
const positionMap = computed(() => new Map(accessControlStore.positions.map(position => [position.id, position.name])))
const groupMap = computed(() => new Map(accessControlStore.userGroups.map(group => [group.id, group.name])))
const userMap = computed(() => new Map(accessControlStore.users.map(user => [user.id, user.displayName])))
const orgUnitsById = computed(() => new Map(accessControlStore.orgUnits.map(unit => [unit.id, unit])))
const orgUnitChildrenMap = computed(() => {
  const grouped = new Map<string, OrgUnitRecord[]>()
  for (const unit of accessControlStore.orgUnits) {
    const parentKey = unit.parentId ?? ''
    const items = grouped.get(parentKey) ?? []
    items.push(unit)
    grouped.set(parentKey, items)
  }
  for (const [key, items] of grouped) {
    grouped.set(key, [...items].sort((left, right) => left.name.localeCompare(right.name)))
  }
  return grouped
})
const expandedOrgUnitIds = ref<string[]>(['org-root'])

const userOptions = computed(() =>
  accessControlStore.users.map(user => ({ label: user.displayName, value: user.id })),
)
const orgUnitOptions = computed(() =>
  accessControlStore.orgUnits.map(unit => ({ label: unit.name, value: unit.id })),
)
const parentOptions = computed(() => [
  { label: t('accessControl.common.list.rootUnit'), value: '' },
  ...accessControlStore.orgUnits.map(unit => ({ label: unit.name, value: unit.id })),
])

const filteredOrgUnits = computed(() => {
  const normalizedQuery = unitsQuery.value.trim().toLowerCase()
  return [...accessControlStore.orgUnits]
    .sort((left, right) => left.name.localeCompare(right.name))
    .filter((unit) => {
      if (!normalizedQuery) {
        return true
      }

      return [
        unit.name,
        unit.code,
        orgUnitMap.value.get(unit.parentId ?? '') ?? '',
      ].join(' ').toLowerCase().includes(normalizedQuery)
    })
})

const unitSelection = useAccessControlSelection(() => accessControlStore.orgUnits, {
  getId: unit => unit.id,
  resetOn: [activeSection],
})
const positionSelection = useAccessControlSelection(() => accessControlStore.positions, {
  getId: position => position.id,
  resetOn: [activeSection, roleSubsection],
})
const groupSelection = useAccessControlSelection(() => accessControlStore.userGroups, {
  getId: group => group.id,
  resetOn: [activeSection, roleSubsection],
})
const assignmentSelection = useAccessControlSelection(() => accessControlStore.userOrgAssignments, {
  getId: assignment => assignmentKey(assignment),
  resetOn: [activeSection],
})

const filteredPositions = computed(() => {
  const normalizedQuery = positionsQuery.value.trim().toLowerCase()
  return [...accessControlStore.positions]
    .sort((left, right) => left.name.localeCompare(right.name))
    .filter(position => !normalizedQuery || [
      position.name,
      position.code,
      position.status,
    ].join(' ').toLowerCase().includes(normalizedQuery))
})

const filteredGroups = computed(() => {
  const normalizedQuery = groupsQuery.value.trim().toLowerCase()
  return [...accessControlStore.userGroups]
    .sort((left, right) => left.name.localeCompare(right.name))
    .filter(group => !normalizedQuery || [
      group.name,
      group.code,
      group.status,
    ].join(' ').toLowerCase().includes(normalizedQuery))
})

const filteredAssignments = computed(() => {
  const normalizedQuery = assignmentsQuery.value.trim().toLowerCase()
  return [...accessControlStore.userOrgAssignments]
    .filter(assignment => !hiddenAssignmentKeys.value.includes(assignmentKey(assignment)))
    .sort((left, right) => assignmentTitle(left).localeCompare(assignmentTitle(right)))
    .filter((assignment) => {
      if (!normalizedQuery) {
        return true
      }

      return [
        assignmentTitle(assignment),
        orgUnitMap.value.get(assignment.orgUnitId) ?? assignment.orgUnitId,
        ...assignment.positionIds.map(id => positionMap.value.get(id) ?? id),
        ...assignment.userGroupIds.map(id => groupMap.value.get(id) ?? id),
      ].join(' ').toLowerCase().includes(normalizedQuery)
    })
})

const unitTreeBranchIds = computed(() => {
  const rootChildren = orgUnitChildrenMap.value.get('org-root') ?? []
  const normalizedQuery = unitsQuery.value.trim().toLowerCase()
  if (!normalizedQuery) {
    return rootChildren.map(unit => unit.id)
  }

  return rootChildren
    .filter(unit => branchHasOrgUnitMatch(unit.id, normalizedQuery))
    .map(unit => unit.id)
})

const unitsPagination = usePagination(unitTreeBranchIds, {
  pageSize: 8,
  resetOn: [unitsQuery],
})
const positionsPagination = usePagination(filteredPositions, {
  pageSize: 8,
  resetOn: [positionsQuery, roleSubsection],
})
const groupsPagination = usePagination(filteredGroups, {
  pageSize: 8,
  resetOn: [groupsQuery, roleSubsection],
})
const assignmentsPagination = usePagination(filteredAssignments, {
  pageSize: 8,
  resetOn: [assignmentsQuery],
})

const selectedOrgUnit = computed(() =>
  accessControlStore.orgUnits.find(unit => unit.id === selectedOrgUnitId.value) ?? null,
)
const selectedPosition = computed(() =>
  accessControlStore.positions.find(position => position.id === selectedPositionId.value) ?? null,
)
const selectedGroup = computed(() =>
  accessControlStore.userGroups.find(group => group.id === selectedGroupId.value) ?? null,
)
const selectedAssignment = computed(() =>
  accessControlStore.userOrgAssignments.find(assignmentKeyMatchesSelection) ?? null,
)
const pagedTreeBranchIdSet = computed(() => new Set(unitsPagination.pagedItems.value))
const selectedUnitsForDelete = computed(() =>
  unitSelection.selectedIds.value
    .map(id => accessControlStore.orgUnits.find(unit => unit.id === id) ?? null)
    .filter((unit): unit is NonNullable<typeof unit> => Boolean(unit)),
)
const selectedPositionsForDelete = computed(() =>
  positionSelection.selectedIds.value
    .map(id => accessControlStore.positions.find(position => position.id === id) ?? null)
    .filter((position): position is NonNullable<typeof position> => Boolean(position)),
)
const selectedGroupsForDelete = computed(() =>
  groupSelection.selectedIds.value
    .map(id => accessControlStore.userGroups.find(group => group.id === id) ?? null)
    .filter((group): group is NonNullable<typeof group> => Boolean(group)),
)
const selectedAssignmentsForDelete = computed(() =>
  assignmentSelection.selectedIds.value
    .map(id => accessControlStore.userOrgAssignments.find(assignment => assignmentKey(assignment) === id) ?? null)
    .filter((assignment): assignment is NonNullable<typeof assignment> => Boolean(assignment)),
)
const visibleOrgHierarchyItems = computed<OrgHierarchyItem[]>(() => {
  const root = orgUnitsById.value.get('org-root')
  if (!root) {
    return []
  }

  const normalizedQuery = unitsQuery.value.trim().toLowerCase()
  const rootExpanded = normalizedQuery
    ? true
    : expandedOrgUnitIds.value.includes(root.id)

  const items: OrgHierarchyItem[] = [{
    id: root.id,
    label: root.name,
    description: root.code,
    depth: 0,
    expandable: (orgUnitChildrenMap.value.get(root.id) ?? []).length > 0,
    expanded: rootExpanded,
    selectable: true,
    testId: `access-control-org-unit-record-${root.id}`,
    contentTestId: `access-control-org-unit-node-${root.id}`,
    record: root,
  }]

  if (!rootExpanded) {
    return items
  }

  for (const childId of unitsPagination.pagedItems.value) {
    appendOrgHierarchyItems(items, childId, 1, normalizedQuery)
  }

  return items
})
const allVisibleUnitsSelected = computed(() =>
  unitSelection.isPageSelected(
    visibleOrgHierarchyItems.value
      .filter(item => item.id !== 'org-root')
      .map(item => item.record),
  ),
)
const allVisiblePositionsSelected = computed(() =>
  positionSelection.isPageSelected(positionsPagination.pagedItems.value),
)
const allVisibleGroupsSelected = computed(() =>
  groupSelection.isPageSelected(groupsPagination.pagedItems.value),
)
const allVisibleAssignmentsSelected = computed(() =>
  assignmentSelection.isPageSelected(assignmentsPagination.pagedItems.value),
)

watch(visibleOrgHierarchyItems, (items) => {
  if (selectedOrgUnitId.value && !items.some(item => item.id === selectedOrgUnitId.value)) {
    selectedOrgUnitId.value = ''
  }
}, { immediate: true })

watch(orgUnitChildrenMap, (childrenMap) => {
  const topLevelIds = (childrenMap.get('org-root') ?? []).map(unit => unit.id)
  const knownIds = new Set(accessControlStore.orgUnits.map(unit => unit.id))
  const next = expandedOrgUnitIds.value.filter(id => id === 'org-root' || knownIds.has(id))
  if (!expandedOrgUnitIds.value.length || (expandedOrgUnitIds.value.length === 1 && expandedOrgUnitIds.value[0] === 'org-root')) {
    expandedOrgUnitIds.value = ['org-root', ...topLevelIds]
    return
  }
  if (!next.includes('org-root')) {
    next.unshift('org-root')
  }
  expandedOrgUnitIds.value = Array.from(new Set(next))
}, { immediate: true })

watch(positionsPagination.pagedItems, (positions) => {
  if (selectedPositionId.value && !positions.some(position => position.id === selectedPositionId.value)) {
    selectedPositionId.value = ''
  }
}, { immediate: true })

watch(groupsPagination.pagedItems, (groups) => {
  if (selectedGroupId.value && !groups.some(group => group.id === selectedGroupId.value)) {
    selectedGroupId.value = ''
  }
}, { immediate: true })

watch(assignmentsPagination.pagedItems, (assignments) => {
  if (selectedAssignmentKey.value && !assignments.some(assignmentKeyMatchesSelection)) {
    selectedAssignmentKey.value = ''
  }
}, { immediate: true })

watch(() => accessControlStore.orgUnits.map(unit => [unit.id, unit.status] as const), (records) => {
  const nextOverrides = { ...unitStatusOverrides.value }
  let changed = false

  for (const [unitId, status] of Object.entries(unitStatusOverrides.value)) {
    const matched = records.find(([recordId]) => recordId === unitId)
    if (!matched || matched[1] === status) {
      delete nextOverrides[unitId]
      changed = true
    }
  }

  if (changed) {
    unitStatusOverrides.value = nextOverrides
  }
}, { immediate: true })

watch(() => accessControlStore.positions.map(position => [position.id, position.status] as const), (records) => {
  const nextOverrides = { ...positionStatusOverrides.value }
  let changed = false

  for (const [positionId, status] of Object.entries(positionStatusOverrides.value)) {
    const matched = records.find(([recordId]) => recordId === positionId)
    if (!matched || matched[1] === status) {
      delete nextOverrides[positionId]
      changed = true
    }
  }

  if (changed) {
    positionStatusOverrides.value = nextOverrides
  }
}, { immediate: true })

watch(() => accessControlStore.userGroups.map(group => [group.id, group.status] as const), (records) => {
  const nextOverrides = { ...groupStatusOverrides.value }
  let changed = false

  for (const [groupId, status] of Object.entries(groupStatusOverrides.value)) {
    const matched = records.find(([recordId]) => recordId === groupId)
    if (!matched || matched[1] === status) {
      delete nextOverrides[groupId]
      changed = true
    }
  }

  if (changed) {
    groupStatusOverrides.value = nextOverrides
  }
}, { immediate: true })

watch(selectedOrgUnit, (unit) => {
  Object.assign(unitEditForm, unit ? toUnitForm(unit) : createEmptyUnitForm())
}, { immediate: true })

watch(selectedPosition, (position) => {
  Object.assign(positionEditForm, position ? toPositionForm(position) : createEmptyPositionForm())
}, { immediate: true })

watch(selectedGroup, (group) => {
  Object.assign(groupEditForm, group ? toGroupForm(group) : createEmptyGroupForm())
}, { immediate: true })

watch(selectedAssignment, (assignment) => {
  Object.assign(assignmentEditForm, assignment ? toAssignmentForm(assignment) : createEmptyAssignmentForm())
}, { immediate: true })

watch(
  () => [activeSection.value, roleSubsection.value],
  () => {
    submitError.value = ''
  },
)

function createEmptyUnitForm(): UnitFormState {
  return {
    code: '',
    name: '',
    parentId: '',
    status: 'active',
  }
}

function createEmptyPositionForm(): PositionFormState {
  return {
    code: '',
    name: '',
    status: 'active',
  }
}

function createEmptyGroupForm(): GroupFormState {
  return {
    code: '',
    name: '',
    status: 'active',
  }
}

function createEmptyAssignmentForm(): AssignmentFormState {
  return {
    userId: accessControlStore.users[0]?.id ?? '',
    orgUnitId: accessControlStore.orgUnits[0]?.id ?? '',
    isPrimary: true,
    positionIds: [],
    userGroupIds: [],
  }
}

function toUnitForm(unit: OrgUnitRecord): UnitFormState {
  return {
    code: unit.code,
    name: unit.name,
    parentId: unit.parentId ?? '',
    status: unit.status,
  }
}

function toPositionForm(position: PositionRecord): PositionFormState {
  return {
    code: position.code,
    name: position.name,
    status: position.status,
  }
}

function toGroupForm(group: UserGroupRecord): GroupFormState {
  return {
    code: group.code,
    name: group.name,
    status: group.status,
  }
}

function toAssignmentForm(assignment: UserOrgAssignmentRecord): AssignmentFormState {
  return {
    userId: assignment.userId,
    orgUnitId: assignment.orgUnitId,
    isPrimary: assignment.isPrimary,
    positionIds: [...assignment.positionIds],
    userGroupIds: [...assignment.userGroupIds],
  }
}

function assignmentKey(record: Pick<UserOrgAssignmentRecord, 'userId' | 'orgUnitId'>) {
  return `${record.userId}:${record.orgUnitId}`
}

function hideAssignment(record: Pick<UserOrgAssignmentRecord, 'userId' | 'orgUnitId'>) {
  const key = assignmentKey(record)
  if (!hiddenAssignmentKeys.value.includes(key)) {
    hiddenAssignmentKeys.value = [...hiddenAssignmentKeys.value, key]
  }
}

function showAssignment(record: Pick<UserOrgAssignmentRecord, 'userId' | 'orgUnitId'>) {
  const key = assignmentKey(record)
  hiddenAssignmentKeys.value = hiddenAssignmentKeys.value.filter(item => item !== key)
}

function assignmentKeyMatchesSelection(assignment: Pick<UserOrgAssignmentRecord, 'userId' | 'orgUnitId'>) {
  return assignmentKey(assignment) === selectedAssignmentKey.value
}

function assignmentTitle(assignment: Pick<UserOrgAssignmentRecord, 'userId' | 'orgUnitId'>) {
  return userMap.value.get(assignment.userId) ?? assignment.userId
}

function assignmentOrgLabel(assignment: Pick<UserOrgAssignmentRecord, 'orgUnitId'>) {
  return orgUnitMap.value.get(assignment.orgUnitId) ?? assignment.orgUnitId
}

function labelValues(ids: string[], labelMap: Map<string, string>) {
  return ids.map(id => labelMap.get(id) ?? id)
}

function positionSummary(assignment?: Pick<UserOrgAssignmentRecord, 'positionIds'> | null) {
  return formatList(labelValues(assignment?.positionIds ?? [], positionMap.value)) || t('accessControl.common.list.noPositions')
}

function groupSummary(assignment?: Pick<UserOrgAssignmentRecord, 'userGroupIds'> | null) {
  return formatList(labelValues(assignment?.userGroupIds ?? [], groupMap.value)) || t('accessControl.common.list.noGroups')
}

function selectUnit(unitId: string) {
  selectedOrgUnitId.value = unitId
  submitError.value = ''
  expandOrgUnitAncestors(unitId)
}

function selectPosition(positionId: string) {
  selectedPositionId.value = positionId
  submitError.value = ''
}

function selectGroup(groupId: string) {
  selectedGroupId.value = groupId
  submitError.value = ''
}

function selectAssignment(record: UserOrgAssignmentRecord) {
  selectedAssignmentKey.value = assignmentKey(record)
  submitError.value = ''
}

function openCreateUnitDialog() {
  Object.assign(unitCreateForm, createEmptyUnitForm())
  submitError.value = ''
  createUnitDialogOpen.value = true
}

function openCreatePositionDialog() {
  Object.assign(positionCreateForm, createEmptyPositionForm())
  submitError.value = ''
  createPositionDialogOpen.value = true
}

function openCreateGroupDialog() {
  Object.assign(groupCreateForm, createEmptyGroupForm())
  submitError.value = ''
  createGroupDialogOpen.value = true
}

function openCreateAssignmentDialog() {
  Object.assign(assignmentCreateForm, createEmptyAssignmentForm())
  submitError.value = ''
  createAssignmentDialogOpen.value = true
}

function validateUnitForm(form: UnitFormState) {
  if (!form.code.trim() || !form.name.trim()) {
    return t('accessControl.org.validation.unitRequired')
  }
  return ''
}

function validatePositionForm(form: PositionFormState) {
  if (!form.code.trim() || !form.name.trim()) {
    return t('accessControl.org.validation.positionRequired')
  }
  return ''
}

function validateGroupForm(form: GroupFormState) {
  if (!form.code.trim() || !form.name.trim()) {
    return t('accessControl.org.validation.groupRequired')
  }
  return ''
}

function validateAssignmentForm(form: AssignmentFormState) {
  if (!form.userId || !form.orgUnitId) {
    return t('accessControl.org.validation.assignmentRequired')
  }
  return ''
}

function toUnitPayload(form: UnitFormState): OrgUnitUpsertRequest {
  return {
    code: form.code.trim(),
    name: form.name.trim(),
    parentId: form.parentId || undefined,
    status: form.status,
  }
}

function toPositionPayload(form: PositionFormState): PositionUpsertRequest {
  return {
    code: form.code.trim(),
    name: form.name.trim(),
    status: form.status,
  }
}

function toGroupPayload(form: GroupFormState): UserGroupUpsertRequest {
  return {
    code: form.code.trim(),
    name: form.name.trim(),
    status: form.status,
  }
}

function toAssignmentPayload(form: AssignmentFormState): UserOrgAssignmentUpsertRequest {
  return {
    userId: form.userId,
    orgUnitId: form.orgUnitId,
    isPrimary: form.isPrimary,
    positionIds: [...form.positionIds],
    userGroupIds: [...form.userGroupIds],
  }
}

function isStatusSaving(ids: string[], recordId: string) {
  return ids.includes(recordId)
}

function listStatus(
  target: Record<string, string>,
  recordId: string,
  fallbackStatus: string,
) {
  return target[recordId] ?? fallbackStatus
}

function setStatusOverride(
  target: typeof unitStatusOverrides | typeof positionStatusOverrides | typeof groupStatusOverrides,
  recordId: string,
  status: string,
) {
  target.value = {
    ...target.value,
    [recordId]: status,
  }
}

function clearStatusOverride(
  target: typeof unitStatusOverrides | typeof positionStatusOverrides | typeof groupStatusOverrides,
  recordId: string,
) {
  const { [recordId]: _ignored, ...rest } = target.value
  target.value = rest
}

function isRootOrgUnit(unit: OrgUnitRecord) {
  return unit.id === 'org-root'
}

function hasUnitChildren(unitId: string) {
  return (orgUnitChildrenMap.value.get(unitId) ?? []).length > 0
}

function canDeleteOrgUnit(unit: OrgUnitRecord) {
  return !isRootOrgUnit(unit) && !hasUnitChildren(unit.id)
}

function branchHasOrgUnitMatch(branchId: string, normalizedQuery: string): boolean {
  const unit = orgUnitsById.value.get(branchId)
  if (!unit) {
    return false
  }

  const currentMatches = [
    unit.name,
    unit.code,
    orgUnitMap.value.get(unit.parentId ?? '') ?? '',
  ].join(' ').toLowerCase().includes(normalizedQuery)

  if (currentMatches) {
    return true
  }

  return (orgUnitChildrenMap.value.get(unit.id) ?? []).some(child =>
    branchHasOrgUnitMatch(child.id, normalizedQuery),
  )
}

function toggleOrgUnitExpanded(unitId: string) {
  const next = new Set(expandedOrgUnitIds.value)
  if (next.has(unitId)) {
    next.delete(unitId)
  } else {
    next.add(unitId)
  }
  expandedOrgUnitIds.value = Array.from(next)
}

function expandOrgUnitAncestors(unitId: string) {
  const next = new Set(expandedOrgUnitIds.value)
  let current = orgUnitsById.value.get(unitId)
  while (current?.parentId) {
    next.add(current.parentId)
    current = orgUnitsById.value.get(current.parentId)
  }
  next.add('org-root')
  expandedOrgUnitIds.value = Array.from(next)
}

function appendOrgHierarchyItems(
  items: OrgHierarchyItem[],
  unitId: string,
  depth: number,
  normalizedQuery: string,
) {
  const unit = orgUnitsById.value.get(unitId)
  if (!unit) {
    return
  }

  if (normalizedQuery && !branchHasOrgUnitMatch(unit.id, normalizedQuery)) {
    return
  }

  const children = orgUnitChildrenMap.value.get(unit.id) ?? []
  const expanded = normalizedQuery
    ? true
    : expandedOrgUnitIds.value.includes(unit.id)
  items.push({
    id: unit.id,
    label: unit.name,
    description: unit.code,
    depth,
    expandable: children.length > 0,
    expanded,
    selectable: true,
    testId: `access-control-org-unit-record-${unit.id}`,
    contentTestId: `access-control-org-unit-node-${unit.id}`,
    record: unit,
  })

  if (!children.length || !expanded) {
    return
  }

  for (const child of children) {
    appendOrgHierarchyItems(items, child.id, depth + 1, normalizedQuery)
  }
}

function toggleUnitSelection(unitId: string, value: boolean) {
  unitSelection.toggleSelection(unitId, value)
}

function toggleAllVisibleUnits(value: boolean) {
  unitSelection.selectPage(
    visibleOrgHierarchyItems.value
      .filter(item => item.id !== 'org-root')
      .map(item => item.record),
    value,
  )
}

function togglePositionSelection(positionId: string, value: boolean) {
  positionSelection.toggleSelection(positionId, value)
}

function toggleAllVisiblePositions(value: boolean) {
  positionSelection.selectPage(positionsPagination.pagedItems.value, value)
}

function toggleGroupSelection(groupId: string, value: boolean) {
  groupSelection.toggleSelection(groupId, value)
}

function toggleAllVisibleGroups(value: boolean) {
  groupSelection.selectPage(groupsPagination.pagedItems.value, value)
}

function toggleAssignmentSelection(assignment: UserOrgAssignmentRecord, value: boolean) {
  assignmentSelection.toggleSelection(assignmentKey(assignment), value)
}

function toggleAllVisibleAssignments(value: boolean) {
  assignmentSelection.selectPage(assignmentsPagination.pagedItems.value, value)
}

function getOrgHierarchyRecord(itemId: string) {
  return orgUnitsById.value.get(itemId) ?? null
}

async function notifyBulkDeleteResult(
  successCount: number,
  failureCount: number,
  skippedCount: number,
  clearSelection: () => void,
) {
  await nextTick()

  const body = t('accessControl.common.bulk.resultBody', {
    success: successCount,
    failure: failureCount,
    skipped: skippedCount,
  })

  if (successCount > 0 && failureCount === 0) {
    if (skippedCount > 0) {
      await notifyWarning(t('accessControl.common.bulk.resultPartialTitle'), body)
    } else {
      clearSelection()
      await notifySuccess(t('accessControl.common.bulk.resultAllSuccessTitle'), body)
    }
    return
  }

  if (successCount > 0 || skippedCount > 0) {
    await notifyWarning(t('accessControl.common.bulk.resultPartialTitle'), body)
    return
  }

  await notifyError(t('accessControl.common.bulk.resultFailureTitle'), body)
}

async function toggleUnitStatus(unit: OrgUnitRecord, enabled: boolean) {
  const nextStatus = enabled ? 'active' : 'disabled'
  if (isStatusSaving(togglingUnitIds.value, unit.id) || nextStatus === unit.status) {
    return
  }

  submitError.value = ''
  setStatusOverride(unitStatusOverrides, unit.id, nextStatus)
  togglingUnitIds.value = [...togglingUnitIds.value, unit.id]

  try {
    await accessControlStore.updateOrgUnit(unit.id, {
      code: unit.code,
      name: unit.name,
      parentId: unit.parentId ?? undefined,
      status: nextStatus,
    })
    await notifySuccess(
      t(enabled ? 'accessControl.org.feedback.toastUnitEnabled' : 'accessControl.org.feedback.toastUnitDisabled'),
      unit.name,
    )
  } catch (error) {
    clearStatusOverride(unitStatusOverrides, unit.id)
    submitError.value = error instanceof Error ? error.message : t('accessControl.org.feedback.updateUnitStatusFailed')
  } finally {
    togglingUnitIds.value = togglingUnitIds.value.filter(id => id !== unit.id)
  }
}

async function togglePositionStatus(position: PositionRecord, enabled: boolean) {
  const nextStatus = enabled ? 'active' : 'disabled'
  if (isStatusSaving(togglingPositionIds.value, position.id) || nextStatus === position.status) {
    return
  }

  submitError.value = ''
  setStatusOverride(positionStatusOverrides, position.id, nextStatus)
  togglingPositionIds.value = [...togglingPositionIds.value, position.id]

  try {
    await accessControlStore.updatePosition(position.id, {
      code: position.code,
      name: position.name,
      status: nextStatus,
    })
    await notifySuccess(
      t(enabled ? 'accessControl.org.feedback.toastPositionEnabled' : 'accessControl.org.feedback.toastPositionDisabled'),
      position.name,
    )
  } catch (error) {
    clearStatusOverride(positionStatusOverrides, position.id)
    submitError.value = error instanceof Error ? error.message : t('accessControl.org.feedback.updatePositionStatusFailed')
  } finally {
    togglingPositionIds.value = togglingPositionIds.value.filter(id => id !== position.id)
  }
}

async function toggleGroupStatus(group: UserGroupRecord, enabled: boolean) {
  const nextStatus = enabled ? 'active' : 'disabled'
  if (isStatusSaving(togglingGroupIds.value, group.id) || nextStatus === group.status) {
    return
  }

  submitError.value = ''
  setStatusOverride(groupStatusOverrides, group.id, nextStatus)
  togglingGroupIds.value = [...togglingGroupIds.value, group.id]

  try {
    await accessControlStore.updateUserGroup(group.id, {
      code: group.code,
      name: group.name,
      status: nextStatus,
    })
    await notifySuccess(
      t(enabled ? 'accessControl.org.feedback.toastGroupEnabled' : 'accessControl.org.feedback.toastGroupDisabled'),
      group.name,
    )
  } catch (error) {
    clearStatusOverride(groupStatusOverrides, group.id)
    submitError.value = error instanceof Error ? error.message : t('accessControl.org.feedback.updateGroupStatusFailed')
  } finally {
    togglingGroupIds.value = togglingGroupIds.value.filter(id => id !== group.id)
  }
}

async function handleCreateUnit() {
  submitError.value = validateUnitForm(unitCreateForm)
  if (submitError.value) {
    return
  }

  savingUnit.value = true
  try {
    const record = await accessControlStore.createOrgUnit(toUnitPayload(unitCreateForm))
    selectedOrgUnitId.value = record.id
    createUnitDialogOpen.value = false
    await notifySuccess(t('accessControl.org.feedback.toastUnitSaved'), record.name)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.org.feedback.saveUnitFailed')
  } finally {
    savingUnit.value = false
  }
}

async function handleSaveUnit() {
  if (!selectedOrgUnit.value) {
    return
  }

  submitError.value = validateUnitForm(unitEditForm)
  if (submitError.value) {
    return
  }

  savingUnit.value = true
  try {
    const payload = toUnitPayload(unitEditForm)
    await accessControlStore.updateOrgUnit(selectedOrgUnit.value.id, payload)
    await notifySuccess(t('accessControl.org.feedback.toastUnitSaved'), payload.name)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.org.feedback.saveUnitFailed')
  } finally {
    savingUnit.value = false
  }
}

async function handleDeleteUnit() {
  if (!selectedOrgUnit.value) {
    return
  }

  submitError.value = ''
  if (isRootOrgUnit(selectedOrgUnit.value)) {
    await notifyWarning(
      t('accessControl.org.feedback.rootUnitDeleteBlockedTitle'),
      t('accessControl.org.feedback.rootUnitDeleteBlockedBody'),
    )
    return
  }

  if (hasUnitChildren(selectedOrgUnit.value.id)) {
    await notifyWarning(
      t('accessControl.org.feedback.parentUnitDeleteBlockedTitle'),
      t('accessControl.org.feedback.parentUnitDeleteBlockedBody', { name: selectedOrgUnit.value.name }),
    )
    return
  }

  try {
    const label = selectedOrgUnit.value.name
    await accessControlStore.deleteOrgUnit(selectedOrgUnit.value.id)
    selectedOrgUnitId.value = ''
    await notifySuccess(t('accessControl.org.feedback.toastUnitDeleted'), label)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.org.feedback.deleteUnitFailed')
  }
}

async function handleBulkDeleteUnits() {
  if (!selectedUnitsForDelete.value.length) {
    bulkDeleteUnitsDialogOpen.value = false
    return
  }

  deletingSelectedUnits.value = true
  let successCount = 0
  let failureCount = 0
  let skippedCount = 0

  for (const unit of selectedUnitsForDelete.value) {
    if (!canDeleteOrgUnit(unit)) {
      skippedCount += 1
      continue
    }

    try {
      await accessControlStore.deleteOrgUnit(unit.id)
      successCount += 1
      if (selectedOrgUnitId.value === unit.id) {
        selectedOrgUnitId.value = ''
      }
    } catch {
      failureCount += 1
    }
  }

  deletingSelectedUnits.value = false
  bulkDeleteUnitsDialogOpen.value = false
  unitSelection.setSelection(
    unitSelection.selectedIds.value.filter(id =>
      accessControlStore.orgUnits.some(unit => unit.id === id),
    ),
  )
  await notifyBulkDeleteResult(successCount, failureCount, skippedCount, () => unitSelection.clearSelection())
}

async function handleCreatePosition() {
  submitError.value = validatePositionForm(positionCreateForm)
  if (submitError.value) {
    return
  }

  savingPosition.value = true
  try {
    const record = await accessControlStore.createPosition(toPositionPayload(positionCreateForm))
    selectedPositionId.value = record.id
    createPositionDialogOpen.value = false
    await notifySuccess(t('accessControl.org.feedback.toastPositionSaved'), record.name)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.org.feedback.savePositionFailed')
  } finally {
    savingPosition.value = false
  }
}

async function handleSavePosition() {
  if (!selectedPosition.value) {
    return
  }

  submitError.value = validatePositionForm(positionEditForm)
  if (submitError.value) {
    return
  }

  savingPosition.value = true
  try {
    const payload = toPositionPayload(positionEditForm)
    await accessControlStore.updatePosition(selectedPosition.value.id, payload)
    await notifySuccess(t('accessControl.org.feedback.toastPositionSaved'), payload.name)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.org.feedback.savePositionFailed')
  } finally {
    savingPosition.value = false
  }
}

async function handleDeletePosition() {
  if (!selectedPosition.value) {
    return
  }

  submitError.value = ''
  try {
    const label = selectedPosition.value.name
    await accessControlStore.deletePosition(selectedPosition.value.id)
    selectedPositionId.value = ''
    await notifySuccess(t('accessControl.org.feedback.toastPositionDeleted'), label)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.org.feedback.deletePositionFailed')
  }
}

async function handleBulkDeletePositions() {
  if (!selectedPositionsForDelete.value.length) {
    bulkDeletePositionsDialogOpen.value = false
    return
  }

  deletingSelectedPositions.value = true
  submitError.value = ''
  let successCount = 0
  let failureCount = 0

  for (const position of selectedPositionsForDelete.value) {
    try {
      await accessControlStore.deletePosition(position.id)
      successCount += 1
      if (selectedPositionId.value === position.id) {
        selectedPositionId.value = ''
      }
    } catch {
      failureCount += 1
    }
  }

  deletingSelectedPositions.value = false
  bulkDeletePositionsDialogOpen.value = false
  positionSelection.setSelection(
    positionSelection.selectedIds.value.filter(id =>
      accessControlStore.positions.some(position => position.id === id),
    ),
  )
  await notifyBulkDeleteResult(successCount, failureCount, 0, () => positionSelection.clearSelection())
}

async function handleCreateGroup() {
  submitError.value = validateGroupForm(groupCreateForm)
  if (submitError.value) {
    return
  }

  savingGroup.value = true
  try {
    const record = await accessControlStore.createUserGroup(toGroupPayload(groupCreateForm))
    selectedGroupId.value = record.id
    createGroupDialogOpen.value = false
    await notifySuccess(t('accessControl.org.feedback.toastGroupSaved'), record.name)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.org.feedback.saveGroupFailed')
  } finally {
    savingGroup.value = false
  }
}

async function handleSaveGroup() {
  if (!selectedGroup.value) {
    return
  }

  submitError.value = validateGroupForm(groupEditForm)
  if (submitError.value) {
    return
  }

  savingGroup.value = true
  try {
    const payload = toGroupPayload(groupEditForm)
    await accessControlStore.updateUserGroup(selectedGroup.value.id, payload)
    await notifySuccess(t('accessControl.org.feedback.toastGroupSaved'), payload.name)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.org.feedback.saveGroupFailed')
  } finally {
    savingGroup.value = false
  }
}

async function handleDeleteGroup() {
  if (!selectedGroup.value) {
    return
  }

  submitError.value = ''
  try {
    const label = selectedGroup.value.name
    await accessControlStore.deleteUserGroup(selectedGroup.value.id)
    selectedGroupId.value = ''
    await notifySuccess(t('accessControl.org.feedback.toastGroupDeleted'), label)
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.org.feedback.deleteGroupFailed')
  }
}

async function handleBulkDeleteGroups() {
  if (!selectedGroupsForDelete.value.length) {
    bulkDeleteGroupsDialogOpen.value = false
    return
  }

  deletingSelectedGroups.value = true
  submitError.value = ''
  let successCount = 0
  let failureCount = 0

  for (const group of selectedGroupsForDelete.value) {
    try {
      await accessControlStore.deleteUserGroup(group.id)
      successCount += 1
      if (selectedGroupId.value === group.id) {
        selectedGroupId.value = ''
      }
    } catch {
      failureCount += 1
    }
  }

  deletingSelectedGroups.value = false
  bulkDeleteGroupsDialogOpen.value = false
  groupSelection.setSelection(
    groupSelection.selectedIds.value.filter(id =>
      accessControlStore.userGroups.some(group => group.id === id),
    ),
  )
  await notifyBulkDeleteResult(successCount, failureCount, 0, () => groupSelection.clearSelection())
}

async function handleCreateAssignment() {
  submitError.value = validateAssignmentForm(assignmentCreateForm)
  if (submitError.value) {
    return
  }

  savingAssignment.value = true
  try {
    const record = await accessControlStore.upsertUserOrgAssignment(toAssignmentPayload(assignmentCreateForm))
    showAssignment(record)
    selectedAssignmentKey.value = assignmentKey(record)
    createAssignmentDialogOpen.value = false
    await notifySuccess(t('accessControl.org.feedback.toastAssignmentSaved'), assignmentTitle(record))
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.org.feedback.saveAssignmentFailed')
  } finally {
    savingAssignment.value = false
  }
}

async function handleSaveAssignment() {
  if (!selectedAssignment.value) {
    return
  }

  submitError.value = validateAssignmentForm(assignmentEditForm)
  if (submitError.value) {
    return
  }

  savingAssignment.value = true
  try {
    const payload = toAssignmentPayload(assignmentEditForm)
    const record = await accessControlStore.upsertUserOrgAssignment(payload)
    showAssignment(record)
    selectedAssignmentKey.value = assignmentKey(record)
    await notifySuccess(t('accessControl.org.feedback.toastAssignmentSaved'), assignmentTitle(record))
  } catch (error) {
    submitError.value = error instanceof Error ? error.message : t('accessControl.org.feedback.saveAssignmentFailed')
  } finally {
    savingAssignment.value = false
  }
}

async function handleDeleteAssignment() {
  if (!selectedAssignment.value) {
    return
  }

  submitError.value = ''
  try {
    const label = assignmentTitle(selectedAssignment.value)
    hideAssignment(selectedAssignment.value)
    await accessControlStore.deleteUserOrgAssignment(selectedAssignment.value.userId, selectedAssignment.value.orgUnitId)
    selectedAssignmentKey.value = ''
    await notifySuccess(t('accessControl.org.feedback.toastAssignmentDeleted'), label)
  } catch (error) {
    showAssignment(selectedAssignment.value)
    submitError.value = error instanceof Error ? error.message : t('accessControl.org.feedback.deleteAssignmentFailed')
  }
}

async function handleBulkDeleteAssignments() {
  if (!selectedAssignmentsForDelete.value.length) {
    bulkDeleteAssignmentsDialogOpen.value = false
    return
  }

  deletingSelectedAssignments.value = true
  submitError.value = ''
  let successCount = 0
  let failureCount = 0

  for (const assignment of selectedAssignmentsForDelete.value) {
    try {
      hideAssignment(assignment)
      await accessControlStore.deleteUserOrgAssignment(assignment.userId, assignment.orgUnitId)
      successCount += 1
      if (selectedAssignmentKey.value === assignmentKey(assignment)) {
        selectedAssignmentKey.value = ''
      }
    } catch {
      showAssignment(assignment)
      failureCount += 1
    }
  }

  deletingSelectedAssignments.value = false
  bulkDeleteAssignmentsDialogOpen.value = false
  assignmentSelection.setSelection(
    assignmentSelection.selectedIds.value.filter(id =>
      accessControlStore.userOrgAssignments.some(assignment => assignmentKey(assignment) === id),
    ),
  )
  await notifyBulkDeleteResult(successCount, failureCount, 0, () => assignmentSelection.clearSelection())
}
</script>

<template>
  <div class="space-y-4" data-testid="access-control-org-shell">
    <UiStatusCallout v-if="submitError" tone="error" :description="submitError" />

    <UiTabs v-model="activeSection" :tabs="sectionTabs" data-testid="access-control-org-section-tabs" />

    <UiListDetailWorkspace
      v-if="activeSection === 'units'"
      :has-selection="Boolean(selectedOrgUnit)"
      :detail-title="selectedOrgUnit ? selectedOrgUnit.name : ''"
      :detail-subtitle="t('accessControl.org.units.detailSubtitle')"
      :empty-detail-title="t('accessControl.org.units.emptyTitle')"
      :empty-detail-description="t('accessControl.org.units.emptyDescription')"
    >
      <template #toolbar>
        <UiToolbarRow test-id="access-control-org-units-toolbar">
          <template #search>
            <UiInput v-model="unitsQuery" :placeholder="t('accessControl.org.units.toolbarSearch')" />
          </template>
          <template #actions>
            <span
              v-if="unitSelection.hasSelection.value"
              class="text-xs text-text-secondary"
            >
              {{ t('accessControl.common.selection.selectedCount', { count: unitSelection.selectedCount.value }) }}
            </span>
            <UiButton
              v-if="visibleOrgHierarchyItems.length > 1"
              variant="ghost"
              size="sm"
              @click="toggleAllVisibleUnits(!allVisibleUnitsSelected)"
            >
              {{ t('accessControl.common.selection.selectPage') }}
            </UiButton>
            <UiButton
              v-if="unitSelection.hasSelection.value"
              variant="ghost"
              size="sm"
              @click="unitSelection.clearSelection"
            >
              {{ t('accessControl.common.selection.clear') }}
            </UiButton>
            <UiButton
              v-if="unitSelection.hasSelection.value"
              variant="destructive"
              size="sm"
              data-testid="access-control-org-units-bulk-delete-button"
              @click="bulkDeleteUnitsDialogOpen = true"
            >
              {{ t('accessControl.common.bulk.delete') }}
            </UiButton>
            <UiButton size="sm" @click="openCreateUnitDialog">
              {{ t('accessControl.org.units.create') }}
            </UiButton>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame
          variant="panel"
          padding="md"
          :title="t('accessControl.org.units.listTitle')"
          :subtitle="t('accessControl.common.list.totalUnits', { count: accessControlStore.orgUnits.length })"
        >
          <UiHierarchyList
            v-if="visibleOrgHierarchyItems.length"
            :items="visibleOrgHierarchyItems"
            :selected-id="selectedOrgUnitId"
            class="space-y-2"
            @select="selectUnit"
            @toggle="toggleOrgUnitExpanded"
          >
            <template #leading="{ item }">
              <UiCheckbox
                v-if="item.id !== 'org-root'"
                :model-value="unitSelection.isSelected(item.id)"
                :data-testid="`access-control-org-unit-select-${item.id}`"
                @update:model-value="toggleUnitSelection(item.id, Boolean($event))"
              />
              <div v-else class="size-[14px]" aria-hidden="true" />
            </template>

            <template #default="{ item }">
              <div
                :data-testid="`access-control-org-unit-record-${item.id}`"
                class="min-w-0"
              >
                <div class="truncate text-sm font-medium text-text-primary">
                  {{ item.label }}
                </div>
                <div class="truncate pt-0.5 text-xs text-text-secondary">
                  {{ item.description }}
                </div>
              </div>
            </template>

            <template #meta="{ item }">
              <span v-if="getOrgHierarchyRecord(item.id)" class="truncate text-xs text-text-secondary">
                {{
                  getOrgHierarchyRecord(item.id)?.parentId
                    ? t('accessControl.common.list.parentUnit', {
                      name: orgUnitMap.get(getOrgHierarchyRecord(item.id)?.parentId ?? '') ?? getOrgHierarchyRecord(item.id)?.parentId,
                    })
                    : t('accessControl.common.list.rootUnit')
                }}
              </span>
            </template>

            <template #badges="{ item }">
              <div v-if="getOrgHierarchyRecord(item.id)" class="flex items-center gap-2" @click.stop>
                <UiSwitch
                  :model-value="listStatus(unitStatusOverrides, getOrgHierarchyRecord(item.id)?.id ?? '', getOrgHierarchyRecord(item.id)?.status ?? 'disabled') === 'active'"
                  :disabled="isStatusSaving(togglingUnitIds, getOrgHierarchyRecord(item.id)?.id ?? '')"
                  @update:model-value="getOrgHierarchyRecord(item.id) && toggleUnitStatus(getOrgHierarchyRecord(item.id)!, $event)"
                >
                  <span class="sr-only">
                    {{ t('accessControl.org.units.list.toggleStatus', { name: getOrgHierarchyRecord(item.id)?.name }) }}
                  </span>
                </UiSwitch>
              </div>
            </template>
          </UiHierarchyList>
          <UiEmptyState
            v-else
            :title="t('accessControl.org.units.noListTitle')"
            :description="t('accessControl.org.units.noListDescription')"
          />

          <div class="mt-3 pt-2">
            <UiPagination
              v-model:page="unitsPagination.currentPage.value"
              :page-count="unitsPagination.pageCount.value"
              :previous-label="t('accessControl.common.pagination.previous')"
              :next-label="t('accessControl.common.pagination.next')"
              :summary-label="t('accessControl.common.pagination.summary', { count: unitsPagination.totalItems.value })"
            />
          </div>
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedOrgUnit" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ selectedOrgUnit.name }}</div>
              <UiBadge :label="getStatusLabel(t, selectedOrgUnit.status)" subtle />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">{{ selectedOrgUnit.code }}</div>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField :label="t('accessControl.org.units.fields.code')">
              <UiInput v-model="unitEditForm.code" />
            </UiField>
            <UiField :label="t('accessControl.org.units.fields.name')">
              <UiInput v-model="unitEditForm.name" />
            </UiField>
            <UiField :label="t('accessControl.org.units.fields.parent')">
              <UiSelect v-model="unitEditForm.parentId" :options="parentOptions" />
            </UiField>
            <UiField :label="t('accessControl.org.units.fields.status')">
              <UiSelect v-model="unitEditForm.status" :options="statusOptions" />
            </UiField>
          </div>

          <div class="flex flex-wrap justify-between gap-2">
            <UiButton
              variant="ghost"
              class="text-destructive"
              data-testid="access-control-org-unit-delete-button"
              @click="handleDeleteUnit"
            >
              {{ t('accessControl.org.units.delete') }}
            </UiButton>
            <UiButton :loading="savingUnit" @click="handleSaveUnit">
              {{ t('accessControl.org.units.save') }}
            </UiButton>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>

    <div v-else-if="activeSection === 'roles'" class="space-y-4">
      <UiTabs v-model="roleSubsection" :tabs="roleTabs" />

      <UiListDetailWorkspace
        v-if="roleSubsection === 'positions'"
        :has-selection="Boolean(selectedPosition)"
        :detail-title="selectedPosition ? selectedPosition.name : ''"
        :detail-subtitle="t('accessControl.org.positions.detailSubtitle')"
        :empty-detail-title="t('accessControl.org.positions.emptyTitle')"
        :empty-detail-description="t('accessControl.org.positions.emptyDescription')"
      >
        <template #toolbar>
          <UiToolbarRow test-id="access-control-org-positions-toolbar">
            <template #search>
              <UiInput v-model="positionsQuery" :placeholder="t('accessControl.org.positions.toolbarSearch')" />
            </template>
            <template #actions>
              <span
                v-if="positionSelection.hasSelection.value"
                class="text-xs text-text-secondary"
              >
                {{ t('accessControl.common.selection.selectedCount', { count: positionSelection.selectedCount.value }) }}
              </span>
              <UiButton
                v-if="positionsPagination.pagedItems.value.length"
                variant="ghost"
                size="sm"
                @click="toggleAllVisiblePositions(!allVisiblePositionsSelected)"
              >
                {{ t('accessControl.common.selection.selectPage') }}
              </UiButton>
              <UiButton
                v-if="positionSelection.hasSelection.value"
                variant="ghost"
                size="sm"
                @click="positionSelection.clearSelection"
              >
                {{ t('accessControl.common.selection.clear') }}
              </UiButton>
              <UiButton
                v-if="positionSelection.hasSelection.value"
                variant="destructive"
                size="sm"
                data-testid="access-control-org-positions-bulk-delete-button"
                @click="bulkDeletePositionsDialogOpen = true"
              >
                {{ t('accessControl.common.bulk.delete') }}
              </UiButton>
              <UiButton size="sm" @click="openCreatePositionDialog">
                {{ t('accessControl.org.positions.create') }}
              </UiButton>
            </template>
          </UiToolbarRow>
        </template>

        <template #list>
          <UiPanelFrame
            variant="panel"
            padding="md"
            :title="t('accessControl.org.positions.listTitle')"
            :subtitle="t('accessControl.common.list.totalPositions', { count: positionsPagination.totalItems.value })"
          >
            <div v-if="positionsPagination.pagedItems.value.length" class="space-y-2">
            <UiRecordCard
              v-for="position in positionsPagination.pagedItems.value"
              :key="position.id"
              layout="compact"
                interactive
                :active="selectedPositionId === position.id"
                :title="position.name"
              :description="position.code"
                :test-id="`access-control-org-position-record-${position.id}`"
                @click="selectPosition(position.id)"
              >
                <template #badges>
                  <div class="flex items-center gap-2" @click.stop>
                    <UiCheckbox
                      :model-value="positionSelection.isSelected(position.id)"
                      :data-testid="`access-control-org-position-select-${position.id}`"
                      @update:model-value="togglePositionSelection(position.id, Boolean($event))"
                    />
                    <UiSwitch
                      :model-value="listStatus(positionStatusOverrides, position.id, position.status) === 'active'"
                      :disabled="isStatusSaving(togglingPositionIds, position.id)"
                      @update:model-value="togglePositionStatus(position, $event)"
                    >
                      <span class="sr-only">
                        {{ t('accessControl.org.positions.list.toggleStatus', { name: position.name }) }}
                      </span>
                    </UiSwitch>
                  </div>
                </template>
              </UiRecordCard>
            </div>
            <UiEmptyState
              v-else
              :title="t('accessControl.org.positions.noListTitle')"
              :description="t('accessControl.org.positions.noListDescription')"
            />

            <div class="mt-3 pt-2">
              <UiPagination
                v-model:page="positionsPagination.currentPage.value"
                :page-count="positionsPagination.pageCount.value"
                :previous-label="t('accessControl.common.pagination.previous')"
                :next-label="t('accessControl.common.pagination.next')"
                :summary-label="t('accessControl.common.pagination.summary', { count: positionsPagination.totalItems.value })"
              />
            </div>
          </UiPanelFrame>
        </template>

        <template #detail>
          <div v-if="selectedPosition" class="space-y-4">
            <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
              <div class="flex flex-wrap items-center gap-2">
                <div class="text-sm font-semibold text-foreground">{{ selectedPosition.name }}</div>
                <UiBadge :label="getStatusLabel(t, selectedPosition.status)" subtle />
              </div>
              <div class="mt-2 text-xs text-muted-foreground">{{ selectedPosition.code }}</div>
            </div>

            <div class="grid gap-3 md:grid-cols-2">
              <UiField :label="t('accessControl.org.positions.fields.code')">
                <UiInput v-model="positionEditForm.code" />
              </UiField>
              <UiField :label="t('accessControl.org.positions.fields.name')">
                <UiInput v-model="positionEditForm.name" />
              </UiField>
              <UiField :label="t('accessControl.org.positions.fields.status')">
                <UiSelect v-model="positionEditForm.status" :options="statusOptions" />
              </UiField>
            </div>

            <div class="flex flex-wrap justify-between gap-2">
              <UiButton variant="ghost" class="text-destructive" @click="handleDeletePosition">
                {{ t('accessControl.org.positions.delete') }}
              </UiButton>
              <UiButton :loading="savingPosition" @click="handleSavePosition">
                {{ t('accessControl.org.positions.save') }}
              </UiButton>
            </div>
          </div>
        </template>
      </UiListDetailWorkspace>

      <UiListDetailWorkspace
        v-else
        :has-selection="Boolean(selectedGroup)"
        :detail-title="selectedGroup ? selectedGroup.name : ''"
        :detail-subtitle="t('accessControl.org.groups.detailSubtitle')"
        :empty-detail-title="t('accessControl.org.groups.emptyTitle')"
        :empty-detail-description="t('accessControl.org.groups.emptyDescription')"
      >
        <template #toolbar>
          <UiToolbarRow test-id="access-control-org-groups-toolbar">
            <template #search>
              <UiInput v-model="groupsQuery" :placeholder="t('accessControl.org.groups.toolbarSearch')" />
            </template>
            <template #actions>
              <span
                v-if="groupSelection.hasSelection.value"
                class="text-xs text-text-secondary"
              >
                {{ t('accessControl.common.selection.selectedCount', { count: groupSelection.selectedCount.value }) }}
              </span>
              <UiButton
                v-if="groupsPagination.pagedItems.value.length"
                variant="ghost"
                size="sm"
                @click="toggleAllVisibleGroups(!allVisibleGroupsSelected)"
              >
                {{ t('accessControl.common.selection.selectPage') }}
              </UiButton>
              <UiButton
                v-if="groupSelection.hasSelection.value"
                variant="ghost"
                size="sm"
                @click="groupSelection.clearSelection"
              >
                {{ t('accessControl.common.selection.clear') }}
              </UiButton>
              <UiButton
                v-if="groupSelection.hasSelection.value"
                variant="destructive"
                size="sm"
                data-testid="access-control-org-groups-bulk-delete-button"
                @click="bulkDeleteGroupsDialogOpen = true"
              >
                {{ t('accessControl.common.bulk.delete') }}
              </UiButton>
              <UiButton size="sm" @click="openCreateGroupDialog">
                {{ t('accessControl.org.groups.create') }}
              </UiButton>
            </template>
          </UiToolbarRow>
        </template>

        <template #list>
          <UiPanelFrame
            variant="panel"
            padding="md"
            :title="t('accessControl.org.groups.listTitle')"
            :subtitle="t('accessControl.common.list.totalGroups', { count: groupsPagination.totalItems.value })"
          >
            <div v-if="groupsPagination.pagedItems.value.length" class="space-y-2">
            <UiRecordCard
              v-for="group in groupsPagination.pagedItems.value"
              :key="group.id"
              layout="compact"
                interactive
                :active="selectedGroupId === group.id"
                :title="group.name"
              :description="group.code"
                :test-id="`access-control-org-group-record-${group.id}`"
                @click="selectGroup(group.id)"
              >
                <template #badges>
                  <div class="flex items-center gap-2" @click.stop>
                    <UiCheckbox
                      :model-value="groupSelection.isSelected(group.id)"
                      :data-testid="`access-control-org-group-select-${group.id}`"
                      @update:model-value="toggleGroupSelection(group.id, Boolean($event))"
                    />
                    <UiSwitch
                      :model-value="listStatus(groupStatusOverrides, group.id, group.status) === 'active'"
                      :disabled="isStatusSaving(togglingGroupIds, group.id)"
                      @update:model-value="toggleGroupStatus(group, $event)"
                    >
                      <span class="sr-only">
                        {{ t('accessControl.org.groups.list.toggleStatus', { name: group.name }) }}
                      </span>
                    </UiSwitch>
                  </div>
                </template>
              </UiRecordCard>
            </div>
            <UiEmptyState
              v-else
              :title="t('accessControl.org.groups.noListTitle')"
              :description="t('accessControl.org.groups.noListDescription')"
            />

            <div class="mt-3 pt-2">
              <UiPagination
                v-model:page="groupsPagination.currentPage.value"
                :page-count="groupsPagination.pageCount.value"
                :previous-label="t('accessControl.common.pagination.previous')"
                :next-label="t('accessControl.common.pagination.next')"
                :summary-label="t('accessControl.common.pagination.summary', { count: groupsPagination.totalItems.value })"
              />
            </div>
          </UiPanelFrame>
        </template>

        <template #detail>
          <div v-if="selectedGroup" class="space-y-4">
            <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
              <div class="flex flex-wrap items-center gap-2">
                <div class="text-sm font-semibold text-foreground">{{ selectedGroup.name }}</div>
                <UiBadge :label="getStatusLabel(t, selectedGroup.status)" subtle />
              </div>
              <div class="mt-2 text-xs text-muted-foreground">{{ selectedGroup.code }}</div>
            </div>

            <div class="grid gap-3 md:grid-cols-2">
              <UiField :label="t('accessControl.org.groups.fields.code')">
                <UiInput v-model="groupEditForm.code" />
              </UiField>
              <UiField :label="t('accessControl.org.groups.fields.name')">
                <UiInput v-model="groupEditForm.name" />
              </UiField>
              <UiField :label="t('accessControl.org.groups.fields.status')">
                <UiSelect v-model="groupEditForm.status" :options="statusOptions" />
              </UiField>
            </div>

            <div class="flex flex-wrap justify-between gap-2">
              <UiButton variant="ghost" class="text-destructive" @click="handleDeleteGroup">
                {{ t('accessControl.org.groups.delete') }}
              </UiButton>
              <UiButton :loading="savingGroup" @click="handleSaveGroup">
                {{ t('accessControl.org.groups.save') }}
              </UiButton>
            </div>
          </div>
        </template>
      </UiListDetailWorkspace>
    </div>

    <UiListDetailWorkspace
      v-else
      :has-selection="Boolean(selectedAssignment)"
      :detail-title="selectedAssignment ? assignmentTitle(selectedAssignment) : ''"
      :detail-subtitle="t('accessControl.org.assignments.detailSubtitle')"
      :empty-detail-title="t('accessControl.org.assignments.emptyTitle')"
      :empty-detail-description="t('accessControl.org.assignments.emptyDescription')"
    >
      <template #toolbar>
        <UiToolbarRow test-id="access-control-org-assignments-toolbar">
          <template #search>
            <UiInput v-model="assignmentsQuery" :placeholder="t('accessControl.org.assignments.toolbarSearch')" />
          </template>
          <template #actions>
            <span
              v-if="assignmentSelection.hasSelection.value"
              class="text-xs text-text-secondary"
            >
              {{ t('accessControl.common.selection.selectedCount', { count: assignmentSelection.selectedCount.value }) }}
            </span>
            <UiButton
              v-if="assignmentsPagination.pagedItems.value.length"
              variant="ghost"
              size="sm"
              @click="toggleAllVisibleAssignments(!allVisibleAssignmentsSelected)"
            >
              {{ t('accessControl.common.selection.selectPage') }}
            </UiButton>
            <UiButton
              v-if="assignmentSelection.hasSelection.value"
              variant="ghost"
              size="sm"
              @click="assignmentSelection.clearSelection"
            >
              {{ t('accessControl.common.selection.clear') }}
            </UiButton>
            <UiButton
              v-if="assignmentSelection.hasSelection.value"
              variant="destructive"
              size="sm"
              data-testid="access-control-org-assignments-bulk-delete-button"
              @click="bulkDeleteAssignmentsDialogOpen = true"
            >
              {{ t('accessControl.common.bulk.delete') }}
            </UiButton>
            <UiButton size="sm" @click="openCreateAssignmentDialog">
              {{ t('accessControl.org.assignments.create') }}
            </UiButton>
          </template>
        </UiToolbarRow>
      </template>

      <template #list>
        <UiPanelFrame
          variant="panel"
          padding="md"
          :title="t('accessControl.org.assignments.listTitle')"
          :subtitle="t('accessControl.common.list.totalAssignments', { count: assignmentsPagination.totalItems.value })"
        >
          <div v-if="assignmentsPagination.pagedItems.value.length" class="space-y-2">
            <UiRecordCard
              v-for="assignment in assignmentsPagination.pagedItems.value"
              :key="assignmentKey(assignment)"
              layout="compact"
              interactive
              :active="selectedAssignmentKey === assignmentKey(assignment)"
              :title="assignmentTitle(assignment)"
              :description="assignmentOrgLabel(assignment)"
              :test-id="`access-control-org-assignment-record-${assignmentKey(assignment)}`"
              @click="selectAssignment(assignment)"
            >
              <template #secondary>
                <UiBadge
                  :label="assignment.isPrimary ? t('accessControl.common.list.primaryAssignment') : t('accessControl.common.list.secondaryAssignment')"
                  :tone="assignment.isPrimary ? 'success' : 'default'"
                  subtle
                />
              </template>
              <template #badges>
                <UiCheckbox
                  :model-value="assignmentSelection.isSelected(assignmentKey(assignment))"
                  :data-testid="`access-control-org-assignment-select-${assignmentKey(assignment)}`"
                  @click.stop
                  @update:model-value="toggleAssignmentSelection(assignment, Boolean($event))"
                />
              </template>
              <template #meta>
                <span class="truncate text-xs text-text-secondary">
                  {{ positionSummary(assignment) }}
                </span>
              </template>
            </UiRecordCard>
          </div>
          <UiEmptyState
            v-else
            :title="t('accessControl.org.assignments.noListTitle')"
            :description="t('accessControl.org.assignments.noListDescription')"
          />

          <div class="mt-3 pt-2">
            <UiPagination
              v-model:page="assignmentsPagination.currentPage.value"
              :page-count="assignmentsPagination.pageCount.value"
              :previous-label="t('accessControl.common.pagination.previous')"
              :next-label="t('accessControl.common.pagination.next')"
              :summary-label="t('accessControl.common.pagination.summary', { count: assignmentsPagination.totalItems.value })"
            />
          </div>
        </UiPanelFrame>
      </template>

      <template #detail>
        <div v-if="selectedAssignment" class="space-y-4">
          <div class="rounded-[var(--radius-l)] border border-border bg-muted/35 p-4">
            <div class="flex flex-wrap items-center gap-2">
              <div class="text-sm font-semibold text-foreground">{{ assignmentTitle(selectedAssignment) }}</div>
              <UiBadge
                :label="selectedAssignment.isPrimary ? t('accessControl.common.list.primaryAssignment') : t('accessControl.common.list.secondaryAssignment')"
                :tone="selectedAssignment.isPrimary ? 'success' : 'default'"
                subtle
              />
            </div>
            <div class="mt-2 text-xs text-muted-foreground">{{ assignmentOrgLabel(selectedAssignment) }}</div>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiSurface
              data-testid="access-control-org-assignment-summary-positions"
              variant="subtle"
              padding="md"
            >
              <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">
                {{ t('accessControl.org.assignments.summaryPositions') }}
              </div>
              <div class="mt-2 text-sm text-foreground">{{ positionSummary(selectedAssignment) }}</div>
            </UiSurface>
            <UiSurface
              data-testid="access-control-org-assignment-summary-groups"
              variant="subtle"
              padding="md"
            >
              <div class="text-xs font-semibold uppercase tracking-[0.08em] text-text-tertiary">
                {{ t('accessControl.org.assignments.summaryGroups') }}
              </div>
              <div class="mt-2 text-sm text-foreground">{{ groupSummary(selectedAssignment) }}</div>
            </UiSurface>
          </div>

          <div class="grid gap-3 md:grid-cols-2">
            <UiField :label="t('accessControl.org.assignments.fields.user')">
              <UiSelect v-model="assignmentEditForm.userId" :options="userOptions" />
            </UiField>
            <UiField :label="t('accessControl.org.assignments.fields.orgUnit')">
              <UiSelect v-model="assignmentEditForm.orgUnitId" :options="orgUnitOptions" />
            </UiField>
          </div>

          <UiField :label="t('accessControl.org.assignments.fields.positions')">
            <div class="grid gap-2 rounded-[var(--radius-m)] border border-border bg-muted/35 p-3">
              <UiCheckbox
                v-for="position in accessControlStore.positions"
                :key="position.id"
                v-model="assignmentEditForm.positionIds"
                :value="position.id"
              >
                {{ position.name }}
              </UiCheckbox>
            </div>
          </UiField>

          <UiField :label="t('accessControl.org.assignments.fields.groups')">
            <div class="grid gap-2 rounded-[var(--radius-m)] border border-border bg-muted/35 p-3">
              <UiCheckbox
                v-for="group in accessControlStore.userGroups"
                :key="group.id"
                v-model="assignmentEditForm.userGroupIds"
                :value="group.id"
              >
                {{ group.name }}
              </UiCheckbox>
            </div>
          </UiField>

          <div class="rounded-[var(--radius-m)] border border-border bg-muted/35 p-3">
            <UiCheckbox v-model="assignmentEditForm.isPrimary">
              {{ t('accessControl.org.assignments.fields.isPrimary') }}
            </UiCheckbox>
          </div>

          <div class="flex flex-wrap justify-between gap-2">
            <UiButton variant="ghost" class="text-destructive" @click="handleDeleteAssignment">
              {{ t('accessControl.org.assignments.delete') }}
            </UiButton>
            <UiButton :loading="savingAssignment" @click="handleSaveAssignment">
              {{ t('accessControl.org.assignments.save') }}
            </UiButton>
          </div>
        </div>
      </template>
    </UiListDetailWorkspace>

    <UiDialog
      :open="bulkDeleteUnitsDialogOpen"
      :title="t('accessControl.common.bulk.dialogTitle', { entity: t('accessControl.common.entities.units') })"
      :description="t('accessControl.common.bulk.dialogDescription')"
      @update:open="bulkDeleteUnitsDialogOpen = $event"
    >
      <p class="text-sm text-text-secondary">
        {{ t('accessControl.common.bulk.dialogConfirm', {
          count: selectedUnitsForDelete.length,
          entity: t('accessControl.common.entities.units'),
        }) }}
      </p>

      <template #footer>
        <UiButton variant="ghost" @click="bulkDeleteUnitsDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton
          variant="destructive"
          :loading="deletingSelectedUnits"
          @click="handleBulkDeleteUnits"
        >
          {{ t('accessControl.common.bulk.delete') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="bulkDeletePositionsDialogOpen"
      :title="t('accessControl.common.bulk.dialogTitle', { entity: t('accessControl.common.entities.positions') })"
      :description="t('accessControl.common.bulk.dialogDescription')"
      @update:open="bulkDeletePositionsDialogOpen = $event"
    >
      <p class="text-sm text-text-secondary">
        {{ t('accessControl.common.bulk.dialogConfirm', {
          count: selectedPositionsForDelete.length,
          entity: t('accessControl.common.entities.positions'),
        }) }}
      </p>

      <template #footer>
        <UiButton variant="ghost" @click="bulkDeletePositionsDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton
          variant="destructive"
          :loading="deletingSelectedPositions"
          data-testid="access-control-org-positions-bulk-delete-confirm"
          @click="handleBulkDeletePositions"
        >
          {{ t('accessControl.common.bulk.delete') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="bulkDeleteGroupsDialogOpen"
      :title="t('accessControl.common.bulk.dialogTitle', { entity: t('accessControl.common.entities.groups') })"
      :description="t('accessControl.common.bulk.dialogDescription')"
      @update:open="bulkDeleteGroupsDialogOpen = $event"
    >
      <p class="text-sm text-text-secondary">
        {{ t('accessControl.common.bulk.dialogConfirm', {
          count: selectedGroupsForDelete.length,
          entity: t('accessControl.common.entities.groups'),
        }) }}
      </p>

      <template #footer>
        <UiButton variant="ghost" @click="bulkDeleteGroupsDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton
          variant="destructive"
          :loading="deletingSelectedGroups"
          data-testid="access-control-org-groups-bulk-delete-confirm"
          @click="handleBulkDeleteGroups"
        >
          {{ t('accessControl.common.bulk.delete') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="bulkDeleteAssignmentsDialogOpen"
      :title="t('accessControl.common.bulk.dialogTitle', { entity: t('accessControl.common.entities.assignments') })"
      :description="t('accessControl.common.bulk.dialogDescription')"
      @update:open="bulkDeleteAssignmentsDialogOpen = $event"
    >
      <p class="text-sm text-text-secondary">
        {{ t('accessControl.common.bulk.dialogConfirm', {
          count: selectedAssignmentsForDelete.length,
          entity: t('accessControl.common.entities.assignments'),
        }) }}
      </p>

      <template #footer>
        <UiButton variant="ghost" @click="bulkDeleteAssignmentsDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton
          variant="destructive"
          :loading="deletingSelectedAssignments"
          data-testid="access-control-org-assignments-bulk-delete-confirm"
          @click="handleBulkDeleteAssignments"
        >
          {{ t('accessControl.common.bulk.delete') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="createUnitDialogOpen"
      :title="t('accessControl.org.units.dialogTitle')"
      :description="t('accessControl.org.units.dialogDescription')"
      @update:open="createUnitDialogOpen = $event"
    >
      <div class="grid gap-3 md:grid-cols-2">
        <UiField :label="t('accessControl.org.units.fields.code')">
          <UiInput v-model="unitCreateForm.code" />
        </UiField>
        <UiField :label="t('accessControl.org.units.fields.name')">
          <UiInput v-model="unitCreateForm.name" />
        </UiField>
        <UiField :label="t('accessControl.org.units.fields.parent')">
          <UiSelect v-model="unitCreateForm.parentId" :options="parentOptions" />
        </UiField>
        <UiField :label="t('accessControl.org.units.fields.status')">
          <UiSelect v-model="unitCreateForm.status" :options="statusOptions" />
        </UiField>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="createUnitDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton :loading="savingUnit" @click="handleCreateUnit">
          {{ t('accessControl.org.units.createAction') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="createPositionDialogOpen"
      :title="t('accessControl.org.positions.dialogTitle')"
      :description="t('accessControl.org.positions.dialogDescription')"
      @update:open="createPositionDialogOpen = $event"
    >
      <div class="grid gap-3 md:grid-cols-2">
        <UiField :label="t('accessControl.org.positions.fields.code')">
          <UiInput v-model="positionCreateForm.code" />
        </UiField>
        <UiField :label="t('accessControl.org.positions.fields.name')">
          <UiInput v-model="positionCreateForm.name" />
        </UiField>
        <UiField :label="t('accessControl.org.positions.fields.status')">
          <UiSelect v-model="positionCreateForm.status" :options="statusOptions" />
        </UiField>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="createPositionDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton :loading="savingPosition" @click="handleCreatePosition">
          {{ t('accessControl.org.positions.createAction') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="createGroupDialogOpen"
      :title="t('accessControl.org.groups.dialogTitle')"
      :description="t('accessControl.org.groups.dialogDescription')"
      @update:open="createGroupDialogOpen = $event"
    >
      <div class="grid gap-3 md:grid-cols-2">
        <UiField :label="t('accessControl.org.groups.fields.code')">
          <UiInput v-model="groupCreateForm.code" />
        </UiField>
        <UiField :label="t('accessControl.org.groups.fields.name')">
          <UiInput v-model="groupCreateForm.name" />
        </UiField>
        <UiField :label="t('accessControl.org.groups.fields.status')">
          <UiSelect v-model="groupCreateForm.status" :options="statusOptions" />
        </UiField>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="createGroupDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton :loading="savingGroup" @click="handleCreateGroup">
          {{ t('accessControl.org.groups.createAction') }}
        </UiButton>
      </template>
    </UiDialog>

    <UiDialog
      :open="createAssignmentDialogOpen"
      :title="t('accessControl.org.assignments.dialogTitle')"
      :description="t('accessControl.org.assignments.dialogDescription')"
      @update:open="createAssignmentDialogOpen = $event"
    >
      <div class="space-y-4">
        <div class="grid gap-3 md:grid-cols-2">
          <UiField :label="t('accessControl.org.assignments.fields.user')">
            <UiSelect v-model="assignmentCreateForm.userId" :options="userOptions" />
          </UiField>
          <UiField :label="t('accessControl.org.assignments.fields.orgUnit')">
            <UiSelect v-model="assignmentCreateForm.orgUnitId" :options="orgUnitOptions" />
          </UiField>
        </div>

        <UiField :label="t('accessControl.org.assignments.fields.positions')">
          <div class="grid gap-2 rounded-[var(--radius-m)] border border-border bg-muted/35 p-3">
            <UiCheckbox
              v-for="position in accessControlStore.positions"
              :key="position.id"
              v-model="assignmentCreateForm.positionIds"
              :value="position.id"
            >
              {{ position.name }}
            </UiCheckbox>
          </div>
        </UiField>

        <UiField :label="t('accessControl.org.assignments.fields.groups')">
          <div class="grid gap-2 rounded-[var(--radius-m)] border border-border bg-muted/35 p-3">
            <UiCheckbox
              v-for="group in accessControlStore.userGroups"
              :key="group.id"
              v-model="assignmentCreateForm.userGroupIds"
              :value="group.id"
            >
              {{ group.name }}
            </UiCheckbox>
          </div>
        </UiField>

        <div class="rounded-[var(--radius-m)] border border-border bg-muted/35 p-3">
          <UiCheckbox v-model="assignmentCreateForm.isPrimary">
            {{ t('accessControl.org.assignments.fields.isPrimary') }}
          </UiCheckbox>
        </div>
      </div>

      <template #footer>
        <UiButton variant="ghost" @click="createAssignmentDialogOpen = false">
          {{ t('common.cancel') }}
        </UiButton>
        <UiButton :loading="savingAssignment" @click="handleCreateAssignment">
          {{ t('accessControl.org.assignments.createAction') }}
        </UiButton>
      </template>
    </UiDialog>
  </div>
</template>
