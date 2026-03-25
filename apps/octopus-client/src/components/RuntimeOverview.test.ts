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
  it('drives the first run lifecycle from submit to resume', async () => {
    const fetchMock = vi.fn()
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
              workspace_id: 'project-alpha',
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
              workspace_id: 'project-alpha',
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
              workspace_id: 'project-alpha',
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

    expect(wrapper.text()).toContain('Phase 1')

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
    expect(fetchMock).toHaveBeenCalledTimes(3)
  })
})
