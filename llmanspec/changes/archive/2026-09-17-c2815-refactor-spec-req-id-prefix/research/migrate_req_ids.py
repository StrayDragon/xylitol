#!/usr/bin/env python3
"""One-shot migration: rewrite `@req:<old>` tags (non `r<num>`) to unique `r<1000+N>`.

Rules (see design.md):
- Already-compliant `@req:r<num>` stays untouched.
- Any other `@req:<id>` gets a stable per-file mapping to `r<1000+counter++>`.
- Same old id inside one file maps to the same new number (rule + executable refs).
- Cross-file id collisions are impossible by construction (per-file table + global counter).
- Emits research/req-id-migration-map.md for audit.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]  # <repo>/llmanspec/changes/<id>/research → parents[4] = repo root
SPECS_DIR = ROOT / "llmanspec" / "specs"
OUT_MAP = ROOT / "llmanspec" / "changes" / "c2815-refactor-spec-req-id-prefix" / "research" / "req-id-migration-map.md"

TAG_RE = re.compile(r"@req:([\w-]+)")

# Stage 1: scan and assign
next_number = 1000
files = sorted(p for p in SPECS_DIR.glob("*/*.feature"))
assignments: dict[Path, dict[str, int]] = {}
new_tags: dict[Path, int] = {}
for path in files:
    text = path.read_text(encoding="utf-8")
    ids = set(TAG_RE.findall(text))
    per_file: dict[str, int] = {}
    for old in sorted(ids):
        if re.fullmatch(r"r\d+", old):
            continue  # already compliant, leave as-is
        new_id = next_number
        next_number += 1
        per_file[old] = new_id
    assignments[path] = per_file

print(f"[scan] {len(files)} feature files, assigned {next_number - 1000} new ids (next={next_number})")

# Stage 2: rewrite tags
changed: list[tuple[Path, int]] = []
for path, mapping in assignments.items():
    text = path.read_text(encoding="utf-8")
    if not mapping:
        continue

    def repl(m: re.Match[str]) -> str:
        old = m.group(1)
        if old in mapping:
            return f"@req:r{mapping[old]}"
        return m.group(0)

    new_text = TAG_RE.sub(repl, text)
    if new_text != text:
        path.write_text(new_text, encoding="utf-8")
        changed.append((path, len(mapping)))

print(f"[rewrite] {len(changed)} files modified")

# Stage 3: uniqueness / residue self-check (cross-file uniqueness only)
residue: list[str] = []
all_new: dict[int, str] = {}
cross_dup: dict[int, tuple[str, str]] = {}
for path in SPECS_DIR.glob("*/*.feature"):
    text = path.read_text(encoding="utf-8")
    seen_in_file: set[int] = set()
    for m in TAG_RE.finditer(text):
        tag = m.group(1)
        if not re.fullmatch(r"r\d+", tag):
            residue.append(f"{path}:{tag}")
        else:
            num = int(tag[1:])
            if num in seen_in_file:
                continue  # same file, repeated reference — fine
            seen_in_file.add(num)
            if num in all_new:
                cross_dup[num] = (all_new[num], str(path))
            all_new[num] = str(path)

if cross_dup:
    print(f"[FAIL] cross-file duplicate r-numbers: {len(cross_dup)}", file=sys.stderr)
    for num, (a, b) in sorted(cross_dup.items()):
        print(f"  r{num}: {a} <-> {b}", file=sys.stderr)
    sys.exit(1)
if residue:
    print(f"[FAIL] residual non-r prefix tags: {len(residue)}", file=sys.stderr)
    for r in residue[:20]:
        print(f"  {r}", file=sys.stderr)
    sys.exit(1)
print(f"[check] no residual non-r tags, no cross-file duplicates; {len(all_new)} unique r-numbers present")

# Stage 4: emit mapping table
lines = [
    "# 全仓 @req 前缀迁移映射表（c2815）",
    "",
    f"- 迁移文件数：{len(changed)} / {len(files)}",
    f"- 新分配 id 数：{next_number - 1000}（区间 r1000..r{next_number - 1}）",
    f"- 既有合规 `r<数字>`：保留原号",
    "",
    "| capability 文件 | 旧 id | 新 id |",
    "|---|---|---|",
]
for path, mapping in assignments.items():
    cap = path.parent.name
    for old, new in sorted(mapping.items(), key=lambda kv: kv[1]):
        lines.append(f"| `{cap}` | `{old}` | `r{new}` |")
OUT_MAP.parent.mkdir(parents=True, exist_ok=True)
OUT_MAP.write_text("\n".join(lines) + "\n", encoding="utf-8")
print(f"[map] wrote {OUT_MAP.relative_to(ROOT)}")
