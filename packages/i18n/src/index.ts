import { createI18n } from 'vue-i18n'

export const supportedLocales = ['zh-CN', 'en-US'] as const
export type SupportedLocale = (typeof supportedLocales)[number]

export const messages = {
  'zh-CN': {
    navigation: {
      overview: '总览',
      workspaces: '工作区',
      agents: 'Agents',
      runs: 'Runs',
      inbox: '收件箱',
      triggers: '触发器',
      artifacts: '产物',
      extensions: '扩展',
      nodes: '节点',
      audit: '审计',
      settings: '设置',
    },
    shell: {
      workspace: '默认工作区',
      tagline: '治理优先的 Agent OS 控制面骨架',
      later: '后续阶段',
      theme: '主题',
      language: '语言',
    },
    pages: {
      overviewTitle: '控制面骨架已接入',
      overviewBody: '当前阶段优先交付 Runs、Inbox、Audit 与统一壳层。',
      workspacesTitle: '工作区边界',
      workspacesBody: '后续将补齐成员、权限、策略与导入导出治理。',
      agentsTitle: 'Agent 资产视图',
      agentsBody: 'Agent 将作为工作区内的长期可治理资产落位。',
      runsTitle: 'Runs 优先于聊天入口',
      runsBody: '首条 MVP 将围绕 run、interaction、approval、resume 和 timeline 展开。',
      inboxTitle: '统一收件箱',
      inboxBody: 'ask-user、审批和待恢复事项统一进入 Inbox，而不是分散入口。',
      auditTitle: '审计与时间线',
      auditBody: '执行轨迹、风险动作与恢复行为都需要被审计和回放。',
    },
    status: {
      ready: '当前阶段',
      later: '后续解锁',
    },
  },
  'en-US': {
    navigation: {
      overview: 'Overview',
      workspaces: 'Workspaces',
      agents: 'Agents',
      runs: 'Runs',
      inbox: 'Inbox',
      triggers: 'Triggers',
      artifacts: 'Artifacts',
      extensions: 'Extensions',
      nodes: 'Nodes',
      audit: 'Audit',
      settings: 'Settings',
    },
    shell: {
      workspace: 'Default Workspace',
      tagline: 'Governance-first Agent OS control plane skeleton',
      later: 'Later phase',
      theme: 'Theme',
      language: 'Language',
    },
    pages: {
      overviewTitle: 'Control plane shell is wired',
      overviewBody: 'This phase prioritizes Runs, Inbox, Audit, and the shared application shell.',
      workspacesTitle: 'Workspace boundaries',
      workspacesBody: 'Membership, policy, and blueprint governance land in later phases.',
      agentsTitle: 'Agent assets view',
      agentsBody: 'Agents will be managed as durable governed assets inside a workspace.',
      runsTitle: 'Runs before chat surfaces',
      runsBody: 'The first MVP slice focuses on run, interaction, approval, resume, and timeline.',
      inboxTitle: 'Unified inbox',
      inboxBody: 'Ask-user, approval, and resume work items converge into one Inbox.',
      auditTitle: 'Audit and timeline',
      auditBody: 'Execution traces, risky actions, and resume behavior must stay replayable.',
    },
    status: {
      ready: 'Current phase',
      later: 'Later',
    },
  },
} as const

export function createOctopusI18n(initialLocale: SupportedLocale = 'zh-CN') {
  return createI18n({
    legacy: false,
    locale: initialLocale,
    fallbackLocale: 'en-US',
    messages,
  })
}
