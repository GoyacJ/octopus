# ADR 0001 · 契约源目录与 Phase 1 骨架

- 状态：Accepted
- 日期：2026-03-25

## 背景

仓库已完成 PRD / SAD 重构，但仍停留在文档主导状态。后续若直接进入代码实现，会面临两个风险：

1. 核心对象、共享枚举和事件骨架继续散落在 PRD / SAD 中，缺少实现层可直接消费的契约源。
2. 前端、Hub、Server 和桌面端壳没有正式目录边界，容易在 Phase 1 之前就出现职责漂移。

## 决策

本仓库接受以下决策：

1. 使用 `contracts/v1/core-objects.json`、`contracts/v1/enums.json`、`contracts/v1/events.json` 作为机器可读契约源。
2. 使用 `docs/CONTRACTS.md` 作为契约冻结说明文档，并将其纳入正式文档入口。
3. 建立以下骨架目录：
   - `apps/octopus-client`
   - `packages/contracts`
   - `crates/octopus-hub`
   - `crates/octopus-server`
   - `crates/octopus-tauri`
4. `octopus-hub` 承担核心领域与运行时骨架；`octopus-server` 承担 HTTP/SSE 适配；`octopus-tauri` 只承载桌面桥接壳层语义，不在本阶段声明完整桌面运行能力。

## 后果

正面影响：

- Phase 0 的对象语义和枚举值有了正式契约源。
- Phase 1 的目录边界与技术基线可落地为可构建仓库。
- 后续可在不重写目录结构的前提下推进 GA 纵切片。

代价与限制：

- 需要维护中文说明文档与机器可读契约源的同步。
- Rust 与 TypeScript 在早期仍可能存在手工镜像，需要依靠测试防漂移。
- `octopus-tauri` 在本阶段只提供壳层骨架，不能被描述为已完成桌面应用集成。

