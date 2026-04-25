# ADR-0012 · Capability Testing Boundary（`MockCapabilityRegistry`）

> 状态：Accepted  
> 日期：2026-04-25  
> 关联：ADR-0011（Tool Capability Handle）、ADR-0006（Plugin Trust Levels）

## 1. 背景

ADR-0011 引入了 `ToolCapability` 与 `CapabilityRegistry`，并在落地清单中提到 `harness-contracts/testing` 下的 `MockCapabilityRegistry`。  
当前文档未明确测试 mock 的边界，存在两类风险：

1. 业务层误把 mock 注册器用于生产装配，绕开 trust/capability 校验；
2. 测试为了方便注入了生产不可用 capability，导致“测试可过、生产不可跑”。

## 2. 决策

### 2.1 `MockCapabilityRegistry` 只允许存在于 testing feature

- `MockCapabilityRegistry` 仅在 `harness-contracts` 的 `testing` feature gate 导出；
- release/profile 默认不编译该实现；
- 生产代码路径（`harness-engine` / `harness-tool` / `harness-session`）不得依赖 mock trait 或 mock type。

### 2.2 测试 mock 不绕开策略校验

- `ToolRegistry::register` 在测试中仍执行 `CapabilityPolicy::check`；
- mock 只负责“提供 capability handle”，不负责“授予 capability 权限”；
- 任何 `UserControlled` 来源工具申请敏感 capability，测试与生产都应得到同一 `RegistrationError::CapabilityNotPermitted`。

### 2.3 测试分层约束

- 单元测试：可注入最小 mock capability 集，聚焦工具业务逻辑；
- 集成测试：必须通过 `EngineBuilder` 的生产装配路径构造 `CapabilityRegistry`；
- 端到端测试：禁止直接构造 mock registry，必须走真实 `HarnessBuilder`。

## 3. 影响

### 正向

- 避免 mock 侵入生产路径；
- 保证 capability 权限矩阵在测试/生产行为一致；
- 降低“测试双轨”造成的维护成本。

### 代价

- 部分测试需改为通过 builder 注入真实适配器，样板代码略增；
- 需要在 CI 增加一次 `--no-default-features --features testing` 的契约检查。

## 4. 参考证据

| Evidence ID | 来源 | 要点 |
|---|---|---|
| CC-08 | `docs/architecture/reference-analysis/evidence-index.md` | subagent/tool 能力通过 context 注入，边界需稳定 |
| OC-16 | `docs/architecture/reference-analysis/evidence-index.md` | capability 合约是插件扩展的主接口 |
| HER-008 | `docs/architecture/reference-analysis/evidence-index.md` | agent 级工具能力若无显式边界，容易在主循环硬编码 |
| ADR-006 | `docs/architecture/harness/adr/0006-plugin-trust-levels.md` | trust 分层已是既有基线，capability 策略必须与之一致 |

## 5. 落地清单

| 项 | 责任模块 | 说明 |
|---|---|---|
| `MockCapabilityRegistry` gate 注释 | `harness-contracts` | 在 testing 导出处补“仅测试使用”注释 |
| 测试装配样例 | `harness-engine.md` / `harness-tool.md` | 区分单测 mock 与集成真实装配 |
| CI 检查 | 仓库 CI | 增加 testing feature 组合编译与契约测试 |
