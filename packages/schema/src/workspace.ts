import type {
  CreateProjectRequest as OpenApiCreateProjectRequest,
  ProjectAgentAssignments as OpenApiProjectAgentAssignments,
  ProjectModelAssignments as OpenApiProjectModelAssignments,
  ProjectRecord as OpenApiProjectRecord,
  ProjectToolAssignments as OpenApiProjectToolAssignments,
  UpdateProjectRequest as OpenApiUpdateProjectRequest,
  ProjectWorkspaceAssignments as OpenApiProjectWorkspaceAssignments,
  SystemBootstrapStatus as OpenApiSystemBootstrapStatus,
  WorkspaceSummary as OpenApiWorkspaceSummary,
} from './generated'

export type WorkspaceSummary = OpenApiWorkspaceSummary
export type ProjectModelAssignments = OpenApiProjectModelAssignments
export type ProjectToolAssignments = OpenApiProjectToolAssignments
export type ProjectAgentAssignments = OpenApiProjectAgentAssignments
export type ProjectWorkspaceAssignments = OpenApiProjectWorkspaceAssignments
export type ProjectRecord = OpenApiProjectRecord
export type CreateProjectRequest = OpenApiCreateProjectRequest
export type UpdateProjectRequest = OpenApiUpdateProjectRequest

export type SystemBootstrapStatus = OpenApiSystemBootstrapStatus
