import type {
  ArtifactVersionReference as OpenApiArtifactVersionReference,
  CreateDeliverableVersionInput as OpenApiCreateDeliverableVersionInput,
  DeliverableDetail as OpenApiDeliverableDetail,
  DeliverableSummary as OpenApiDeliverableSummary,
  DeliverableVersionContent as OpenApiDeliverableVersionContent,
  DeliverableVersionSummary as OpenApiDeliverableVersionSummary,
  ForkDeliverableInput as OpenApiForkDeliverableInput,
  PromoteDeliverableInput as OpenApiPromoteDeliverableInput,
} from './generated'

export type ArtifactVersionReference = OpenApiArtifactVersionReference
export type DeliverableSummary = OpenApiDeliverableSummary
export type DeliverableDetail = OpenApiDeliverableDetail
export type DeliverableVersionSummary = OpenApiDeliverableVersionSummary
export type DeliverableVersionContent = OpenApiDeliverableVersionContent
export type CreateDeliverableVersionInput = OpenApiCreateDeliverableVersionInput
export type PromoteDeliverableInput = OpenApiPromoteDeliverableInput
export type ForkDeliverableInput = OpenApiForkDeliverableInput
