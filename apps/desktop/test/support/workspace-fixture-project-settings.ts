import type { ProjectSettingsConfig } from '@octopus/schema'

import type { WorkspaceFixtureState } from './workspace-fixture-state'

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

function isRecord(value: unknown): value is Record<string, any> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

export function updateFixtureProjectSettings(
  state: WorkspaceFixtureState,
  projectId: string,
  updater: (current: ProjectSettingsConfig) => ProjectSettingsConfig,
) {
  const projectConfig = state.runtimeProjectConfigs[projectId]
  if (!projectConfig) {
    throw new Error(`Expected runtime config for project ${projectId}`)
  }

  const projectSource = projectConfig.sources.find(source => source.scope === 'project')
  if (!projectSource) {
    throw new Error(`Expected project runtime source for project ${projectId}`)
  }

  const sourceDocument = isRecord(projectSource.document) ? clone(projectSource.document) : {}
  const currentProjectSettings = isRecord(sourceDocument.projectSettings)
    ? clone(sourceDocument.projectSettings)
    : {}
  const nextProjectSettings = updater(currentProjectSettings as ProjectSettingsConfig)

  state.runtimeProjectConfigs[projectId] = {
    ...projectConfig,
    effectiveConfig: {
      ...((projectConfig.effectiveConfig as Record<string, any>) ?? {}),
      projectSettings: clone(nextProjectSettings),
    },
    sources: projectConfig.sources.map(source => (
      source.scope === 'project'
        ? {
            ...source,
            document: {
              ...sourceDocument,
              approvals: isRecord(sourceDocument.approvals)
                ? sourceDocument.approvals
                : { defaultMode: 'manual' },
              projectSettings: clone(nextProjectSettings),
            },
          }
        : source
    )),
  }
}
