import { createI18n } from 'vue-i18n'

export const messages = {
  'zh-CN': {
    app: {
      subtitle: 'Phase 1 控制面壳，用于承接统一对象模型、主题与多语言基线。',
      locale: '中文',
      theme: '主题',
      light: '浅色',
      dark: '深色',
    },
    demo: {
      eyebrow: '首条纵切片',
      title: '本地演示 Task -> Approval -> Resume -> Artifact',
      subtitle: '用当前最小 HTTP runtime 打通一次真实的受控 Run，并在同一页面展示状态、追踪和审计。',
      mode: 'Local hub demo',
      genericError: '请求失败，请检查本地 runtime 是否已启动。',
      emptyState: '尚未启动',
      emptyDescription: '提交一个需要审批的 task 后，这里会出现 Run / Approval / Inbox 的当前状态。',
      na: 'N/A',
      artifactPending: 'Artifact 会在无需审批的直接完成场景，或审批通过后的 resume 场景中出现。',
      tracePending: 'Trace 事件会在提交、审批和恢复后逐步累积。',
      auditPending: 'Audit 记录会在关键动作落地后出现。',
      fields: {
        title: 'Task title',
        description: 'Task description',
        requestedBy: 'Requested by',
        reviewedBy: 'Reviewed by',
        requiresApproval: '此 demo 默认走审批路径',
      },
      actions: {
        submit: '提交 Task',
        submitting: '提交中...',
        approve: '批准审批',
        reject: '拒绝审批',
        resolving: '处理中...',
        resume: '恢复执行',
        resuming: '恢复中...',
      },
      sections: {
        run: 'Run Detail',
        artifact: 'Artifact',
        trace: 'Trace',
        audit: 'Audit',
      },
      summary: {
        runId: 'Run ID',
        approvalId: 'Approval ID',
        reviewedBy: 'Reviewed By',
        inboxState: 'Inbox State',
      },
    },
  },
  'en-US': {
    app: {
      subtitle: 'Phase 1 control shell for the unified object model, theme, and i18n baseline.',
      locale: 'English',
      theme: 'Theme',
      light: 'Light',
      dark: 'Dark',
    },
    demo: {
      eyebrow: 'First Vertical Slice',
      title: 'Local Task -> Approval -> Resume -> Artifact Demo',
      subtitle: 'Use the current minimal HTTP runtime to walk one controlled run and keep its state, trace, and audit visible on the same page.',
      mode: 'Local hub demo',
      genericError: 'Request failed. Verify the local runtime is running.',
      emptyState: 'Not started',
      emptyDescription: 'Submit a task that requires approval and this panel will start showing the current run, approval, and inbox state.',
      na: 'N/A',
      artifactPending: 'The artifact appears after a direct completion path or after an approved run resumes.',
      tracePending: 'Trace events will accumulate after submit, approval, and resume.',
      auditPending: 'Audit entries appear once the runtime records key actions.',
      fields: {
        title: 'Task title',
        description: 'Task description',
        requestedBy: 'Requested by',
        reviewedBy: 'Reviewed by',
        requiresApproval: 'Keep this demo on the approval path',
      },
      actions: {
        submit: 'Submit Task',
        submitting: 'Submitting...',
        approve: 'Approve',
        reject: 'Reject',
        resolving: 'Resolving...',
        resume: 'Resume Run',
        resuming: 'Resuming...',
      },
      sections: {
        run: 'Run Detail',
        artifact: 'Artifact',
        trace: 'Trace',
        audit: 'Audit',
      },
      summary: {
        runId: 'Run ID',
        approvalId: 'Approval ID',
        reviewedBy: 'Reviewed By',
        inboxState: 'Inbox State',
      },
    },
  },
} as const

export type AppLocale = keyof typeof messages

export const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  fallbackLocale: 'en-US',
  messages,
})
