# M5 Plugin Sources Gate

> 范围：M5-T08
> 结论：通过。M5 可继续 M5-T09 Plugin Contract Test。

## 覆盖

- `FileManifestLoader` 扫描 workspace / user / project 文件源。
- JSON / YAML manifest 解析与 source trust mismatch 拒绝。
- `InlineManifestLoader` 作为测试 / helper source，不读文件。
- `StaticLinkRuntimeLoader` 仅在 `activate()` 路径按 `PluginId` 工厂加载。
- `dynamic-load` feature 只暴露 `DylibRuntimeLoader` unsupported 边界，不引入 unsafe。
- `PluginRegistryBuilder::build()` 未注入 loader 时默认装配 `FileManifestLoader` + `StaticLinkRuntimeLoader`。

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

- 下一任务卡：M5-T09 `Plugin Contract Test`。
- 不启动 M6 / M7；仍等待 M5 完成。
