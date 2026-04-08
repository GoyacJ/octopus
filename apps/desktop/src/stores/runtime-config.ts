import type {
  JsonValue,
  RuntimeConfigPatch,
  RuntimeConfigScope,
  RuntimeConfigValidationResult,
  RuntimeEffectiveConfig,
} from '@octopus/schema'

export type RuntimeConfigDrafts = Record<RuntimeConfigScope, string>
export type RuntimeConfigValidationState = Record<RuntimeConfigScope, RuntimeConfigValidationResult | null>

export function createRuntimeConfigDrafts(): RuntimeConfigDrafts {
  return {
    workspace: '{}',
    project: '{}',
    user: '{}',
  }
}

export function createRuntimeConfigValidationState(): RuntimeConfigValidationState {
  return {
    workspace: null,
    project: null,
    user: null,
  }
}

export function stringifyRuntimeConfigDocument(document?: Record<string, JsonValue>): string {
  return JSON.stringify(document ?? {}, null, 2)
}

export function createRuntimeConfigDraftsFromConfig(config: RuntimeEffectiveConfig | null): RuntimeConfigDrafts {
  const drafts = createRuntimeConfigDrafts()
  if (!config) {
    return drafts
  }

  for (const source of config.sources) {
    drafts[source.scope] = stringifyRuntimeConfigDocument(source.document)
  }

  return drafts
}

export function parseRuntimeConfigDraft(scope: RuntimeConfigScope, draft: string): RuntimeConfigPatch {
  const trimmed = draft.trim()
  const rawValue = trimmed.length ? trimmed : '{}'
  const parsed = JSON.parse(rawValue) as JsonValue

  if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object') {
    throw new Error(`Runtime config for ${scope} must be a JSON object`)
  }

  return {
    scope,
    patch: parsed as Record<string, JsonValue>,
  }
}
