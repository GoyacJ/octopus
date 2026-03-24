import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import { createOctopusI18n } from '@octopus/i18n'
import type { ControlPlaneClient } from '@/lib/control-plane'
import { controlPlaneClientKey } from '@/lib/control-plane'
import InboxPage from './InboxPage.vue'

describe('InboxPage', () => {
  it('submits resume responses for pending inbox items', async () => {
    const client: ControlPlaneClient = {
      listRuns: vi.fn().mockResolvedValue([]),
      createRun: vi.fn(),
      getRun: vi.fn(),
      getRunTimeline: vi.fn().mockResolvedValue([]),
      listInboxItems: vi.fn().mockResolvedValue([
        {
          id: 'inbox-1',
          runId: 'run-1',
          kind: 'ask_user',
          status: 'pending',
          title: 'User input required',
          prompt: 'Provide the missing context',
          responseType: 'text',
          options: [],
          resumeToken: 'resume-token',
          createdAt: '2026-03-24T00:00:00Z',
        },
      ]),
      listAuditEvents: vi.fn().mockResolvedValue([]),
      resumeRun: vi.fn().mockResolvedValue({
        accepted: true,
        deduplicated: false,
        runId: 'run-1',
        status: 'completed',
        run: {
          id: 'run-1',
          workspaceId: 'workspace-1',
          agentId: 'agent-1',
          interactionType: 'ask_user',
          status: 'completed',
          summary: 'Run resumed after user input',
          input: 'Provide the missing context',
          createdAt: '2026-03-24T00:00:00Z',
          updatedAt: '2026-03-24T00:01:00Z',
        },
      }),
    }

    const wrapper = mount(InboxPage, {
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

    await wrapper.get('[data-testid="inbox-response-text"]').setValue('Proceed with the updated scope')
    await wrapper.get('[data-testid="goal-changed"]').setValue(true)
    await wrapper.get('[data-testid="resume-form"]').trigger('submit.prevent')
    await flushPromises()

    expect(client.resumeRun).toHaveBeenCalledWith(
      'run-1',
      expect.objectContaining({
        inboxItemId: 'inbox-1',
        resumeToken: 'resume-token',
        response: expect.objectContaining({
          text: 'Proceed with the updated scope',
          goalChanged: true,
        }),
      }),
    )
  })

  it('submits approval rejections and renders the failed result state', async () => {
    const client: ControlPlaneClient = {
      listRuns: vi.fn().mockResolvedValue([]),
      createRun: vi.fn(),
      getRun: vi.fn(),
      getRunTimeline: vi.fn().mockResolvedValue([]),
      listInboxItems: vi.fn().mockResolvedValue([
        {
          id: 'inbox-approval-1',
          runId: 'run-approval-1',
          kind: 'approval',
          status: 'pending',
          title: 'Approval required',
          prompt: 'Approve or reject the requested action',
          responseType: 'approval',
          options: [],
          resumeToken: 'resume-approval-token',
          createdAt: '2026-03-24T00:00:00Z',
        },
      ]),
      listAuditEvents: vi.fn().mockResolvedValue([]),
      resumeRun: vi.fn().mockResolvedValue({
        accepted: true,
        deduplicated: false,
        runId: 'run-approval-1',
        status: 'failed',
        run: {
          id: 'run-approval-1',
          workspaceId: 'workspace-1',
          agentId: 'agent-1',
          interactionType: 'approval',
          status: 'failed',
          summary: 'Run stopped after approval rejection',
          input: 'Approve the deployment plan',
          createdAt: '2026-03-24T00:00:00Z',
          updatedAt: '2026-03-24T00:01:00Z',
        },
      }),
    }

    const wrapper = mount(InboxPage, {
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

    await wrapper.get('input[value="reject"]').setValue()
    await wrapper.get('[data-testid="inbox-response-text"]').setValue('Risk changed while waiting')
    await wrapper.get('[data-testid="resume-form"]').trigger('submit.prevent')
    await flushPromises()

    expect(client.resumeRun).toHaveBeenCalledWith(
      'run-approval-1',
      expect.objectContaining({
        inboxItemId: 'inbox-approval-1',
        resumeToken: 'resume-approval-token',
        response: expect.objectContaining({
          type: 'approval',
          approved: false,
          text: 'Risk changed while waiting',
        }),
      }),
    )
    expect(wrapper.text()).toContain('Failed')
    expect(wrapper.text()).toContain('Run stopped after approval rejection')
  })
})
