# Octopus Templates

本目录提供 Octopus 的通用模板资产，服务于 [`AI_DEVELOPMENT_PROTOCOL.md`](../AI_DEVELOPMENT_PROTOCOL.md) 与 [`DELIVERY_GOVERNANCE.md`](../DELIVERY_GOVERNANCE.md)。

## 模板清单

- [`task-slice-card.md`](./task-slice-card.md)：在非 trivial 任务开始前收敛最小切片
- [`contract-template.md`](./contract-template.md)：定义正式对象、状态机、事件或接口契约
- [`implementation-plan-template.md`](./implementation-plan-template.md)：把切片转成可执行实施步骤

## 使用顺序

推荐顺序如下：

1. 先填写 `task-slice-card.md`
2. 若任务涉及正式契约，再填写 `contract-template.md`
3. 若任务涉及多步骤实施，再填写 `implementation-plan-template.md`

## 自然语言路由

人类不需要手工判断该先填哪份模板。默认由 AI 按请求内容自动路由：

| 如果用户这样提需求 | AI 应选择什么 |
| --- | --- |
| “帮我分析 / 收敛 / 评审 / 规划这件事” | 先用 `task-slice-card.md` |
| “帮我定义 Run / Approval / Trace 这类正式对象或状态” | `task-slice-card.md` + `contract-template.md` |
| “帮我开始做这个切片 / 写实施方案 / 推进开发” | `task-slice-card.md` + `implementation-plan-template.md` |
| “这个需求既涉及对象定义，也涉及后续实施” | `task-slice-card.md` + `contract-template.md` + `implementation-plan-template.md` |
| “只是改一句话 / 修一个明显错字” | 可跳过模板 |

AI 应在开始 substantial work 前明确说明自己选择了哪条模板路由。

## 约束

- 模板是 SOP 入口，不是真相源
- 模板只能细化 `AGENTS / PRD / SAD / 规范文档`，不能改写它们
- 模板改动后，应同步检查 [`AI_DEVELOPMENT_PROTOCOL.md`](../AI_DEVELOPMENT_PROTOCOL.md) 与 [`DELIVERY_GOVERNANCE.md`](../DELIVERY_GOVERNANCE.md)
