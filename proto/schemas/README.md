# schemas

`proto/schemas/` 保存插件、扩展和 manifest 的正式 schema。

## 当前阶段

1. 先提供 `PluginManifest` 最小 schema。
2. 仅描述扩展治理需要的元数据、入口、权限和能力声明。
3. 实际插件宿主和注册实现必须引用同一份 schema，而不是手写重复结构。
