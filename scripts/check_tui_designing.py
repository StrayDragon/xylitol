#!/usr/bin/env python3
"""QA check: designing modules + generated AGENT-INDEX + state contracts.

Usage:
  python3 scripts/check_tui_designing.py --check
  python3 scripts/check_tui_designing.py --check --verbose
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover
    yaml = None  # type: ignore

REPO = Path(__file__).resolve().parent.parent
DESIGNING = REPO / "designing"
APP = DESIGNING / "app"
INDEX = DESIGNING / "generated" / "AGENT-INDEX.md"
GEN = REPO / "scripts" / "gen_designing_index.py"
AGENTS = DESIGNING / "AGENTS.md"
HEX_RE = re.compile(r"#[0-9a-fA-F]{6}")

REQUIRED_STATES = {
    ("tui", "activity-fold"): ("collapsed", "envelope", "expanded"),
    ("tui", "diff"): ("unified", "side-by-side", "empty"),
    ("tui", "expandable"): ("tool-collapsed", "tool-expanded", "thinking"),
    ("tui", "models"): ("wide", "narrow", "no-thinking", "filter"),
    ("tui", "session-tree"): ("filter",),
    ("tui", "session-resume"): ("default", "id-on"),
    ("tui", "markdown"): ("sample",),
    ("tui", "status"): ("idle", "busy"),
    ("tui", "compaction"): ("collapsed", "expanded"),
}

FORBIDDEN_MODULE_IDS = {"keybindings"}


def fail(msg: str, errors: list[str]) -> None:
    errors.append(msg)


def iter_modules() -> list[tuple[str, Path]]:
    rows: list[tuple[str, Path]] = []
    if not DESIGNING.is_dir():
        return rows
    for surface in sorted(p for p in DESIGNING.iterdir() if p.is_dir()):
        mods = surface / "modules"
        if not mods.is_dir():
            continue
        for path in sorted(p for p in mods.iterdir() if p.is_dir()):
            rows.append((surface.name, path))
    return rows


def load_yaml(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    if yaml is None:
        raise RuntimeError("PyYAML required to parse designing YAML")
    data = yaml.safe_load(text)
    if not isinstance(data, dict):
        raise ValueError(f"{path}: expected mapping")
    return data


def flatten_lines(data: dict) -> str:
    rows = data.get("lines") or []
    out: list[str] = []
    for row in rows:
        if isinstance(row, list):
            parts: list[str] = []
            for span in row:
                if isinstance(span, dict):
                    parts.append(str(span.get("text", "")))
                else:
                    parts.append(str(span))
            out.append("".join(parts))
        elif isinstance(row, str):
            out.append(row)
    return "\n".join(out)


def check_app(errors: list[str]) -> None:
    for name in ("package.json", "index.html", "src/main.ts", "src/shell.css"):
        if not (APP / name).is_file():
            fail(f"designing app missing {name}", errors)
    html = (APP / "index.html").read_text(encoding="utf-8") if (APP / "index.html").is_file() else ""
    css = (APP / "src" / "shell.css").read_text(encoding="utf-8") if (APP / "src" / "shell.css").is_file() else ""
    main = (APP / "src" / "main.ts").read_text(encoding="utf-8") if (APP / "src" / "main.ts").is_file() else ""
    if 'id="copy-handoff"' not in html:
        fail("handoff: index.html must ship copy-handoff", errors)
    if "parseLocation" not in main:
        fail("endpoint: app must parse pathname /tui/<id>/<state>", errors)
    if 'class="keys"' in html:
        fail("no-key-wall: app header must not ship a .keys shortcut wall", errors)
    blob = html + css
    if "(包)" in blob:
        fail("no-impl-noise: '(包)' must not appear in designing app", errors)
    if "color: inherit" not in css:
        fail("rev-no-kind-fg: shell.css must inherit color on selected/rev rows", errors)


def check_module(surface: str, mod: Path, errors: list[str]) -> None:
    if mod.name in FORBIDDEN_MODULE_IDS:
        fail(f"no independent keybindings design: unexpected module {mod.name}", errors)
    if not (mod / "intent.md").is_file():
        fail(f"designing: {surface}/{mod.name} missing intent.md", errors)
    else:
        n = len((mod / "intent.md").read_text(encoding="utf-8").splitlines())
        if n > 80:
            fail(f"designing: {surface}/{mod.name}/intent.md is {n} lines (soft top 80)", errors)
    if not (mod / "draft.yaml").is_file():
        fail(f"designing: {surface}/{mod.name} missing draft.yaml", errors)
    else:
        try:
            draft = load_yaml(mod / "draft.yaml")
        except Exception as e:  # noqa: BLE001
            fail(f"designing: parse {surface}/{mod.name}/draft.yaml: {e}", errors)
            draft = {}
        if "keybindings" in str(draft.get("id", "")).lower() and mod.name != "queue-steer":
            pass
        keys = draft.get("keys") or []
        if keys and not isinstance(keys, list):
            fail(f"designing: {surface}/{mod.name} keys must be a list", errors)
    states_dir = mod / "states"
    states = list(states_dir.glob("*.yaml")) if states_dir.is_dir() else []
    if not states:
        fail(f"designing: {surface}/{mod.name} has no states/*.yaml", errors)
    stems = {p.stem for p in states}
    required = REQUIRED_STATES.get((surface, mod.name))
    if required:
        for stem in required:
            if stem not in stems:
                fail(f"designing: {surface}/{mod.name} missing states/{stem}.yaml", errors)
    for path in [mod / "intent.md", mod / "draft.yaml", *states]:
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        if HEX_RE.search(text):
            fail(f"no-raw-hex: {path.relative_to(REPO)}", errors)
    for path in states:
        try:
            doc = load_yaml(path)
        except Exception as e:  # noqa: BLE001
            fail(f"designing: parse {path.relative_to(REPO)}: {e}", errors)
            continue
        src = flatten_lines(doc)
        if "(包)" in src:
            fail(f"no-impl-noise: '(包)' visible in {path.relative_to(REPO)}", errors)
        for s in doc.get("must_contain") or []:
            if s not in src:
                fail(f"states: {path.relative_to(REPO)} must_contain missing {s!r}", errors)
        for s in doc.get("must_not_contain") or []:
            if s in src:
                fail(f"states: {path.relative_to(REPO)} must_not_contain found {s!r}", errors)
        if any(span.get("rev") for row in (doc.get("lines") or []) if isinstance(row, list) for span in row if isinstance(span, dict)):
            # selected spans exist; kind-fg leak is a CSS concern, already gated in app
            pass


def check_shell(errors: list[str]) -> None:
    """Shell 视图：帧数据与区域注解（module 存在、锚点可解析）。"""
    regions_path = DESIGNING / "tui" / "shell.regions.yaml"
    frame_path = DESIGNING / "generated" / "shell-frame.json"
    if not regions_path.is_file():
        fail("missing designing/tui/shell.regions.yaml", errors)
        return
    try:
        regions_doc = load_yaml(regions_path)
    except Exception as e:  # noqa: BLE001
        fail(f"parse shell.regions.yaml: {e}", errors)
        return
    module_ids = {path.name for _, path in iter_modules()}
    for frame_id, regions in (regions_doc.get("frames") or {}).items():
        if not isinstance(regions, list) or not regions:
            fail(f"shell.regions.yaml: frames/{frame_id} 为空", errors)
            continue
        for region in regions:
            module = region.get("module") if isinstance(region, dict) else None
            if module not in module_ids:
                fail(f"shell.regions.yaml: 未知 module {module!r}（frames/{frame_id}）", errors)
            if not isinstance(region, dict) or not region.get("contains") or not region.get("rows"):
                fail(f"shell.regions.yaml: frames/{frame_id} 区域缺 contains/rows", errors)
    if not frame_path.is_file():
        fail("missing designing/generated/shell-frame.json — run: just export-design-frame", errors)
        return
    try:
        frame_doc = json.loads(frame_path.read_text(encoding="utf-8"))
    except Exception as e:  # noqa: BLE001
        fail(f"parse shell-frame.json: {e}", errors)
        return
    frames = frame_doc.get("frames") or []
    if not frames:
        fail("shell-frame.json: 无帧", errors)
    missing = set((regions_doc.get("frames") or {})) - {f.get("id") for f in frames}
    if missing:
        fail(f"shell-frame.json 缺帧: {sorted(missing)} — run: just export-design-frame", errors)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="non-mutating gate")
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()

    errors: list[str] = []
    if not AGENTS.is_file():
        fail(f"missing {AGENTS.relative_to(REPO)}", errors)
    else:
        agents = AGENTS.read_text(encoding="utf-8")
        if "/tui/" not in agents:
            fail("designing/AGENTS.md must document pathname /tui/", errors)
        if "复制路径" not in agents and "handoff" not in agents.lower():
            fail("designing/AGENTS.md must mention copy paths / handoff", errors)
    mods = iter_modules()
    if not mods:
        fail("missing designing/*/modules", errors)
    for surface, mod in mods:
        check_module(surface, mod, errors)
    check_app(errors)
    check_shell(errors)

    want = subprocess.check_output(
        [sys.executable, str(GEN), "--stdout"],
        cwd=REPO,
        text=True,
    )
    if not INDEX.is_file():
        fail("missing generated/AGENT-INDEX.md — run: just gen-designing-index", errors)
    elif INDEX.read_text(encoding="utf-8") != want:
        fail("stale generated/AGENT-INDEX.md — run: just gen-designing-index", errors)

    if errors:
        print("check_tui_designing: FAIL", file=sys.stderr)
        for e in errors:
            print(f"  - {e}", file=sys.stderr)
        return 1
    if args.verbose:
        print(f"ok: designing ({len(mods)} modules, index fresh)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
