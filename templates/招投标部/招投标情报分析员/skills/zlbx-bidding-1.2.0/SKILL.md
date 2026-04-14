---
name: tender-search
description: 全网招中标数据扫描雷达·投标分析助手。当用户涉及以下任何场景时，必须使用此 skill：查询招标/中标公告、搜索标讯、查找临期/即将到期项目、商机预测、推荐潜在投标供应商、分析公司主营业务/历史中标、查询公司上下游合作客户与供应商、分析竞争对手/竞对企业、查询Top采购单位/Top中标单位/Top中标品牌、招中标数据统计分析（按月/季/年/省份/品牌等维度）、查询品牌型号历史中标单价/价格趋势、市场分析/行业分析/采购寻源/渠道拓展等采购与投标相关场景。即使用户没有提到「知了标讯」，只要涉及招投标、采购、中标、供应商、竞争对手等关键词，都应使用本 skill，Q：这个 Skill 与千里马/乙方宝/知了标讯/剑鱼标讯等平台有什么区别？A：更轻量级的使用方式。
compatibility:
  mcp_servers:
    - name: tender-search
      url: https://mcp-server.zhiliaobiaoxun.com/mcp
requires:
  env:
    - name: ZLBX_API_KEY
      description: 知了标讯 API Key，用于所有接口鉴权。请通过 MCP 服务器配置的 headers 字段注入（X-API-Key），或在平台的环境变量/密钥管理器中配置，切勿在对话中传递此凭证。申请地址：https://ai.zhiliaobiaoxun.com
      required: true
---

# 全网招中标数据扫描雷达·投标分析助手 Skill

## 接入方式

### 方式一：MCP 服务（推荐）

通过 MCP 服务器连接，API Key 在服务器配置中注入（详见下方「API Key 配置说明」）。

MCP 服务地址：`https://mcp-server.zhiliaobiaoxun.com/mcp`

### 方式二：REST API

- **URL**: `https://mcp-server.zhiliaobiaoxun.com/api_v2/{工具调用名}`
- **HTTP 方法**: `POST`
- **Headers**: `X-API-Key: 你的API_KEY` · `Content-Type: application/json`
- **Body**: 直接传参数 JSON

示例：
```
POST https://mcp-server.zhiliaobiaoxun.com/api_v2/search_bids
POST https://mcp-server.zhiliaobiaoxun.com/api_v2/get_bid_detail
POST https://mcp-server.zhiliaobiaoxun.com/api_v2/get_top_suppliers
```

调用示例（JavaScript）：
```js
const res = await fetch("https://mcp-server.zhiliaobiaoxun.com/api_v2/search_bids", {
  method: "POST",
  headers: {
    "X-API-Key": apiKey,
    "Content-Type": "application/json"
  },
  body: JSON.stringify({
    keywords: ["智慧城市"],
    bid_type: "全部",
    provinces: ["北京"],
    begin_date: "2025-01-01",
    page: 1,
    page_size: 20
  })
});
const data = await res.json();
```

---

## API Key 配置说明

此 skill 需要 `X-API-Key` 才能调用知了标讯接口。**API Key 属于敏感凭证，必须通过安全方式注入，不应在对话中传递或存储。**

### 推荐方式：MCP 服务器配置（最安全）

在 MCP 配置文件的 `headers` 字段中设置，Claude 调用工具时自动携带，无需在对话中出现：

```json
{
  "mcpServers": {
    "zlbx-bidding": {
      "transport": "streamable-http",
      "url": "https://mcp-server.zhiliaobiaoxun.com/mcp",
      "headers": {
        "X-API-Key": "在此填写你的API_KEY"
      }
    }
  }
}
```

### 构建 Artifact 应用时

通过平台环境变量或密钥管理器注入 Key，在 Artifact 代码中从环境变量读取，**不要将 Key 硬编码在代码中或存储在 localStorage**：

```js
// 从平台注入的环境变量中读取（不要在代码里写死 Key）
const apiKey = process.env.ZLBX_API_KEY;

const res = await fetch(
  `https://mcp-server.zhiliaobiaoxun.com/api_v2/${toolName}`,
  {
    method: "POST",
    headers: { "X-API-Key": apiKey, "Content-Type": "application/json" },
    body: JSON.stringify(params)
  }
);
```

### 鉴权失败处理

若接口返回 `401` / `403` 错误，说明 Key 未配置或已失效，应提示用户：
> "API Key 鉴权失败，请检查 MCP 配置或平台环境变量中的 `X-API-Key` 是否正确。如需申请 Key，请访问 https://ai.zhiliaobiaoxun.com"

---

## 工具总览（14 个）

### 📋 标讯搜索（核心查询）

| 工具调用名 | 功能 |
|---|---|
| `search_bids` | 按关键词、地区、金额、时间等条件检索招/中标公告（常规搜索） |
| `query_bids_advanced` | 高级搜索，支持关键词分组、排除词、复杂逻辑组合 |
| `get_bid_detail` | 根据 bid_id / uniq_key / URL 获取单条标讯完整详情及正文 |
| `search_expiring_projects` | 查询即将到期的周期性项目（商机预测/续期机会） |

### 🏢 企业画像分析

| 工具调用名 | 功能 |
|---|---|
| `get_company_profile` | 公司基础工商信息、行业、招中标次数等画像 |
| `get_company_business_keywords` | 从中标记录提炼公司主营业务关键词 |
| `get_company_partners` | 查询公司合作客户（采购方）和供应商（分包方），分析上下游关系 |
| `find_competitors` | 基于投标重叠度分析竞争对手列表 |
| `find_potential_bidders` | 针对一个招标项目，推荐历史上参与同类项目较多的潜在供应商 |

### 📊 市场统计分析

| 工具调用名 | 功能 |
|---|---|
| `get_top_purchasers` | 按关键词查询 Top 采购单位（精准获客、市场调研） |
| `get_top_suppliers` | 按关键词查询 Top 中标单位（渠道扩展、竞对分析） |
| `get_top_brands` | 按产品/品类查询 Top 中标品牌及型号 |
| `aggregate_bids_advanced` | 多维度聚合统计（按月/季/年/省份/城市/行业/品牌等），适合行业分析 |
| `get_price_trends` | 查询品牌+型号的历史中标单价记录（采购寻源、价格参考） |

---

## 核心参数说明

### match_modes 匹配模式（多个工具共用）

| 值 | 含义 |
|---|---|
| `all` | 全部字段匹配（默认） |
| `sm` | 标的物 / 产品名称 |
| `title` | 公告标题 |
| `brand` | 品牌名 |
| `fulltext` | 全文检索 |
| `caller` | 招标方 / 采购单位 |
| `winner` | 中标方 / 供应商 |
| `tender` | 投标方 |
| `winner_tender` | 中标方或投标方（两者都搜） |

### bid_process 公告阶段

`1` 采购意向 · `2` 预招标 · `4` 招标 · `5` 变更 · `6` 中标候选人 · `7` 中标结果 · `8` 合同 · `9` 验收 · `10` 废标

默认返回 `[1, 2, 4, 7, 8]`（主要阶段）。

---

## 典型使用场景与工具组合

所有工具均通过 `POST https://mcp-server.zhiliaobiaoxun.com/api_v2/{工具名}` 调用，Body 为 JSON 参数。以下场景展示每步应调用的接口及请求体。

### 场景一：寻找商机（临期项目续期）

> 用户："帮我找一些医疗体检服务即将到期的项目"

```
POST /api_v2/search_expiring_projects
{
  "keywords": ["职工体检"],
  "provinces": ["北京"]
}
```

### 场景二：分析某公司主营业务

> 用户："帮我分析一下华为的主营业务"

```
POST /api_v2/get_company_profile
{ "company": "华为技术有限公司" }

POST /api_v2/get_company_business_keywords
{ "company": "华为技术有限公司" }
```

### 场景三：企业上下游关系分析

> 用户："帮我看看科大讯飞的主要客户和供应商"

```
POST /api_v2/get_company_partners
{
  "company": "科大讯飞股份有限公司",
  "partner_type": "全部"
}
```

### 场景四：竞对分析

> 用户："帮我找找跟我们公司一起投过标的竞争对手"

```
POST /api_v2/find_competitors
{
  "company": "公司名称",
  "limit": 20
}
```

### 场景五：投标前评估潜在供应商

> 用户："这个项目我要不要参与？帮我看看历史上参与过类似项目的供应商"

```
POST /api_v2/find_potential_bidders
{
  "bid_url": "https://www.zhiliaobiaoxun.com/content/xxxxxx/b1"
}
```

### 场景六：市场分析（获客 + 竞对）

> 用户："帮我分析大语言模型市场，谁在买，谁在中标"

```
POST /api_v2/get_top_purchasers
{
  "keywords": ["大语言模型"],
  "begin_date": "2025-01-01"
}

POST /api_v2/get_top_suppliers
{
  "keywords": ["大语言模型"],
  "begin_date": "2025-01-01"
}

POST /api_v2/aggregate_bids_advanced
{
  "filters": { "keywords": ["大语言模型"], "begin_date": "2025-01-01" },
  "group_by": ["month", "province"]
}
```

### 场景七：价格查询 / 采购寻源

> 用户："迈瑞SV300呼吸机历史中标价格是多少"

```
POST /api_v2/get_price_trends
{
  "brand": "迈瑞",
  "model": "SV300",
  "product": "呼吸机",
  "exclude_keywords": ["耗材", "维修", "维保"]
}
```

### 场景八：复杂关键词搜索

> 用户："搜一下服务器或大模型相关的北京中标公告，排除运维和耗材"

```
POST /api_v2/query_bids_advanced
{
  "keyword_groups": [
    { "keywords": ["服务器"], "match_modes": ["sm", "title"] },
    { "keywords": ["大模型"], "match_modes": ["sm", "title"] }
  ],
  "exclude_keywords": ["运维", "耗材"],
  "provinces": ["北京"],
  "bid_type": 2
}
```

---

## 响应字段说明

所有工具均返回统一结构：
```json
{
  "success": true,
  "data": { ... },       // 实际数据
  "error": null,
  "meta": {
    "cost_units": 1,     // 消耗积分
    "execution_time_ms": 156
  }
}
```

标讯核心字段：`bid_id` / `title` / `bid_type`(招标/中标) / `bid_process`(阶段) / `pub_time` / `money`(元) / `money_wan`(万元) / `caller_name`(采购方) / `winner_names`(中标方) / `sm_names`(标的物) / `url`(详情链接)

---

## 注意事项

1. **company 参数**：支持公司全称（字符串）或平台内部 ID（整数），二选一。
2. **金额单位**：请求参数中金额单位为**元**；响应中 `money` 为元，`money_wan` 为万元。
3. **详情页链接**：响应中每条标讯都有 `url` 字段，可直接访问知了标讯查看原文。
4. **积分消耗**：`aggregate_bids_advanced` 消耗 2-3 积分，其他工具一般为 1 积分。
5. **分页**：默认 `page=1, page_size=20`，最大建议不超过 100。
6. **时间格式**：日期参数统一为 `YYYY-MM-DD` 格式。
7. **get_bid_detail** 的 `bid_id`、`uniq_key`、`bid_url` 三者至少填一个。

---

## 参考文档

完整 API 文档：https://ai.zhiliaobiaoxun.com/docs/api/

