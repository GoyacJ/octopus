# M5 Engine Interrupt Gate

> 范围：M5-T12
> 结论：通过。M5 可继续 M5-T13 ResultBudget 集成 + 工具结果注入。

## 覆盖

- `RunContext` 携带 per-run `CancellationToken`，`EngineRunner::run(session, input, ctx)` 签名保持不变。
- `InterruptCause` 映射到 `EndReason`：User / Parent / System 为主动 `Cancelled`，Timeout 为 `Interrupted`，Budget 为 `TokenBudgetExhausted`。
- 主循环在 hook 前、model infer 前、model stream 期间、tool dispatch 前和 tool dispatch 期间检查 cancellation。
- tool dispatch 期间取消会传播到 `harness_tool::InterruptToken`。
- cancellation 路径只写一次 `RunEnded`，且不写后续 assistant completion。

## 边界

- 本卡不实现真实 provider request abort。
- 工具结果重新注入与多轮工具迭代留给 M5-T13。
- Capability assembly 留给 M5-T14。

## 验证

```bash
cargo test -p octopus-harness-engine --test interrupt --all-features
cargo test -p octopus-harness-engine --all-features
cargo clippy -p octopus-harness-engine --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
bash scripts/spec-consistency.sh
bash scripts/dep-boundary-check.sh
cargo check --workspace --all-features
```

## 后续

- 下一任务卡：M5-T13 `ResultBudget integration + tool result injection`。
- 不启动 M6 / M7；仍等待 M5 完成。
