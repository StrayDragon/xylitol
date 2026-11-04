#!/usr/bin/env python3
"""TUI chrome visual preview (no product code).

Renders mock frames for the editor chrome variants discussed in
c3130-update-tui-editor-chrome. Outputs ANSI to stdout and an HTML file.

Usage:
  python3 visual_preview.py --variant all
  python3 visual_preview.py --variant plain-hex-summary --no-color
"""
from __future__ import annotations
import argparse
import html
import os
import sys
import unicodedata

RESET = "\033[0m"
ACCENT = "\033[38;5;39m"
MUTED = "\033[38;5;245m"
BORDER = "\033[38;5;240m"
SUCCESS = "\033[38;5;114m"
WARN = "\033[38;5;214m"
DIM = "\033[2m"

def disp_width(s: str) -> int:
    w = 0
    for ch in s:
        if unicodedata.combining(ch):
            continue
        ea = unicodedata.east_asian_width(ch)
        w += 2 if ea in ("W", "F") else 1
    return w

def pad(s: str, width: int) -> str:
    return s + " " * max(0, width - disp_width(s))

def truncate(s: str, width: int) -> str:
    if disp_width(s) <= width:
        return s
    out = ""
    for ch in s:
        if disp_width(out + ch) > width - 1:
            break
        out += ch
    return out + "…"

SPINNERS = {
    "hex": ["⬡", "⬢", "⬣", "⬢"],
    "diamond": ["◇", "◆", "◈", "◆"],
    "pulse": ["⬢", "⬢", "⬢", "⬢"],
    "ascii": ["o", "O", "0", "O"],
}

def spinner_glyph(kind: str, frame: int) -> str:
    frames = SPINNERS.get(kind, SPINNERS["hex"])
    return frames[frame % len(frames)]

def top_border(width: int, stage: str, fold: str, corners: str, color: bool) -> list[tuple[str, str]]:
    right = "╮" if corners == "rounded" else "─"
    left = "╭" if corners == "rounded" else "─"
    title = f" {stage} " if stage else ""
    trailing = 3
    fixed = disp_width(left) + disp_width(right) + disp_width(title) + trailing
    dash_n = max(0, width - fixed)
    parts = [(left, "border"), ("─" * dash_n, "border")]
    if title:
        parts.append((title, "muted"))
    parts.append(("─" * trailing, "border"))
    parts.append((right, "border"))
    return parts

def bottom_border(width: int, fold: str, corners: str, color: bool) -> list[tuple[str, str]]:
    fold_labels = {"quiet": "全折", "summary": "摘要", "recent": "最近", "full": "全展"}
    fold_chip = f"[▤ {fold_labels.get(fold, fold)}]"
    right = "╯" if corners == "rounded" else "─"
    left = "╰" if corners == "rounded" else "─"
    trailing = 3
    fixed = disp_width(left) + disp_width(right) + disp_width(fold_chip) + trailing + 1
    dash_n = max(0, width - fixed)
    parts = [(left, "border"), ("─" * dash_n, "border"), (" ", "plain")]
    parts.append((fold_chip, "accent" if fold != "summary" else "muted"))
    parts.append(("─" * trailing, "border"))
    parts.append((right, "border"))
    return parts

def colorize(parts: list[tuple[str, str]], color: bool) -> str:
    if not color:
        return "".join(t for t, _ in parts)
    codes = {
        "accent": ACCENT,
        "muted": MUTED,
        "border": BORDER,
        "success": SUCCESS,
        "warn": WARN,
        "dim": DIM,
        "plain": "",
    }
    out = ""
    for text, kind in parts:
        code = codes.get(kind, "")
        if code:
            out += code + text + RESET
        else:
            out += text
    return out

def line(parts: list[tuple[str, str]], width: int, color: bool) -> str:
    raw = "".join(t for t, _ in parts)
    if disp_width(raw) < width:
        parts = parts + [(" " * (width - disp_width(raw)), "plain")]
    return colorize(parts, color)

def frame(variant: str, width: int, color: bool) -> list[str]:
    corners, spinner, fold = variant.split("-")
    lines: list[str] = []
    stage = "新功能A"

    if fold == "quiet":
        lines.append(line([("❯ ", "muted"), (truncate("line 001 这是一段用于观察提交后占用面积的长文本内容。", width - 2), "plain")], width, color))
        lines.append(line([("  ... (77 more lines, click to expand)", "muted")], width, color))
        lines.append("")
        lines.append(line([("▸ Worked for 12s", "muted")], width, color))
        lines.append(line([("  结论：当前 thinking 流式阶段总是展开，工具开始后会被折叠成一行。", "plain")], width, color))
    elif fold == "summary":
        lines.append(line([("❯ ", "muted"), (truncate("line 001 这是一段用于观察提交后占用面积的长文本内容。", width - 2), "plain")], width, color))
        lines.append(line([("  ... (77 more lines, click to expand)", "muted")], width, color))
        lines.append("")
        lines.append(line([("▸ Thinking · 我先梳理一下这个改动的边界：用户想要在 tmux 里观察…", "muted")], width, color))
        lines.append(line([("▸ Ran 1 command", "muted")], width, color))
        lines.append(line([("  结论：当前 thinking 流式阶段总是展开，工具开始后会被折叠成一行。", "plain")], width, color))
    elif fold == "recent":
        lines.append(line([("❯ ", "muted"), (truncate("line 001 这是一段用于观察提交后占用面积的长文本内容。", width - 2), "plain")], width, color))
        lines.append(line([("  ... (77 more lines, click to expand)", "muted")], width, color))
        lines.append("")
        lines.append(line([("▾ Thinking", "accent")], width, color))
        lines.append(line([("  我先梳理一下这个改动的边界：用户想要在 tmux 里观察 TUI 的流式渲染。", "plain")], width, color))
        lines.append(line([("▾ Bash $ echo hello-from-lab", "success")], width, color))
        lines.append(line([("  hello-from-lab", "plain")], width, color))
        lines.append(line([("  结论：当前 thinking 流式阶段总是展开，工具开始后会被折叠成一行。", "plain")], width, color))
    else:
        lines.append(line([("❯ ", "muted"), (truncate("line 001 这是一段用于观察提交后占用面积的长文本内容。", width - 2), "plain")], width, color))
        lines.append(line([("  line 002 … line 080（全展开）", "plain")], width, color))
        lines.append("")
        lines.append(line([("▾ Thinking", "accent")], width, color))
        lines.append(line([("  我先梳理一下这个改动的边界：用户想要在 tmux 里观察 TUI 的流式渲染。", "plain")], width, color))
        lines.append(line([("▾ Bash $ true # 这是一段很长的工具参数用来观察工具头换行占用面积。", "success")], width, color))
        lines.append(line([("  (no output)", "muted")], width, color))
        lines.append(line([("  结论：当前 thinking 流式阶段总是展开，工具开始后会被折叠成一行。", "plain")], width, color))

    glyph = spinner_glyph(spinner, 1)
    lines.append("")
    lines.append(line([(glyph + " ", "accent"), ("Drafting reply", "muted")], width, color))
    lines.append(line(top_border(width, stage, fold, corners, color), width, color))
    prefix = "❯ "
    sidebar = "◨"
    content_w = max(0, width - disp_width(prefix) - disp_width(sidebar) - 1)
    lines.append(line([(prefix, "muted"), (" " * content_w, "plain"), (" ", "plain"), (sidebar, "accent")], width, color))
    lines.append(line(bottom_border(width, fold, corners, color), width, color))
    lines.append(line([("~/Projects/xylitol · local-model · high", "muted")], width, color))
    return lines

ANSI_CLASS = {
    "38;5;39": "accent",
    "38;5;245": "muted",
    "38;5;240": "border",
    "38;5;114": "success",
    "38;5;214": "warn",
    "2": "dim",
}

def ansi_to_html(line: str) -> str:
    import re
    out = []
    cls = None
    for part in re.split(r"(\033\[[0-9;]*m)", line):
        if part.startswith("\033["):
            code = part[2:-1]
            if code == "0":
                cls = None
            else:
                cls = ANSI_CLASS.get(code, cls)
        else:
            esc = html.escape(part)
            out.append(f'<span class="{cls}">{esc}</span>' if cls else esc)
    return "".join(out)

def html_frames(frames: dict[str, list[str]]) -> str:
    css = """
    body { background: #111; color: #ddd; font-family: Menlo, monospace; padding: 24px; }
    pre { background: #1a1a1a; border: 1px solid #333; padding: 12px; overflow-x: auto; }
    h2 { color: #9cf; font-size: 14px; margin-top: 24px; }
    .accent { color: #4fc3f7; }
    .muted { color: #9aa0a6; }
    .border { color: #5f6368; }
    .success { color: #81c784; }
    .warn { color: #ffb74d; }
    .dim { color: #666; }
    """
    def spanify(line: str) -> str:
        return ansi_to_html(line)
    parts = ["<!doctype html><html><head><meta charset='utf-8'><style>", css, "</style></head><body>"]
    parts.append("<h1>TUI chrome visual preview</h1>")
    parts.append("<p>Plain text rendering of the mock frames. ANSI colors are visible when running the script in a terminal.</p>")
    for name, lines in frames.items():
        parts.append(f"<h2>{html.escape(name)}</h2><pre>")
        parts.append("\n".join(spanify(l) for l in lines))
        parts.append("</pre>")
    parts.append("</body></html>")
    return "".join(parts)

def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--variant", default="all")
    ap.add_argument("--width", type=int, default=100)
    ap.add_argument("--no-color", action="store_true")
    ap.add_argument("--html", default=os.path.join(os.path.dirname(os.path.abspath(__file__)), "visual_preview.html"))
    args = ap.parse_args()

    variants = [
        "rounded-pulse-summary",
        "rounded-hex-summary",
        "plain-pulse-summary",
        "plain-hex-summary",
        "plain-diamond-summary",
        "rounded-pulse-quiet",
        "rounded-pulse-recent",
        "rounded-pulse-full",
    ]
    if args.variant != "all":
        variants = [args.variant]

    color = not args.no_color and sys.stdout.isatty()
    frames = {}
    ansi_frames = {}
    for v in variants:
        frames[v] = frame(v, args.width, color)
        ansi_frames[v] = frame(v, args.width, True)

    if args.variant == "all":
        for name, lines in frames.items():
            print(f"\n=== {name} ===")
            for l in lines:
                print(l)
    else:
        for l in frames[variants[0]]:
            print(l)

    with open(args.html, "w", encoding="utf-8") as f:
        f.write(html_frames(ansi_frames))
    print(f"\nHTML written: {args.html}", file=sys.stderr)
    return 0

if __name__ == "__main__":
    sys.exit(main())
