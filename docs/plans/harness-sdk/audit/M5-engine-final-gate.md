# M5 Engine Final Gate

> 范围：M5-T15 / M5 final
> 结论：通过，含 final gate repair。M5 进入待评审状态；M6 / M7 未启动。

## 覆盖

- Engine E2E 覆盖 `Engine.run(session) → model tool call → tool execute → tool result reinject → model final response → RunEnded`。
- ResultBudget offload 后只把 budgeted result 注入模型上下文。
- Capability missing 在 engine build 阶段 fail-closed。
- grace call、cancellation initiator、iteration budget 和单次 `RunEnded` 由 engine test suite 覆盖。
- EventStore redactor pipeline 在正式 engine E2E 中覆盖持久化路径。
- `crates/octopus-harness-session/tests/e2e_minimal.rs` 不存在；M3 `run_turn.rs` 保留。

## Repair 覆盖

- 真实 provider 的 `ToolUseStart + ToolUseInputJson` 流已在 engine 层聚合为 `ToolCall`，不要求 provider 产出 `ToolUseComplete`。
- grace call 状态下模型继续发起 tool call 时，Engine 直接 `RunEnded(MaxIterationsReached)`，不写 `ToolUseRequested`。
- Plugin coordinator strategy 需要 manifest `capabilities.coordinator_strategy` 声明后才注入 handle 并允许占用 `CoordinatorStrategy` slot。
- Engine 主循环通过 `SteeringDrain` 窄接口在每轮 infer 前 drain steering；默认未注入时 no-op。

## 验证

```bash
cargo test -p octopus-harness-engine --all-features
cargo test -p octopus-harness-tool --all-features
cargo test -p octopus-harness-context --all-features
cargo test -p octopus-harness-session --all-features
cargo test -p octopus-harness-observability --all-features
cargo test -p octopus-harness-plugin --all-features
cargo test -p octopus-harness-skill --all-features
cargo clippy -p octopus-harness-engine --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
bash scripts/spec-consistency.sh
bash scripts/dep-boundary-check.sh
cargo check --workspace --all-features
cargo test --workspace --all-features --no-fail-fast
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## 后续

- M5 待 maintainer 评审。
- M6 只能在 M5 gate 评审通过后单独规划。
