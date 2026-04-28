# M5 Skill Plugin Source Gate

> 范围：M5-T09.5
> 结论：通过。M5 可继续 M5-T10 EngineRunner trait + Engine 骨架。

## 覆盖

- `PluginSource` 从 `plugin_root/skills/*.md` 加载 skill。
- 加载结果使用 `SkillSource::Plugin(plugin_id)`。
- 不新增 `harness-skill -> harness-plugin` 依赖；仅使用 `harness-contracts::PluginId`。
- 非 `skills/` 目录下的同级 markdown 不作为 plugin skill 加载。

## 验证

```bash
cargo test -p octopus-harness-skill --all-features
cargo clippy -p octopus-harness-skill --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
bash scripts/spec-consistency.sh
bash scripts/dep-boundary-check.sh
cargo check --workspace --all-features
```

## 后续

- 下一任务卡：M5-T10 `EngineRunner trait + Engine skeleton`。
- 不启动 M6 / M7；仍等待 M5 完成。
