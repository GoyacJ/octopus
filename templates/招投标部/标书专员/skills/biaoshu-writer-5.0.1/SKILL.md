---
name: biaoshu-writer
description: 标书撰写器 v5.2.1 - 投标技术标文档自动生成工具。当用户需要根据招标要求、评分标准生成技术标Word文档时使用。支持解析txt、pdf、docx、xlsx格式的招标文件，自动分章节编写内容，最终转换为Word格式。适用场景：发送招标文件要求生成技术标、投标文档编写、分章节标书制作、交通工程（高速/航道）技术标。
metadata:
  openclaw:
    requires:
      python: ["python-docx", "pdfplumber", "openpyxl", "PyPDF2"]
    install:
      - id: pip
        kind: pip
        packages:
          - python-docx
          - pdfplumber
          - openpyxl
          - PyPDF2
        label: 安装 Python 依赖库
---

# 标书撰写器 v5.2.1

投标技术标文档自动生成工具。发送招标文件 → 自动生成符合评分标准的技术标 Word 文档。

---

## 环境安装

### 依赖库
```bash
pip install python-docx pdfplumber openpyxl PyPDF2
```

### 字体
将 `SimSun.ttf` 复制到 `~/Library/Fonts/`（从 Windows 系统 `C:\Windows\Fonts\` 复制，或网上下载）

### 安装技能
```bash
openclaw skills install <skill路径>
```

---

## 快速使用

1. 发送招标文件（txt / pdf / docx / xlsx）给 AI
2. AI 自动解析并生成大纲
3. 确认大纲后并发编写各章节
4. 字数检查 + AI 去痕处理
5. 转换生成 Word 文档

---

## 执行流程

```
发送招标文件 → 解析评分标准 → 生成4级标题大纲 → 
 Owen确认大纲 → 并发编写章节 → 字数检查 → 
 AI去痕 → 汇总转Word
```

### 字数检查规则

公式：目标字数 = 评分分值 × (总页数 ÷ 总分) × 780  
合格范围：目标 × 0.75 ~ 1.25

| 分值 | 目标字数 | 合格范围 |
|------|---------|---------|
| 5分  | 20,000字 | 15,000~25,000字 |
| 4分  | 16,000字 | 12,000~20,000字 |

不达标 → 打回重写 → 达标后继续

---

## 格式参数

### 默认值（v5.2.1）

| 参数 | 默认值 | 说明 |
|------|--------|------|
| font | SimSun | 正文字体 |
| body-size | **16pt** | 正文字号（三号标准） |
| title-level | 36pt | 一级标题 |
| sub-level | 32pt | 二级标题 |
| line-spacing | 26pt | 行距 |
| margins | 2cm | 页边距 |
| first-line-indent | 0.74cm | 首行缩进（2字符） |

### 常用模板

```
【政府标书】SimSun / 16pt / 26磅 / 2cm
【高速公路】SimSun / 16pt / 28磅 / 2.5cm
【航道工程】SimSun / 16pt / 26磅 / 2cm
```

### Markdown 格式指令（顶部加注释块）
```markdown
<!-- doc-format
font: SimSun
body-size: 16pt
line-spacing: 26pt
margins: 2cm
-->
```

---

## 内容规则

- 每小节 ≥ 3 个独立段落
- 每章节配表格
- 禁用"我方/我们"，用"将/项目组"
- 禁止金额/预算描述
- 严格按评分标准编写，不泛泛而谈

---

## 核心脚本

| 脚本 | 功能 |
|------|------|
| `parse_bid_files.py` | 解析 txt / pdf / docx / xlsx 招标文件 |
| `convert_to_word.py` | Markdown → Word 转换（格式指令/表格/标题清理） |
| `check_chapter_words.py` | 章节字数检查（公式化计算） |
| `install-deps.sh` | 安装 Python 依赖 |
| `check-font.sh` | 检查 SimSun.ttf 字体 |

---

## 版本历史

| 版本 | 日期 | 更新 |
|------|------|------|
| v5.2.1 | 2026-03-27 | 表格表头白色填充；清理标题括号；跳过水平线 |
| v5.1 | 2026-03-27 | 三号字体 16pt（标准）；clean_heading_text() |
| v5.0 | 2026-03-21 | 格式指令支持；多格式解析；humanizer-zh 集成 |
