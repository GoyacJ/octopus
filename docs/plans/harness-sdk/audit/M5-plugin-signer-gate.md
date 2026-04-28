# M5 Plugin Signer Gate

> 日期：2026-04-27
> 范围：M5-T07
> 结论：通过。M5 可继续 M5-T08 PluginSource Discovery + dynamic-load。

## 覆盖

- `octopus-harness-plugin` 已新增 `TrustedSignerStore`、`TrustedSigner`、`SignerId`、`SignerProvenance` 与 `SignerStoreEvent`。
- `StaticTrustedSignerStore` 已覆盖 active signer 查询、精确 signer 查询与 revoked 判定。
- `ManifestSigner` 已实现 manifest canonical payload 生成与 Ed25519 验签。
- `PluginRegistry::discover()` 已在 manifest-only 阶段执行 AdminTrusted 签名校验，不实例化插件。
- `PluginRegistryBuilder` 已支持 `with_signer_store` / `with_trusted_signer`，并拒绝二者混用。
- `UserControlled` manifest 即使携带签名也不会升级 trust level。
- signer 治理与 ADR-0013 `IntegritySigner` 保持实现、依赖和配置隔离。

## Gate

通过项：

- `cargo test -p octopus-harness-plugin --all-features`
- `cargo clippy -p octopus-harness-plugin --all-targets --all-features -- -D warnings`
- `cargo fmt --all -- --check`
- `bash scripts/spec-consistency.sh`
- `bash scripts/dep-boundary-check.sh`
- `cargo check --workspace --all-features`

## 后续

- 下一任务卡：M5-T08 `4 源 PluginSource Discovery + dynamic-load`。
- M5 执行顺序保持不变：plugin 全部完成后再启动 engine。
