# ADR 0001: Monorepo and Boundary Rules

- Status: Accepted
- Date: 2026-03-26

---

## Context

Octopus 当前处于文档优先重建阶段，需要把目标态架构、首版 GA 实施边界与 AI 开发规则统一到同一套仓库边界模型中。

如果没有明确的 monorepo 与边界规则，后续很容易出现以下问题：

- 共享逻辑堆积到应用目录
- Rust 与 TypeScript 各自定义一套契约
- 运行时核心逻辑与 UI 逻辑相互渗透
- 契约、实现、文档的事实源不一致
- AI 因上下文局部最优而持续产生越层实现

因此需要在仓库层面对“目录职责、契约事实源、依赖方向、禁止事项”做出明确决策。

---

## Decision

### 1. 采用 Monorepo 组织方式

仓库采用 monorepo，统一承载：

- 最终应用
- Rust 共享库
- TypeScript 共享包
- 跨语言共享 schema
- 架构与工程规范文档
- ADR 与实施蓝图

### 2. 顶层目录职责固定

#### `apps/`

用于最终应用和装配层。  
不作为共享核心逻辑事实源。

#### `crates/`

用于 Rust 共享库与运行时核心实现。  
承载核心运行时、领域逻辑、基础设施适配等 Rust 侧能力。

#### `packages/`

用于 TypeScript / 前端共享包。  
承载 UI、前端共享消费层、前端 API client、前端共享状态模型等。

#### `schemas/`

用于跨语言共享契约事实源。  
凡是跨 Rust / TypeScript 共同依赖的 command、query、event、DTO、关键状态枚举，应优先进入此目录。

### 3. `schemas/` 是跨语言共享契约的唯一事实源

不允许在 `crates/` 与 `packages/` 中分别维护平行版本的共享契约。

### 4. 应用层不得承载共享核心逻辑

`apps/` 只做装配。  
一旦逻辑具有复用潜力或属于平台核心能力，应提取到 `crates/` 或 `packages/`。

### 5. 必须继续遵守架构分层

即使在同一顶层目录中，也必须继续遵守：

- Entry / Interface
- Application
- Domain
- Infrastructure

禁止因为“都在同一个 crate / package / app 中”而放松边界。

### 6. 契约变更先于实现

任何跨语言共享对象或公共接口变更，必须先更新 `schemas/`，再推进实现与消费层调整。

### 7. 核心结构决策必须通过 ADR 沉淀

以下事项必须通过 ADR 记录：

- 新增核心领域对象
- 调整顶层目录职责
- 修改契约事实源规则
- 引入新运行模型
- 调整关键状态机语义
- 调整授权 / 预算 / 审批求值边界

---

## Consequences

### Positive

- 仓库真相归属更清晰
- 契约一致性更强
- 更适合 AI 开发与自动化生成
- 更容易控制共享逻辑漂移
- 更容易做阶段门与结构化评审
- 更适合长期演进

### Negative

- 初期需要更多结构性约束
- 新增对象时需要先思考目录归属
- 契约变更流程会比“直接写代码”更慢
- 需要持续维护 ADR 与相关文档

### Trade-off

我们接受前期约束变多，以换取中后期复杂系统的可维护性、可解释性与多语言一致性。

---

## Rejected Alternatives

### 1. 以应用目录为中心，后续再抽共享逻辑

拒绝原因：

- 很容易导致共享逻辑长期滞留在 app 中
- AI 开发会反复沿最短路径堆逻辑
- 后续抽离成本高，且会污染边界

### 2. 让 Rust 与 TypeScript 各自定义契约，再人工对齐

拒绝原因：

- 容易漂移
- 容易发生字段、语义、状态不一致
- 会削弱 schema 作为边界事实源的价值

### 3. 不做严格目录职责，按功能自由摆放

拒绝原因：

- 早期看似灵活，后期很难治理
- 对 AI 开发极不友好
- 会让评审与重构成本显著升高

---

## Rules Derived from This ADR

基于本 ADR，立即生效以下规则：

1. 新增共享契约时，先检查是否应进入 `schemas/`
2. 新增共享 Rust 逻辑时，优先进入 `crates/`
3. 新增前端共享逻辑时，优先进入 `packages/`
4. `apps/` 不得成为共享真相存放地
5. 契约变更必须先于实现
6. 目录归属不明确时，不得直接编码
7. 任何核心边界调整都应补 ADR

---

## Follow-up

建议后续补充：

- `docs/governance/repo-structure-guidelines.md`
- `docs/governance/schema-first-guidelines.md`
- `docs/governance/ai-phase-gates.md`
- `docs/architecture/ga-implementation-blueprint.md`

---

## Summary

Octopus 采用 monorepo，不是为了“把代码放在一个仓库里”，而是为了把应用、共享实现、跨语言契约与架构边界纳入同一个可治理的事实体系中。
