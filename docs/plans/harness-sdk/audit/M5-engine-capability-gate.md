# M5 Engine Capability Gate

> 范围：M5-T14
> 结论：通过。M5 可继续 M5-T15 Engine Contract Test + E2E。

## 覆盖

- `with_blob_store(...)` 在 build 时安装 `ToolCapability::BlobReader`。
- `CapabilityRegistry::contains(...)` 支持 engine build 校验工具 `required_capabilities`。
- `EngineBuilder::with_capability<T>(...)` 支持显式 capability 注入。
- builder 显式注入覆盖 base registry 同名 capability。
- 缺失 required capability 时 build fail-closed。
- `subagent-tool` 仅保持编译边界，不引入 M6 行为。

## 验证

```bash
cargo test -p octopus-harness-engine --test capability --all-features
cargo test -p octopus-harness-engine --test main_loop --all-features
```

## 后续

- 下一任务卡：M5-T15 `Engine Contract Test + E2E`。
- 不启动 M6 / M7；仍等待 M5 完成。
