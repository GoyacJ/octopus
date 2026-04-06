import { afterEach, describe, expect, it } from 'vitest'

import { enumLabel, resolveCopy, translate } from '@/i18n/copy'
import i18n from '@/plugins/i18n'

const originalLocale = i18n.global.locale.value

describe('desktop i18n smoke coverage', () => {
  afterEach(() => {
    i18n.global.locale.value = originalLocale
  })

  it('switches shell labels and enum labels between zh-CN and en-US', () => {
    i18n.global.locale.value = 'zh-CN'

    expect(translate('dashboard.header.eyebrow')).toBe('工作区 Dashboard')
    expect(translate('conversation.stream.title')).toBe('消息流')
    expect(translate('settings.tabs.general')).toBe('通用设置')
    expect(resolveCopy('sidebar.navigation.overview')).toBe('概览')
    expect(resolveCopy('connections.header.title')).toBe('工作区连接')
    expect(enumLabel('conversationIntent', 'paused')).toBe('已暂停')

    i18n.global.locale.value = 'en-US'

    expect(translate('dashboard.header.eyebrow')).toBe('Workspace Dashboard')
    expect(translate('conversation.stream.title')).toBe('Message Stream')
    expect(translate('settings.tabs.general')).toBe('General Settings')
    expect(resolveCopy('sidebar.navigation.overview')).toBe('Overview')
    expect(resolveCopy('connections.header.title')).toBe('Workspace Connections')
    expect(enumLabel('conversationIntent', 'paused')).toBe('Paused')
  })
})
