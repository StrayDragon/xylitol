#!/usr/bin/env python3
"""QA check: DESIGN playground static mock vs known-drift rules + L2 fixtures.

SSOT format: llmanspec/changes/c625-*/design.md (L1/L2)
Fixtures: src/app/tui/design/fixtures/*.yaml
HTML: src/app/tui/design/playground/index.html

Usage:
  python3 scripts/check_tui_design_playground.py
  python3 scripts/check_tui_design_playground.py --check
  python3 scripts/check_tui_design_playground.py --check --verbose
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover
    yaml = None  # type: ignore

REPO = Path(__file__).resolve().parent.parent
HTML = REPO / "src" / "app" / "tui" / "design" / "playground" / "index.html"
DESIGN_DIR = REPO / "src" / "app" / "tui" / "design"
FIXTURES = DESIGN_DIR / "fixtures"
AGENTS = DESIGN_DIR / "AGENTS.md"
README = DESIGN_DIR / "playground" / "README.md"

CHANGE_ID_RE = re.compile(r"\bc\d{3}(?:-\w+)?\b")
HEX_RE = re.compile(r"#[0-9a-fA-F]{6}")


def fail(msg: str, errors: list[str]) -> None:
    errors.append(msg)


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def extract_js_object_entry(html: str, const_name: str, key: str) -> str | None:
    """Extract backtick template for CONST[key] or CONST = { key: { editor: `...` } }."""
    # TREEP = { filter: `...`, ... }
    m = re.search(
        rf"const {re.escape(const_name)}\s*=\s*\{{(.*?)\n\s*\}};",
        html,
        re.S,
    )
    if not m:
        return None
    body = m.group(1)
    # key: `...`  (possibly multiline)
    km = re.search(
        rf"(?:^|,)\s*{re.escape(key)}\s*:\s*`([^`]*)`",
        body,
        re.S | re.M,
    )
    if km:
        return km.group(1)
    # key: { editor: `...`, footer: "..." }
    km2 = re.search(
        rf"(?:^|,)\s*{re.escape(key)}\s*:\s*\{{(.*?)\n\s*\}},",
        body,
        re.S | re.M,
    )
    if not km2:
        # last entry without trailing comma
        km2 = re.search(
            rf"(?:^|,)\s*{re.escape(key)}\s*:\s*\{{(.*?)\n\s*\}}",
            body,
            re.S | re.M,
        )
    if not km2:
        return None
    nested = km2.group(1)
    em = re.search(r"editor\s*:\s*`([^`]*)`", nested, re.S)
    return em.group(1) if em else None


def panel_inner(html: str, panel_id: str) -> str | None:
    m = re.search(
        rf'<section[^>]*id="{re.escape(panel_id)}"[^>]*>(.*?)</section>',
        html,
        re.S,
    )
    return m.group(1) if m else None


def check_global(html: str, errors: list[str]) -> None:
    if re.search(r'class="keys"', html) or re.search(r"<span class=\"keys\">", html):
        fail("no-impl-noise: header .keys tip wall must not exist", errors)
    if "(包)" in html:
        fail("no-impl-noise: '(包)' label must not appear in playground HTML", errors)
    if "secondary: true" in html or "classList.add(\"secondary\")" in html:
        fail("no-impl-noise: secondary nav styling must not be used", errors)
    if "demo 已有" in html:
        fail("no-impl-noise: 'demo 已有' implementation noise in HTML", errors)
    if ".rev *" not in html and ".rev," not in html:
        fail("rev-no-kind-fg: CSS must include .rev * { color: inherit } (or equivalent)", errors)
    elif "color: inherit" not in html:
        fail("rev-no-kind-fg: .rev descendants must use color: inherit", errors)
    for pid in ("panel-models", "panel-tree-power", "panel-pending"):
        if f'id="{pid}"' not in html:
            fail(f"required-slots: missing {pid}", errors)
    models = panel_inner(html, "panel-models") or ""
    treep = panel_inner(html, "panel-tree-power") or ""
    pending = panel_inner(html, "panel-pending") or ""
    if "term-slot" not in models:
        fail("term-slot: panel-models must use term-slot", errors)
    if "term-slot" not in treep:
        fail("term-slot: panel-tree-power must use term-slot", errors)
    if "term-slot" not in pending:
        fail("term-slot: panel-pending must use term-slot", errors)
    # Next-wave panels: no change-id noise in visible markup (not comments-only — scan panel text)
    for name, chunk in (
        ("panel-models", models),
        ("panel-tree-power", treep),
        ("panel-pending", pending),
    ):
        # strip HTML comments
        visible = re.sub(r"<!--.*?-->", "", chunk, flags=re.S)
        if CHANGE_ID_RE.search(visible):
            fail(
                f"no-impl-noise: change-id in Next-wave panel {name} "
                f"(keep history titles elsewhere; not here): {CHANGE_ID_RE.findall(visible)}",
                errors,
            )
    for const, key in (
        ("MODELS", "wide"),
        ("MODELS", "narrow"),
        ("MODELS", "noThinking"),
        ("MODELS", "filter"),
        ("PENDING_RUNTIME", "nextTurn"),
        ("PENDING_RUNTIME", "thinking"),
        ("TREEP", "filter"),
    ):
        src = extract_js_object_entry(html, const, key)
        if src is None:
            fail(f"source-missing: const {const}.{key} template not found", errors)
            continue
        if CHANGE_ID_RE.search(src):
            fail(
                f"no-impl-noise: change-id inside {const}.{key}: {CHANGE_ID_RE.findall(src)}",
                errors,
            )


def check_tokens_from(errors: list[str]) -> None:
    for path in DESIGN_DIR.glob("*.md"):
        if path.name in {"AGENTS.md"}:
            continue
        text = read(path)
        if not text.startswith("---"):
            continue
        fm = text.split("---", 2)[1] if text.count("---") >= 2 else ""
        if "tokens_from:" not in fm and path.name != "DESIGN.md":
            # DESIGN.md is root; children need tokens_from
            if path.parent == DESIGN_DIR and path.name.endswith(".md"):
                fail(f"tokens-from: {path.relative_to(REPO)} missing tokens_from in frontmatter", errors)


def check_docs(errors: list[str]) -> None:
    agents = read(AGENTS)
    readme = read(README)
    if "check_tui_design_playground" not in agents and "playground lint" not in agents.lower():
        if "lint" not in agents.lower():
            fail("docs: design/AGENTS.md must mention playground lint", errors)
    if "check_tui_design_playground" not in readme and "lint" not in readme.lower():
        fail("docs: playground README must mention lint", errors)


def load_fixture(path: Path) -> dict:
    text = read(path)
    if yaml is not None:
        data = yaml.safe_load(text)
        if not isinstance(data, dict):
            raise ValueError(f"{path}: expected mapping")
        return data
    # Minimal YAML subset without PyYAML
    data: dict = {}
    current_list: list | None = None
    current_key: str | None = None
    nest: dict | None = None
    nest_key: str | None = None
    for raw in text.splitlines():
        if not raw.strip() or raw.strip().startswith("#"):
            continue
        if nest is not None and raw.startswith("  ") and not raw.startswith("    "):
            # still in assert_html?
            pass
        m = re.match(r"^([a-z_]+):\s*(.*)$", raw)
        m2 = re.match(r"^  ([a-z_]+):\s*(.*)$", raw)
        if raw.startswith("assert_html:"):
            nest = {}
            nest_key = "assert_html"
            data["assert_html"] = nest
            current_list = None
            continue
        if nest is not None and m2:
            k, v = m2.group(1), m2.group(2).strip()
            if v == "":
                current_list = []
                nest[k] = current_list
                current_key = k
            elif v.startswith("[") and v.endswith("]"):
                inner = v[1:-1].strip()
                nest[k] = [x.strip().strip("'\"") for x in inner.split(",") if x.strip()] if inner else []
                current_list = None
            elif v in ("true", "false"):
                nest[k] = v == "true"
                current_list = None
            else:
                nest[k] = v.strip("'\"")
                current_list = None
            continue
        if raw.startswith("  - ") and current_list is not None:
            current_list.append(raw[4:].strip().strip("'\""))
            continue
        if m and not raw.startswith(" "):
            nest = None
            k, v = m.group(1), m.group(2).strip()
            if v == "":
                current_list = []
                data[k] = current_list
                current_key = k
            else:
                data[k] = v.strip("'\"")
                current_list = None
                current_key = k
    return data


def resolve_source(html: str, source: str) -> str | None:
    # treep.filter / models.open.editor / pending.nextTurn.editor
    parts = source.split(".")
    if parts[0] == "treep" and len(parts) == 2:
        return extract_js_object_entry(html, "TREEP", parts[1])
    if parts[0] == "resume" and len(parts) == 2:
        return extract_js_object_entry(html, "RESUME", parts[1])
    if parts[0] == "compaction" and len(parts) == 2:
        # COMPACTION.progress.html / .collapsed.html …
        return extract_js_object_entry(html, "COMPACTION", parts[1])
    if parts[0] == "models" and len(parts) >= 2:
        return extract_js_object_entry(html, "MODELS", parts[1])
    if parts[0] == "pending" and len(parts) >= 2:
        return extract_js_object_entry(html, "PENDING_RUNTIME", parts[1])
    if parts[0] == "mcpCue" and len(parts) >= 2:
        return extract_js_object_entry(html, "MCP_CUE", parts[1])
    return None


def check_fixtures(html: str, errors: list[str]) -> None:
    if not FIXTURES.is_dir():
        fail("fixtures: design/fixtures/ missing", errors)
        return
    paths = sorted(FIXTURES.glob("*.yaml"))
    if not paths:
        fail("fixtures: no *.yaml in design/fixtures/", errors)
        return
    ids = {p.stem for p in paths}
    for need in ("session-tree.filter", "models.wide"):
        if need not in ids:
            fail(f"fixtures: missing {need}.yaml", errors)
    for path in paths:
        try:
            fix = load_fixture(path)
        except Exception as e:  # noqa: BLE001
            fail(f"fixtures: parse {path.name}: {e}", errors)
            continue
        fid = str(fix.get("id", path.stem))
        if f'data-design-fixture="{fid}"' not in html:
            fail(f"fixtures: HTML missing data-design-fixture=\"{fid}\"", errors)
        source = str(fix.get("source", ""))
        src = resolve_source(html, source) if source else None
        if source and src is None:
            fail(f"fixtures: {fid} source {source!r} not found in HTML", errors)
            continue
        if src is None:
            continue
        for s in fix.get("must_contain") or []:
            if s not in src:
                fail(f"fixtures: {fid} must_contain missing {s!r}", errors)
        for s in fix.get("must_not_contain") or []:
            if s in src:
                fail(f"fixtures: {fid} must_not_contain found {s!r}", errors)
        ah = fix.get("assert_html") or {}
        if not isinstance(ah, dict):
            continue
        # Skip selected-row asserts when fixture does not care (e.g. pending strip).
        if "selected_class" not in ah and "selected_forbids_substrings" not in ah:
            if ah.get("status_after_selected"):
                pass
            else:
                continue
        sel_cls = ah.get("selected_class", "rev")
        # selected chunks: lines/divs containing class rev
        selected_bits = re.findall(
            rf'class="[^"]*\b{re.escape(sel_cls)}\b[^"]*"[^>]*>(.*?)</',
            src,
            re.S,
        )
        if not selected_bits:
            # template may use class="row rev">...</div>
            selected_bits = re.findall(
                rf'class="[^"]*\b{re.escape(sel_cls)}\b[^"]*">(.*?)</div>',
                src,
                re.S,
            )
        if not selected_bits:
            fail(f"fixtures: {fid} no selected .{sel_cls} segment in source", errors)
        else:
            for bit in selected_bits:
                for bad in ah.get("selected_forbids_substrings") or []:
                    if bad in bit:
                        fail(
                            f"fixtures: {fid} selected row must not contain {bad!r} (got {bit!r})",
                            errors,
                        )
        if ah.get("status_after_selected"):
            pat = ah.get("status_pattern") or r"\([0-9]+/[0-9]+\)"
            sel_m = re.search(rf'class="[^"]*\b{re.escape(sel_cls)}\b', src)
            st_m = re.search(pat, src)
            if not st_m:
                fail(f"fixtures: {fid} status_pattern {pat!r} not found", errors)
            elif sel_m and st_m.start() < sel_m.start():
                fail(f"fixtures: {fid} status must appear after selected row", errors)


def check_raw_hex(html: str, errors: list[str]) -> None:
    # Ignore comments
    stripped = re.sub(r"<!--.*?-->", "", html, flags=re.S)
    # Allow in tokens.css references? only index.html
    for m in HEX_RE.finditer(stripped):
        # skip if inside url( or content already var(
        start = max(0, m.start() - 40)
        ctx = stripped[start : m.end() + 10]
        if "var(--" in ctx and m.group(0) in ctx:
            # fallback inside var(--x,#hex) — still ban for SSOT
            fail(f"no-raw-hex: {m.group(0)} in index.html near {ctx.strip()!r}", errors)
            break


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="non-mutating gate (default)")
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="print ok summary on success (default: silent success; errors always print)",
    )
    args = parser.parse_args()

    errors: list[str] = []
    if not HTML.is_file():
        print(f"error: missing {HTML}", file=sys.stderr)
        return 1

    html = read(HTML)
    check_global(html, errors)
    check_raw_hex(html, errors)
    check_tokens_from(errors)
    check_fixtures(html, errors)
    check_docs(errors)

    if errors:
        print("check_tui_design_playground: FAIL", file=sys.stderr)
        for e in errors:
            print(f"  - {e}", file=sys.stderr)
        return 1
    if args.verbose:
        print("ok: playground L1/L2 checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
