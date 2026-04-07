import re
import time
from html import unescape
from typing import Any
from urllib.parse import urljoin, urlparse
from urllib.request import Request, urlopen


class CollectorError(Exception):
    def __init__(self, message: str):
        self.message = message
        super().__init__(message)


class GgzyCollector:
    BASE_URL = "https://www.ggzy.gov.cn/"
    BUDGET_EXHAUSTED_MESSAGE = "collect budget exhausted"
    MIN_REQUEST_BUDGET_SECONDS = 2.0
    MIN_PROJECT_BUDGET_SECONDS = 10.0
    LIST_ITEM_PATTERN = re.compile(
        r"<li[^>]*>\s*"
        r'<a\s+href="(?P<href>/information/deal/html/a/[^"]+\.html)"[^>]*>'
        r"(?P<title>(?:(?!</a>).)*)</a>\s*"
        r"<span>(?P<published_at>\d{4}-\d{2}-\d{2})</span>",
        re.IGNORECASE | re.DOTALL,
    )
    ACTUAL_DETAIL_PATTERNS = (
        re.compile(r"firstLastUrl\s*=\s*'(?P<url>/information/deal/html/b/[^']+\.html)'"),
        re.compile(r"showDetail\([^,]+,[^,]+,\s*'(?P<url>/information/deal/html/b/[^']+\.html)'\)"),
    )
    TITLE_PATTERNS = (
        re.compile(r"<h4[^>]*class=\"h4_o\"[^>]*>(?P<value>.*?)</h4>", re.IGNORECASE | re.DOTALL),
        re.compile(r"<title>(?P<value>.*?)</title>", re.IGNORECASE | re.DOTALL),
    )
    PUBLISHED_AT_PATTERN = re.compile(r"发布时间：\s*(?P<value>\d{4}-\d{2}-\d{2}(?:\s+\d{2}:\d{2}(?::\d{2})?)?)")
    PLATFORM_PATTERN = re.compile(r"<label[^>]*id=\"platformName\"[^>]*>(?P<value>.*?)</label>", re.IGNORECASE | re.DOTALL)
    ORIGINAL_URL_PATTERN = re.compile(r"<a[^>]+href=\"(?P<url>[^\"]+)\"[^>]*>\s*原文链接地址\s*</a>", re.IGNORECASE | re.DOTALL)
    DETAIL_CONTENT_PATTERN = re.compile(r"<div[^>]*class=\"detail_content\"[^>]*>(?P<value>.*?)</div>", re.IGNORECASE | re.DOTALL)
    ALLOWED_TITLE_KEYWORDS = ("公开招标", "招标公告", "采购公告")
    EXCLUDED_TITLE_KEYWORDS = ("中标", "成交", "结果", "更正", "变更", "澄清", "终止", "废标", "答疑", "资格预审结果")
    SUPPORTED_STAGE_CODES = {"0101", "0201"}
    REGION_BY_CODE = {
        "110000": "北京", "120000": "天津", "130000": "河北", "140000": "山西", "150000": "内蒙古",
        "210000": "辽宁", "220000": "吉林", "230000": "黑龙江", "310000": "上海", "320000": "江苏",
        "330000": "浙江", "340000": "安徽", "350000": "福建", "360000": "江西", "370000": "山东",
        "410000": "河南", "420000": "湖北", "430000": "湖南", "440000": "广东", "450000": "广西",
        "460000": "海南", "500000": "重庆", "510000": "四川", "520000": "贵州", "530000": "云南",
        "540000": "西藏", "610000": "陕西", "620000": "甘肃", "630000": "青海", "640000": "宁夏",
        "650000": "新疆", "660000": "兵团", "0067": "甘肃",
    }

    def __init__(self, targeting: dict[str, Any] | None = None, *, list_url: str | None = None, max_projects: int = 10, timeout_seconds: int = 8, budget_seconds: int = 95, detail_text_limit: int = 2000) -> None:
        self.list_url = (list_url or self.BASE_URL).strip() or self.BASE_URL
        self.max_projects = max(1, max_projects)
        self.timeout_seconds = max(5, timeout_seconds)
        self.budget_seconds = max(15, budget_seconds)
        self.detail_text_limit = max(500, detail_text_limit)
        targeting = targeting or {}
        self.targeting = {
            "mode": str(targeting.get("mode", "broad")).strip().lower(),
            "profile_key": str(targeting.get("profile_key", "")).strip(),
            "profile_title": str(targeting.get("profile_title", "")).strip(),
            "keywords": self._normalize_targeting_terms(targeting.get("keywords"), limit=6),
            "regions": self._normalize_targeting_terms(targeting.get("regions"), limit=4),
            "qualification_terms": self._normalize_targeting_terms(targeting.get("qualification_terms"), limit=5),
            "industry_terms": self._normalize_targeting_terms(targeting.get("industry_terms"), limit=5),
        }

    def collect(self) -> dict[str, Any]:
        deadline = time.monotonic() + self.budget_seconds
        list_html = self._fetch_text(self.list_url, referer=self.BASE_URL, deadline=deadline)
        list_items = self._parse_list_items(list_html)
        candidate_items = self._prioritize_list_items(list_items)
        matched_projects, fallback_projects, failures = [], [], []
        incomplete_count = 0
        budget_exhausted = False
        candidate_limit = self.max_projects * 3 if self._is_targeted_mode() else self.max_projects
        for item in candidate_items[:candidate_limit]:
            if self._remaining_seconds(deadline) < self.MIN_PROJECT_BUDGET_SECONDS:
                budget_exhausted = True
                failures.append("collect budget exhausted before visiting all candidate project pages")
                break
            try:
                project = self._collect_one(item, deadline=deadline)
                if not self._is_complete_project(project):
                    incomplete_count += 1
                    failures.append(f"{item['detail_url']}: incomplete project fields")
                    continue
                if self._matches_targeting(project):
                    matched_projects.append(project)
                    if len(matched_projects) >= self.max_projects:
                        break
                else:
                    fallback_projects.append(project)
                    if not self._is_targeted_mode() and len(fallback_projects) >= self.max_projects:
                        break
            except CollectorError as exc:
                failures.append(f"{item['detail_url']}: {exc.message}")
                if self._is_budget_exhausted_message(exc.message):
                    budget_exhausted = True
                    break
        projects = matched_projects[: self.max_projects] if self._is_targeted_mode() else (matched_projects[: self.max_projects] or fallback_projects[: self.max_projects])
        return {"projects": projects, "meta": {"list_url": self.list_url, "candidate_count": len(candidate_items), "project_count": len(projects), "matched_project_count": len(matched_projects), "incomplete_count": incomplete_count, "failure_count": len(failures), "budget_seconds": self.budget_seconds, "budget_exhausted": budget_exhausted, "remaining_seconds": round(self._remaining_seconds(deadline), 2), "targeting": self.targeting, "failures": failures[:5]}}

    def _collect_one(self, item: dict[str, str], *, deadline: float) -> dict[str, Any]:
        outer_url = item["detail_url"]
        actual_url = self._derive_actual_detail_url(outer_url)
        try:
            detail_html = self._fetch_text(actual_url, referer=outer_url, deadline=deadline)
        except CollectorError as exc:
            if self._is_budget_exhausted_message(exc.message):
                raise
            outer_html = self._fetch_text(outer_url, referer=self.list_url, deadline=deadline)
            actual_url = self._extract_actual_detail_url(outer_html, outer_url)
            detail_html = outer_html if actual_url == outer_url else self._fetch_text(actual_url, referer=outer_url, deadline=deadline)
        title = self._clean_text(self._extract_first(detail_html, self.TITLE_PATTERNS) or item["title"])
        if not self._is_supported_title(title):
            raise CollectorError("detail page is not a supported notice type")
        detail_text = self._truncate_detail_text(self._extract_detail_text(detail_html))
        published_at = self._normalize_datetime(self._extract_first(detail_html, (self.PUBLISHED_AT_PATTERN,)) or item["published_at"])
        platform_name = self._clean_text(self._extract_first(detail_html, (self.PLATFORM_PATTERN,)) or "")
        original_url = self._clean_text(self._extract_first(detail_html, (self.ORIGINAL_URL_PATTERN,), group_name="url") or "")
        region = self._extract_region(platform_name, actual_url, detail_text)
        project_code = self._extract_first_text(detail_text, [r"(?:招标项目编号|项目编号|采购项目编号|招标编号|项目代码)[:：]?\s*([A-Za-z0-9\-_]+)"])
        tender_unit = self._sanitize_organization_name(self._extract_first_text(detail_text, [r"(?:招标人|采购人|采购单位|招标单位|项目业主)\s*[:：]\s*([^\n]+)", r"(?:招标人|采购人|采购单位|招标单位|项目业主)\s*为\s*([^\n]+)"]))
        budget_text = self._extract_first_text(detail_text, [r"(?:项目投资总额|投资总额|预算金额|采购预算|最高限价|招标控制价)\s*[为:：]?\s*([0-9][^\n，。,；;]{0,40}(?:元|万元|亿元))"])
        deadline_text = self._normalize_datetime(self._extract_first_text(detail_text, [r"(?:投标文件递交的截止时间为|投标截止时间|提交投标文件截止时间|响应文件提交截止时间|响应文件开启时间|开标时间)\s*[为:：]?\s*([0-9]{4}[年/-]\d{1,2}[月/-]\d{1,2}(?:日)?\s*\d{1,2}(?::|时)\d{2}(?::\d{2})?)", r"(?:投标文件递交的截止时间为|投标截止时间|提交投标文件截止时间|响应文件提交截止时间|响应文件开启时间|开标时间)\s*[为:：]?\s*([0-9]{4}-\d{2}-\d{2}\s+\d{2}:\d{2}(?::\d{2})?)"]))
        qualification_requirements = self._extract_qualification_requirements(detail_text)
        notice_type = self._infer_notice_type(title)
        source_notice_id = project_code or self._extract_notice_id(actual_url)
        keywords = self._build_keywords([title, region, tender_unit, project_code, *qualification_requirements[:2]])
        if not source_notice_id:
            raise CollectorError("missing source notice id")
        return {"source": "ggzy", "source_notice_id": source_notice_id, "title": title, "notice_type": notice_type, "region": region, "published_at": published_at, "detail_url": outer_url, "canonical_url": actual_url, "project_code": project_code, "tender_unit": tender_unit, "budget_text": budget_text, "deadline_text": deadline_text, "detail_text": detail_text, "qualification_requirements": qualification_requirements, "keywords": keywords, "original_url": original_url, "platform_name": platform_name}

    def _parse_list_items(self, html_text: str) -> list[dict[str, str]]:
        items, seen_urls = [], set()
        for match in self.LIST_ITEM_PATTERN.finditer(html_text):
            title = self._clean_text(match.group("title"))
            href = urljoin(self.BASE_URL, self._clean_text(match.group("href")))
            published_at = self._normalize_datetime(match.group("published_at"))
            stage_code = self._extract_stage_code(href)
            if not href or href in seen_urls or stage_code not in self.SUPPORTED_STAGE_CODES:
                continue
            if title and any(keyword in title for keyword in self.EXCLUDED_TITLE_KEYWORDS):
                continue
            seen_urls.add(href)
            items.append({"title": title, "detail_url": href, "published_at": published_at})
        return items

    def _prioritize_list_items(self, items: list[dict[str, str]]) -> list[dict[str, str]]:
        if not self._is_targeted_mode():
            return items
        scored_items, has_positive_score = [], False
        for item in items:
            score = self._score_targeting_text(" ".join([item.get("title", ""), item.get("detail_url", "")]))
            if score > 0:
                has_positive_score = True
            scored_items.append((score, item))
        scored_items.sort(key=lambda row: row[0], reverse=True)
        return ([item for score, item in scored_items if score > 0] + [item for score, item in scored_items if score <= 0]) if has_positive_score else [item for _, item in scored_items]

    def _fetch_text(self, url: str, *, referer: str, deadline: float | None = None) -> str:
        request_timeout = float(self.timeout_seconds)
        if deadline is not None:
            remaining_seconds = self._remaining_seconds(deadline)
            if remaining_seconds <= self.MIN_REQUEST_BUDGET_SECONDS:
                raise CollectorError(self.BUDGET_EXHAUSTED_MESSAGE)
            request_timeout = min(request_timeout, remaining_seconds)
        request = Request(url, headers={"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36", "Accept-Language": "zh-CN,zh;q=0.9", "Referer": referer})
        try:
            with urlopen(request, timeout=request_timeout) as response:
                raw_bytes = response.read()
                charset = response.headers.get_content_charset() or "utf-8"
        except Exception as exc:
            raise CollectorError(f"failed to fetch ggzy page: {url} ({exc})") from exc
        try:
            return raw_bytes.decode(charset, errors="replace")
        except LookupError:
            return raw_bytes.decode("utf-8", errors="replace")

    def _derive_actual_detail_url(self, outer_url: str) -> str:
        return outer_url.replace("/html/a/", "/html/b/")

    def _extract_actual_detail_url(self, outer_html: str, outer_url: str) -> str:
        for pattern in self.ACTUAL_DETAIL_PATTERNS:
            match = pattern.search(outer_html)
            if match:
                return urljoin(self.BASE_URL, match.group("url"))
        return self._derive_actual_detail_url(outer_url)

    def _extract_detail_text(self, detail_html: str) -> str:
        content_html = self._extract_first(detail_html, (self.DETAIL_CONTENT_PATTERN,)) or detail_html
        normalized = re.sub(r"<\s*br\s*/?>", "\n", content_html, flags=re.IGNORECASE)
        normalized = re.sub(r"</p\s*>", "\n", normalized, flags=re.IGNORECASE)
        normalized = re.sub(r"</tr\s*>", "\n", normalized, flags=re.IGNORECASE)
        normalized = re.sub(r"</td\s*>", "\t", normalized, flags=re.IGNORECASE)
        normalized = re.sub(r"<script.*?</script>", "", normalized, flags=re.IGNORECASE | re.DOTALL)
        normalized = re.sub(r"<style.*?</style>", "", normalized, flags=re.IGNORECASE | re.DOTALL)
        normalized = re.sub(r"<[^>]+>", "", normalized)
        normalized = unescape(normalized).replace("\xa0", " ").replace("\r", "\n")
        normalized = re.sub(r"\n\s*\n+", "\n\n", normalized)
        normalized = re.sub(r"[ \t]+", " ", normalized)
        return normalized.strip()

    def _truncate_detail_text(self, detail_text: str) -> str:
        if len(detail_text) <= self.detail_text_limit:
            return detail_text
        truncated = detail_text[: self.detail_text_limit].strip()
        if " " in truncated:
            truncated = truncated.rsplit(" ", 1)[0].strip()
        return f"{truncated}\n...[truncated]"

    def _extract_first(self, text: str, patterns: tuple[re.Pattern[str], ...], *, group_name: str = "value") -> str:
        for pattern in patterns:
            match = pattern.search(text)
            if match:
                return match.group(group_name)
        return ""

    def _extract_first_text(self, text: str, patterns: list[str]) -> str:
        for pattern in patterns:
            match = re.search(pattern, text, re.IGNORECASE)
            if match:
                return self._clean_text(match.group(1))
        return ""

    def _extract_qualification_requirements(self, detail_text: str) -> list[str]:
        lines = [self._clean_text(line) for line in detail_text.splitlines() if self._clean_text(line)]
        heading_index = -1
        for idx, line in enumerate(lines):
            if any(keyword in line for keyword in ("资格要求", "资质要求", "投标人资格要求", "供应商资格要求")):
                heading_index = idx
                break
        requirements = []
        if heading_index >= 0:
            for line in lines[heading_index: heading_index + 6]:
                if line not in requirements:
                    requirements.append(line[:200])
        if not requirements:
            for line in lines:
                if any(keyword in line for keyword in ("资质", "业绩", "联合体", "营业执照", "安全生产许可证")):
                    if line not in requirements:
                        requirements.append(line[:200])
                if len(requirements) >= 4:
                    break
        return requirements[:4]

    def _infer_notice_type(self, title: str) -> str:
        if "公开招标" in title: return "公开招标公告"
        if "采购公告" in title: return "采购公告"
        if "招标公告" in title: return "招标公告"
        return ""

    def _extract_region(self, platform_name: str, detail_url: str, detail_text: str) -> str:
        text_region = self._extract_first_text(detail_text, [r"项目所在地区[:：]\s*([^\n]+)", r"所属地区[:：]\s*([^\n]+)", r"行政区域[:：]\s*([^\n]+)"])
        if text_region:
            return self._clean_text(text_region)[:60]
        match = re.search(r"（(?P<value>[^）]+)）", platform_name)
        if match:
            return self._clean_text(match.group("value")).replace("省", "").replace("市", "")
        for part in [part for part in urlparse(detail_url).path.split("/") if part]:
            if part in self.REGION_BY_CODE:
                return self.REGION_BY_CODE[part]
        return ""

    def _extract_notice_id(self, url: str) -> str:
        return urlparse(url).path.rsplit("/", 1)[-1].replace(".html", "").strip()

    def _extract_stage_code(self, url: str) -> str:
        match = re.search(r"/information/deal/html/[ab]/[^/]+/(?P<stage>\d{4})/", url)
        return match.group("stage") if match else ""

    def _normalize_datetime(self, raw_value: str) -> str:
        value = self._clean_text(raw_value).replace("/", "-")
        if not value:
            return ""
        if "年" in value:
            value = value.replace("年", "-").replace("月", "-").replace("日", " ").replace("时", ":").replace("分", "")
            value = re.sub(r"\s+", " ", value).strip()
        if re.match(r"^(\d{4}-\d{2}-\d{2})$", value):
            return re.sub(r"^(\d{4}-\d{2}-\d{2})$", r"\1 00:00:00", value)
        if re.match(r"^(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2})$", value):
            return re.sub(r"^(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2})$", r"\1:00", value)
        return value

    def _build_keywords(self, candidates: list[str]) -> list[str]:
        keywords = []
        for candidate in candidates:
            cleaned = self._clean_text(candidate)
            if cleaned and cleaned not in keywords:
                keywords.append(cleaned[:80])
            if len(keywords) >= 6:
                break
        return keywords

    def _sanitize_organization_name(self, value: str) -> str:
        cleaned = self._clean_text(value)
        if not cleaned: return ""
        for token in ("。", "，", ",", "；", ";", "项目已具备", "现对", "项目资金", "建设资金"):
            if token in cleaned:
                cleaned = cleaned.split(token, 1)[0].strip()
        return cleaned[:120]

    def _is_supported_title(self, title: str) -> bool:
        cleaned = self._clean_text(title)
        return bool(cleaned) and not any(keyword in cleaned for keyword in self.EXCLUDED_TITLE_KEYWORDS) and any(keyword in cleaned for keyword in self.ALLOWED_TITLE_KEYWORDS)

    def _is_complete_project(self, project: dict[str, Any]) -> bool:
        required_fields = ("source_notice_id", "title", "notice_type", "region", "published_at", "detail_url", "canonical_url", "detail_text")
        return not any(not self._clean_text(project.get(field, "")) for field in required_fields)

    def _matches_targeting(self, project: dict[str, Any]) -> bool:
        if not self._is_targeted_mode(): return False
        text = " ".join([str(project.get("title", "")), str(project.get("region", "")), str(project.get("project_code", "")), str(project.get("tender_unit", "")), str(project.get("detail_text", "")), " ".join(project.get("qualification_requirements", [])), " ".join(project.get("keywords", []))])
        return self._score_targeting_text(text) > 0

    def _score_targeting_text(self, text: str) -> int:
        if not self._is_targeted_mode(): return 0
        if self.targeting["regions"] and not any(term in text for term in self.targeting["regions"]): return 0
        score = 0
        for term in self.targeting["keywords"]:
            if term in text: score += 4
        for term in self.targeting["regions"]:
            if term in text: score += 3
        for term in self.targeting["qualification_terms"]:
            if term in text: score += 3
        for term in self.targeting["industry_terms"]:
            if term in text: score += 2
        return score

    def _normalize_targeting_terms(self, value: Any, *, limit: int) -> list[str]:
        normalized = []
        items = value if isinstance(value, list) else []
        for item in items:
            term = self._clean_text(str(item))
            if term and term not in normalized:
                normalized.append(term[:80])
            if len(normalized) >= limit:
                break
        return normalized

    def _is_targeted_mode(self) -> bool:
        return self.targeting.get("mode") == "targeted" and any(self.targeting[key] for key in ("keywords", "regions", "qualification_terms", "industry_terms"))

    def _remaining_seconds(self, deadline: float) -> float:
        return max(0.0, deadline - time.monotonic())

    def _is_budget_exhausted_message(self, message: str) -> bool:
        return str(message).startswith(self.BUDGET_EXHAUSTED_MESSAGE)

    def _clean_text(self, value: str) -> str:
        text = unescape(str(value or ""))
        text = re.sub(r"<[^>]+>", " ", text)
        text = text.replace("\xa0", " ").replace("：", ":")
        text = re.sub(r"\s+", " ", text)
        return text.strip()
