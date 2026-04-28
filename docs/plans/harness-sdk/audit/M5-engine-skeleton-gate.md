# M5 Engine Skeleton Gate

> 范围：M5-T10
> 结论：通过。M5 可继续 M5-T11 主循环。

## 覆盖

- `EngineRunner` trait 在 `harness-engine` 内定义并保持对象安全。
- `Engine` / `EngineBuilder` 提供稳定 `EngineId`。
- `LoopState` 暴露 AwaitingModel / ProcessingToolUses / ApplyingHookResults / MergingContext / Ended 五态。
- `EngineRunner::run` 在 M5-T10 仅返回明确未实现错误；主循环实现留给 M5-T11。

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

- 下一任务卡：M5-T11 `Engine main loop`。
- 不启动 M6 / M7；仍等待 M5 完成。
