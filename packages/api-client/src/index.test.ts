import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createApiClient } from './index'

const runFixture = {
  id: 'run-1',
  workspaceId: 'workspace-1',
  agentId: 'agent-1',
  interactionType: 'ask_user',
  status: 'waiting_input',
  summary: 'Waiting for user input',
  input: 'Need more context',
  createdAt: '2026-03-24T00:00:00Z',
  updatedAt: '2026-03-24T00:00:00Z',
} as const

describe('createApiClient', () => {
  const fetchMock = vi.fn<typeof fetch>()
  const client = createApiClient({
    baseUrl: 'http://localhost:4173/api/v1',
    fetch: fetchMock as typeof fetch,
  })

  beforeEach(() => {
    fetchMock.mockReset()
  })

  it('lists runs from the generated runs endpoint', async () => {
    fetchMock.mockResolvedValue(
      new Response(JSON.stringify({ items: [runFixture] }), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      }),
    )

    const runs = await client.listRuns()

    expect(fetchMock).toHaveBeenCalledWith(
      'http://localhost:4173/api/v1/runs',
      expect.objectContaining({ method: 'GET' }),
    )
    expect(runs[0]?.interactionType).toBe('ask_user')
  })

  it('posts structured resume payloads', async () => {
    fetchMock.mockResolvedValue(
      new Response(
        JSON.stringify({
          accepted: true,
          deduplicated: false,
          runId: 'run-1',
          status: 'completed',
          run: { ...runFixture, status: 'completed', summary: 'Run resumed after user input' },
        }),
        {
          status: 202,
          headers: { 'content-type': 'application/json' },
        },
      ),
    )

    await client.resumeRun('run-1', {
      inboxItemId: 'inbox-1',
      resumeToken: 'resume-token',
      idempotencyKey: 'resume-1',
      response: {
        type: 'text',
        text: 'Proceed with the updated scope',
        goalChanged: true,
        values: [],
      },
    })

    expect(fetchMock).toHaveBeenCalledWith(
      'http://localhost:4173/api/v1/runs/run-1/resume',
      expect.objectContaining({
        method: 'POST',
        body: JSON.stringify({
          inboxItemId: 'inbox-1',
          resumeToken: 'resume-token',
          idempotencyKey: 'resume-1',
          response: {
            type: 'text',
            text: 'Proceed with the updated scope',
            goalChanged: true,
            values: [],
          },
        }),
      }),
    )
  })
})
