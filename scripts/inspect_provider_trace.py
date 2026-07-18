#!/usr/bin/env python3
"""Token-efficient provider-trace.jsonl inspector (c1265 / inspect skill).

Prints compact summaries only — never dumps full SSE payloads.
Default path: ~/.xylitol/logs/provider-trace.jsonl
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import time
from collections import Counter
from pathlib import Path
from typing import Any

DEFAULT_PATH = Path.home() / ".xylitol" / "logs" / "provider-trace.jsonl"
SCHEMA = "xylitol.provider_trace.v1"
MAX_TEXT_PREVIEW = 48

_SINCE_RE = re.compile(
    r"^\s*(?:(?P<h>\d+)\s*h)?\s*(?:(?P<m>\d+)\s*m)?\s*(?:(?P<s>\d+)\s*s)?\s*$",
    re.I,
)


def parse_since(spec: str) -> int | None:
    """Return cutoff ts_unix_ns, or None if empty. Accepts 30m / 1h / 90s / 1h30m."""
    spec = (spec or "").strip()
    if not spec:
        return None
    m = _SINCE_RE.match(spec)
    if not m or not any(m.group(g) for g in ("h", "m", "s")):
        raise SystemExit(f"error: bad --since {spec!r} (use e.g. 30m, 1h, 90s, 1h30m)")
    secs = (
        int(m.group("h") or 0) * 3600
        + int(m.group("m") or 0) * 60
        + int(m.group("s") or 0)
    )
    return time.time_ns() - secs * 1_000_000_000


def load_rows(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    if not path.is_file():
        return rows
    with path.open() as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError:
                continue
    return rows


def apply_filters(
    rows: list[dict[str, Any]],
    *,
    request_id: str | None,
    turn_id: str | None,
    trace_id: str | None,
    since_ns: int | None,
) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for r in rows:
        if request_id and r.get("request_id") != request_id:
            continue
        if turn_id and r.get("turn_id") != turn_id:
            continue
        if trace_id and r.get("trace_id") != trace_id:
            continue
        if since_ns is not None:
            ts = r.get("ts_unix_ns")
            if not isinstance(ts, int) or ts < since_ns:
                continue
        out.append(r)
    return out


def filter_rid(rows: list[dict[str, Any]], rid: str | None) -> list[dict[str, Any]]:
    if not rid:
        return rows
    return [r for r in rows if r.get("request_id") == rid]


def latest_request_id(rows: list[dict[str, Any]]) -> str | None:
    for r in reversed(rows):
        rid = r.get("request_id")
        if rid:
            return str(rid)
    return None


def latest_turn_id(rows: list[dict[str, Any]]) -> str | None:
    for r in reversed(rows):
        tid = r.get("turn_id")
        if tid:
            return str(tid)
    return None


def cmd_summary(rows: list[dict[str, Any]]) -> None:
    kinds: Counter[str] = Counter()
    variants: Counter[str] = Counter()
    events: Counter[str] = Counter()
    life_names: Counter[str] = Counter()
    rids: Counter[str] = Counter()
    turns: Counter[str] = Counter()
    for r in rows:
        kinds[str(r.get("kind") or "?")] += 1
        if r.get("variant"):
            variants[str(r["variant"])] += 1
        if r.get("event"):
            events[str(r["event"])] += 1
        if r.get("kind") == "lifecycle" and r.get("name"):
            life_names[str(r["name"])] += 1
        if r.get("request_id"):
            rids[str(r["request_id"])] += 1
        if r.get("turn_id"):
            turns[str(r["turn_id"])] += 1
    print(f"lines={len(rows)} schema_hint={SCHEMA}")
    print("kinds", dict(kinds.most_common()))
    print("variants", dict(variants.most_common(12)))
    print("top_raw_events", dict(events.most_common(8)))
    if life_names:
        print("lifecycle_names", dict(life_names.most_common()))
    print(
        "request_ids",
        len(rids),
        "latest",
        latest_request_id(rows) or "-",
        "turn_ids",
        len(turns),
        "latest_turn",
        (latest_turn_id(rows) or "-")[:8] + ("…" if latest_turn_id(rows) else ""),
    )


def cmd_recent(rows: list[dict[str, Any]], n: int) -> None:
    for r in rows[-n:]:
        kind = r.get("kind")
        bits = [
            f"kind={kind}",
            f"rid={(r.get('request_id') or '')[:8] or '-'}…",
        ]
        if r.get("turn_id"):
            bits.append(f"turn={str(r['turn_id'])[:8]}…")
        if r.get("event"):
            bits.append(f"event={r['event']}")
        if r.get("variant"):
            bits.append(f"variant={r['variant']}")
        if r.get("name"):
            bits.append(f"name={r['name']}")
        if kind == "lifecycle" and r.get("phase"):
            bits.append(f"phase={r['phase']}")
        text = r.get("text")
        if isinstance(text, str) and text and kind in ("raw", "mapped"):
            preview = text.replace("\n", "\\n")
            if len(preview) > MAX_TEXT_PREVIEW:
                preview = preview[: MAX_TEXT_PREVIEW - 1] + "…"
            bits.append(f"text={preview!r}")
        print(" ".join(bits))


def cmd_requests(rows: list[dict[str, Any]], n: int) -> None:
    order: list[str] = []
    seen: set[str] = set()
    for r in reversed(rows):
        rid = r.get("request_id")
        if not rid or rid in seen:
            continue
        seen.add(str(rid))
        order.append(str(rid))
        if len(order) >= n:
            break
    for rid in order:
        subset = filter_rid(rows, rid)
        kinds = Counter(str(x.get("kind") or "?") for x in subset)
        print(f"{rid}  n={len(subset)}  {dict(kinds)}")


def cmd_turns(rows: list[dict[str, Any]], n: int) -> None:
    order: list[str] = []
    seen: set[str] = set()
    for r in reversed(rows):
        tid = r.get("turn_id")
        if not tid or tid in seen:
            continue
        seen.add(str(tid))
        order.append(str(tid))
        if len(order) >= n:
            break
    for tid in order:
        subset = [r for r in rows if r.get("turn_id") == tid]
        names = Counter(
            str(r.get("name") or "?")
            for r in subset
            if r.get("kind") == "lifecycle"
        )
        print(f"{tid}  n={len(subset)}  lifecycle={dict(names)}")


def cmd_lag(rows: list[dict[str, Any]], rid: str | None) -> None:
    if not rid:
        rid = latest_request_id(rows)
    if not rid:
        print("error: no request_id in file")
        sys.exit(2)
    subset = filter_rid(rows, rid)
    raw_t = next(
        (
            int(r["ts_unix_ns"])
            for r in subset
            if r.get("kind") == "raw"
            and "function_call_arguments.delta" in str(r.get("event") or "")
            and isinstance(r.get("ts_unix_ns"), int)
        ),
        None,
    )
    map_t = next(
        (
            int(r["ts_unix_ns"])
            for r in subset
            if r.get("kind") == "mapped"
            and r.get("variant") in ("ToolCallStart", "ToolCallDelta")
            and isinstance(r.get("ts_unix_ns"), int)
        ),
        None,
    )
    life = sum(1 for r in subset if r.get("kind") == "lifecycle")
    print(f"request_id={rid}")
    print(f"t_first_args_delta_ns={raw_t}")
    print(f"t_first_mapped_tool_ns={map_t}")
    if raw_t is not None and map_t is not None:
        print(f"lag_ms={(map_t - raw_t) / 1e6:.3f}")
    elif raw_t is None and map_t is not None:
        print("note=mapped tools without args-delta raw (Completions dialect or missing emit)")
    elif raw_t is None and map_t is None:
        textish = any(r.get("variant") == "TextDelta" for r in subset)
        print(
            "note=no native tool stream; text-only"
            if textish
            else "note=no tool signals"
        )
    print(f"lifecycle_events={life}")
    print(f"rows_in_request={len(subset)}")


def cmd_lifecycle(rows: list[dict[str, Any]], rid: str | None, turn: str | None) -> None:
    subset = rows
    if rid:
        subset = filter_rid(subset, rid)
    if turn:
        subset = [r for r in subset if r.get("turn_id") == turn]
    if not rid and not turn:
        # Prefer latest turn window when available; else latest request.
        turn = latest_turn_id(subset)
        if turn:
            subset = [r for r in subset if r.get("turn_id") == turn]
        else:
            rid = latest_request_id(subset)
            if rid:
                subset = filter_rid(subset, rid)
    life = [r for r in subset if r.get("kind") == "lifecycle"]
    print(f"request_id={rid or '*'} turn_id={(turn or '*')[:36]}")
    print(f"lifecycle_n={len(life)}")
    for r in life[-40:]:
        print(
            f"  name={r.get('name') or '-'} phase={r.get('phase') or '-'} "
            f"turn={(r.get('turn_id') or '')[:8] or '-'} "
            f"tool={r.get('tool_name') or '-'}"
        )


def cmd_channel(rows: list[dict[str, Any]], rid: str | None) -> None:
    """Compact thinking vs text mix-up check."""
    if not rid:
        rid = latest_request_id(rows)
    subset = filter_rid(rows, rid) if rid else rows[-200:]
    print(f"request_id={rid or 'tail200'}")
    for r in subset:
        if r.get("kind") != "raw":
            continue
        ev = str(r.get("event") or "")
        if "reasoning" in ev or "output_text" in ev or "content_block" in ev:
            text = str(r.get("text") or "").replace("\n", "\\n")
            if len(text) > MAX_TEXT_PREVIEW:
                text = text[: MAX_TEXT_PREVIEW - 1] + "…"
            print(f"  raw event={ev} text={text!r}")
    mapped = Counter(
        str(r.get("variant"))
        for r in subset
        if r.get("kind") == "mapped" and r.get("variant")
    )
    print("mapped_variants", dict(mapped))


def add_common_filters(p: argparse.ArgumentParser) -> None:
    p.add_argument("--request-id", default="", help="filter by request_id")
    p.add_argument("--turn-id", default="", help="filter by turn_id (c1265)")
    p.add_argument("--trace-id", default="", help="filter by fastrace trace_id")
    p.add_argument(
        "--since",
        default="",
        help="only rows newer than duration (30m, 1h, 90s, 1h30m)",
    )


def main() -> None:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--path",
        type=Path,
        default=DEFAULT_PATH,
        help=f"JSONL path (default {DEFAULT_PATH})",
    )
    add_common_filters(p)
    sub = p.add_subparsers(dest="cmd", required=True)

    sub.add_parser("summary", help="kinds/variants/request_id counts")
    recent = sub.add_parser("recent", help="last N compact lines")
    recent.add_argument("-n", type=int, default=30)
    reqs = sub.add_parser("requests", help="latest request_ids with kind counts")
    reqs.add_argument("-n", type=int, default=8)
    turns = sub.add_parser("turns", help="latest turn_ids with lifecycle names")
    turns.add_argument("-n", type=int, default=8)
    lag = sub.add_parser("lag", help="args-delta → mapped tool lag for one request")
    # lag keeps optional override; global --request-id also applies via filter
    lag.add_argument("--rid", dest="lag_rid", default="", help="alias: prefer this request")
    life = sub.add_parser("lifecycle", help="react.turn/stream/tool.execute events")
    life.add_argument("--rid", dest="life_rid", default="", help="prefer this request_id")
    ch = sub.add_parser("channel", help="raw reasoning/text vs mapped variants")

    args = p.parse_args()
    since_ns = parse_since(args.since)
    rows = load_rows(args.path)
    rows = apply_filters(
        rows,
        request_id=args.request_id or None,
        turn_id=args.turn_id or None,
        trace_id=args.trace_id or None,
        since_ns=since_ns,
    )
    if not rows and args.cmd != "summary":
        print(f"error: empty or missing {args.path} (after filters)", file=sys.stderr)
        sys.exit(1)

    if args.cmd == "summary":
        cmd_summary(rows)
    elif args.cmd == "recent":
        cmd_recent(rows, args.n)
    elif args.cmd == "requests":
        cmd_requests(rows, args.n)
    elif args.cmd == "turns":
        cmd_turns(rows, args.n)
    elif args.cmd == "lag":
        cmd_lag(rows, args.lag_rid or args.request_id or None)
    elif args.cmd == "lifecycle":
        cmd_lifecycle(
            rows,
            args.life_rid or args.request_id or None,
            args.turn_id or None,
        )
    elif args.cmd == "channel":
        cmd_channel(rows, args.request_id or None)


if __name__ == "__main__":
    main()
