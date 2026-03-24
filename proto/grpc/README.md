# grpc

`proto/grpc/` 保存内部 `Control Plane ↔ Node Runtime` 的 Protobuf 契约源。

## 当前阶段

1. 先定义 node 注册、run 派发、执行事件上报和恢复握手的最小接口。
2. 包名和 service 名保持显式版本化。
3. 后续 Rust transport、节点执行面和测试夹具都应以此目录为准。
