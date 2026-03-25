# contracts

`contracts/` 是 `octopus` 在 `doc-first rebuild` 阶段的机器可读契约源目录。

当前目录的目标不是锁定数据库表、HTTP payload 或 protobuf 细节，而是冻结：

- 核心对象最小字段集合
- 共享枚举值
- 事件骨架
- `CapabilityCatalog` schema 与首版 capability seed
- capability card 模板

## 目录说明

| 路径 | 作用 |
| --- | --- |
| `contracts/v1/core-objects.json` | 冻结核心对象与 capability runtime 对象的最小字段集合 |
| `contracts/v1/enums.json` | 冻结跨平面共享枚举 |
| `contracts/v1/events.json` | 冻结事件骨架与最小 payload 字段 |
| `contracts/v1/capabilities.json` | 冻结 capability descriptor schema 与首版 capability seed |
| `contracts/templates/capability-card.template.md` | 统一 capability card 模板 |

## 维护规则

- 文件名、对象名、字段名、枚举值与事件名使用英文。
- 中文语义解释以 [docs/CONTRACTS.md](../docs/CONTRACTS.md) 为准。
- 若 capability、对象、枚举或事件变化，必须同步更新本目录、`docs/CONTRACTS.md` 与相关 ADR。
- `alarm`、`reminder`、`recipes`、`weather`、`places` 与 provider-specific connectors 默认不进入首版 capability seed；若未来引入，只能先登记为 adapter 或 connector-backed capability。
