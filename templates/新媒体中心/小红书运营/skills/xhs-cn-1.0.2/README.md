# 🌟 xiaohongshu — 小红书全链路运营 Skill

<p align="center">
  <strong>小红书全链路运营 AI 助手 | Prompt-based + Python 自动化</strong>
</p>

<p align="center">
  二维码登录 · 内容创作 · 笔记发布 · 搜索浏览 · 互动评论 · 数据分析
</p>

<p align="center">
  <a href="https://github.com/PalmPalm7/xiaohongshu-mcp-skill">Source</a> ·
  <a href="./SKILL.md">SKILL.md</a> ·
  <a href="#-安全与隐私">Security</a> ·
  License: MIT
</p>

---

## 📖 简介

`xiaohongshu` 是一个小红书全链路运营 AI 技能，提供两大核心能力：

- **🗣️ Prompt-based**：通过自然语言对话获取内容创作、发布策略、运营分析等专业建议
- **🐍 Python 自动化**：基于 [Playwright](https://playwright.dev/python/) 浏览器自动化，实际执行登录、搜索、发布、互动等操作

**灵感参考：**
- [xiaohongshu-all-in-one](https://clawhub.ai/richardx0319/xiaohongshu-all-in-one) — 小红书全能助手
- [xiaohongshu-search-summarizer](https://clawhub.ai/piekill/xiaohongshu-search-summarizer) — 小红书搜索总结工具
- [xiaohongshu-mcp-skill](https://clawhub.ai/PalmPalm7/xiaohongshu-mcp-skill) — 小红书 MCP Skill

---

## ✨ 功能特性

### 🗣️ Prompt-based 运营支持

| 模块 | 能力 |
|------|-----|
| 📝 内容创作 | 爆款标题、正文撰写、标签推荐、多风格改写 |
| 📤 发布策略 | 最佳发布时间、封面建议、月度内容日历 |
| 🔍 搜索分析 | 关键词热度、SEO优化、竞品分析 |
| 💬 互动运营 | 评论回复、互动引导、负面应对、私信模板 |
| 📊 数据分析 | 数据复盘、增长诊断、爆款分析 |
| 🎯 综合运营 | 账号定位、品牌合作、矩阵规划 |

### 🐍 Python 自动化操作

| 操作 | 函数 | 说明 |
|------|------|------|
| 🔐 二维码登录 | `login_by_qrcode()` | 手机扫码登录，Cookie 自动持久化 |
| ✅ 登录检测 | `check_login()` | 自动加载已保存 Cookie，检测登录状态 |
| 🔍 搜索 | `search_notes()` | 关键词搜索，支持排序/筛选 |
| 📄 详情 | `get_note_detail()` | 获取笔记完整数据 |
| 📤 发图文 | `publish_image_note()` | 自动切换「上传图文」Tab，上传图片发布笔记 |
| 🎬 发视频 | `publish_video_note()` | 上传视频发布笔记 |
| ❤️ 点赞 | `like_note()` | 点赞笔记 |
| ⭐ 收藏 | `collect_note()` | 收藏笔记 |
| 💬 评论 | `comment_note()` | 发布评论 |
| 👥 关注 | `follow_user()` | 关注用户 |
| 💬 获取评论 | `get_note_comments()` | 获取评论列表 |
| 👤 用户信息 | `get_user_profile()` | 获取用户资料 |

### 🔄 自动化发布流程

```
🚀 启动浏览器 → 🍪 加载Cookie → 🔐 检测登录
    │                                  │
    │                        未登录 → 📱 扫码登录 → 保存Cookie
    │                                  │
    ▼◄─────────────────────────────────┘
📸 准备图片 → 🌐 打开创作者中心 → 🔄 切换「上传图文」Tab
    │
    ▼
📤 上传图片 → ✏️ 填写标题 → 📝 填写正文 → 🚀 点击发布 → ✅ 完成！
```

**关键技术点：**
- 创作者中心默认选中「上传视频」Tab，自动通过 JS `scrollIntoView` + `click` 切换到「上传图文」
- 优先匹配 `accept='.jpg,.jpeg,.png,.webp'` 的 file input，避免误用视频上传入口
- 正文区域使用 TipTap/ProseMirror `contenteditable` 编辑器，通过 `focus` + `insertText` 填充

---

## 🚀 快速开始

### 安装

```bash
# Clone the repository
git clone https://github.com/your-repo/xiaohongshu.git
cd xiaohongshu

# Create virtual environment (recommended)
python -m venv .venv
source .venv/bin/activate  # macOS/Linux

# Install the package
pip install -e .

# Install browser (required for automation)
playwright install chromium
```

### 二维码登录 + 自动发布（一键完成）

```bash
.venv/bin/python examples/login_and_publish.py
```

首次运行会弹出浏览器窗口显示小红书登录二维码，用手机扫码即可。登录成功后：
- 🍪 Cookie 自动保存到 `cookies.json`，后续免扫码
- 📸 自动生成测试图片并发布到创作者中心
- 📸 每步操作截图保存到 `screenshots/`

### 配置（可选）

```bash
cp .env.example .env
# 编辑 .env 填写 cookie（如果不用二维码登录的话）
```

> 💡 **推荐使用二维码登录**，无需手动获取 Cookie。

### 使用示例

#### Prompt-based（自然语言）

```
帮我写一篇关于"咖啡店探店"的小红书笔记，包含标题、正文、标签和互动引导语
```

#### Python API

```python
import asyncio
from pathlib import Path
from xiaohongshu.client import XHSClient
from xiaohongshu.config import XHSConfig, BrowserConfig, StorageConfig

async def main():
    config = XHSConfig(
        cookie="",
        browser=BrowserConfig(browser_type="chromium", headless=False),
        storage=StorageConfig(cookie_file=Path("./cookies.json")),
    )

    async with XHSClient(config) as client:
        # 🔐 QR login (auto-skipped if cookie valid)
        if not await client.check_login():
            await client.login_by_qrcode(timeout=120)

        # 🔍 Search
        results = await client.search_notes("春季穿搭", limit=5)
        for note in results.notes:
            print(f"{note.title} - ❤️ {note.liked_count}")

        # 📤 Publish
        await client.publish_image_note(
            title="春日穿搭分享 🌸",
            content="今天的OOTD～",
            image_paths=["outfit.jpg"],
            tags=["穿搭", "OOTD"],
        )

        # ❤️ Interact
        await client.like_note("note_id")
        await client.comment_note("note_id", "好好看！")
        await client.follow_user("user_id")

asyncio.run(main())
```

#### 同步 API（更简洁）

```python
from xiaohongshu.api import search, like, comment, publish_image

results = search("春季穿搭", limit=5)
like("note_id")
comment("note_id", "太赞了！")
publish_image("标题", "内容", ["img.jpg"], tags=["标签"])
```

> 📌 完整使用指南请参阅 [SKILL.md](./SKILL.md)

---

## 📁 项目结构

```
xiaohongshu/
├── SKILL.md                   # Skill 定义文件（name/license/description + 完整使用指南）
├── README.md                  # 项目说明文档
├── pyproject.toml             # Python 项目配置与依赖
├── .env.example               # 环境变量配置模板
├── .gitattributes             # Git 属性配置
├── .gitignore                 # Git 忽略规则
├── xiaohongshu/               # Python 核心包
│   ├── __init__.py            # 包入口与导出
│   ├── client.py              # XHSClient 浏览器自动化客户端（核心）
│   ├── api.py                 # 同步 API 便捷函数
│   ├── config.py              # 配置管理（环境变量加载）
│   └── models.py              # Pydantic 数据模型定义
├── examples/
│   ├── demo.py                # 基础使用示例
│   └── login_and_publish.py   # 二维码登录 + 自动发布完整示例
├── cookies.json               # 登录后自动保存的 Cookie（git ignored）
├── screenshots/               # 每步操作的截图（git ignored）
└── test_images/               # 自动生成的测试图片（git ignored）
```

---

## 🏗️ 技术栈

| 技术 | 用途 |
|------|------|
| [Python 3.10+](https://python.org) | 编程语言 |
| [Playwright](https://playwright.dev/python/) | 浏览器自动化引擎 |
| [Pydantic](https://docs.pydantic.dev/) | 数据模型与验证 |
| [Pillow](https://pillow.readthedocs.io/) | 测试图片生成 |
| [httpx](https://www.python-httpx.org/) | HTTP 客户端 |
| [Loguru](https://github.com/Delgan/loguru) | 日志管理 |
| [python-dotenv](https://github.com/theskumar/python-dotenv) | 环境变量管理 |

---

## ⚠️ 安全与隐私

> ⚠️ **使用前必读** — 本技能会操作您的小红书账号，请务必了解以下安全事项。

### 敏感文件

| 文件/变量 | 风险等级 | 说明 |
|-----------|---------|------|
| `cookies.json` | 🔴 高 | 等同于登录凭据，已加入 `.gitignore`，**绝不提交到仓库** |
| `.env` (`XHS_COOKIE`) | 🔴 高 | Cookie 明文，同上 |
| `.env` (`PROXY_USERNAME/PASSWORD`) | 🟡 中 | 代理凭据，仅使用可信代理 |

### 安全建议

- 🧪 **首次使用建议在测试账号上验证**，避免在主账号上产生意外操作
- 🔒 在隔离的虚拟环境中运行（`python -m venv .venv`）
- 🌐 配置代理时，所有流量（含 Cookie）会经过代理 — 仅信任的代理
- 🔍 使用前建议审查 [`client.py`](./xiaohongshu/client.py) 源码
- 📋 请遵守 [小红书社区规范](https://www.xiaohongshu.com/agreement)，本项目仅供学习交流

> 📌 完整的环境变量清单和权限声明请参阅 [SKILL.md](./SKILL.md#-安全与隐私)

---

## 其他注意事项

- **二维码有效期**：QR 码约 60 秒过期，超时后自动刷新（最多等待 120 秒）
- **频率控制**：默认操作间隔 2-5 秒随机延时，模拟人工操作节奏
- **截图记录**：每步关键操作自动截图到 `screenshots/`，方便调试

---

## 🔗 相关资源

- [小红书创作者中心](https://creator.xiaohongshu.com/)
- [小红书社区规范](https://www.xiaohongshu.com/agreement)
- [xiaohongshu-all-in-one](https://clawhub.ai/richardx0319/xiaohongshu-all-in-one)
- [xiaohongshu-search-summarizer](https://clawhub.ai/piekill/xiaohongshu-search-summarizer)
- [xiaohongshu-mcp-skill](https://clawhub.ai/PalmPalm7/xiaohongshu-mcp-skill)

---

## 📄 License

MIT License — 详见 [SKILL.md](./SKILL.md)
