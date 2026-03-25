import { createPinia } from 'pinia'
import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'

import { i18n } from '@/i18n'
import ShellHomeView from '@/views/ShellHomeView.vue'

import RuntimeOverview from './RuntimeOverview.vue'

describe('RuntimeOverview', () => {
  it('renders the runtime shell headline and core contract sections', () => {
    const wrapper = mount(RuntimeOverview)

    expect(wrapper.text()).toContain('Unified Agent Runtime Platform')
    expect(wrapper.text()).toContain('Run')
    expect(wrapper.text()).toContain('Chat')
  })
})

describe('ShellHomeView', () => {
  it('drives the task lifecycle through the store-backed control plane', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            items: [],
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            items: [
              {
                space: {
                  id: 'knowledge-space-alpha',
                  workspace_id: 'workspace-alpha',
                  name: 'Workspace Alpha Shared Knowledge',
                  owner_refs: ['owner-1'],
                  scope: 'project:project-alpha',
                  state: 'active',
                  created_at: '2026-03-26T00:00:00Z',
                  updated_at: '2026-03-26T00:00:00Z',
                },
                candidates: [],
                assets: [],
              },
            ],
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            items: [],
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            items: [],
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            run: {
              id: 'run-1',
              project_id: 'project-alpha',
              run_type: 'task',
              status: 'waiting_approval',
              idempotency_key: 'task:project-alpha:run-1',
              requested_by: 'operator-1',
              title: 'Review remote hub policy',
              checkpoint_token: 'resume:run-1',
              created_at: '2026-03-25T00:00:00Z',
              updated_at: '2026-03-25T00:00:00Z',
            },
            artifact: null,
            approval: {
              id: 'approval-1',
              run_id: 'run-1',
              approval_type: 'execution',
              state: 'pending',
              target_ref: 'run-1',
              requested_at: '2026-03-25T00:00:01Z',
              reviewed_by: null,
            },
            inbox_item: {
              id: 'inbox-1',
              workspace_id: 'workspace-alpha',
              owner_ref: 'reviewer.execution',
              state: 'open',
              priority: 'high',
              target_ref: 'run-1',
              dedupe_key: 'approval:run-1',
            },
            trace: [
              {
                name: 'RunStateChanged',
                message: 'Run run-1 entered planning',
                occurred_at: '2026-03-25T00:00:00Z',
              },
            ],
            audit: [
              {
                action: 'task.submitted',
                actor: 'operator-1',
                target_ref: 'run-1',
                occurred_at: '2026-03-25T00:00:00Z',
              },
            ],
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            run: {
              id: 'run-1',
              project_id: 'project-alpha',
              run_type: 'task',
              status: 'paused',
              idempotency_key: 'task:project-alpha:run-1',
              requested_by: 'operator-1',
              title: 'Review remote hub policy',
              checkpoint_token: 'resume:run-1',
              created_at: '2026-03-25T00:00:00Z',
              updated_at: '2026-03-25T00:02:00Z',
            },
            artifact: null,
            approval: {
              id: 'approval-1',
              run_id: 'run-1',
              approval_type: 'execution',
              state: 'approved',
              target_ref: 'run-1',
              requested_at: '2026-03-25T00:00:01Z',
              reviewed_by: 'reviewer-1',
            },
            inbox_item: {
              id: 'inbox-1',
              workspace_id: 'workspace-alpha',
              owner_ref: 'reviewer.execution',
              state: 'resolved',
              priority: 'high',
              target_ref: 'run-1',
              dedupe_key: 'approval:run-1',
            },
            trace: [
              {
                name: 'ApprovalResolved',
                message: 'Approval approval-1 approved by reviewer-1',
                occurred_at: '2026-03-25T00:02:00Z',
              },
            ],
            audit: [
              {
                action: 'approval.approved',
                actor: 'reviewer-1',
                target_ref: 'approval-1',
                occurred_at: '2026-03-25T00:02:00Z',
              },
            ],
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            run: {
              id: 'run-1',
              project_id: 'project-alpha',
              run_type: 'task',
              status: 'completed',
              idempotency_key: 'task:project-alpha:run-1',
              requested_by: 'operator-1',
              title: 'Review remote hub policy',
              checkpoint_token: 'resume:run-1',
              created_at: '2026-03-25T00:00:00Z',
              updated_at: '2026-03-25T00:03:00Z',
            },
            artifact: {
              id: 'artifact-1',
              project_id: 'project-alpha',
              run_id: 'run-1',
              version: 1,
              title: 'Artifact for Review remote hub policy',
              content_ref: 'Generated after explicit resume',
              state: 'current',
              created_at: '2026-03-25T00:03:00Z',
            },
            approval: {
              id: 'approval-1',
              run_id: 'run-1',
              approval_type: 'execution',
              state: 'approved',
              target_ref: 'run-1',
              requested_at: '2026-03-25T00:00:01Z',
              reviewed_by: 'reviewer-1',
            },
            inbox_item: {
              id: 'inbox-1',
              workspace_id: 'workspace-alpha',
              owner_ref: 'reviewer.execution',
              state: 'resolved',
              priority: 'high',
              target_ref: 'run-1',
              dedupe_key: 'approval:run-1',
            },
            trace: [
              {
                name: 'RunStateChanged',
                message: 'Run run-1 completed after resume',
                occurred_at: '2026-03-25T00:03:00Z',
              },
            ],
            audit: [
              {
                action: 'artifact.created',
                actor: 'operator-1',
                target_ref: 'artifact-1',
                occurred_at: '2026-03-25T00:03:00Z',
              },
            ],
          }),
        ),
      )

    vi.stubGlobal('fetch', fetchMock)

    const wrapper = mount(ShellHomeView, {
      global: {
        plugins: [createPinia(), i18n],
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('Phase 1')
    expect(fetchMock).toHaveBeenNthCalledWith(1, '/api/v1/automations')
    expect(fetchMock).toHaveBeenNthCalledWith(2, '/api/v1/knowledge/spaces')
    expect(fetchMock).toHaveBeenNthCalledWith(3, '/api/v1/runs')
    expect(fetchMock).toHaveBeenNthCalledWith(4, '/api/v1/inbox')

    await wrapper.get('[data-test="task-title"]').setValue('Review remote hub policy')
    await wrapper.get('[data-test="task-description"]').setValue('Need approval before artifact generation')
    await wrapper.get('[data-test="task-submit"]').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('waiting_approval')
    expect(wrapper.text()).toContain('approval-1')

    await wrapper.get('[data-test="approve-run"]').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('paused')
    expect(wrapper.text()).toContain('reviewer-1')

    await wrapper.get('[data-test="resume-run"]').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('Artifact for Review remote hub policy')
    expect(wrapper.text()).toContain('artifact.created')
    expect(fetchMock).toHaveBeenCalledTimes(7)

    const submitRequest = fetchMock.mock.calls[4]
    expect(submitRequest[0]).toBe('/api/v1/runs/task')
    expect(JSON.parse((submitRequest[1] as RequestInit).body as string)).toMatchObject({
      workspace_id: 'workspace-alpha',
      project_id: 'project-alpha',
      title: 'Review remote hub policy',
    })
  })

  it('creates an automation and delivers a manual trigger from the homepage', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            items: [],
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            items: [],
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            items: [],
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            items: [
              {
                space: {
                  id: 'knowledge-space-alpha',
                  workspace_id: 'workspace-alpha',
                  name: 'Workspace Alpha Shared Knowledge',
                  owner_refs: ['owner-1'],
                  scope: 'project:project-alpha',
                  state: 'active',
                  created_at: '2026-03-26T00:00:00Z',
                  updated_at: '2026-03-26T00:00:00Z',
                },
                candidates: [],
                assets: [],
              },
            ],
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            automation: {
              id: 'automation-1',
              workspace_id: 'workspace-alpha',
              project_id: 'project-alpha',
              name: 'Manual drift detector',
              trigger_ids: ['trigger-1'],
              state: 'active',
              requires_approval: true,
              last_run_id: null,
              created_at: '2026-03-25T00:00:00Z',
              updated_at: '2026-03-25T00:00:00Z',
            },
            trigger: {
              id: 'trigger-1',
              automation_id: 'automation-1',
              source_type: 'manual_event',
              dedupe_key: 'automation:automation-1',
              owner_ref: 'automation:automation-1',
              state: 'active',
              created_at: '2026-03-25T00:00:00Z',
            },
            latest_delivery: null,
            latest_run: null,
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            delivery: {
              id: 'delivery-1',
              trigger_id: 'trigger-1',
              source_type: 'manual_event',
              dedupe_key: 'manual-event-001',
              state: 'succeeded',
              run_id: 'run-watch-1',
              failure_reason: null,
              occurred_at: '2026-03-25T00:01:00Z',
            },
            run: {
              run: {
                id: 'run-watch-1',
                project_id: 'project-alpha',
                run_type: 'watch',
                status: 'waiting_approval',
                idempotency_key: 'trigger:trigger-1:manual-event-001',
                requested_by: 'operator-1',
                title: 'Investigate configuration drift',
                checkpoint_token: 'resume:run-watch-1',
                created_at: '2026-03-25T00:01:00Z',
                updated_at: '2026-03-25T00:01:00Z',
              },
              artifact: null,
              approval: {
                id: 'approval-watch-1',
                run_id: 'run-watch-1',
                approval_type: 'execution',
                state: 'pending',
                target_ref: 'run-watch-1',
                requested_at: '2026-03-25T00:01:00Z',
                reviewed_by: null,
              },
              inbox_item: {
                id: 'inbox-watch-1',
                workspace_id: 'workspace-alpha',
                owner_ref: 'reviewer.execution',
                state: 'open',
                priority: 'high',
                target_ref: 'run-watch-1',
                dedupe_key: 'approval:run-watch-1',
              },
              trace: [
                {
                  name: 'TriggerDelivered',
                  message: 'Trigger trigger-1 delivered manual-event-001',
                  occurred_at: '2026-03-25T00:01:00Z',
                },
              ],
              audit: [
                {
                  action: 'trigger.delivered',
                  actor: 'operator-1',
                  target_ref: 'delivery-1',
                  occurred_at: '2026-03-25T00:01:00Z',
                },
              ],
            },
          }),
        ),
      )

    vi.stubGlobal('fetch', fetchMock)

    const wrapper = mount(ShellHomeView, {
      global: {
        plugins: [createPinia(), i18n],
      },
    })

    await flushPromises()

    await wrapper.get('[data-test="automation-name"]').setValue('Manual drift detector')
    await wrapper.get('[data-test="automation-create"]').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('Manual drift detector')
    expect(wrapper.text()).toContain('manual_event')

    await wrapper.get('[data-test="delivery-dedupe-key"]').setValue('manual-event-001')
    await wrapper.get('[data-test="trigger-deliver"]').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('watch')
    expect(wrapper.text()).toContain('succeeded')

    const createAutomationRequest = fetchMock.mock.calls[4]
    expect(createAutomationRequest[0]).toBe('/api/v1/automations')
    expect(JSON.parse((createAutomationRequest[1] as RequestInit).body as string)).toMatchObject({
      workspace_id: 'workspace-alpha',
      project_id: 'project-alpha',
      trigger_source: 'manual_event',
    })
  })

  it('creates an MCP automation, delivers an MCP event, and promotes a knowledge candidate', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify({ items: [] })))
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            items: [
              {
                space: {
                  id: 'knowledge-space-alpha',
                  workspace_id: 'workspace-alpha',
                  name: 'Workspace Alpha Shared Knowledge',
                  owner_refs: ['owner-1'],
                  scope: 'project:project-alpha',
                  state: 'active',
                  created_at: '2026-03-26T00:00:00Z',
                  updated_at: '2026-03-26T00:00:00Z',
                },
                candidates: [],
                assets: [],
              },
            ],
          }),
        ),
      )
      .mockResolvedValueOnce(new Response(JSON.stringify({ items: [] })))
      .mockResolvedValueOnce(new Response(JSON.stringify({ items: [] })))
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            automation: {
              id: 'automation-mcp-1',
              workspace_id: 'workspace-alpha',
              project_id: 'project-alpha',
              name: 'Confluence sync',
              trigger_ids: ['trigger-mcp-1'],
              state: 'active',
              requires_approval: false,
              last_run_id: null,
              created_at: '2026-03-26T00:00:00Z',
              updated_at: '2026-03-26T00:00:00Z',
            },
            trigger: {
              id: 'trigger-mcp-1',
              automation_id: 'automation-mcp-1',
              source_type: 'mcp_event',
              dedupe_key: 'automation:automation-mcp-1',
              owner_ref: 'automation:automation-mcp-1',
              state: 'active',
              created_at: '2026-03-26T00:00:00Z',
              mcp_binding: {
                server_name: 'confluence',
                event_name: 'page.updated',
              },
            },
            latest_delivery: null,
            latest_run: null,
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            items: [
              {
                delivery: {
                  id: 'delivery-mcp-1',
                  trigger_id: 'trigger-mcp-1',
                  source_type: 'mcp_event',
                  dedupe_key: 'evt-001',
                  state: 'succeeded',
                  run_id: 'run-mcp-1',
                  failure_reason: null,
                  occurred_at: '2026-03-26T00:01:00Z',
                },
                run: {
                  run: {
                    id: 'run-mcp-1',
                    project_id: 'project-alpha',
                    run_type: 'watch',
                    status: 'completed',
                    idempotency_key: 'trigger:trigger-mcp-1:evt-001',
                    requested_by: 'operator-1',
                    title: 'Confluence page updated',
                    checkpoint_token: null,
                    created_at: '2026-03-26T00:01:00Z',
                    updated_at: '2026-03-26T00:01:00Z',
                  },
                  artifact: {
                    id: 'artifact-mcp-1',
                    project_id: 'project-alpha',
                    run_id: 'run-mcp-1',
                    version: 1,
                    title: 'Artifact for Confluence page updated',
                    content_ref: 'Remote page update',
                    state: 'current',
                    created_at: '2026-03-26T00:01:00Z',
                  },
                  approval: null,
                  inbox_item: null,
                  trace: [],
                  audit: [],
                },
              },
            ],
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            candidate: {
              id: 'candidate-1',
              knowledge_space_id: 'knowledge-space-alpha',
              run_id: 'run-mcp-1',
              artifact_id: 'artifact-mcp-1',
              title: 'Artifact for Confluence page updated',
              summary: 'Remote page update',
              status: 'candidate',
              trust_level: 'low',
              source_ref: 'run-mcp-1',
              created_by: 'operator-1',
              created_at: '2026-03-26T00:02:00Z',
              promoted_asset_id: null,
            },
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            run: {
              id: 'run-mcp-1',
              project_id: 'project-alpha',
              run_type: 'watch',
              status: 'completed',
              idempotency_key: 'trigger:trigger-mcp-1:evt-001',
              requested_by: 'operator-1',
              title: 'Confluence page updated',
              checkpoint_token: null,
              created_at: '2026-03-26T00:01:00Z',
              updated_at: '2026-03-26T00:02:00Z',
            },
            artifact: {
              id: 'artifact-mcp-1',
              project_id: 'project-alpha',
              run_id: 'run-mcp-1',
              version: 1,
              title: 'Artifact for Confluence page updated',
              content_ref: 'Remote page update',
              state: 'current',
              created_at: '2026-03-26T00:01:00Z',
            },
            approval: null,
            inbox_item: null,
            trace: [],
            audit: [],
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            candidate: {
              id: 'candidate-1',
              knowledge_space_id: 'knowledge-space-alpha',
              run_id: 'run-mcp-1',
              artifact_id: 'artifact-mcp-1',
              title: 'Artifact for Confluence page updated',
              summary: 'Remote page update',
              status: 'verified_shared',
              trust_level: 'low',
              source_ref: 'run-mcp-1',
              created_by: 'operator-1',
              created_at: '2026-03-26T00:02:00Z',
              promoted_asset_id: 'asset-1',
            },
            asset: {
              id: 'asset-1',
              knowledge_space_id: 'knowledge-space-alpha',
              title: 'Artifact for Confluence page updated',
              summary: 'Remote page update',
              layer: 'shared',
              status: 'verified_shared',
              trust_level: 'low',
              source_ref: 'candidate-1',
              created_at: '2026-03-26T00:03:00Z',
            },
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            run: {
              id: 'run-mcp-1',
              project_id: 'project-alpha',
              run_type: 'watch',
              status: 'completed',
              idempotency_key: 'trigger:trigger-mcp-1:evt-001',
              requested_by: 'operator-1',
              title: 'Confluence page updated',
              checkpoint_token: null,
              created_at: '2026-03-26T00:01:00Z',
              updated_at: '2026-03-26T00:03:00Z',
            },
            artifact: {
              id: 'artifact-mcp-1',
              project_id: 'project-alpha',
              run_id: 'run-mcp-1',
              version: 1,
              title: 'Artifact for Confluence page updated',
              content_ref: 'Remote page update',
              state: 'current',
              created_at: '2026-03-26T00:01:00Z',
            },
            approval: null,
            inbox_item: null,
            trace: [],
            audit: [],
          }),
        ),
      )

    vi.stubGlobal('fetch', fetchMock)

    const wrapper = mount(ShellHomeView, {
      global: {
        plugins: [createPinia(), i18n],
      },
    })

    await flushPromises()

    await wrapper.get('[data-test="automation-name"]').setValue('Confluence sync')
    await wrapper.get('[data-test="automation-source"]').setValue('mcp_event')
    await wrapper.get('[data-test="automation-mcp-server"]').setValue('confluence')
    await wrapper.get('[data-test="automation-mcp-event"]').setValue('page.updated')
    await wrapper.get('[data-test="automation-create"]').trigger('click')
    await flushPromises()

    await wrapper.get('[data-test="delivery-dedupe-key"]').setValue('evt-001')
    await wrapper.get('[data-test="trigger-deliver"]').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('Confluence sync')
    expect(wrapper.text()).toContain('mcp_event')
    expect(wrapper.text()).toContain('Artifact for Confluence page updated')

    await wrapper.get('[data-test="knowledge-create-candidate"]').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('candidate')
    expect(wrapper.text()).toContain('low')

    await wrapper.get('[data-test="knowledge-promote-candidate-1"]').trigger('click')
    await flushPromises()

    expect(wrapper.text()).toContain('verified_shared')
    expect(wrapper.text()).toContain('asset-1')

    const createAutomationRequest = fetchMock.mock.calls[4]
    expect(createAutomationRequest[0]).toBe('/api/v1/automations')
    expect(JSON.parse((createAutomationRequest[1] as RequestInit).body as string)).toMatchObject({
      trigger_source: 'mcp_event',
      mcp_binding: {
        server_name: 'confluence',
        event_name: 'page.updated',
      },
    })
  })
})
