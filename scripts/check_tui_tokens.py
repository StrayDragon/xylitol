#!/usr/bin/env python3
"""QA check: crates/xylitol-tui Palette hex vs src/app/tui/DESIGN.md frontmatter.

SSOT: DESIGN.md colors.* (dark) + colors_light.* (light).
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DESIGN = REPO / "src" / "app" / "tui" / "DESIGN.md"
PALETTE = REPO / "crates" / "xylitol-tui" / "src" / "theme" / "palette.rs"

TOKEN_TO_FIELD = {
    "on-surface": "on_surface",
    "muted": "muted",
    "accent": "accent",
    "user": "user",
    "assistant": "assistant",
    "tool": "tool",
    "error": "error",
    "warning": "warning",
    "success": "success",
    "diff-added": "diff_added",
    "diff-removed": "diff_removed",
    "diff-context": "diff_context",
    "diff-added-bg": "diff_added_bg",
    "diff-removed-bg": "diff_removed_bg",
    "diff-added-word-bg": "diff_added_word_bg",
    "diff-removed-word-bg": "diff_removed_word_bg",
    "surface": "surface",
    "tool-pending-bg": "tool_pending_bg",
    "tool-success-bg": "tool_success_bg",
    "tool-error-bg": "tool_error_bg",
    "user-message-bg": "user_message_bg",
    "skill-ref": "skill_ref",
}


def parse_color_block(fm: str, key: str) -> list[tuple[str, str]]:
    block = re.search(rf"^{key}:\n(.*?)(?=^[a-z_]+:|\Z)", fm, re.S | re.M)
    if not block:
        raise SystemExit(f"DESIGN.md: missing {key}: block")
    pairs = re.findall(
        r"^\s+([a-z0-9-]+):\s*\"?(#[0-9a-fA-F]{6})\"?",
        block.group(1),
        re.M,
    )
    if not pairs:
        raise SystemExit(f"DESIGN.md: no color tokens in {key}")
    return [(n, h.lower()) for n, h in pairs]


def parse_schemes(text: str) -> tuple[list[tuple[str, str]], list[tuple[str, str]]]:
    m = re.search(r"^---\n(.*?)\n---", text, re.S | re.M)
    if not m:
        raise SystemExit("DESIGN.md: missing YAML frontmatter")
    fm = m.group(1)
    dark = parse_color_block(fm, "colors")
    light = parse_color_block(fm, "colors_light")
    dark_names = [n for n, _ in dark]
    light_names = [n for n, _ in light]
    if dark_names != light_names:
        raise SystemExit(
            "DESIGN.md: colors / colors_light keys must match "
            f"(dark={dark_names!r} light={light_names!r})"
        )
    return dark, light


def hex_from_rgb_call(r: int, g: int, b: int) -> str:
    return f"#{r:02x}{g:02x}{b:02x}"


def parse_palette_fn(src: str, fn_name: str) -> dict[str, str]:
    m = re.search(
        rf"pub const fn {fn_name}\(\) -> Self \{{(.*?)^\    \}}",
        src,
        re.S | re.M,
    )
    if not m:
        raise SystemExit(f"palette.rs: missing pub const fn {fn_name}()")
    body = m.group(1)
    out: dict[str, str] = {}
    for field, r, g, b in re.findall(
        r"(\w+):\s*rgb\(\s*0x([0-9a-fA-F]+)\s*,\s*0x([0-9a-fA-F]+)\s*,\s*0x([0-9a-fA-F]+)\s*\)",
        body,
    ):
        out[field] = hex_from_rgb_call(int(r, 16), int(g, 16), int(b, 16))
    if not out:
        raise SystemExit(f"palette.rs: no rgb(...) fields in {fn_name}()")
    return out


def check_palette(
    dark: list[tuple[str, str]], light: list[tuple[str, str]]
) -> list[str]:
    errs: list[str] = []
    if not PALETTE.is_file():
        return [f"palette.rs not found at {PALETTE}"]
    src = PALETTE.read_text(encoding="utf-8")
    got_dark = parse_palette_fn(src, "dark")
    got_light = parse_palette_fn(src, "light")
    for scheme_name, pairs, got in (
        ("Palette::dark", dark, got_dark),
        ("Palette::light", light, got_light),
    ):
        for token, want in pairs:
            field = TOKEN_TO_FIELD.get(token)
            if field is None:
                errs.append(f"DESIGN token {token!r} has no Palette field mapping")
                continue
            have = got.get(field)
            if have is None:
                errs.append(f"{scheme_name}: missing field {field} (token {token})")
            elif have != want:
                errs.append(
                    f"{scheme_name}.{field}: palette {have} != DESIGN {want} ({token})"
                )
        expected_fields = {TOKEN_TO_FIELD[t] for t, _ in pairs if t in TOKEN_TO_FIELD}
        missing_in_design = set(got) - expected_fields
        if missing_in_design:
            errs.append(
                f"{scheme_name}: fields not in DESIGN mapping: "
                f"{sorted(missing_in_design)}"
            )
    return errs


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--check",
        action="store_true",
        help="fail if Palette diverges from DESIGN.md",
    )
    ap.add_argument(
        "--verbose",
        action="store_true",
        help="print ok summary (default: silent success)",
    )
    args = ap.parse_args()
    if not DESIGN.is_file():
        print(f"error: DESIGN.md not found at {DESIGN}", file=sys.stderr)
        return 1
    dark, light = parse_schemes(DESIGN.read_text(encoding="utf-8"))
    errs = check_palette(dark, light)
    if errs:
        print("check-tui-tokens FAILED:", file=sys.stderr)
        for e in errs:
            print(f"  - {e}", file=sys.stderr)
        return 1
    if args.verbose:
        print(
            f"ok: Palette matches DESIGN.md "
            f"({len(dark)} dark / {len(light)} light)"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
