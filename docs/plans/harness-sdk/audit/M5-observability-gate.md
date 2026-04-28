# M5 Observability Gate

> 日期：2026-04-27
> 范围：M5-T01 / T02 / T03 / T03.5 / T04
> 结论：通过。M5 可进入 plugin 阶段，engine 仍不得提前启动。

## 覆盖

- `octopus-harness-observability` 已实现 Tracer / OTel shell / UsageAccumulator / DefaultRedactor / ReplayEngine。
- Redactor contract 覆盖 `NoopRedactor` 与 `DefaultRedactor` 幂等性。
- Journal redactor pipeline 覆盖 `JsonlEventStore` / `SqliteEventStore` / `InMemoryEventStore` 写入前脱敏。
- ReplayEngine 覆盖 EventStore replay stream、projection reconstruction、session diff、JSONL / Markdown export。

## Gate

通过项：

- `cargo test -p octopus-harness-observability --all-features`
- `cargo clippy -p octopus-harness-observability --all-targets --all-features -- -D warnings`
- `cargo test -p octopus-harness-skill --all-features`
- `cargo test -p octopus-harness-mcp --all-features`
- `cargo clippy -p octopus-harness-observability -p octopus-harness-mcp -p octopus-harness-skill --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `bash scripts/spec-consistency.sh`
- `bash scripts/dep-boundary-check.sh`
- `cargo check --workspace --all-features`

## 后续

- 下一任务卡：M5-T05 `octopus-harness-plugin` ManifestLoader / RuntimeLoader 二分接口。
- M5 执行顺序保持不变：plugin 完成后再启动 engine。
