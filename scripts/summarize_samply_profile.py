#!/usr/bin/env python3
"""Token-efficient samply / Firefox-profiler JSON summary for xylitol (c1520).

Default: keep only `processName == xylitol` so agent-spawned cargo / rustc /
rust-analyzer / lspz / cc1 do not dominate the report.

Never prints full stacks dump — top-N leaves + buckets only.

Examples:
  python3 scripts/summarize_samply_profile.py target/profile/tui-baseline.json.gz
  python3 scripts/summarize_samply_profile.py target/profile/B-scroll.json.gz \\
      --addr2line ./target/release/xylitol -o target/profile/B-scroll.summary.txt
"""

from __future__ import annotations

import argparse
import collections
import gzip
import json
import subprocess
import sys
from pathlib import Path

# Agent tools / build toolchain often appear in the same samply session when the
# profiled xylitol run invokes MCP (lspz), cargo check, etc. Filter them out.
EXCLUDE_EXACT = {
    "rustc",
    "cc1",
    "cc",
    "as",
    "ar",
    "rust-lld",
    "cargo",
    "rust-analyzer",
    "lspz",
    "build-script-build",
    "build-script-main",
    "clippy-driver",
    "rustdoc",
    "samply",
}
EXCLUDE_SUB = (
    "rustc",
    "cc1",
    "cargo",
    "rust-analyzer",
    "lspz",
    "clang",
    "lld",
    "samply",
)


def proc_name(th: dict) -> str:
    return (th.get("processName") or th.get("name") or "").strip()


def keep_thread(th: dict, *, product: str) -> bool:
    p = proc_name(th)
    n = (th.get("name") or "").strip()
    pl, nl = p.lower(), n.lower()
    if pl in EXCLUDE_EXACT or nl in EXCLUDE_EXACT:
        return False
    if any(s in pl for s in EXCLUDE_SUB) or any(s in nl for s in EXCLUDE_SUB):
        return False
    return pl == product.lower() or product.lower() in pl


def sample_count(th: dict) -> int:
    samples = th.get("samples") or {}
    n = samples.get("length")
    if n is not None:
        return int(n)
    stack = samples.get("stack")
    return len(stack) if isinstance(stack, list) else 0


def string_at(th: dict, idx: int | None) -> str | None:
    if idx is None or idx < 0:
        return None
    sa = th.get("stringArray")
    if sa is None:
        st = th.get("stringTable") or {}
        sa = st.get("string") if isinstance(st, dict) else None
    if not sa or idx >= len(sa):
        return None
    return sa[idx]


def leaf_counts(th: dict) -> collections.Counter[str]:
    """Approximate self-time: leaf frame of each sample stack node."""
    samples = th.get("samples") or {}
    stacks = samples.get("stack") or []
    length = samples.get("length", len(stacks))
    frame = (th.get("stackTable") or {}).get("frame") or []
    funcs = (th.get("frameTable") or {}).get("func") or []
    fname = (th.get("funcTable") or {}).get("name") or []
    counts: collections.Counter[str] = collections.Counter()
    for i in range(min(int(length), len(stacks))):
        st = stacks[i]
        if st is None or st < 0 or st >= len(frame):
            continue
        fr = frame[st]
        if fr is None or fr < 0 or fr >= len(funcs):
            continue
        fi = funcs[fr]
        if fi is None or fi < 0 or fi >= len(fname):
            continue
        name = string_at(th, fname[fi]) or f"func#{fi}"
        counts[name] += 1
    return counts


def shorten_path(loc: str, repo: Path) -> str:
    home = str(Path.home())
    loc = loc.replace(f"{home}/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/", "$CARGO/")
    loc = loc.replace(
        f"{home}/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/",
        "$RUST/",
    )
    loc = loc.replace(f"{repo}/", "$XY/")
    return loc


def addr2line_batch(binary: Path, addrs: list[str]) -> list[tuple[str, str]]:
    if not addrs:
        return []
    cmd = ["llvm-addr2line", "-e", str(binary), "-f", "-C", *addrs]
    try:
        out = subprocess.run(cmd, capture_output=True, text=True, check=False)
    except FileNotFoundError:
        cmd[0] = "addr2line"
        out = subprocess.run(cmd, capture_output=True, text=True, check=False)
    lines = out.stdout.splitlines()
    it = iter(lines)
    pairs: list[tuple[str, str]] = []
    for _ in addrs:
        func = next(it, "??")
        loc = next(it, "??")
        pairs.append((func, loc))
    return pairs


def bucket(func: str, loc: str) -> str:
    L, F = loc.lower(), func.lower()
    if "unicode" in L or "grapheme" in F or "segmentation" in L:
        return "unicode/grapheme"
    if "xylitol-tui" in L or "packages/xylitol" in L:
        return "xylitol-tui"
    if "/src/app/" in L or "/src/agent/" in L or "/src/infra/" in L:
        return "xylitol app/agent/infra"
    if "crossterm" in L:
        return "crossterm"
    if "pulldown" in L or "markdown" in L:
        return "markdown"
    if "alloc" in L or "hashbrown" in L:
        return "alloc"
    if "core/src" in L or "std/src" in L or "$RUST/" in loc:
        return "stdlib/core"
    return "other"


# Inclusive stack buckets for c1505 / TUI hotspots (any frame in sample stack).
INCLUSIVE_RULES: list[tuple[str, tuple[str, ...]]] = [
    ("scroll_render", ("render_scrollback", "scrollback.rs")),
    ("wrap", ("wrap_text_with_ansi", "wrap_text")),
    ("width", ("visible_width", "strip_ansi", "ansi_escape_len")),
    ("do_render", ("do_render", "differential")),
    ("markdown", ("markdown::", "pulldown", "Markdown::")),
    ("upper_clone", ("upper_cache", "clone_from")),
]


def walk_stack_frames(
    stack_idx: int,
    *,
    frame_of_stack: list,
    prefix_of_stack: list,
) -> list[int]:
    out: list[int] = []
    seen: set[int] = set()
    cur = stack_idx
    while cur is not None and cur >= 0 and cur not in seen:
        seen.add(cur)
        if cur >= len(frame_of_stack):
            break
        fr = frame_of_stack[cur]
        if fr is not None and fr >= 0:
            out.append(int(fr))
        pref = prefix_of_stack[cur] if cur < len(prefix_of_stack) else None
        if pref is None or pref < 0:
            break
        cur = int(pref)
    return out


def inclusive_hotspots(
    th: dict,
    binary: Path | None,
    *,
    leaf_total: int,
) -> list[str]:
    """Count samples whose stack contains known TUI symbols (addr2line)."""
    lines: list[str] = []
    if not binary or not binary.is_file() or leaf_total <= 0:
        return lines

    samples = th.get("samples") or {}
    stacks = samples.get("stack") or []
    length = int(samples.get("length", len(stacks)))
    st = th.get("stackTable") or {}
    frame_of_stack = st.get("frame") or []
    prefix_of_stack = st.get("prefix") or []
    ft = th.get("frameTable") or {}
    addrs = ft.get("address") or []
    funcs = ft.get("func") or []
    fname = (th.get("funcTable") or {}).get("name") or []

    # Collect unique native addresses used on any sample stack.
    used_addrs: set[int] = set()
    sample_frames: list[list[int]] = []
    for i in range(min(length, len(stacks))):
        si = stacks[i]
        if si is None or si < 0:
            sample_frames.append([])
            continue
        frames = walk_stack_frames(
            int(si), frame_of_stack=frame_of_stack, prefix_of_stack=prefix_of_stack
        )
        sample_frames.append(frames)
        for fr in frames:
            if fr < len(addrs) and addrs[fr] is not None:
                used_addrs.add(int(addrs[fr]))

    if not used_addrs:
        return lines

    # Firefox profiler / samply often store runtime-relative addresses.
    # Match prior leaf path: stringArray already has 0x… for unknown leaves;
    # for inclusive we resolve via address hex like the leaf path when present.
    hex_addrs = [f"0x{a:x}" for a in sorted(used_addrs)]
    # Cap addr2line batch size for speed
    if len(hex_addrs) > 4000:
        hex_addrs = hex_addrs[:4000]

    pairs = addr2line_batch(binary, hex_addrs)
    resolved_by_hex: dict[str, tuple[str, str]] = {
        h: (func, loc) for h, (func, loc) in zip(hex_addrs, pairs)
    }

    bags: collections.Counter[str] = collections.Counter()
    for frames in sample_frames:
        if not frames:
            continue
        texts: list[str] = []
        for fr in frames:
            # Prefer demangled func table name when present
            if fr < len(funcs) and funcs[fr] is not None:
                name = string_at(th, funcs[fr])
                if name:
                    texts.append(name)
            if fr < len(addrs) and addrs[fr] is not None:
                h = f"0x{int(addrs[fr]):x}"
                if h in resolved_by_hex:
                    func, loc = resolved_by_hex[h]
                    texts.append(f"{func} {loc}")
        blob = "\n".join(texts).lower()
        hit_any = False
        for label, needles in INCLUSIVE_RULES:
            if any(n.lower() in blob for n in needles):
                bags[label] += 1
                hit_any = True
        if hit_any:
            bags["_any_listed"] += 1

    lines.append("=== INCLUSIVE STACK HOTSPOTS (any frame; % of main samples) ===")
    for label, _ in INCLUSIVE_RULES:
        v = bags.get(label, 0)
        lines.append(f"  {100.0 * v / leaf_total:5.1f}%  {label}: {v}/{leaf_total}")
    any_v = bags.get("_any_listed", 0)
    lines.append(f"  {100.0 * any_v / leaf_total:5.1f}%  (any listed): {any_v}/{leaf_total}")
    return lines


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("profile", type=Path, help="profile.json or profile.json.gz")
    ap.add_argument(
        "--product",
        default="xylitol",
        help="process name to keep (default: xylitol)",
    )
    ap.add_argument(
        "--all-processes",
        action="store_true",
        help="do not filter agent-spawned toolchain processes",
    )
    ap.add_argument("--top", type=int, default=25, help="top leaf count")
    ap.add_argument(
        "--addr2line",
        type=Path,
        default=None,
        help="ELF with debuginfo (e.g. target/release/xylitol)",
    )
    ap.add_argument("-o", "--output", type=Path, default=None)
    args = ap.parse_args()

    path: Path = args.profile
    opener = gzip.open if path.suffix == ".gz" or path.name.endswith(".json.gz") else open
    with opener(path, "rb") as f:  # type: ignore[arg-type]
        data = json.load(f)

    threads = data.get("threads") or []
    repo = Path(__file__).resolve().parent.parent

    kept_samples = dropped_samples = 0
    kept: list[dict] = []
    for th in threads:
        n = sample_count(th)
        if args.all_processes or keep_thread(th, product=args.product):
            kept.append(th)
            kept_samples += n
        else:
            dropped_samples += n

    lines: list[str] = []
    def out(s: str = "") -> None:
        lines.append(s)

    total = kept_samples + dropped_samples
    out(f"=== FILTER ({path.name}) ===")
    out(f"threads total={len(threads)} kept={len(kept)}")
    if total:
        out(
            f"samples kept={kept_samples} dropped={dropped_samples} "
            f"({100.0 * dropped_samples / total:.1f}% noise)"
        )
    out(f"kept processes: {sorted({proc_name(th) for th in kept})}")
    out()

    ranked = sorted(
        ((sample_count(th), th.get("name"), proc_name(th), th) for th in kept),
        reverse=True,
        key=lambda x: x[0],
    )
    out("=== KEPT THREADS (by samples) ===")
    for n, name, p, _ in ranked[:12]:
        out(f"  {n:6d}  name={name!r:30s} process={p!r}")
    out()

    if not ranked:
        text = "\n".join(lines) + "\n"
        if args.output:
            args.output.write_text(text, encoding="utf-8")
        sys.stdout.write(text)
        return 0

    main_th = next(
        (
            th
            for n, name, p, th in ranked
            if name == args.product and p == args.product
        ),
        ranked[0][3],
    )
    counts = leaf_counts(main_th)
    leaf_total = sum(counts.values())
    out(f"=== MAIN LEAF SELF (thread={main_th.get('name')!r} samples={leaf_total}) ===")

    top = counts.most_common(max(args.top, 60))
    addrs = [(name, c) for name, c in top if name.startswith("0x")]
    named = [(name, c) for name, c in top if not name.startswith("0x")]
    for name, c in named[: args.top]:
        out(f"  {100.0 * c / leaf_total:5.1f}% {c:6d}  {name[:120]}")

    binary = args.addr2line
    if binary is None:
        cand = repo / "target" / "release" / args.product
        if cand.is_file():
            binary = cand

    resolved: list[tuple[int, str, str, str]] = []
    if binary and binary.is_file() and addrs:
        pairs = addr2line_batch(binary, [a for a, _ in addrs])
        for (addr, c), (func, loc) in zip(addrs, pairs):
            resolved.append((c, addr, func, shorten_path(loc, repo)))
        out()
        out(f"=== TOP {args.top} (addr2line via {binary}) ===")
        for c, addr, func, loc in resolved[: args.top]:
            out(f"  {100.0 * c / leaf_total:5.1f}% {c:6d}  {func[:110]}")
            out(f"           {loc}")
        bags: collections.Counter[str] = collections.Counter()
        xy_files: collections.Counter[str] = collections.Counter()
        for c, _addr, func, loc in resolved:
            bags[bucket(func, loc)] += c
            if loc.startswith("$XY/"):
                xy_files[loc.split(":")[0]] += c
        out()
        out("=== BUCKETS (addr2line top set) ===")
        for k, v in bags.most_common():
            out(f"  {100.0 * v / leaf_total:5.1f}%  {k}: {v}")
        if xy_files:
            out()
            out("=== $XY FILES IN ADDR2LINE SET ===")
            for f, c in xy_files.most_common(15):
                out(f"  {100.0 * c / leaf_total:5.1f}% {c:6d}  {f}")
        for line in inclusive_hotspots(main_th, binary, leaf_total=leaf_total):
            if line.startswith("==="):
                out()
            out(line)
    elif addrs:
        out()
        out("(pass --addr2line ./target/release/xylitol to resolve 0x… leaves)")
        for name, c in addrs[: args.top]:
            out(f"  {100.0 * c / leaf_total:5.1f}% {c:6d}  {name}")

    text = "\n".join(lines) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(text, encoding="utf-8")
        print(f"wrote {args.output}", file=sys.stderr)
    sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
