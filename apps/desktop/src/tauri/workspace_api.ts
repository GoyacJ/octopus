import type {
  AgentRecord,
  AutomationRecord,
  ChangeCurrentUserPasswordResponse,
  CredentialBinding,
  ImportWorkspaceAgentBundlePreview,
  ImportWorkspaceAgentBundleResult,
  ModelCatalogSnapshot,
  ProjectAgentLinkRecord,
  ProjectTeamLinkRecord,
  ToolRecord,
  WorkspaceMcpServerDocument,
  WorkspaceSkillDocument,
  WorkspaceSkillFileDocument,
  WorkspaceSkillTreeDocument,
  WorkspaceToolCatalogSnapshot,
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
    auth: {
      async login(input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/auth/login',
          'post',
          {
            body: JSON.stringify(input),
          },
        )
      },
      async registerOwner(input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/auth/register-owner',
          'post',
          {
            body: JSON.stringify(input),
          },
        )
      },
      async logout() {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/auth/logout', 'post', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async session() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/auth/session', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
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
    artifacts: {
      async listWorkspace() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/artifacts', 'get', {
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
      async previewImportBundle(input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/agents/import-preview',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(input),
          },
        ) as unknown as ImportWorkspaceAgentBundlePreview
      },
      async importBundle(input) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/agents/import',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(input),
          },
        ) as unknown as ImportWorkspaceAgentBundleResult
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
      async getToolCatalog() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/tool-catalog', 'get', {
          session: assertWorkspaceRequestReady(context),
        }) as unknown as WorkspaceToolCatalogSnapshot
      },
      async setToolDisabled(patch) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/catalog/tool-catalog/disable', 'patch', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(patch),
        }) as unknown as WorkspaceToolCatalogSnapshot
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
    rbac: {
      async getUserCenterOverview() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/user-center/overview',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        )
      },
      async listUsers() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/users', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async createUser(record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/users', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(record),
        })
      },
      async updateUser(userId, record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/users/{userId}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            userId,
          },
          body: JSON.stringify(record),
        })
      },
      async deleteUser(userId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/users/{userId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            userId,
          },
        })
      },
      async updateCurrentUserProfile(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/user-center/profile', 'patch', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        })
      },
      async changeCurrentUserPassword(input) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/user-center/profile/password', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(input),
        }) as ChangeCurrentUserPasswordResponse
      },
      async listRoles() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/roles', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async createRole(record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/roles', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(record),
        })
      },
      async updateRole(roleId, record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/roles/{roleId}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            roleId,
          },
          body: JSON.stringify(record),
        })
      },
      async deleteRole(roleId) {
        await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/roles/{roleId}', 'delete', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            roleId,
          },
        })
      },
      async listPermissions() {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/rbac/permissions',
          'get',
          {
            session: assertWorkspaceRequestReady(context),
          },
        )
      },
      async createPermission(record) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/rbac/permissions',
          'post',
          {
            session: assertWorkspaceRequestReady(context),
            body: JSON.stringify(record),
          },
        )
      },
      async updatePermission(permissionId, record) {
        return await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/rbac/permissions/{permissionId}',
          'patch',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              permissionId,
            },
            body: JSON.stringify(record),
          },
        )
      },
      async deletePermission(permissionId) {
        await fetchWorkspaceOpenApi(
          context.connection,
          '/api/v1/workspace/rbac/permissions/{permissionId}',
          'delete',
          {
            session: assertWorkspaceRequestReady(context),
            pathParams: {
              permissionId,
            },
          },
        )
      },
      async listMenus() {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/menus', 'get', {
          session: assertWorkspaceRequestReady(context),
        })
      },
      async createMenu(record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/menus', 'post', {
          session: assertWorkspaceRequestReady(context),
          body: JSON.stringify(record),
        })
      },
      async updateMenu(menuId, record) {
        return await fetchWorkspaceOpenApi(context.connection, '/api/v1/workspace/rbac/menus/{menuId}', 'patch', {
          session: assertWorkspaceRequestReady(context),
          pathParams: {
            menuId,
          },
          body: JSON.stringify(record),
        })
      },
    },
  }
}
