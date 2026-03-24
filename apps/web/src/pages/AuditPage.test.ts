import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { flushPromises, mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import { createOctopusI18n } from '@octopus/i18n'
import type { ControlPlaneClient } from '@/lib/control-plane'
import { controlPlaneClientKey } from '@/lib/control-plane'
import AuditPage from './AuditPage.vue'

describe('AuditPage', () => {
  it('renders audit events from the control-plane client', async () => {
    const client: ControlPlaneClient = {
      listRuns: vi.fn().mockResolvedValue([]),
      createRun: vi.fn(),
      getRun: vi.fn(),
      getRunTimeline: vi.fn().mockResolvedValue([]),
      listInboxItems: vi.fn().mockResolvedValue([]),
      listAuditEvents: vi.fn().mockResolvedValue([
        {
          id: 'audit-1',
          actorId: 'system',
          subjectType: 'run',
          subjectId: 'run-1',
          action: 'run.resume.accepted',
          summary: 'Accepted a governed resume request',
          occurredAt: '2026-03-24T00:02:00Z',
        },
      ]),
      resumeRun: vi.fn(),
    }

    const wrapper = mount(AuditPage, {
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

    expect(wrapper.text()).toContain('run.resume.accepted')
    expect(wrapper.text()).toContain('Accepted a governed resume request')
  })
})
