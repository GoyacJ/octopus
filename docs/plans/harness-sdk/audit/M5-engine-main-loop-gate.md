# M5 Engine Main Loop Gate

> 范围：M5-T11
> 结论：通过。M5 可继续 M5-T12 中断 + EndReason 映射。

## 覆盖

- `EngineBuilder::build()` 对缺失依赖 fail-closed。
- `EngineRunner::run(session, input, ctx)` 校验 session/context 一致性。
- 主循环写入 `RunStarted`、`UserMessageAppended`、`AssistantDeltaProduced`、`AssistantMessageCompleted`、`RunEnded`。
- 工具调用路径写入 `ToolUseRequested`、权限请求 / 解析事件、审批事件、`ToolUseCompleted`。
- 模型 infer 失败和 stream 中段失败均写入 `RunEnded(Error)`。
- iteration budget 的 grace call 场景写入 `GraceCallTriggered`。

## 边界

- 本卡只完成 engine-owned turn 编排的最小主循环。
- 工具结果重新注入与多轮工具迭代留给 M5-T13。
- 中断来源与 `EndReason::Cancelled { initiator }` 映射留给 M5-T12。
- Capability assembly 留给 M5-T14。

## 验证

```bash
cargo test -p octopus-harness-engine --all-features
cargo clippy -p octopus-harness-engine --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
bash scripts/spec-consistency.sh
bash scripts/dep-boundary-check.sh
cargo check --workspace --all-features
```

## 后续

- 下一任务卡：M5-T12 `Interrupt + EndReason mapping`。
- 不启动 M6 / M7；仍等待 M5 完成。
