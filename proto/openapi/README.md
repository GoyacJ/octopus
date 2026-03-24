# openapi

`proto/openapi/` 保存对外控制面 HTTP API 的正式契约源。

## 当前阶段

1. 先建立资源边界与基础 schema。
2. 优先覆盖 `workspaces`、`agents`、`runs`、`inbox`、`audit`、`resume`。
3. 在实现落地前，避免手写漂移 DTO。

## 维护规则

1. 文件名采用 `<domain>.v<major>.yaml`。
2. breaking 变更必须显式通过新版本或 ADR 说明。
3. 与前端 client 生成和服务端 transport 映射保持同步。
