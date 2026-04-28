# M5 Plugin Registry Gate

> 日期：2026-04-27
> 范围：M5-T06
> 结论：通过。M5 可继续 M5-T07 TrustedSignerStore / ManifestSigner。

## 覆盖

- `octopus-harness-plugin` 已新增 `PluginRegistry`、`PluginRegistryBuilder`、`PluginLifecycleState` 与 `PluginRegistrySnapshot`。
- `PluginActivationContext` 已扩展为 capability-scoped handles。
- Capability handle 按 manifest 声明范围注入，未声明能力为 `None`。
- `ToolRegistration` / `HookRegistration` / `McpRegistration` / `SkillRegistration` / `MemoryProviderRegistration` / `CoordinatorStrategyRegistration` 已落位。
- Activation 只在 `PluginRegistry::activate` 路径调用 `PluginRuntimeLoader::load`。
- Activation result 已校验 `registered_* ⊆ declared_*`。
- `CapabilitySlotManager` 已覆盖 `MemoryProvider`、`CoordinatorStrategy`、`CustomToolset(name)` 独占槽位。

## Gate

通过项：

- `cargo test -p octopus-harness-plugin --all-features`
- `cargo clippy -p octopus-harness-plugin --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `bash scripts/spec-consistency.sh`
- `bash scripts/dep-boundary-check.sh`
- `cargo check --workspace --all-features`

## 后续

- 下一任务卡：M5-T07 `TrustedSignerStore + ManifestSigner`。
- M5 执行顺序保持不变：plugin 全部完成后再启动 engine。
