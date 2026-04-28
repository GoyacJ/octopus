# M5 Engine Result Inject Gate

> 范围：M5-T13
> 结论：通过。M5 可继续 M5-T14 CapabilityRegistry 装配。

## 覆盖

- `result_inject.rs` 把 assistant tool call 转为 `MessagePart::ToolUse`，把工具结果转为 `MessageRole::Tool` + `MessagePart::ToolResult`。
- 主循环改为 bounded iteration loop，工具结果进入同一 run 的工作消息历史。
- 注入给模型的是 ResultBudget 处理后的 `ToolResult`。
- `ToolResultOffloaded` 从 tool emitter 进入 engine event stream。
- `OverflowAction::Reject` 作为工具失败事件和 tool result 文本回注，不升级为 engine run error。
- 多轮工具调用受 `max_iterations` 限制，`RunEnded` 只写一次。

## 验证

```bash
cargo test -p octopus-harness-engine --test main_loop --all-features
```

## 后续

- 下一任务卡：M5-T14 `CapabilityRegistry assembly`。
- 不启动 M6 / M7；仍等待 M5 完成。
