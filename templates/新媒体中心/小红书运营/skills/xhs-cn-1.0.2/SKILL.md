---
name: xiaohongshu
license: MIT
version: 0.1.0
source: https://github.com/PalmPalm7/xiaohongshu-mcp-skill
homepage: https://github.com/PalmPalm7/xiaohongshu-mcp-skill
description: |
  小红书全链路运营技能，提供从内容创作、笔记发布、搜索浏览、互动评论到数据分析的一站式运营支持。
  支持二维码登录、Cookie 持久化、创作者中心图文/视频 Tab 自动切换及 TipTap 富文本编辑器自动填充。
env_vars:
  optional:
    - name: XHS_COOKIE
      description: "Xiaohongshu cookie string for authentication (alternative to QR login)"
      sensitive: true
    - name: PROXY_SERVER
      description: "HTTP proxy server address (e.g. http://proxy:8080)"
    - name: PROXY_USERNAME
      description: "Proxy authentication username"
      sensitive: true
    - name: PROXY_PASSWORD
      description: "Proxy authentication password"
      sensitive: true
    - name: BROWSER_TYPE
      description: "Browser engine: chromium | firefox | webkit (default: chromium)"
    - name: HEADLESS
      description: "Run browser in headless mode: true | false (default: true)"
    - name: COOKIE_FILE
      description: "Path to save/load browser cookies (default: ./cookies.json)"
    - name: SCREENSHOT_DIR
      description: "Directory for screenshots (default: ./screenshots)"
    - name: DOWNLOAD_DIR
      description: "Directory for downloaded files (default: ./downloads)"
    - name: MIN_REQUEST_DELAY
      description: "Minimum delay between requests in seconds (default: 2)"
    - name: MAX_REQUEST_DELAY
      description: "Maximum delay between requests in seconds (default: 5)"
local_files:
  - path: cookies.json
    description: "Persisted browser cookies (auto-saved after QR login). Contains session credentials — treat as highly sensitive."
    sensitive: true
  - path: screenshots/
    description: "Auto-captured screenshots of each automation step"
  - path: .env
    description: "Environment variable overrides (may contain cookie/proxy credentials)"
    sensitive: true
permissions:
  - "Launches a non-headless Chromium browser (headless=False for QR login)"
  - "Reads/writes local files: cookies.json, .env, screenshots/, downloads/"
  - "Performs actions on your Xiaohongshu account: publish, like, comment, follow"
  - "Optional: routes traffic through configured proxy (PROXY_SERVER)"
---
# 🌟 xiaohongshu — 小红书全链路运营 Skill

## 技能概述

`xiaohongshu` 是一个小红书全链路运营 AI 技能，包含两大核心能力：

1. **Prompt-based 内容运营** — 通过自然语言对话，获取专业级的内容创作、发布策略、运营分析建议
2. **Python 自动化操作** — 基于 Playwright 浏览器自动化，实际执行登录、搜索、发布、互动等平台操作

覆盖小红书平台的 **账号登录 → 内容创作 → 笔记发布 → 搜索浏览 → 互动评论 → 数据分析** 全链路运营场景。

---

## 核心能力

### Prompt-based 能力（自然语言交互）

| 模块 | 能力说明 |
|------|---------  |
| 📝 内容创作 | 生成爆款标题、正文、标签，支持多种风格和品类 |
| 📤 发布策略 | 提供发布策略建议、最佳时间推荐、封面优化建议 |
| 🔍 搜索分析 | 关键词分析、热门话题追踪、竞品笔记研究、SEO优化 |
| 💬 互动运营 | 智能评论回复、互动话术生成、粉丝维护策略 |
| 📊 数据分析 | 数据复盘、内容表现诊断、增长策略建议 |

### Python 自动化能力（实际操作）

| 模块 | 能力说明 | API 函数 |
|------|---------|----------|
| 🔐 二维码登录 | 通过扫描二维码完成小红书登录，支持 Cookie 持久化与自动复用 | `login_by_qrcode()` |
| ✅ 登录检测 | 检查当前是否已登录，自动加载已保存的 Cookie | `check_login()` |
| 🔍 搜索笔记 | 按关键词搜索，支持排序和类型筛选 | `search_notes()` |
| 📄 获取详情 | 获取笔记完整信息（标题、正文、图片、视频、标签、数据） | `get_note_detail()` |
| 📤 发布图文 | 自动切换到「上传图文」tab，上传图片并填写标题/正文/标签发布 | `publish_image_note()` |
| 🎬 发布视频 | 上传视频并发布笔记 | `publish_video_note()` |
| ❤️ 点赞 | 对笔记进行点赞操作 | `like_note()` |
| ⭐ 收藏 | 对笔记进行收藏/取消收藏 | `collect_note()` |
| 💬 评论 | 在笔记下发布评论 | `comment_note()` |
| 👥 关注 | 关注指定用户 | `follow_user()` |
| 💬 获取评论 | 获取笔记的评论列表 | `get_note_comments()` |
| 👤 用户信息 | 获取用户主页信息 | `get_user_profile()` |

---

## 自动化发布流程

二维码登录 + 图文发布的完整自动化流程如下：

```
🚀 启动浏览器 (headless=False)
    │
    ▼
🍪 加载 cookies.json（若存在）
    │
    ▼
🔐 检查登录状态 ──── 未登录 ──→ 📱 显示二维码（手机扫码，最长120秒）
    │                                      │
  已登录                               扫码成功 → 保存 Cookie
    │                                      │
    ▼◄─────────────────────────────────────┘
📸 生成/准备待发布图片
    │
    ▼
🌐 打开创作者中心 (creator.xiaohongshu.com/publish/publish)
    │
    ▼
🔄 自动切换到「上传图文」Tab（默认页面是「上传视频」Tab）
    │
    ▼
📤 通过 file input 上传图片（自动识别 accept='.jpg,.jpeg,.png,.webp' 的输入框）
    │
    ▼
✏️ 填写标题（定位 placeholder="填写标题" 的 input）
    │
    ▼
📝 填写正文（通过 TipTap/ProseMirror contenteditable 编辑器）
    │
    ▼
🏷️ 添加标签（可选）
    │
    ▼
🚀 点击「发布」按钮
    │
    ▼
✅ 发布成功！
```

### 关键技术细节

- **Tab 切换**：创作者中心默认选中「上传视频」tab，`publish_image_note()` 会通过 JavaScript 自动 `scrollIntoView` + `click` 切换到「上传图文」tab
- **File Input 识别**：优先查找 `accept` 属性包含图片格式（`.jpg,.jpeg,.png,.webp`）的 file input，避免误用视频上传入口
- **富文本编辑器**：正文区域使用 TipTap/ProseMirror 的 `contenteditable` div，通过 `focus` + `insertText` 方式填充内容
- **Cookie 持久化**：登录后自动保存到 `cookies.json`，下次启动自动加载，免去重复扫码

---

## 安装与配置

### 1. 安装依赖

```bash
# Clone the repository
git clone https://github.com/your-repo/xiaohongshu.git
cd xiaohongshu

# Create virtual environment (recommended)
python -m venv .venv
source .venv/bin/activate  # macOS/Linux
# or: .venv\Scripts\activate  # Windows

# Install the package
pip install -e .

# Install Playwright browser
playwright install chromium
```

### 2. 配置

```bash
# Copy the example config
cp .env.example .env

# Edit .env (optional: cookie will be auto-saved after QR login)
```

> 💡 **推荐使用二维码登录**：无需手动获取 Cookie，运行 `login_and_publish.py` 即可通过手机扫码自动登录并保存 Cookie。

---

## 使用方法

### 方式一：自然语言交互（Prompt-based）

直接通过对话即可使用，以下是各场景的使用示例：

#### 📝 内容创作

```
帮我写一篇关于"春季穿搭"的小红书笔记，风格要清新自然，适合20-25岁女性用户
```

```
我写了一篇关于自制咖啡的笔记，帮我生成5个吸引眼球的标题，要求有emoji
```

```
帮我把以下内容改写成3个不同风格的小红书笔记版本（种草风、测评风、教程风）：[你的内容]
```

#### 📤 发布策略

```
我的账号主要做美食类内容，粉丝以上班族为主，帮我制定一周的最佳发布时间表
```

```
帮我制定一个为期一个月的小红书发布计划，我的赛道是家居好物，目前粉丝500
```

#### 🔍 搜索分析

```
分析"平价护肤"这个关键词在小红书上的热度和竞争情况，给出长尾关键词建议
```

```
帮我分析小红书上做"极简生活"赛道的头部博主，总结他们的内容策略
```

#### 💬 互动评论

```
有粉丝在我的美食笔记下问"这个酱料在哪里买的"，帮我生成一个亲切又有互动感的回复
```

```
帮我生成一套小红书私信回复模板，包括：新关注感谢、合作咨询回复、产品问询回复
```

#### 📊 运营分析

```
我最近一篇笔记获得了2000赞、500收藏、100评论、5万阅读，帮我分析数据表现和优化方向
```

```
我的小红书账号做了3个月，粉丝增长缓慢（800粉），主要做穿搭，帮我诊断并改进
```

#### 🎯 综合运营

```
我要发一篇关于"夏日防晒"的小红书笔记，请帮我：
1. 写出正文内容（种草风格）
2. 生成5个爆款标题
3. 推荐相关热门标签
4. 建议最佳发布时间
5. 写一段互动引导语
```

---

### 方式二：二维码登录 + 自动发布（一键完成）

```bash
# Run the login-and-publish example
.venv/bin/python examples/login_and_publish.py
```

首次运行会弹出浏览器窗口显示小红书登录二维码，用手机扫码即可。登录成功后：
- Cookie 自动保存到 `cookies.json`
- 生成测试图片并自动发布到创作者中心
- 后续运行自动复用 Cookie，无需再次扫码

---

### 方式三：Python API 调用（自动化操作）

#### 异步 API（推荐）

```python
import asyncio
from pathlib import Path
from xiaohongshu.client import XHSClient
from xiaohongshu.config import XHSConfig, BrowserConfig, StorageConfig

async def main():
    config = XHSConfig(
        cookie="",
        browser=BrowserConfig(browser_type="chromium", headless=False),
        storage=StorageConfig(
            cookie_file=Path("./cookies.json"),
            screenshot_dir=Path("./screenshots"),
        ),
    )

    async with XHSClient(config) as client:
        # 🔐 QR code login (auto-skipped if cookie is valid)
        is_logged_in = await client.check_login()
        if not is_logged_in:
            await client.login_by_qrcode(timeout=120)

        # 🔍 Search notes
        results = await client.search_notes("春季穿搭", limit=10)
        for note in results.notes:
            print(f"{note.title} - ❤️ {note.liked_count}")

        # 📄 Get note details
        note = await client.get_note_detail("note_id_here")
        print(f"Title: {note.title}")
        print(f"Content: {note.content}")

        # 📤 Publish an image note
        result = await client.publish_image_note(
            title="我的穿搭分享 🌸",
            content="今天分享一套春日穿搭～",
            image_paths=["./outfit1.jpg", "./outfit2.jpg"],
            tags=["春季穿搭", "OOTD"],
        )
        print(f"Published: {result.success}")

        # 🎬 Publish a video note
        result = await client.publish_video_note(
            title="一分钟学会拉花 ☕",
            content="超简单的咖啡拉花教程...",
            video_path="./latte_art.mp4",
            tags=["咖啡", "拉花教程"],
        )

        # ❤️ Like a note
        await client.like_note("note_id_here")

        # ⭐ Collect a note
        await client.collect_note("note_id_here")

        # 💬 Comment on a note
        await client.comment_note("note_id_here", "太好看了！")

        # 👥 Follow a user
        await client.follow_user("user_id_here")

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

---

## 项目结构

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

## ⚠️ 安全与隐私

> **重要：** 使用本技能前请仔细阅读以下安全提示。

### 🔑 凭据安全

| 敏感文件/变量 | 说明 | 处理建议 |
|---------------|------|----------|
| `cookies.json` | 登录后自动保存的会话 Cookie，**等同于账号登录凭据** | 已加入 `.gitignore`，绝不提交到任何仓库 |
| `.env` 中的 `XHS_COOKIE` | 手动配置的 Cookie 字符串 | 同上，绝不提交 |
| `.env` 中的 `PROXY_USERNAME` / `PROXY_PASSWORD` | 代理认证凭据 | 仅在信任的代理服务器上使用 |

### 🛡️ 安全建议

1. **先用测试账号**：首次使用建议在一次性/测试账号上验证行为，避免在主账号上产生意外操作
2. **隔离环境**：在独立的虚拟环境（`python -m venv`）中运行，避免污染系统 Python
3. **代理风险**：如果配置了 `PROXY_SERVER`，所有流量（包括 Cookie）都会经过该代理，仅使用可信代理
4. **检查代码**：建议使用前审查 `client.py` 源码，确认自动化行为符合预期
5. **headless 模式**：二维码登录需要 `headless=False`（显示浏览器界面），确认在安全的桌面环境中运行

### 📋 权限声明

本技能运行时会执行以下操作（均在用户明确调用后触发）：

- 启动 Chromium 浏览器（QR 登录时为非 headless 模式）
- 读写本地文件：`cookies.json`、`screenshots/`、`downloads/`
- 在您的小红书账号上执行操作：发布笔记、点赞、评论、收藏、关注
- 可选：通过配置的代理服务器路由网络流量

### ⚖️ 合规声明

- 请遵守 [小红书社区规范](https://www.xiaohongshu.com/agreement) 和使用条款
- 本项目仅供学习交流，不鼓励任何违反平台规则的操作
- 使用自动化工具产生的任何后果由用户自行承担

---

## 其他注意事项

- **二维码有效期**：QR 码约 60 秒过期，超时后会自动刷新（最多等待 120 秒）
- **频率控制**：默认操作间隔 2-5 秒随机延时，模拟人工操作节奏
- **截图记录**：每步关键操作会自动截图到 `screenshots/`，方便调试
