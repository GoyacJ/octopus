import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

import type {
  ModelCatalogSnapshot,
  ToolRecord,
  WorkspaceMcpServerDocument,
  WorkspaceSkillDocument,
  WorkspaceSkillFileDocument,
  WorkspaceSkillTreeDocument,
  WorkspaceToolCatalogSnapshot,
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
  const toolCatalogsByConnection = ref<Record<string, WorkspaceToolCatalogSnapshot>>({})
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
    toolCatalogsByConnection,
    toolsByConnection,
  })
  const actions = createCatalogActions({
    snapshots,
    toolCatalogsByConnection,
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
