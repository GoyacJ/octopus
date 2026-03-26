# Octopus Contracts

本目录承载 Octopus 在 `doc-first rebuild` 阶段的正式 contract 文档，用于把 `PRD/SAD` 中已经确认的对象语义收敛成可引用的契约层。

## 当前目录结构

- [`runtime/run.md`](./runtime/run.md)：`Run` 权威执行壳 contract
- [`runtime/trace-event.md`](./runtime/trace-event.md)：`TraceEvent` 观测事件 envelope contract
- [`runtime/capability-visibility-tool-search.md`](./runtime/capability-visibility-tool-search.md)：`CapabilityVisibilityResult` 与 `ToolSearchResultItem` contract
- [`governance/approval-request.md`](./governance/approval-request.md)：`ApprovalRequest` contract
- [`interaction/inbox-item.md`](./interaction/inbox-item.md)：`InboxItem` contract

## 使用规则

1. contract 只固化正式对象、状态机、事件 envelope、actor/scope 和治理约束
2. contract 不等于 API 路由、数据库表结构或具体代码类型
3. contract 只能细化 [`docs/PRD.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/PRD.md) 与 [`docs/SAD.md`](/Users/goya/Work/weilaizhihuigu/super-agent/octopus/docs/SAD.md)，不能改写它们
4. 进入实现前，切片级 plan 与代码实现都应直接引用这些 contract，而不是重新在聊天中解释对象语义

## 当前范围

首批 contract 只覆盖 Phase 2 既定的五类对象：

本轮 5 份 contract 已作为 Phase 2 文档基线收口；后续切片若需扩展对象集合，应按同样路径补充 contract，而不是在实现中临时发明对象语义。

- `Run`
- `ApprovalRequest`
- `InboxItem`
- `TraceEvent`
- `CapabilityVisibility / ToolSearch`

以下内容明确不在本轮：

- `A2A`
- `DiscussionSession`
- `ResidentAgentSession`
- `Org Knowledge Graph`
- API wire 形状、数据库表字段、测试实现
