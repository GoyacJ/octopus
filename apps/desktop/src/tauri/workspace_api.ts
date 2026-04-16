import type {
  AccessExperienceResponse,
  AccessMemberSummary,
  AccessAuditListResponse,
  AccessAuditQuery,
  AccessSessionRecord,
  AccessRoleRecord,
  AccessUserRecord,
  AccessUserPresetUpdateRequest,
  AgentRecord,
  AutomationRecord,
  AuthorizationSnapshot,
  CapabilityAssetDisablePatch,
  CapabilityManagementProjection,
  ChangeCurrentUserPasswordResponse,
  CreateProjectPromotionRequestInput,
  CredentialBinding,
  DataPolicyRecord,
  EnterpriseAuthSuccess,
  EnterpriseSessionSummary,
  ExportWorkspaceAgentBundleResult,
  FeatureDefinition,
  ImportWorkspaceAgentBundlePreview,
  ImportWorkspaceAgentBundleResult,
  MenuDefinition,
  MenuGateResult,
  MenuPolicyRecord,
  ModelCatalogSnapshot,
  OrgUnitRecord,
  PermissionDefinition,
  PositionRecord,
  ProjectAgentLinkRecord,
  ProjectPromotionRequest,
  ProjectTeamLinkRecord,
  ProtectedResourceDescriptor,
  ProtectedResourceMetadataUpsertRequest,
  ResourcePolicyRecord,
  ReviewProjectPromotionRequestInput,
  RoleBindingRecord,
  SystemAuthStatus,
  ToolRecord,
  UserGroupRecord,
  UserOrgAssignmentRecord,
  WorkspaceMcpServerDocument,
  WorkspaceSkillDocument,
  WorkspaceSkillFileDocument,
  WorkspaceSkillTreeDocument,
} from '@octopus/schema'

import { fetchWorkspaceOpenApi } from './shared'
import type { WorkspaceClient, WorkspaceClientContext } from './workspace-client'

export function assertWorkspaceConnectionReady(context: WorkspaceClientContext): void {
  if (!context.connection.baseUrl || context.connection.status !== 'connected') {
    throw new Error(`Workspace connection ${context.connection.workspaceConnectionId} is unavailable`)
  }
}

export function assertWorkspaceRequestReady(context: WorkspaceClientContext) {
  assertWorkspaceConnectionReady(context)
  if (!context.session?.token) {
    throw new Error(`Workspace session is unavailable for ${context.connection.workspaceConnectionId}`)
  }

  return context.session
}

export function createWorkspaceApi(context: WorkspaceClientContext): Omit<WorkspaceClient, 'runtime'> {
  return {
    system: {
      async bootstrap() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/system/bootstrap',
          'get',
          {
            workspace: context.connection,
          },
        )
      },
    },
    enterpriseAuth: {
      async getStatus() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/system/auth/status',
          'get',
          {
            session: context.session,
          },
        ) as SystemAuthStatus
      },
      async login(input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/system/auth/login',
          'post',
          {
            body: JSON.stringify(input),
          },
        ) as EnterpriseAuthSuccess
      },
      async bootstrapAdmin(input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/system/auth/bootstrap-admin',
          'post',
          {
            body: JSON.stringify(input),
          },
        ) as EnterpriseAuthSuccess
      },
      async session() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/system/auth/session',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        ) as EnterpriseSessionSummary
      },
    },
    workspace: {
      async get() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async getOverview() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/overview', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async listPromotionRequests() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/promotion-requests',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        ) as ProjectPromotionRequest[]
      },
      async reviewPromotionRequest(requestId, input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/promotion-requests/{requestId}/review',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(input satisfies ReviewProjectPromotionRequestInput),
            pathParams: {
              requestId,
            },
          },
        ) as ProjectPromotionRequest
      },
    },
    projects: {
      async list() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/projects', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async create(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/projects', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        })
      },
      async update(projectId, input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/projects/{projectId}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
          pathParams: {
            projectId,
          },
        })
      },
      async getDashboard(projectId) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/dashboard',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId,
            },
          },
        )
      },
      async listPromotionRequests(projectId) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/promotion-requests',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId,
            },
          },
        ) as ProjectPromotionRequest[]
      },
      async createPromotionRequest(projectId, input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/promotion-requests',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(input satisfies CreateProjectPromotionRequestInput),
            pathParams: {
              projectId,
            },
          },
        ) as ProjectPromotionRequest
      },
    },
    resources: {
      async listWorkspace() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/resources', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async listProject(projectId) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/resources',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId,
            },
          },
        )
      },
      async createWorkspace(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/resources', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        })
      },
      async createProject(projectId, input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/resources',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(input),
            pathParams: {
              projectId,
            },
          },
        )
      },
      async createProjectFolder(projectId, input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/resources/folder',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(input),
            pathParams: {
              projectId,
            },
          },
        )
      },
      async importWorkspace(input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/resources/import',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(input),
          },
        )
      },
      async importProject(projectId, input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/resources/import',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(input),
            pathParams: {
              projectId,
            },
          },
        )
      },
      async getDetail(resourceId) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/resources/{resourceId}',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              resourceId,
            },
          },
        )
      },
      async getContent(resourceId) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/resources/{resourceId}/content',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              resourceId,
            },
          },
        )
      },
      async listChildren(resourceId) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/resources/{resourceId}/children',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              resourceId,
            },
          },
        )
      },
      async promote(resourceId, input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/resources/{resourceId}/promote',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(input),
            pathParams: {
              resourceId,
            },
          },
        )
      },
      async updateWorkspace(resourceId, input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/resources/{resourceId}',
          'patch',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(input),
            pathParams: {
              resourceId,
            },
          },
        )
      },
      async updateProject(projectId, resourceId, input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/resources/{resourceId}',
          'patch',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(input),
            pathParams: {
              projectId,
              resourceId,
            },
          },
        )
      },
      async deleteWorkspace(resourceId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/resources/{resourceId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            resourceId,
          },
        })
      },
      async deleteProject(projectId, resourceId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/projects/{projectId}/resources/{resourceId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            projectId,
            resourceId,
          },
        })
      },
    },
    filesystem: {
      async listDirectories(path) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/filesystem/directories',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            queryParams: {
              path,
            },
          },
        )
      },
    },
    artifacts: {
      async listWorkspace() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/artifacts', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
    },
    inbox: {
      async list() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/inbox', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
    },
    knowledge: {
      async listWorkspace() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/knowledge', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async listProject(projectId) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/knowledge',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId,
            },
          },
        )
      },
    },
    pet: {
      async getDashboard() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/pet/dashboard', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async getSnapshot(projectId) {
        const session = assertWorkspaceRequestReady(context)
        if (projectId) {
          return await fetchWorkspaceOpenApi(context.connection, '/api/v1/projects/{projectId}/pet', 'get', {
            session,
            pathParams: {
              projectId,
            },
          })
        }
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/pet', 'get', {
          session,
        })
      },
      async savePresence(input, projectId) {
        const session = assertWorkspaceRequestReady(context)
        if (projectId) {
          return await fetchWorkspaceOpenApi(
            context.connection,
            '/api/v1/projects/{projectId}/pet/presence',
            'patch',
            {
              session,
              pathParams: {
                projectId,
              },
              body: JSON.stringify(input),
            },
          )
        }
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/pet/presence', 'patch', {
          session,
          body: JSON.stringify(input),
        })
      },
      async bindConversation(input, projectId) {
        const session = assertWorkspaceRequestReady(context)
        if (projectId) {
          return await fetchWorkspaceOpenApi(
            context.connection,
            '/api/v1/projects/{projectId}/pet/conversation',
            'put',
            {
              session,
              pathParams: {
                projectId,
              },
              body: JSON.stringify(input),
            },
          )
        }
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/pet/conversation', 'put', {
          session,
          body: JSON.stringify(input),
        })
      },
    },
    agents: {
      async list() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/agents', 'get', {
          session: assertWorkspaceRequestReady(context),
        }) as unknown as AgentRecord[]
      },
      async create(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/agents', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        }) as unknown as AgentRecord
      },
      async update(agentId, input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/agents/{agentId}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            agentId,
          },
          body: JSON.stringify(input),
        }) as unknown as AgentRecord
      },
      async delete(agentId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/agents/{agentId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            agentId,
          },
        })
      },
      async copyToWorkspace(agentId) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/agents/{agentId}/copy-to-workspace',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              agentId,
            },
          },
        ) as unknown as ImportWorkspaceAgentBundleResult
      },
      async copyToProject(projectId, agentId) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/agents/{agentId}/copy-to-project',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId,
              agentId,
            },
          },
        ) as unknown as ImportWorkspaceAgentBundleResult
      },
      async previewImportBundle(input, projectId) {
        const session = assertWorkspaceRequestReady(context)
        if (projectId) {
          return await fetchWorkspaceOpenApi(
            context.connection,
            '/api/v1/projects/{projectId}/agents/import-preview',
            'post',
            {
              session,
              pathParams: {
                projectId,
              },
              body: JSON.stringify(input),
            },
          ) as unknown as ImportWorkspaceAgentBundlePreview
        }
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/agents/import-preview',
          'post',
          {
            session,
            body: JSON.stringify(input),
          },
        ) as unknown as ImportWorkspaceAgentBundlePreview
      },
      async importBundle(input, projectId) {
        const session = assertWorkspaceRequestReady(context)
        if (projectId) {
          return await fetchWorkspaceOpenApi(
            context.connection,
            '/api/v1/projects/{projectId}/agents/import',
            'post',
            {
              session,
              pathParams: {
                projectId,
              },
              body: JSON.stringify(input),
            },
          ) as unknown as ImportWorkspaceAgentBundleResult
        }
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/agents/import',
          'post',
          {
            session,
            body: JSON.stringify(input),
          },
        ) as unknown as ImportWorkspaceAgentBundleResult
      },
      async exportBundle(input, projectId) {
        const session = assertWorkspaceRequestReady(context)
        if (projectId) {
          return await fetchWorkspaceOpenApi(
            context.connection,
            '/api/v1/projects/{projectId}/agents/export',
            'post',
            {
              session,
              pathParams: {
                projectId,
              },
              body: JSON.stringify(input),
            },
          ) as unknown as ExportWorkspaceAgentBundleResult
        }
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/agents/export',
          'post',
          {
            session,
            body: JSON.stringify(input),
          },
        ) as unknown as ExportWorkspaceAgentBundleResult
      },
      async listProjectLinks(projectId) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/agent-links',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId,
            },
          },
        ) as unknown as ProjectAgentLinkRecord[]
      },
      async linkProject(input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/agent-links',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId: input.projectId,
            },
            body: JSON.stringify(input),
          },
        ) as unknown as ProjectAgentLinkRecord
      },
      async unlinkProject(projectId, agentId) {
        await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/agent-links/{agentId}',
          'delete',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId,
              agentId,
            },
          },
        )
      },
    },
    teams: {
      async list() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/teams', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async create(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/teams', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        })
      },
      async update(teamId, input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/teams/{teamId}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            teamId,
          },
          body: JSON.stringify(input),
        })
      },
      async delete(teamId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/teams/{teamId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            teamId,
          },
        })
      },
      async copyToWorkspace(teamId) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/teams/{teamId}/copy-to-workspace',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              teamId,
            },
          },
        ) as unknown as ImportWorkspaceAgentBundleResult
      },
      async copyToProject(projectId, teamId) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/teams/{teamId}/copy-to-project',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId,
              teamId,
            },
          },
        ) as unknown as ImportWorkspaceAgentBundleResult
      },
      async listProjectLinks(projectId) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/team-links',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId,
            },
          },
        ) as unknown as ProjectTeamLinkRecord[]
      },
      async linkProject(input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/team-links',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId: input.projectId,
            },
            body: JSON.stringify(input),
          },
        ) as unknown as ProjectTeamLinkRecord
      },
      async unlinkProject(projectId, teamId) {
        await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/projects/{projectId}/team-links/{teamId}',
          'delete',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              projectId,
              teamId,
            },
          },
        )
      },
    },
    catalog: {
      async getSnapshot() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/models', 'get', {
          session: assertWorkspaceRequestReady(context),
        }) as unknown as ModelCatalogSnapshot
      },
      async getManagementProjection() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/catalog/management-projection',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        ) as unknown as CapabilityManagementProjection
      },
      async setAssetDisabled(patch: CapabilityAssetDisablePatch) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/management-projection/disable', 'patch', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(patch),
        }) as unknown as CapabilityManagementProjection
      },
      async getSkill(skillId) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/skills/{skillId}', 'get', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            skillId,
          },
        }) as unknown as WorkspaceSkillDocument
      },
      async getSkillTree(skillId) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/skills/{skillId}/tree', 'get', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            skillId,
          },
        }) as unknown as WorkspaceSkillTreeDocument
      },
      async getSkillFile(skillId, relativePath) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/catalog/skills/{skillId}/files/{relativePath}',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              skillId,
              relativePath,
            },
          },
        ) as WorkspaceSkillFileDocument
      },
      async createSkill(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/skills', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        }) as unknown as WorkspaceSkillDocument
      },
      async updateSkill(skillId, input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/skills/{skillId}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            skillId,
          },
          body: JSON.stringify(input),
        }) as unknown as WorkspaceSkillDocument
      },
      async updateSkillFile(skillId, relativePath, input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/catalog/skills/{skillId}/files/{relativePath}',
          'patch',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              skillId,
              relativePath,
            },
            body: JSON.stringify(input),
          },
        ) as WorkspaceSkillFileDocument
      },
      async importSkillArchive(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/skills/import-archive', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        }) as unknown as WorkspaceSkillDocument
      },
      async importSkillFolder(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/skills/import-folder', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        }) as unknown as WorkspaceSkillDocument
      },
      async copySkillToManaged(skillId, input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/skills/{skillId}/copy-to-managed', 'post', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            skillId,
          },
          body: JSON.stringify(input),
        }) as unknown as WorkspaceSkillDocument
      },
      async deleteSkill(skillId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/skills/{skillId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            skillId,
          },
        })
      },
      async getMcpServer(serverName) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/mcp-servers/{serverName}', 'get', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            serverName,
          },
        }) as unknown as WorkspaceMcpServerDocument
      },
      async copyMcpServerToManaged(serverName) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/catalog/mcp-servers/{serverName}/copy-to-managed',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              serverName,
            },
          },
        ) as unknown as WorkspaceMcpServerDocument
      },
      async createMcpServer(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/mcp-servers', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        }) as unknown as WorkspaceMcpServerDocument
      },
      async updateMcpServer(serverName, input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/mcp-servers/{serverName}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            serverName,
          },
          body: JSON.stringify(input),
        }) as unknown as WorkspaceMcpServerDocument
      },
      async deleteMcpServer(serverName) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/mcp-servers/{serverName}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            serverName,
          },
        })
      },
      async listModels() {
        const snapshot = await this.getSnapshot()
        return snapshot.models
      },
      async listProviderCredentials() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/catalog/provider-credentials',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        ) as unknown as CredentialBinding[]
      },
      async listTools() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/tools', 'get', {
          session: assertWorkspaceRequestReady(context),
        }) as unknown as ToolRecord[]
      },
      async createTool(record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/tools', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(record),
        }) as unknown as ToolRecord
      },
      async updateTool(toolId, record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/tools/{toolId}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            toolId,
          },
          body: JSON.stringify(record),
        }) as unknown as ToolRecord
      },
      async deleteTool(toolId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/tools/{toolId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            toolId,
          },
        })
      },
    },
    automations: {
      async list() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/automations', 'get', {
          session: assertWorkspaceRequestReady(context),
        }) as unknown as AutomationRecord[]
      },
      async create(record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/automations', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(record),
        }) as unknown as AutomationRecord
      },
      async update(automationId, record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/automations/{automationId}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            automationId,
          },
          body: JSON.stringify(record),
        }) as unknown as AutomationRecord
      },
      async delete(automationId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/automations/{automationId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            automationId,
          },
        })
      },
    },
    profile: {
      async updateCurrentUserProfile(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/personal-center/profile', 'patch', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        })
      },
      async changeCurrentUserPassword(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/personal-center/profile/password', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        }) as ChangeCurrentUserPasswordResponse
      },
    },
    accessControl: {
      async getCurrentAuthorization() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/authorization/current',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        ) as AuthorizationSnapshot
      },
      async getAccessExperience() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/experience',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        ) as AccessExperienceResponse
      },
      async listMembers() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/members',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        ) as AccessMemberSummary[]
      },
      async listAudit(query: AccessAuditQuery = {}) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/audit',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
            queryParams: query,
          },
        ) as AccessAuditListResponse
      },
      async listSessions() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/sessions',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        ) as AccessSessionRecord[]
      },
      async revokeCurrentSession() {
        await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/sessions/current/revoke',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
          },
        )
      },
      async revokeSession(sessionId) {
        await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/sessions/{sessionId}/revoke',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              sessionId,
            },
          },
        )
      },
      async revokeUserSessions(userId) {
        await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/users/{userId}/sessions/revoke',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              userId,
            },
          },
        )
      },
      async listUsers() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/users', 'get', {
          session: assertWorkspaceRequestReady(context),
        }) as AccessUserRecord[]
      },
      async updateUserPreset(userId, record) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/users/{userId}/preset',
          'put',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              userId,
            },
            body: JSON.stringify(record satisfies AccessUserPresetUpdateRequest),
          },
        ) as AccessMemberSummary
      },
      async createUser(record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/users', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(record),
        }) as AccessUserRecord
      },
      async updateUser(userId, record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/users/{userId}', 'put', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            userId,
          },
          body: JSON.stringify(record),
        }) as AccessUserRecord
      },
      async deleteUser(userId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/users/{userId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            userId,
          },
        })
      },
      async listOrgUnits() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/org/units', 'get', {
          session: assertWorkspaceRequestReady(context),
        }) as OrgUnitRecord[]
      },
      async createOrgUnit(record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/org/units', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(record),
        }) as OrgUnitRecord
      },
      async updateOrgUnit(orgUnitId, record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/org/units/{orgUnitId}', 'put', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            orgUnitId,
          },
          body: JSON.stringify(record),
        }) as OrgUnitRecord
      },
      async deleteOrgUnit(orgUnitId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/org/units/{orgUnitId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            orgUnitId,
          },
        })
      },
      async listPositions() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/org/positions', 'get', {
          session: assertWorkspaceRequestReady(context),
        }) as PositionRecord[]
      },
      async createPosition(record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/org/positions', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(record),
        }) as PositionRecord
      },
      async updatePosition(positionId, record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/org/positions/{positionId}', 'put', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            positionId,
          },
          body: JSON.stringify(record),
        }) as PositionRecord
      },
      async deletePosition(positionId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/org/positions/{positionId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            positionId,
          },
        })
      },
      async listUserGroups() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/org/groups', 'get', {
          session: assertWorkspaceRequestReady(context),
        }) as UserGroupRecord[]
      },
      async createUserGroup(record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/org/groups', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(record),
        }) as UserGroupRecord
      },
      async updateUserGroup(groupId, record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/org/groups/{groupId}', 'put', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            groupId,
          },
          body: JSON.stringify(record),
        }) as UserGroupRecord
      },
      async deleteUserGroup(groupId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/org/groups/{groupId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            groupId,
          },
        })
      },
      async listUserOrgAssignments() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/org/assignments', 'get', {
          session: assertWorkspaceRequestReady(context),
        }) as UserOrgAssignmentRecord[]
      },
      async upsertUserOrgAssignment(record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/org/assignments', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(record),
        }) as UserOrgAssignmentRecord
      },
      async deleteUserOrgAssignment(userId, orgUnitId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/org/assignments/{userId}/{orgUnitId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            userId,
            orgUnitId,
          },
        })
      },
      async listRoles() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/roles', 'get', {
          session: assertWorkspaceRequestReady(context),
        }) as AccessRoleRecord[]
      },
      async createRole(record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/roles', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(record),
        }) as AccessRoleRecord
      },
      async updateRole(roleId, record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/roles/{roleId}', 'put', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            roleId,
          },
          body: JSON.stringify(record),
        }) as AccessRoleRecord
      },
      async deleteRole(roleId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/access/roles/{roleId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            roleId,
          },
        })
      },
      async listPermissionDefinitions() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/policies/permission-definitions',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        ) as PermissionDefinition[]
      },
      async listRoleBindings() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/policies/role-bindings',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        ) as RoleBindingRecord[]
      },
      async createRoleBinding(record) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/policies/role-bindings',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(record),
          },
        ) as RoleBindingRecord
      },
      async updateRoleBinding(bindingId, record) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/policies/role-bindings/{bindingId}',
          'put',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              bindingId,
            },
            body: JSON.stringify(record),
          },
        ) as RoleBindingRecord
      },
      async deleteRoleBinding(bindingId) {
        await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/policies/role-bindings/{bindingId}',
          'delete',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              bindingId,
            },
          },
        )
      },
      async listDataPolicies() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/policies/data-policies',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        ) as DataPolicyRecord[]
      },
      async createDataPolicy(record) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/policies/data-policies',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(record),
          },
        ) as DataPolicyRecord
      },
      async updateDataPolicy(policyId, record) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/policies/data-policies/{policyId}',
          'put',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              policyId,
            },
            body: JSON.stringify(record),
          },
        ) as DataPolicyRecord
      },
      async deleteDataPolicy(policyId) {
        await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/policies/data-policies/{policyId}',
          'delete',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              policyId,
            },
          },
        )
      },
      async listResourcePolicies() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/policies/resource-policies',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        ) as ResourcePolicyRecord[]
      },
      async createResourcePolicy(record) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/policies/resource-policies',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(record),
          },
        ) as ResourcePolicyRecord
      },
      async updateResourcePolicy(policyId, record) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/policies/resource-policies/{policyId}',
          'put',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              policyId,
            },
            body: JSON.stringify(record),
          },
        ) as ResourcePolicyRecord
      },
      async deleteResourcePolicy(policyId) {
        await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/policies/resource-policies/{policyId}',
          'delete',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              policyId,
            },
          },
        )
      },
      async listMenuDefinitions() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/menus/definitions',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        ) as MenuDefinition[]
      },
      async listFeatureDefinitions() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/menus/features',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        ) as FeatureDefinition[]
      },
      async listMenuGateResults() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/menus/gates',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        ) as MenuGateResult[]
      },
      async listMenuPolicies() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/menus/policies',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        ) as MenuPolicyRecord[]
      },
      async createMenuPolicy(record) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/menus/policies',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(record),
          },
        ) as MenuPolicyRecord
      },
      async updateMenuPolicy(menuId, record) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/menus/policies/{menuId}',
          'put',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              menuId,
            },
            body: JSON.stringify(record),
          },
        ) as MenuPolicyRecord
      },
      async deleteMenuPolicy(menuId) {
        await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/menus/policies/{menuId}',
          'delete',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              menuId,
            },
          },
        )
      },
      async listProtectedResources() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/protected-resources',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        ) as ProtectedResourceDescriptor[]
      },
      async upsertProtectedResource(resourceType, resourceId, record: ProtectedResourceMetadataUpsertRequest) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/access/protected-resources/{resourceType}/{resourceId}',
          'put',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              resourceType,
              resourceId,
            },
            body: JSON.stringify(record),
          },
        ) as ProtectedResourceDescriptor
      },
    },
  }
}
