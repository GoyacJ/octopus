# proto

`proto/` 是正式契约源目录，不存放运行时代码。

## 子目录职责

1. `openapi/`
2. `grpc/`
3. `schemas/`

## 契约源规则

1. 外部控制面 HTTP 契约只放在 `openapi/`。
2. 内部 gRPC / Protobuf 契约只放在 `grpc/`。
3. 插件、扩展、manifest 等 schema 只放在 `schemas/`。
4. 任何运行时代码、手工 DTO 和业务逻辑都不得进入 `proto/`。

## 版本与命名

1. 契约文件使用显式版本后缀或版本目录，如 `v1`。
2. 新版本优先新增，不直接覆盖旧版本的已对外承诺结构。
3. OpenAPI、Protobuf、Schema 的版本语义应保持一致，避免跨层漂移。

## 生成与校验

1. OpenAPI 是外部 HTTP API 的唯一契约源。
2. Protobuf + Buf 是内部 RPC 的唯一契约源。
3. JSON Schema 是插件与扩展 manifest 的唯一契约源。
4. 后续前端类型、后端传输层和适配层必须基于这些契约生成或映射，禁止手写游离副本。
5. 契约成型后，默认校验入口包括：
   - `buf lint`
   - OpenAPI lint
   - 前后端生成代码同步检查

## 文档同步

以下变更必须同步相关文档：

1. 新增或调整 OpenAPI 资源。
2. 新增或调整 Protobuf service / message。
3. 新增或调整插件 manifest schema。
4. 影响运行时边界、扩展机制或组件 API 的契约改动。
