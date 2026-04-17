import { afterEach, describe, expect, it } from 'vitest'

import { enumLabel } from '@/i18n/copy'
import i18n from '@/plugins/i18n'
import { localizeModelRuntimeMessage } from '@/views/workspace/models-security'

const originalLocale = i18n.global.locale.value

describe('models i18n', () => {
  afterEach(() => {
    i18n.global.locale.value = originalLocale
  })

  it('localizes ApiKey copy, surface enums, and capability enums in zh-CN and en-US', () => {
    i18n.global.locale.value = 'zh-CN'
    expect(String(i18n.global.t('models.table.columns.credentialRef'))).toBe('ApiKey')
    expect(String(i18n.global.t('models.detail.credentialRef'))).toBe('ApiKey')
    expect(enumLabel('modelSurface', 'conversation')).toBe('对话')
    expect(enumLabel('modelCapability', 'tool_calling')).toBe('工具调用')

    i18n.global.locale.value = 'en-US'
    expect(String(i18n.global.t('models.table.columns.credentialRef'))).toBe('ApiKey')
    expect(String(i18n.global.t('models.detail.credentialRef'))).toBe('ApiKey')
    expect(enumLabel('modelSurface', 'conversation')).toBe('Conversation')
    expect(enumLabel('modelCapability', 'tool_calling')).toBe('Tool Calling')
  })

  it('maps known runtime feedback into localized model messages', () => {
    i18n.global.locale.value = 'zh-CN'
    expect(localizeModelRuntimeMessage(
      i18n.global.t,
      'workspace: unknown runtime config key `toolCatalog`',
    )).toBe('运行时配置中存在未知字段 toolCatalog。')

    i18n.global.locale.value = 'en-US'
    expect(localizeModelRuntimeMessage(
      i18n.global.t,
      'missing configured credential env var `OPENAI_API_KEY` for provider `openai`',
    )).toBe('Environment variable OPENAI_API_KEY is missing, so openai cannot connect.')
  })
})
