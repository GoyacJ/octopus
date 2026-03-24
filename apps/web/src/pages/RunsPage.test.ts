import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import { createOctopusI18n } from '@octopus/i18n'
import type { ControlPlaneClient } from '@/lib/control-plane'
import { controlPlaneClientKey } from '@/lib/control-plane'
import RunsPage from './RunsPage.vue'

function createClientStub(): ControlPlaneClient {
  return {
    listRuns: vi.fn().mockResolvedValue([
      {
        id: 'run-1',
        workspaceId: 'workspace-1',
        agentId: 'agent-1',
        interactionType: 'ask_user',
        status: 'waiting_input',
        summary: 'Need updated rollout notes',
        input: 'Collect updated rollout notes',
        createdAt: '2026-03-24T00:00:00Z',
        updatedAt: '2026-03-24T00:00:00Z',
      },
    ]),
    createRun: vi.fn().mockResolvedValue({
      id: 'run-2',
      workspaceId: 'workspace-1',
      agentId: 'agent-1',
      interactionType: 'approval',
      status: 'waiting_approval',
      summary: 'Waiting for approval before completion',
      input: 'Approve the deployment plan',
      createdAt: '2026-03-24T00:00:00Z',
      updatedAt: '2026-03-24T00:00:00Z',
    }),
    getRun: vi.fn(),
    getRunTimeline: vi.fn().mockResolvedValue([
      {
        id: 'timeline-1',
        runId: 'run-1',
        type: 'run.waiting_input',
        summary: 'Waiting for user input before completion',
        occurredAt: '2026-03-24T00:00:00Z',
      },
    ]),
    listInboxItems: vi.fn().mockResolvedValue([]),
    listAuditEvents: vi.fn().mockResolvedValue([]),
    resumeRun: vi.fn(),
  }
}

describe('RunsPage', () => {
  it('renders live runs and submits create-run requests', async () => {
    const client = createClientStub()
    const wrapper = mount(RunsPage, {
      global: {
        plugins: [
          [VueQueryPlugin, { queryClient: new QueryClient() }],
          createOctopusI18n('en-US'),
        ],
        provide: {
          [controlPlaneClientKey as symbol]: client,
        },
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('Need updated rollout notes')

    await wrapper.get('[data-testid="run-input"]').setValue('Approve the deployment plan')
    await wrapper.get('[data-testid="interaction-type"]').setValue('approval')
    await wrapper.get('[data-testid="create-run-form"]').trigger('submit.prevent')
    await flushPromises()

    expect(client.createRun).toHaveBeenCalledWith(
      expect.objectContaining({
        input: 'Approve the deployment plan',
        interactionType: 'approval',
      }),
    )
  })
})
