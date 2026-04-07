# Agent 批量导入与预置 Seed PRD / 详细设计

**版本**: v1.0  
**状态**: 已实现  
**日期**: 2026-04-07

## 1. 背景

Octopus 需要支持两类能力：

1. 工作区管理员从外部目录批量导入 Agent 配置与关联 skills。
2. 正式版首次初始化空工作区时，自动带出一批预置 Agent 与去重后的 managed skills。

本方案的目标不是做一个通用文件导入器，而是落一个受控、可重复执行、可重导入、可发布的工作区级 Agent bundle 机制。

## 2. 本次交付范围

### 2.1 已交付能力

- 工作区级智能体页增加 `Import Agent` 入口。
- 支持选择一个 agent 根目录并先做预览。
- 预览展示：
  - 检测到的部门数、agent 数
  - 创建 / 更新 / 跳过数量
  - 去重后 skill 数
  - 被过滤文件数
  - warning / error 列表
- 用户确认后执行正式导入。
- 导入后的 agent 自动带上：
  - 当前全部内置工具
  - 该 agent 对应的全部导入 skill
- 支持按来源重导入，不重复创建 agent。
- 提供正式版 bundled seed：
  - 首次初始化空工作区时自动写入 `SQLite + data/skills`
  - 预置 agent 自动关联去重后的 skills

### 2.2 明确不做

- 不做项目级导入入口。
- 不做逐项勾选导入。
- 不根据部门自动创建 Team。
- 不保留外部 `model: opus` 到当前 agent 记录。
- 不按外部 `tools:` 精确落权限；导入和 seed 统一使用当前全部 builtin tools。

## 3. 输入目录约束

导入器只接受一个“agent 根目录”的递归文件集，按以下结构识别：

```text
<部门>/<Agent目录>/<同名Agent>.md
<部门>/<Agent目录>/skills/<skill目录>/SKILL.md
<部门>/<Agent目录>/skills/<skill目录>/references/**
<部门>/<Agent目录>/skills/<skill目录>/scripts/**
```

### 3.1 Agent 识别规则

- 必须存在 `<部门>/<Agent目录>/<同名Agent>.md`
- `name`
  - 优先 frontmatter `name`
  - 缺失时回退目录名
- `description`
  - 优先 frontmatter `description`
  - 缺失时回退正文首个非空段落
- `personality`
  - 优先正文中 `# 角色定义` 后首个非空段落
  - 其次回退 `# Role Definition`
  - 再回退 `name`
- `prompt`
  - 去掉 frontmatter 后的完整 markdown 正文
- `tags`
  - 至少写入部门名
- `scope`
  - 固定为 `workspace`
- `status`
  - 固定为 `active`
- `builtinToolKeys`
  - 固定为当前全部 builtin tool keys
- `mcpServerNames`
  - 固定为空数组

### 3.2 Skill 识别规则

- 只识别 `skills/<skillDir>/SKILL.md`
- 保留 skill 目录下业务文件，包括：
  - `SKILL.md`
  - `references/**`
  - `scripts/**`
  - 其余合法业务文件
- 过滤以下目录：
  - `node_modules`
  - `.git`
  - `.cache`
  - `.turbo`
  - `dist`
  - `build`
  - `coverage`
  - `__pycache__`
  - `.venv`
  - `venv`

## 4. 产品规则

### 4.1 导入行为

- 入口只在工作区智能体页显示。
- 流程固定为：
  1. 选择目录
  2. 调预览接口
  3. 打开预览对话框
  4. 用户确认
  5. 执行正式导入
  6. 刷新 agent store 和 skill catalog
  7. 展示结果报告

### 4.2 重导入行为

- 同一来源的 agent 重导入走 `update`
- 不重复创建
- skill 允许复用已存在 managed skill

### 4.3 失败处理

- 执行策略为“逐项继续”
- 单个 agent 失败不阻塞其他 agent
- 单个 skill 失败时：
  - agent 仍可成功导入
  - 该 skill 从该 agent 结果中剔除
  - 在结果中报告失败原因

## 5. 数据契约

### 5.1 Schema

新增 feature schema 文件：

- `packages/schema/src/agent-import.ts`

导出类型包括：

- `ImportWorkspaceAgentBundlePreviewInput`
- `ImportWorkspaceAgentBundlePreview`
- `ImportWorkspaceAgentBundleInput`
- `ImportWorkspaceAgentBundleResult`
- `ImportedAgentPreviewItem`
- `ImportedSkillPreviewItem`
- `ImportIssue`

`packages/schema/src/index.ts` 仅作为 export surface。

### 5.2 前端 adapter / store

已接入：

- `apps/desktop/src/tauri/workspace-client.ts`
- `apps/desktop/src/stores/agent.ts`
- `apps/desktop/src/tauri/shell.ts`

方法包括：

- `agents.previewImportBundle(...)`
- `agents.importBundle(...)`
- `pickAgentBundleFolder()`

### 5.3 服务端接口

工作区级 API：

- `POST /api/v1/workspace/agents/import-preview`
- `POST /api/v1/workspace/agents/import`

## 6. 持久化设计

### 6.1 不修改现有 AgentRecord 对外契约

重导入来源追踪不污染现有 agent 编辑表单结构，来源元数据单独存 SQLite 表。

### 6.2 新增元数据表

- `agent_import_sources`
- `skill_import_sources`

### 6.3 字段职责

最少保存：

- `source_kind`
  - `bundled_seed`
  - `user_import`
- `source_id`
- `source_path`
- `content_hash`
- `agent_id` 或 `skill_slug`
- `department`
- `last_imported_at`

### 6.4 来源标识规则

- agent: `<部门>/<agent目录>`
- skill: `<部门>/<agent目录>/skills/<skill目录>`

### 6.5 重导入匹配规则

1. 优先按 `source_kind + source_id` 找已有映射
2. 命中则更新已有目标
3. skill 若未命中来源映射，则尝试按 slug/hash 复用
4. agent 若未命中来源映射，则新建

## 7. Skill 去重设计

### 7.1 canonical slug

- 优先 frontmatter `name`
- 缺失时回退目录名
- 再统一 slugify

### 7.2 去重策略

- 同名同内容只导入一次
- 当前已验证：同名 skill 不存在内容变体
- 若工作区已有同 slug 且 hash 相同，则直接复用
- 若同 slug 但 hash 不同，则落成 `slug-<hash8>`

### 7.3 managed skill 存储

目标目录固定为：

```text
data/skills/<slug>/**
```

skill id 通过 managed skill 路径派生，供 agent `skill_ids` 引用。

## 8. Seed 设计

### 8.1 为什么不用原始目录直接入仓

原始目录带有大量重复 skill 和外部环境噪音，不适合作为正式资产直接跟仓库发布。

因此 seed 采用“转换后产物”入仓：

- `manifest.json`
- `skills/<slug>/**`

### 8.2 Seed 产物位置

```text
crates/octopus-infra/seed/agent-bundle/
```

### 8.3 Seed 生成脚本

```text
scripts/prepare-agent-bundle-seed.mjs
pnpm prepare:agent-bundle-seed <bundle-root>
```

当前已生成的数据来自：

```text
E:\agents\chinese-按部门分类1
```

生成结果：

- `155` agents
- `380` skill sources
- `72` unique skills

### 8.4 Seed 初始化规则

空工作区初始化时：

1. 先检查工作区是否已有 agent
2. 若为空，再检查是否已有 managed skills
3. 若仍为空，则尝试写入 bundled seed
4. 若 bundled seed 不存在，才回退旧默认 agent

### 8.5 为什么当前 dev 工作区可能看不到 seed

当前开发模式下，桌面后端工作区根默认是仓库根目录。  
如果仓库根下已经存在 `data/main.db` 且已有 agent，初始化逻辑不会覆盖写入 seed。

这不是 bug，是明确的保护行为。

## 9. 代码落点

### 9.1 核心后端

- `crates/octopus-infra/src/agent_import.rs`
  - 目录解析
  - 预览计划
  - 执行导入
  - skill 去重
  - 来源映射写入
  - bundled seed 写入
- `crates/octopus-infra/src/lib.rs`
  - DB 初始化时建 import source 表
  - 空工作区初始化时触发 bundled seed
  - `InfraWorkspaceService` 暴露 preview / import

### 9.2 服务端

- `crates/octopus-server/src/lib.rs`

### 9.3 桌面端

- `apps/desktop/src/views/agents/AgentCenterView.vue`
- `apps/desktop/src/views/agents/AgentBundleImportDialog.vue`
- `apps/desktop/src/stores/agent.ts`
- `apps/desktop/src/tauri/workspace-client.ts`
- `apps/desktop/src/tauri/shell.ts`
- `apps/desktop/src-tauri/src/commands.rs`
- `apps/desktop/src-tauri/src/lib.rs`

## 10. 后续维护指南

### 10.1 什么时候需要重新生成 seed

满足以下任一条件时需要重新生成：

- 外部预置 agent 源目录有新增、删除、重命名
- skill 内容更新
- 需要调整预置 agent 与 skill 的绑定关系
- 需要发布新的默认预置包

### 10.2 标准更新流程

1. 准备新的外部 bundle 根目录
2. 运行：

```bash
pnpm prepare:agent-bundle-seed <bundle-root>
```

3. 检查生成结果：
  - `crates/octopus-infra/seed/agent-bundle/manifest.json`
  - `crates/octopus-infra/seed/agent-bundle/skills/*`
4. 跑验证：

```bash
cargo check -p octopus-infra
cargo check -p octopus-server
cargo test -p octopus-infra agent_import
pnpm -C apps/desktop typecheck
```

5. 使用一个空工作区做 smoke test，确认：
  - agent 数正确
  - `data/skills` 数正确
  - 打开 agent 编辑时 builtin tools 全选
  - 对应 skills 已全选

### 10.3 不建议手工改的文件

以下文件应优先通过脚本或代码生成，而不是手工维护：

- `crates/octopus-infra/seed/agent-bundle/manifest.json`
- `crates/octopus-infra/seed/agent-bundle/skills/**`

### 10.4 可以安全调整的区域

- 预览 UI 展示文案
- 结果报告展示方式
- 过滤目录名单
- personality 提取规则
- slugify 规则
- issue 文案

但这些调整都要同步验证：

- 预览统计是否变化
- 重导入是否仍稳定
- 旧工作区是否不会被 seed 覆盖

### 10.5 升级注意事项

#### 新增导入字段时

必须同步更新：

1. `packages/schema/src/agent-import.ts`
2. workspace client
3. server route
4. infra parser / executor
5. 前端预览 / 结果 UI

#### 调整 agent 记录默认能力时

必须同时检查两条链：

1. 用户手动导入
2. bundled seed 首次初始化

两条链的默认能力行为需要保持一致。

#### 调整 managed skill 目录规则时

必须确认：

- 旧 skill id 是否仍可稳定解析
- catalog 是否能继续发现 skill
- agent 中已保存的 `skill_ids` 是否兼容

## 11. 运行与排障

### 11.1 如何验证 seed 是否真正写入

检查工作区：

- `data/main.db`
- `data/skills`

重点 SQL：

```sql
select count(*) from agents;
select count(*) from agent_import_sources;
select count(*) from skill_import_sources;
select count(*) from agents
where json_array_length(skill_ids) > 0
  and json_array_length(builtin_tool_keys) > 0;
```

### 11.2 典型问题

#### 问题 1：界面里看不到预置 agent

原因通常是当前工作区不是空工作区。

处理方式：

- 用全新工作区启动应用验证
- 或切换 / 清空当前 dev 工作区

#### 问题 2：打开桌面窗口看到 `127.0.0.1` 拒绝连接

说明 Tauri 窗口起来了，但 Vite dev server 没活着。  
这与 seed 逻辑无关，需要先恢复前端 dev server。

#### 问题 3：重导入创建了重复 skill

优先检查：

- slugify 是否变化
- skill hash 是否变化
- 来源映射表是否存在旧脏数据

#### 问题 4：某个 agent 被导入但没挂上预期 skill

优先检查：

- 该 skill 是否缺少 `SKILL.md`
- skill 是否被过滤目录吞掉
- 单个 skill 导入是否失败并被 issue 记录

## 12. 建议补充的后续工作

以下不是 v1 阻塞项，但建议后续补齐：

1. 增加 server / infra 层更完整的导入路由与 seed 初始化测试
2. 增加一个 dev workspace override 机制，方便直接在桌面端验证空工作区 seed
3. 在文档或脚本里提供一键 smoke test
4. 如果后续有发布流程，增加 seed 更新检查项，避免忘记重新生成 `manifest.json`

## 13. 验证结论

本次本地实现已验证：

- `cargo check -p octopus-infra` 通过
- `cargo check -p octopus-server` 通过
- `cargo test -p octopus-infra agent_import` 通过
- `pnpm -C apps/desktop typecheck` 通过

空工作区 smoke test 已确认：

- `agents = 155`
- `agent_import_sources = 155`
- `skill_import_sources = 380`
- `data/skills` 目录数 = `72`

这表明 bundled seed 与工作区级导入主链均已打通。
