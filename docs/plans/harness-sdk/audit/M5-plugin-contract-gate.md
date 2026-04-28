# M5 Plugin Contract Gate

> 范围：M5-T09
> 结论：通过。M5 可继续 M5-T09.5 Skill plugin source 集成。

## 覆盖

- `tests/contract.rs` 接入 ManifestLoader / RuntimeLoader 一致性 contract。
- Discovery 阶段的 ManifestLoader validation error 不进入 RuntimeLoader。
- Activate 阶段按声明顺序选择第一个 `can_load == true` 的 RuntimeLoader。
- RuntimeLoader 返回的 `Plugin::manifest()` 必须与 Discovery 阶段的 `ManifestRecord.manifest` 一致。

## 验证

```bash
cargo test -p octopus-harness-plugin --all-features
cargo clippy -p octopus-harness-plugin --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
bash scripts/spec-consistency.sh
bash scripts/dep-boundary-check.sh
cargo check --workspace --all-features
```

## 后续

- 下一任务卡：M5-T09.5 `Skill plugin source integration`。
- 不启动 M6 / M7；仍等待 M5 完成。
