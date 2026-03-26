# ADR 20260326: Phase 1 Minimal Repo Topology

**状态**: `approved`
**日期**: `2026-03-26`

## 背景

Octopus 当前仓库已经回到 `doc-first rebuild` 基线。Phase 1 需要决定：未来进入实现阶段时，最小 repo topology 应该如何恢复，才能同时满足以下要求：

- 不把目标态误写成当前已实现现实
- 不一次性恢复历史全量 monorepo 骨架
- 保持 `Hub` 为事实源、`Run` 为权威执行壳
- 给 Phase 2 contract 与 Phase 4 最小代码骨架建立提供稳定边界

这是一个长期影响实现结构的决策，因此需要 ADR，而不是只在聊天或设计文档里口头约定。

## 备选方案

### 方案 A：最小 `apps + crates` 拓扑，`packages` 延后

- Phase 4 只引入 `apps/octopus-desktop/` 与 `crates/octopus-hub/`
- Remote Hub 入口 `crates/octopus-server/` 延后到 Phase 6
- root `Cargo.toml` 仅在 Rust 目录实际创建时引入
- root `package.json` 与 `pnpm-workspace.yaml` 延后到第二个 JS 消费者出现时
- `packages/` 默认不创建

### 方案 B：提前恢复完整 monorepo

- 在 Phase 4 之前或 Phase 4 刚开始时，直接恢复 `apps/`、`packages/`、`crates/`、root Rust workspace、root pnpm workspace、CI/workflows 等
- 用“未来一定会用到”作为预先建树的理由

### 方案 C：单一 Tauri app 拓扑，Hub 逻辑先放进 `src-tauri`

- 只恢复 `apps/octopus-desktop/`
- 先不创建 `crates/octopus-hub/`
- Hub 核心逻辑先放在 Tauri app 内部，未来再抽离

## 决策

选择 **方案 A：最小 `apps + crates` 拓扑，`packages` 延后**。

## 决策理由

1. 它是唯一同时满足 “最小化恢复” 与 “保留 Hub 核心可复用性” 的方案。
2. 相比方案 B，它不会把 Phase 4 变成对历史仓库的整体复活，也不会制造虚假的“已经具备完整 workspace”认知。
3. 相比方案 C，它不会把 Tauri 壳层与 Hub 核心耦合在一起，能为后续 Remote Hub 入口保留稳定复用边界。
4. 它与现有总计划顺序一致：Slice A 先围绕 Desktop + 本地 Hub 主链路完成，Remote Hub 入口留给 Phase 6。

## 影响

### 正面影响

- Phase 4 可以保持非常小的实现骨架
- Phase 2 contract 可以先稳定正式对象，再决定具体落点
- 后续 `crates/octopus-server/` 能复用 `crates/octopus-hub/`，而不是从 Tauri app 中倒抽核心逻辑
- TypeScript 共享代码只有在出现第二个真实消费者时才进入 `packages/`

### 负面影响

- 未来若出现共享 TS 代码，需要再单独引入 `packages/` 切片
- 远程 Hub 入口不会在第一批代码骨架中出现，需要到后续阶段再补

## 明确拒绝的做法

- 以“以后迟早会用到”为理由，提前恢复完整历史 `apps/ packages/ crates/` 树
- 在只有一个 JS 消费者时就引入 root pnpm workspace
- 先把 Hub 逻辑塞进 `src-tauri`，再指望后面无痛抽离
- 恢复根目录 `contracts/`，而不是使用 `docs/contracts/`

## 风险

- 若后续需求偷偷把 `Web`、`Mobile` 或 `A2A` 带入骨架阶段，方案 A 也会被错误扩写
- 若 Phase 4 未严格约束到 Slice A，团队仍可能借“顺手”恢复多余目录

## 后续动作

1. `[done]` 产出 Phase 1 的 task slice card
2. `[done]` 产出 Phase 1 的项目骨架设计文档
3. `[done]` 修正 `SAD` 中与当前 tracked tree 不一致的事实源表述
4. `[done]` 获得人类对该最小 topology 的确认，并完成 Phase 2 首批 contract 定义
