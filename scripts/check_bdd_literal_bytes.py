#!/usr/bin/env python3
"""QA check: BDD assertion literals MUST byte-exist in the repo.

背景（显示失真防复发）：
  本仓库/工具的显示层会把形如 `<...>` 的文本当作 HTML 外观样式解释，
  导致经渲染显示读取源码/文本时，`< think>`、`<read-files>`、`Arc<dyn XyModel>`
  一类含尖括号的内容可能被隐藏或改写。若干 BDD 探针曾因此写下与源码
  字节不一致的断言字面量（如把 `" thinking"` 写成 `<|im_start|>`），仅在
  字节级核对（python repr / od hex）后才收敛真值。

  本门禁把这一隐患机械化：凡 `tests/bdd/steps_*.rs` 中 `contains(...)` /
  `matches(...)` 的、含 ASCII 尖括号（`<`/`>`）的字符串字面量，MUST 以其
  完整字节在仓库文本树中真实存在；否则报 ERROR。这样任何「显示失真 →
  写错字面量」的复发（以及笔误、空格漂移）都会在 `just qa` 阶段被捕获，
  而不依赖人工判读。

规则边界（有意窄化，避免误报）：
  - 只检查含 `<` 或 `>` 的字面量（显示失真高风险区）；普通等宽字符串不查。
  - 只检查直接 `contains("…")` / `matches("…")` 调用的字面量，跳过
    `contains(&format!(…))`、`matches!` 宏、`r#"…"#` 原始串、跨行拼接。
  - 长度 < 4 的字面量跳过（如单独的 `"<"`）。
  - 转义按 Rust 规则解码（`\"` `\\` `\n` `\t` `\r`）后再做字节比对。

Usage:
  python3 scripts/check_bdd_literal_bytes.py --check
  python3 scripts/check_bdd_literal_bytes.py --check --verbose
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
STEPS_DIR = REPO / "tests" / "bdd"

# 仓库文本索引根；命中对象不限于代码——feature、docs、模板亦可能被探针断言。
# tests/bdd/ 是断言文本的来源，被排除以防「字面量必在自身中出现」的自证偏差。
TEXT_ROOTS = [
    "src",
    "tests",
    "packages",
    "llmanspec",
    "docs",
    "scripts",
    "designing",
    "tools",
]
EXCLUDED_DIRS = {REPO / "tests" / "bdd"}
TEXT_FILES = [
    "Cargo.toml",
    "Cargo.lock",
    "justfile",
    "AGENTS.md",
    "rust-toolchain.toml",
    "rustfmt.toml",
    ".editorconfig",
    ".gitignore",
]

COMPILED = re.compile(r'(?:contains|matches)\(\s*"((?:[^"\\]|\\.)*)"')


def _is_negated(line: str, quote_pos: int) -> bool:
    """True when the contains/matches call is a negative assertion
    (`!x.contains(…)` / `assert!(!x.contains(…))`) — the literal must NOT
    exist in the target, so presence is not required."""
    pre = line[:quote_pos]
    call_start = max(pre.rfind("contains("), pre.rfind("matches("))
    if call_start < 0:
        return False
    head = pre[:call_start].rstrip()
    idx = head.rfind("!")
    if idx < 0:
        return False
    tail = head[idx + 1:].strip()
    # `!x.contains(…` / `!a.b.contains(…` — receiver chain between `!` and the
    # call can only be identifiers / member (`.` `::`) access.
    return (
        re.fullmatch(r"[A-Za-z_]\w*(?:[.:][A-Za-z_]\w*)*\.?", tail) is not None
    )


def _decode_rust_escapes(s: str) -> bytes:
    out = bytearray()
    i = 0
    n = len(s)
    while i < n:
        c = s[i]
        if c == "\\" and i + 1 < n:
            nxt = s[i + 1]
            mapping = {
                '"': b'"',
                "\\": b"\\",
                "n": b"\n",
                "t": b"\t",
                "r": b"\r",
                "0": b"\0",
            }
            if nxt in mapping:
                out += mapping[nxt]
                i += 2
                continue
        out += c.encode("utf-8")
        i += 1
    return bytes(out)


def _iter_literals(path: Path):
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return
    for ln, line in enumerate(text.split("\n"), 1):
        for m in COMPILED.finditer(line):
            lit = _decode_rust_escapes(m.group(1))
            if len(lit) < 4:
                continue
            if b"<" not in lit and b">" not in lit:
                continue
            if _is_negated(line, m.start(1) - 1):
                continue
            yield ln, lit


def _text_blobs() -> list[bytes]:
    blobs: list[bytes] = []
    for root in TEXT_ROOTS:
        d = REPO / root
        if not d.exists():
            continue
        for p in sorted(d.rglob("*")):
            if not p.is_file():
                continue
            if any(p.is_relative_to(ex) for ex in EXCLUDED_DIRS):
                continue
            try:
                raw = p.read_bytes()
            except OSError:
                continue
            if raw[:1024].find(b"\0") >= 0:
                continue  # 二进制
            blobs.append(raw)
    for rel in TEXT_FILES:
        p = REPO / rel
        if p.exists():
            try:
                blobs.append(p.read_bytes())
            except OSError:
                continue
    return blobs


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit nonzero when any literal byte mismatch is found",
    )
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()

    if not STEPS_DIR.exists():
        print(f"error: {STEPS_DIR} missing", file=sys.stderr)
        return 1

    caught = []
    total = 0
    for f in sorted(STEPS_DIR.glob("steps_*.rs")):
        pending = list(_iter_literals(f))
        if not pending:
            continue
        total += len(pending)
        blobs = _text_blobs()
        for ln, lit in pending:
            if not any(lit in blob for blob in blobs):
                caught.append((f.name, ln, lit))

    if args.verbose and total:
        print(f"checked {total} angle-bracket literal(s) against repo bytes")

    if caught:
        print(
            f"error: {len(caught)} BDD angle-bracket literal(s) missing from repo bytes "
            "(display-layer <…> drift or typo — use python repr/od to confirm the true source):",
            file=sys.stderr,
        )
        for name, ln, lit in caught:
            print(f"  - {name}:{ln}: {lit!r}", file=sys.stderr)
        return 1

    if args.verbose:
        print("ok: all checked literals present in repo bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
