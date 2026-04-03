import { afterEach, describe, expect, it } from 'vitest'

import { enumLabel, resolveCopy, translate } from '@/i18n/copy'
import i18n from '@/plugins/i18n'

const originalLocale = i18n.global.locale.value

describe('desktop i18n smoke coverage', () => {
  afterEach(() => {
    i18n.global.locale.value = originalLocale
  })

  it('switches shell labels, enum labels, and seeded mock copy between zh-CN and en-US', () => {
    i18n.global.locale.value = 'zh-CN'

    expect(translate('dashboard.header.eyebrow')).toBe('工作区 Dashboard')
    expect(translate('conversation.stream.title')).toBe('消息流')
    expect(translate('settings.tabs.general')).toBe('通用')
    expect(resolveCopy('mock.workspace.ws-local.name')).toBe('本地枢纽')
    expect(resolveCopy('mock.project.proj-redesign.summary')).toContain('PRD 模块')
    expect(enumLabel('conversationIntent', 'paused')).toBe('已暂停')

    i18n.global.locale.value = 'en-US'

    expect(translate('dashboard.header.eyebrow')).toBe('Workspace Dashboard')
    expect(translate('conversation.stream.title')).toBe('Message Stream')
    expect(translate('settings.tabs.general')).toBe('General')
    expect(resolveCopy('mock.workspace.ws-local.name')).toBe('Local Hub')
    expect(resolveCopy('mock.project.proj-redesign.summary')).toContain('PRD modules')
    expect(enumLabel('conversationIntent', 'paused')).toBe('Paused')
  })
})
