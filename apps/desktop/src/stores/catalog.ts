import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  CapabilityManagementProjection,
  ModelCatalogSnapshot,
  ToolRecord,
  WorkspaceMcpServerDocument,
  WorkspaceSkillDocument,
  WorkspaceSkillFileDocument,
  WorkspaceSkillTreeDocument,
} from '@octopus/schema'

import {
  activeWorkspaceConnectionId,
} from './workspace-scope'
import { createCatalogActions } from './catalog_actions'
import { createCatalogFilters } from './catalog_filters'

export type {
  CatalogConfiguredModelOption,
  CatalogConfiguredModelRow,
  CatalogCredentialSummary,
  CatalogDefaultSelectionRow,
  CatalogDiagnosticSummary,
  CatalogFilterOption,
  CatalogModelRow,
  CatalogProviderSummary,
} from './catalog_normalizers'

export const useCatalogStore = defineStore('catalog', () => {
  const snapshots = ref<Record<string, ModelCatalogSnapshot>>({})
  const managementProjectionsByConnection = ref<Record<string, CapabilityManagementProjection>>({})
  const skillDocumentsByConnection = ref<Record<string, Record<string, WorkspaceSkillDocument>>>({})
  const skillTreesByConnection = ref<Record<string, Record<string, WorkspaceSkillTreeDocument>>>({})
  const skillFilesByConnection = ref<Record<string, Record<string, WorkspaceSkillFileDocument>>>({})
  const mcpDocumentsByConnection = ref<Record<string, Record<string, WorkspaceMcpServerDocument>>>({})
  const toolsByConnection = ref<Record<string, ToolRecord[]>>({})
  const requestTokens = ref<Record<string, number>>({})
  const errors = ref<Record<string, string>>({})

  const activeConnectionId = computed(() => activeWorkspaceConnectionId())
  const error = computed(() => errors.value[activeConnectionId.value] ?? '')
  const filters = createCatalogFilters({
    activeConnectionId,
    snapshots,
    managementProjectionsByConnection,
    toolsByConnection,
  })
  const actions = createCatalogActions({
    snapshots,
    managementProjectionsByConnection,
    skillDocumentsByConnection,
    skillTreesByConnection,
    skillFilesByConnection,
    mcpDocumentsByConnection,
    toolsByConnection,
    requestTokens,
    errors,
  })

  return {
    error,
    ...filters,
    ...actions,
  }
})
