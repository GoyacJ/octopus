import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import { ConversationIntent } from '@octopus/schema'
import { useWorkbenchStore } from '@/stores/workbench'

describe('useWorkbenchStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('switches workspace and keeps project-scoped surfaces in sync', () => {
    const store = useWorkbenchStore()

    store.selectWorkspace('ws-enterprise')

    expect(store.activeWorkspace?.id).toBe('ws-enterprise')
    expect(store.activeProject?.workspaceId).toBe('ws-enterprise')
    expect(store.workspaceProjects.every((project) => project.workspaceId === 'ws-enterprise')).toBe(true)
    expect(store.workspaceInbox.every((item) => item.workspaceId === 'ws-enterprise')).toBe(true)
  })

  it('requests artifact review by creating an approval item and updating conversation intent', () => {
    const store = useWorkbenchStore()

    store.requestArtifactReview('art-roadmap')

    expect(store.activeConversation?.intent).toBe(ConversationIntent.REVIEW)
    expect(store.workspaceInbox.some((item) => item.relatedId === 'art-roadmap' && item.type === 'knowledge_promotion_approval')).toBe(true)
  })

  it('approves a pending inbox item and resumes the linked run', () => {
    const store = useWorkbenchStore()
    const pendingItem = store.workspaceInbox.find((item) => item.status === 'pending')

    expect(pendingItem).toBeDefined()

    store.resolveInboxItem(pendingItem!.id, 'approve')

    expect(store.workspaceInbox.find((item) => item.id === pendingItem!.id)?.status).toBe('resolved')
    expect(store.activeConversation?.intent).toBe(ConversationIntent.EXECUTE)
    expect(store.activeRun?.status).toBe('running')
  })

  it('shows project-scoped agent copies in the current workspace list', () => {
    const store = useWorkbenchStore()

    store.selectProject('proj-redesign')
    store.createProjectAgentCopy('agent-architect')

    expect(store.workspaceAgents.some((agent) => agent.id === 'agent-architect-copy-proj-redesign')).toBe(true)
  })
})
