# Octopus 产品介绍 Deck

## Audience

- 面向第一次接触 Octopus 的客户、合作方、内部产品介绍场景。
- 默认受众偏企业决策者、产品负责人、技术负责人。

## Objective

- 用一份中文 deck 说明 Octopus 是什么。
- 说明它为什么不是普通聊天工具。
- 说明它怎么把任务变成可运行、可治理、可交付的智能体流水线。
- 说明它适合哪些团队与业务环节。

## Narrative Arc

1. 先定义产品。
2. 再解释工作流。
3. 再用界面和能力证明产品不是概念。
4. 再讲治理、部署和安全边界。
5. 最后收束到业务价值。

## Visual System

- Base: 暖白底 `#FBF7F2`
- Primary: Octopus 橙 `#FF7A1A`
- Secondary: 浅金 `#FFC36B`
- Surface: 奶白卡片 `#FFFDF9`
- Ink: 深灰黑 `#171717`
- Muted: 灰棕 `#6E6257`
- Support: 浅蓝 `#DCE9F8`
- Style: 大留白、圆角卡片、浅投影、橙色高亮、真实产品截图为主
- Typography: `PingFang SC` 优先；无该字体时回退系统中文无衬线

## Slide Plan

### Slide 1

- Title: `Octopus 产品介绍`
- Message: 把 AI 对话窗口变成高效的交付流水线。
- Supporting points:
  - 一个任务，一个智能体
  - 100% 本地运行
  - 从描述需求到交付物生成
- Visual:
  - 左侧大标题与一句定义
  - 右侧使用 `output/playwright/home-page.png` 的 hero 区域裁切
  - 角落放 `apps/website/public/logo.png`
- Sources:
  - `apps/website/locales/zh-CN.json` -> `site.description`
  - `apps/website/locales/zh-CN.json` -> `pages.product.heroTitle`
  - `apps/website/locales/zh-CN.json` -> `pages.product.heroHighlight`
  - `output/playwright/home-page.png`
  - `apps/website/public/logo.png`

### Slide 2

- Title: `它不是聊天窗口，而是数字员工操作系统`
- Message: Octopus 把一次性问答，变成持久化、可追踪、可审批的任务执行系统。
- Supporting points:
  - 聊天窗口只负责对话，不负责交付
  - Octopus 为每个任务分配持久化智能体
  - 任务、上下文、执行链路、交付物持续保留
- Layout:
  - 左侧“传统 AI 工具” vs “Octopus” 对比
  - 右侧放 `apps/website/public/screenshots/agent.png`
- Sources:
  - `apps/website/locales/zh-CN.json` -> `pages.product.body`
  - `apps/octopus-website/src/components/sections/hero.tsx`
  - `apps/octopus-website/src/components/sections/features.tsx`
  - `apps/website/public/screenshots/agent.png`

### Slide 3

- Title: `工作方式`
- Message: 从一句任务描述开始，Octopus 自动把上下文、执行和交付串成闭环。
- Supporting points:
  - 描述交付目标
  - 自动关联项目文件、历史工作、团队规范
  - 为任务派发自主智能体并持续运行
  - 全程监控、审批和结果沉淀
- Visual:
  - 上半部分 4 步流程图
  - 下半部分放 `apps/website/public/screenshots/conversation.png`
- Sources:
  - `apps/website/locales/zh-CN.json` -> `pages.home.subtitle`
  - `apps/octopus-website/src/components/sections/workflow.tsx`
  - `apps/octopus-website/src/components/sections/platform.tsx`
  - `apps/website/public/screenshots/conversation.png`

### Slide 4

- Title: `真实界面，不是概念演示`
- Message: 产品已经覆盖任务入口、指挥中心、数字员工和工具工作台。
- Supporting points:
  - 对话与任务入口
  - 项目仪表盘与状态监控
  - 数字员工资源池
  - 工具与 MCP 集成管理
- Visual:
  - 2x2 截图拼贴
  - `apps/website/public/screenshots/conversation.png`
  - `apps/website/public/screenshots/dashboard.png`
  - `apps/website/public/screenshots/agent.png`
  - `apps/website/public/screenshots/builtin.png`
- Sources:
  - `apps/octopus-website/src/components/sections/platform.tsx`
  - `apps/website/locales/zh-CN.json` -> `pages.product.features.desktop`
  - `apps/website/locales/zh-CN.json` -> `pages.product.features.sandbox`
  - `apps/website/locales/zh-CN.json` -> `pages.product.features.telemetry`
  - `apps/website/public/screenshots/conversation.png`
  - `apps/website/public/screenshots/dashboard.png`
  - `apps/website/public/screenshots/agent.png`
  - `apps/website/public/screenshots/builtin.png`

### Slide 5

- Title: `平台能力与治理边界`
- Message: 它既能干活，也知道边界在哪里。
- Supporting points:
  - 连接数据库、内部系统和外部工具
  - 浏览器与隔离环境支持自动调研和执行
  - 执行过程可回放、可审计
  - 权限、角色、菜单和访问范围可控
  - 100% 本地运行，支持离线与内网
  - 深度适配信创基础设施
- Visual:
  - 左侧放 `apps/website/public/screenshots/rbac.png`
  - 右侧 6 条能力与治理卡片
- Sources:
  - `apps/website/locales/zh-CN.json` -> `pages.product.governance`
  - `apps/website/locales/zh-CN.json` -> `pages.product.features.mcp`
  - `apps/website/locales/zh-CN.json` -> `pages.product.features.enterprise`
  - `apps/website/locales/zh-CN.json` -> `pages.home.features.private`
  - `apps/website/locales/zh-CN.json` -> `pages.home.features.localization`
  - `apps/website/locales/zh-CN.json` -> `pages.home.features.security`
  - `apps/website/public/screenshots/rbac.png`

### Slide 6

- Title: `它能帮哪些团队`
- Message: 从个人到企业，从市场到财务，Octopus 覆盖真实工作链路。
- Supporting points:
  - 个人：调研、写作、代码、知识管理
  - 团队：多智能体并行协作
  - 企业：私有化、权限治理、审计日志
  - 横向示例：市场、销售、研发、内容运营、客服、财务、人力
- Visual:
  - 左侧 3 个场景层级卡片
  - 右侧 6 个典型任务胶囊卡片
- Sources:
  - `apps/website/locales/zh-CN.json` -> `pages.scenarios`
  - `apps/octopus-website/src/components/sections/usecases.tsx`

### Slide 7

- Title: `把 AI 从零散工具，升级为正式生产体系`
- Message: Octopus 的价值不只是提升单点效率，而是把执行、治理和沉淀放进同一平台。
- Supporting points:
  - 降本增效：并行处理复杂任务
  - 统一治理：权限、审批、审计进入系统层
  - 平台升级：知识、成果物和执行数据持续沉淀
  - Closing line: `任务进，结果出`
- Visual:
  - 中间 3 张价值卡片
  - 底部放一条简洁 closing statement 和小尺寸 `dashboard.png`
- Sources:
  - `scripts/build-octopus-proposal-docx.mjs`
  - `apps/website/locales/zh-CN.json` -> `pages.product.title`
  - `apps/website/public/screenshots/dashboard.png`

## Asset Notes

- `output/docx-assets/*` 当前缺失，不作为依赖。
- 本次 deck 先只用仓库现有截图与品牌图。
- 若视觉密度不够，再补少量程序化形状和背景渐变，不引入会分散注意力的外部素材。

## Editability Plan

- 所有标题、正文、流程文字、标签、卡片内容都用原生文本对象。
- 截图只作为图像层，不承载关键说明文字。
- 不把关键信息烫进位图。
