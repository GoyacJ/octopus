import { describe, expect, it } from 'vitest'

import enUS from '@/locales/en-US.json'
import zhCN from '@/locales/zh-CN.json'

function collectLeafPaths(value: unknown, prefix = ''): string[] {
  if (Array.isArray(value)) {
    return value.flatMap((item, index) => collectLeafPaths(item, prefix ? `${prefix}.${index}` : `${index}`))
  }

  if (value && typeof value === 'object') {
    return Object.entries(value).flatMap(([key, nestedValue]) =>
      collectLeafPaths(nestedValue, prefix ? `${prefix}.${key}` : key),
    )
  }

  return prefix ? [prefix] : []
}

describe('desktop locale registry', () => {
  it('keeps zh-CN and en-US leaf keys in parity', () => {
    expect(collectLeafPaths(zhCN).sort()).toEqual(collectLeafPaths(enUS).sort())
  })

  it('covers the required shell, enum, and mock namespaces', () => {
    const keys = collectLeafPaths(zhCN)

    expect(keys).toContain('sidebar.workspace.label')
    expect(keys).toContain('sidebar.projectRail.title')
    expect(keys).toContain('dashboard.header.eyebrow')
    expect(keys).toContain('conversation.controls.title')
    expect(keys).toContain('contextPane.host.title')
    expect(keys).toContain('knowledge.hero.cards.total')
    expect(keys).toContain('trace.timeline.title')
    expect(keys).toContain('agents.list.title')
    expect(keys).toContain('teams.list.title')
    expect(keys).toContain('settings.tabs.general')
    expect(keys).toContain('automations.placeholder.title')
    expect(keys).toContain('connections.product.title')
    expect(keys).toContain('enum.conversationIntent.paused')
    expect(keys).toContain('enum.teamMode.hybrid')
    expect(keys).toContain('mock.workspace.ws-local.name')
    expect(keys).toContain('mock.project.proj-redesign.summary')
    expect(keys).toContain('mock.conversation.conv-redesign.summary')
    expect(keys).toContain('mock.agent.agent-coder.role')
    expect(keys).toContain('mock.team.team-launch.description')
    expect(keys).toContain('mock.settingsPage.version-meta.description')
    expect(keys).toContain('mock.automation.auto-launch-daily.description')
    expect(keys).toContain('mock.connection.conn-shared-enterprise.label')
  })
})
