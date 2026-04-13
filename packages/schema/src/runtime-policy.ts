import type {
  RuntimeEventKind as OpenApiRuntimeEventKind,
  RuntimePolicySnapshot as OpenApiRuntimePolicySnapshot,
  RuntimeUsageSummary as OpenApiRuntimeUsageSummary,
} from './generated'
import type { DecisionAction } from './shared'

export type RuntimeEventKind = OpenApiRuntimeEventKind
export type RuntimeSessionPolicySnapshot = OpenApiRuntimePolicySnapshot
export type RuntimeUsageSummary = OpenApiRuntimeUsageSummary
export type RuntimeDecisionAction = DecisionAction
