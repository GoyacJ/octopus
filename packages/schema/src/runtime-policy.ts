import type {
  RuntimeAuthChallengeSummary as OpenApiRuntimeAuthChallengeSummary,
  RuntimeAuthStateSummary as OpenApiRuntimeAuthStateSummary,
  RuntimeEventKind as OpenApiRuntimeEventKind,
  RuntimeMediationOutcome as OpenApiRuntimeMediationOutcome,
  RuntimePolicyDecisionSummary as OpenApiRuntimePolicyDecisionSummary,
  RuntimePolicySnapshot as OpenApiRuntimePolicySnapshot,
  RuntimeUsageSummary as OpenApiRuntimeUsageSummary,
} from './generated'
import type { DecisionAction } from './shared'

export type RuntimeEventKind = OpenApiRuntimeEventKind
export type RuntimeMediationOutcome = OpenApiRuntimeMediationOutcome
export type RuntimeAuthChallengeSummary = OpenApiRuntimeAuthChallengeSummary
export type RuntimeAuthStateSummary = OpenApiRuntimeAuthStateSummary
export type RuntimePolicyDecisionSummary = OpenApiRuntimePolicyDecisionSummary
export type RuntimeSessionPolicySnapshot = OpenApiRuntimePolicySnapshot
export type RuntimeUsageSummary = OpenApiRuntimeUsageSummary
export type RuntimeDecisionAction = DecisionAction
