---
name: ggzy-collector
description: Collect and structure public tender opportunity notices from ggzy.gov.cn. Use when the task is to search, collect, filter, or monitor 招标公告 / 采购公告 / 公开招标公告 from 全国公共资源交易平台, especially for project lead discovery, bid opportunity screening, regional targeting, qualification keyword filtering, or producing a structured project list for downstream bidding analysis.
---

Use `scripts/collect_ggzy.py` as the entry point.

Default command:

```bash
python skills/ggzy-collector/scripts/collect_ggzy.py
```

Targeted collection examples:

```bash
python skills/ggzy-collector/scripts/collect_ggzy.py --keyword 医疗 --region 江苏
python skills/ggzy-collector/scripts/collect_ggzy.py --keyword 水利 --qualification-term 营业执照 --qualification-term 业绩
```

Workflow:

1. Run the collector script.
2. Read the JSON output.
3. Use `projects` as the source of truth.
4. Prefer project initiation notices only.
5. Do not download attachments.
6. Do not enter bid-writing workflow inside this skill.

What the collector does:

- Parse ggzy public list pages from `https://www.ggzy.gov.cn/`
- Follow `/information/deal/html/a/...` into real detail pages `/information/deal/html/b/...`
- Keep only:
  - `招标公告`
  - `采购公告`
  - `公开招标公告`
- Exclude:
  - `中标`
  - `成交`
  - `结果`
  - `更正`
  - `变更`
  - `澄清`
  - `终止`
  - `废标`
  - `答疑`
- Extract structured fields:
  - title
  - notice_type
  - region
  - published_at
  - detail_url
  - canonical_url
  - project_code
  - tender_unit
  - budget_text
  - deadline_text
  - detail_text
  - qualification_requirements
  - keywords

If the user needs post-processing such as opportunity scoring, competitor comparison, or shortlist generation, perform that after the collector returns results.
