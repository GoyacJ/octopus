import { flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { useRuntimeControlStore } from './useRuntimeControlStore'

describe('useRuntimeControlStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.restoreAllMocks()
  })

  it('submits tasks with explicit workspace and project context', async () => {
    const fetchMock = vi.fn()
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
              title: 'Review runtime wiring',
              checkpoint_token: null,
              created_at: '2026-03-25T00:00:00Z',
              updated_at: '2026-03-25T00:00:00Z',
            },
            artifact: {
              id: 'artifact-1',
              project_id: 'project-alpha',
              run_id: 'run-1',
              version: 1,
              title: 'Artifact for Review runtime wiring',
              content_ref: 'Direct path without approval',
              state: 'current',
              created_at: '2026-03-25T00:00:00Z',
            },
            approval: null,
            inbox_item: null,
            trace: [],
            audit: [],
          }),
        ),
      )

    vi.stubGlobal('fetch', fetchMock)

    const store = useRuntimeControlStore()
    await store.submitTask({
      workspace_id: 'workspace-alpha',
      project_id: 'project-alpha',
      title: 'Review runtime wiring',
      description: 'Direct path without approval',
      requested_by: 'operator-1',
      requires_approval: false,
    })

    expect(store.currentRunDetail?.run.id).toBe('run-1')
    expect(JSON.parse((fetchMock.mock.calls[0]?.[1] as RequestInit).body as string)).toMatchObject({
      workspace_id: 'workspace-alpha',
      project_id: 'project-alpha',
    })
  })

  it('loads automations, creates one, and replays trigger deliveries without duplicating local state', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify({ items: [] })))
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            automation: {
              id: 'automation-1',
              workspace_id: 'workspace-alpha',
              project_id: 'project-alpha',
              name: 'Nightly workspace scan',
              trigger_ids: ['trigger-1'],
              state: 'active',
              requires_approval: false,
              last_run_id: null,
              created_at: '2026-03-25T00:00:00Z',
              updated_at: '2026-03-25T00:00:00Z',
            },
            trigger: {
              id: 'trigger-1',
              automation_id: 'automation-1',
              source_type: 'cron',
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
              source_type: 'cron',
              dedupe_key: 'cron-2026-03-26T00:00',
              state: 'succeeded',
              run_id: 'run-1',
              failure_reason: null,
              occurred_at: '2026-03-26T00:00:00Z',
            },
            run: {
              run: {
                id: 'run-1',
                project_id: 'project-alpha',
                run_type: 'automation',
                status: 'completed',
                idempotency_key: 'trigger:trigger-1:cron-2026-03-26T00:00',
                requested_by: 'operator-1',
                title: 'Nightly workspace scan',
                checkpoint_token: null,
                created_at: '2026-03-26T00:00:00Z',
                updated_at: '2026-03-26T00:00:00Z',
              },
              artifact: null,
              approval: null,
              inbox_item: null,
              trace: [],
              audit: [],
            },
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            delivery: {
              id: 'delivery-1',
              trigger_id: 'trigger-1',
              source_type: 'cron',
              dedupe_key: 'cron-2026-03-26T00:00',
              state: 'succeeded',
              run_id: 'run-1',
              failure_reason: null,
              occurred_at: '2026-03-26T00:00:00Z',
            },
            run: {
              run: {
                id: 'run-1',
                project_id: 'project-alpha',
                run_type: 'automation',
                status: 'completed',
                idempotency_key: 'trigger:trigger-1:cron-2026-03-26T00:00',
                requested_by: 'operator-1',
                title: 'Nightly workspace scan',
                checkpoint_token: null,
                created_at: '2026-03-26T00:00:00Z',
                updated_at: '2026-03-26T00:00:00Z',
              },
              artifact: null,
              approval: null,
              inbox_item: null,
              trace: [],
              audit: [],
            },
          }),
        ),
      )

    vi.stubGlobal('fetch', fetchMock)

    const store = useRuntimeControlStore()
    await store.loadAutomations()
    await store.createAutomation({
      workspace_id: 'workspace-alpha',
      project_id: 'project-alpha',
      name: 'Nightly workspace scan',
      trigger_source: 'cron',
      requested_by: 'operator-1',
      requires_approval: false,
    })

    expect(store.automations).toHaveLength(1)

    await store.deliverTrigger({
      trigger_id: 'trigger-1',
      dedupe_key: 'cron-2026-03-26T00:00',
      requested_by: 'operator-1',
      description: 'Scan the workspace',
    })
    await store.deliverTrigger({
      trigger_id: 'trigger-1',
      dedupe_key: 'cron-2026-03-26T00:00',
      requested_by: 'operator-1',
      description: 'Scan the workspace',
    })

    expect(store.currentRunDetail?.run.id).toBe('run-1')
    expect(store.automations).toHaveLength(1)
    expect(store.automations[0]?.latest_delivery?.id).toBe('delivery-1')
  })

  it('loads knowledge spaces, creates a candidate from the current run, and promotes it', async () => {
    const fetchMock = vi.fn()
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
            candidate: {
              id: 'candidate-1',
              knowledge_space_id: 'knowledge-space-alpha',
              run_id: 'run-1',
              artifact_id: 'artifact-1',
              title: 'Artifact for Review runtime wiring',
              summary: 'Direct path without approval',
              status: 'candidate',
              trust_level: 'high',
              source_ref: 'run-1',
              created_by: 'operator-1',
              created_at: '2026-03-26T00:01:00Z',
              promoted_asset_id: null,
            },
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
              idempotency_key: 'task:run-1',
              requested_by: 'operator-1',
              title: 'Review runtime wiring',
              checkpoint_token: null,
              created_at: '2026-03-26T00:00:00Z',
              updated_at: '2026-03-26T00:01:00Z',
            },
            artifact: {
              id: 'artifact-1',
              project_id: 'project-alpha',
              run_id: 'run-1',
              version: 1,
              title: 'Artifact for Review runtime wiring',
              content_ref: 'Direct path without approval',
              state: 'current',
              created_at: '2026-03-26T00:00:00Z',
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
              run_id: 'run-1',
              artifact_id: 'artifact-1',
              title: 'Artifact for Review runtime wiring',
              summary: 'Direct path without approval',
              status: 'verified_shared',
              trust_level: 'high',
              source_ref: 'run-1',
              created_by: 'operator-1',
              created_at: '2026-03-26T00:01:00Z',
              promoted_asset_id: 'asset-1',
            },
            asset: {
              id: 'asset-1',
              knowledge_space_id: 'knowledge-space-alpha',
              title: 'Artifact for Review runtime wiring',
              summary: 'Direct path without approval',
              layer: 'shared',
              status: 'verified_shared',
              trust_level: 'high',
              source_ref: 'candidate-1',
              created_at: '2026-03-26T00:02:00Z',
            },
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
              idempotency_key: 'task:run-1',
              requested_by: 'operator-1',
              title: 'Review runtime wiring',
              checkpoint_token: null,
              created_at: '2026-03-26T00:00:00Z',
              updated_at: '2026-03-26T00:02:00Z',
            },
            artifact: {
              id: 'artifact-1',
              project_id: 'project-alpha',
              run_id: 'run-1',
              version: 1,
              title: 'Artifact for Review runtime wiring',
              content_ref: 'Direct path without approval',
              state: 'current',
              created_at: '2026-03-26T00:00:00Z',
            },
            approval: null,
            inbox_item: null,
            trace: [],
            audit: [],
          }),
        ),
      )

    vi.stubGlobal('fetch', fetchMock)

    const store = useRuntimeControlStore()
    store.currentRunDetail = {
      run: {
        id: 'run-1',
        project_id: 'project-alpha',
        run_type: 'task',
        status: 'completed',
        idempotency_key: 'task:run-1',
        requested_by: 'operator-1',
        title: 'Review runtime wiring',
        checkpoint_token: null,
        created_at: '2026-03-26T00:00:00Z',
        updated_at: '2026-03-26T00:00:00Z',
      },
      artifact: {
        id: 'artifact-1',
        project_id: 'project-alpha',
        run_id: 'run-1',
        version: 1,
        title: 'Artifact for Review runtime wiring',
        content_ref: 'Direct path without approval',
        state: 'current',
        created_at: '2026-03-26T00:00:00Z',
      },
      approval: null,
      inbox_item: null,
      trace: [],
      audit: [],
    }

    await store.loadKnowledgeSpaces()
    const candidate = await store.createCandidateFromRun('knowledge-space-alpha')
    const promotion = await store.promoteCandidate('candidate-1')

    expect(candidate?.id).toBe('candidate-1')
    expect(promotion?.asset.id).toBe('asset-1')
    expect(store.knowledgeSpaces[0]?.candidates[0]?.status).toBe('verified_shared')
    expect(store.knowledgeSpaces[0]?.assets[0]?.id).toBe('asset-1')
  })

  it('loads run and inbox snapshots and refreshes them from runtime events', async () => {
    const eventSources: Array<{
      url: string
      onmessage: ((event: MessageEvent<string>) => void) | null
      close: () => void
      emit: (payload: unknown) => void
    }> = []
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            items: [
              {
                id: 'run-1',
                project_id: 'project-alpha',
                run_type: 'task',
                status: 'waiting_approval',
                idempotency_key: 'task:run-1',
                requested_by: 'operator-1',
                title: 'Review runtime wiring',
                checkpoint_token: 'resume:run-1',
                created_at: '2026-03-26T00:00:00Z',
                updated_at: '2026-03-26T00:00:00Z',
              },
            ],
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            items: [
              {
                id: 'inbox-1',
                workspace_id: 'workspace-alpha',
                owner_ref: 'reviewer.execution',
                state: 'open',
                priority: 'high',
                target_ref: 'run-1',
                dedupe_key: 'approval:run-1',
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
              idempotency_key: 'task:run-1',
              requested_by: 'operator-1',
              title: 'Review runtime wiring',
              checkpoint_token: 'resume:run-1',
              created_at: '2026-03-26T00:00:00Z',
              updated_at: '2026-03-26T00:01:00Z',
            },
            artifact: null,
            approval: {
              id: 'approval-1',
              run_id: 'run-1',
              approval_type: 'execution',
              state: 'approved',
              target_ref: 'run-1',
              requested_at: '2026-03-26T00:00:30Z',
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
            trace: [],
            audit: [],
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            items: [
              {
                id: 'run-1',
                project_id: 'project-alpha',
                run_type: 'task',
                status: 'paused',
                idempotency_key: 'task:run-1',
                requested_by: 'operator-1',
                title: 'Review runtime wiring',
                checkpoint_token: 'resume:run-1',
                created_at: '2026-03-26T00:00:00Z',
                updated_at: '2026-03-26T00:01:00Z',
              },
            ],
          }),
        ),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            items: [
              {
                id: 'inbox-1',
                workspace_id: 'workspace-alpha',
                owner_ref: 'reviewer.execution',
                state: 'resolved',
                priority: 'high',
                target_ref: 'run-1',
                dedupe_key: 'approval:run-1',
              },
            ],
          }),
        ),
      )

    vi.stubGlobal('fetch', fetchMock)
    vi.stubGlobal(
      'EventSource',
      vi.fn().mockImplementation((url: string) => {
        const source: {
          url: string
          onmessage: ((event: MessageEvent<string>) => void) | null
          close: ReturnType<typeof vi.fn>
          emit: (payload: unknown) => void
        } = {
          url,
          onmessage: null,
          close: vi.fn(),
          emit(payload: unknown) {
            source.onmessage?.(
              new MessageEvent('message', {
                data: JSON.stringify(payload),
              }),
            )
          },
        }

        eventSources.push(source)
        return source
      }),
    )

    const store = useRuntimeControlStore()
    await store.loadRuns()
    await store.loadInboxItems()
    store.startRuntimeEventStream()

    expect(store.runs[0]?.status).toBe('waiting_approval')
    expect(store.inboxItems[0]?.state).toBe('open')
    expect(eventSources[0]?.url).toBe('/api/v1/events/stream')

    eventSources[0]?.emit({
      sequence: 1,
      topic: 'approval.updated',
      occurred_at: '2026-03-26T00:01:00Z',
      run_id: 'run-1',
      workspace_id: 'workspace-alpha',
    })
    await flushPromises()

    expect(store.currentRunDetail?.run.status).toBe('paused')
    expect(store.runs[0]?.status).toBe('paused')
    expect(store.inboxItems[0]?.state).toBe('resolved')
  })
})
