import argparse
import base64
import json
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--targeting-json-b64", default="")
    parser.add_argument("--profile-key", default="")
    parser.add_argument("--profile-title", default="")
    parser.add_argument("--keyword", action="append", default=[])
    parser.add_argument("--region", action="append", default=[])
    parser.add_argument("--qualification-term", action="append", default=[])
    parser.add_argument("--industry-term", action="append", default=[])
    parser.add_argument("--list-url", default="https://www.ggzy.gov.cn/")
    parser.add_argument("--max-projects", type=int, default=10)
    parser.add_argument("--timeout-seconds", type=int, default=8)
    parser.add_argument("--budget-seconds", type=int, default=95)
    parser.add_argument("--detail-text-limit", type=int, default=2000)
    return parser.parse_args()


def decode_targeting_payload(raw_value: str) -> dict:
    if not raw_value.strip():
        return {}
    try:
        padding = "=" * (-len(raw_value) % 4)
        decoded = base64.urlsafe_b64decode(f"{raw_value}{padding}".encode("ascii"))
        payload = json.loads(decoded.decode("utf-8"))
    except Exception:
        return {}
    return payload if isinstance(payload, dict) else {}


def main() -> int:
    base_dir = Path(__file__).resolve().parent
    if str(base_dir) not in sys.path:
        sys.path.insert(0, str(base_dir))
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")

    from ggzy_collector import GgzyCollector

    args = parse_args()
    targeting = decode_targeting_payload(args.targeting_json_b64)
    if not targeting:
        targeting = {
            "mode": "targeted"
            if any([args.keyword, args.region, args.qualification_term, args.industry_term])
            else "broad",
            "profile_key": args.profile_key,
            "profile_title": args.profile_title,
            "keywords": args.keyword,
            "regions": args.region,
            "qualification_terms": args.qualification_term,
            "industry_terms": args.industry_term,
        }

    payload = GgzyCollector(
        targeting=targeting,
        list_url=args.list_url,
        max_projects=args.max_projects,
        timeout_seconds=args.timeout_seconds,
        budget_seconds=args.budget_seconds,
        detail_text_limit=args.detail_text_limit,
    ).collect()
    print(json.dumps({"projects": payload.get("projects", []), "meta": payload.get("meta", {})}, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
