#!/usr/bin/env python3
"""Spec↔code anchor check (code-as-SSOT reconciliation, c2827 follow-up).

Scans `llmanspec/specs/**/*.feature` for `# verified-by: <ref>` comments and
verifies each ref resolves to real code/test evidence on disk. Rules WITHOUT
anchors are not a failure here — they are reported by `--report` (the
spec↔code alignment matrix) and triaged via the specs-compact flow.

Ref grammar (whitespace-tolerant):
    # verified-by: <path/to/file.rs>            → file must exist
    # verified-by: fn <test_or_impl_name>       → `fn <name>` found in src/ or packages/ or tests/
    # verified-by: script <check_script_name>   → scripts/<name> exists

Modes:
    (default)  silent success; exit 1 only when an existing verified-by ref
               no longer resolves (stale anchor = spec claims evidence that
               the code no longer has).
    --report   print the alignment matrix (per capability: rules / with
               scenario / with anchor / naked) and exit 0.
    --verbose  also list naked rule ids in default mode.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SPECS = REPO / "llmanspec" / "specs"
# Sub-project roots (subproject-llmanspec-discovery).
SUB_ROOT_SPECS = sorted((REPO / "packages").glob("*/llmanspec/specs"))


def collect_evidence_index() -> tuple[set[str], set[str]]:
    """Return (all_fn_names, all_rel_paths) for anchor resolution."""
    fns: set[str] = set()
    rels: set[str] = set()
    roots = [REPO / "src", REPO / "tests", REPO / "scripts"] + sorted(
        (REPO / "packages").glob("*/src")
    )
    for root in roots:
        if not root.exists():
            continue
        for p in root.rglob("*.rs"):
            rels.add(p.relative_to(REPO).as_posix())
            try:
                text = p.read_text(encoding="utf-8", errors="ignore")
            except OSError:
                continue
            for line in text.splitlines():
                t = line.strip()
                for kw in ("fn ", "struct ", "enum ", "const ", "static "):
                    if t.startswith(kw) or f" {kw}" in t:
                        name = (
                            t.split(kw, 1)[1]
                            .split("(", 1)[0]
                            .split("<", 1)[0]
                            .split(":", 1)[0]
                            .split("{", 1)[0]
                            .strip()
                        )
                        if name:
                            fns.add(name)
        for p in root.rglob("*.py"):
            rels.add(p.relative_to(REPO).as_posix())
    return fns, rels


def ref_resolves(ref: str, fns: set[str], rels: set[str]) -> bool:
    ref = ref.strip()
    if ref.startswith("fn "):
        return ref[3:].strip() in fns
    if ref.startswith("script "):
        return (REPO / "scripts" / ref[7:].strip()).exists()
    if (REPO / ref).exists():
        return True
    return ref in fns


def parse_specs() -> dict[str, list[dict]]:
    """capability → list of rule dicts {req, has_scenario, anchors}."""
    out: dict[str, list[dict]] = {}
    all_features = sorted(SPECS.rglob("*.feature"))
    for d in SUB_ROOT_SPECS:
        all_features += sorted(d.rglob("*.feature"))
    for feature in all_features:
        cap = feature.stem
        lines = feature.read_text(encoding="utf-8").splitlines()
        rule_starts = [i for i, l in enumerate(lines) if l.lstrip().startswith("规则:")]
        for si, start in enumerate(rule_starts):
            end = rule_starts[si + 1] if si + 1 < len(rule_starts) else len(lines)
            block = lines[start:end]
            req = None
            for back in range(start, max(start - 3, -1), -1):
                if "@req:" in lines[back]:
                    req = lines[back].split("@req:", 1)[1].strip()
                    break
            out.setdefault(cap, []).append(
                {
                    "req": req or "?",
                    "has_scenario": any(
                        b.lstrip().startswith("场景:") for b in block
                    ),
                    "anchors": [
                        b.split("verified-by:", 1)[1].strip()
                        for b in block
                        if "verified-by:" in b
                    ],
                }
            )
    return out


def main() -> int:
    args = sys.argv[1:]
    report = "--report" in args
    verbose = "--verbose" in args or report
    fns, rels = collect_evidence_index()
    specs = parse_specs()

    broken: list[str] = []
    total = with_scenario = with_anchor = 0
    naked_report: list[tuple[str, int]] = []
    for cap, rules in specs.items():
        cap_naked = 0
        for r in rules:
            total += 1
            covered = r["has_scenario"] or bool(r["anchors"])
            if r["has_scenario"]:
                with_scenario += 1
            if r["anchors"]:
                with_anchor += 1
            if not covered:
                cap_naked += 1
            for ref in r["anchors"]:
                if not ref_resolves(ref, fns, rels):
                    broken.append(f"{cap}/{r['req']}: unresolved verified-by: {ref}")
        if cap_naked:
            naked_report.append((cap, cap_naked))

    naked_total = sum(n for _, n in naked_report)
    if report:
        print(f"capabilities: {len(specs)}  rules: {total}")
        print(
            f"with-scenario: {with_scenario}  with-anchor: {with_anchor}  "
            f"naked(no scenario & no anchor): {naked_total}"
        )
        print("\nnaked rules per capability (desc):")
        for cap, n in sorted(naked_report, key=lambda x: -x[1]):
            print(f"  {n:>4}  {cap}")
        return 0

    if broken:
        print("error: stale spec anchors (code-as-SSOT violation):", file=sys.stderr)
        for b in broken:
            print(f"  - {b}", file=sys.stderr)
        return 1
    if verbose:
        for cap, n in sorted(naked_report, key=lambda x: -x[1]):
            print(f"note: {cap}: {n} naked rule(s) (no scenario / no anchor)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
