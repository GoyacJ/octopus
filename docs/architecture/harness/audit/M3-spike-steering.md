# M3 Spike · Steering Queue Long Turn

> 状态：M3-S02 已提交待评审
> 范围：ADR-0017 safe merge point、capacity、TTL、prompt cache 影响

## 结果

- 长 turn 中的 steering message 不在 model/tool 执行中途合并。
- driver 只在 safe merge point 调用 `drain_and_merge`。
- capacity 超限触发 `DropOldest`，并记录 `SteeringMessageDroppedEvent`。
- TTL 过期消息在 drain 时丢弃，并记录 `TtlExpired`。
- spike 的 prompt cache 模型验证 steering 追加在 user 滚动区，不改变 stable prefix；对照组损失为 0%，低于 5% 阈值。

## 覆盖

```bash
cargo test -p octopus-harness-session --test spike_steering --all-features
```

## 边界

- 本 spike 使用 mock long-turn driver。
- 不引入正式 engine。
- 真 engine 的 safe merge point 接入留给 M5。

