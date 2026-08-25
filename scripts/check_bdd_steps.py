#!/usr/bin/env python3
"""QA check: static cross-check of BDD feature steps vs rstest-bdd registrations.

Encodes the dead-step audit method (see cb4dff5e) as a permanent gate:

  1. Orphan pattern — a #[given]/#[when]/#[then] literal under tests/bdd that
     matches no step line in any feature file (dead registration).
  2. Unresolvable bound scenario — a scenario targeted by #[scenario(path,name)]
     whose step lines do not all resolve to a registered pattern of the right
     kind; cargo test would fail at runtime with "Step not found".

Scenarios without a #[scenario] binding are info-only: they are `@human`
constraint rules (single-track feature-as-spec) audited by llman itself.

Usage:
  python3 scripts/check_bdd_steps.py --check
  python3 scripts/check_bdd_steps.py --check --verbose
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
BDD_DIR = REPO / "tests" / "bdd"
FEATURE_DIRS = [REPO / "llmanspec" / "specs", REPO / "tests" / "features"]

# zh-CN keyword set must include 假定/假设 variants or orphan scans misfire.
KEYWORD_KINDS = {
    "假如": "given",
    "假设": "given",
    "假定": "given",
    "Given": "given",
    "当": "when",
    "When": "when",
    "那么": "then",
    "Then": "then",
}
AND_KEYWORDS = {"并且", "And"}

STEP_ATTR_RE = re.compile(
    r'#\[(?P<kind>given|when|then)\(\s*"(?P<pat>(?:[^"\\]|\\.)*)"\s*\)\s*\]', re.S
)
SCENARIO_ATTR_RE = re.compile(r"#\[scenario\s*\((?P<args>[^)]*)\)\s*\]", re.S)
SCENARIO_TITLE_RE = re.compile(r"^\s*(?:场景|Scenario):\s*(.+?)\s*$")


def collect_patterns() -> dict[tuple[str, str], list[str]]:
    """kind+pattern -> defining file locations."""
    registry: dict[tuple[str, str], list[str]] = {}
    for rs in sorted(BDD_DIR.rglob("*.rs")):
        text = rs.read_text(encoding="utf-8")
        for m in STEP_ATTR_RE.finditer(text):
            key = (m.group("kind"), m.group("pat"))
            registry.setdefault(key, []).append(
                str(rs.relative_to(REPO)) + f":{text[: m.start()].count(chr(10)) + 1}"
            )
    return registry


def unescape_rust(s: str) -> str:
    """Undo the Rust string escapes STEP_ATTR_RE captured (\" → ", \\\\ → \\)."""
    return re.sub(r"\\(.)", r"\1", s)


def pattern_to_regex(pat: str) -> re.Pattern[str]:
    """Translate an rstest-bdd pattern to a fullmatch regex ({...} -> .+)."""
    parts = re.split(r"\{[^{}]*\}", unescape_rust(pat))
    return re.compile("^" + ".+".join(re.escape(p) for p in parts) + "$", re.S)


def parse_feature(
    path: Path,
) -> tuple[list[tuple[str, str]], dict[str, list[tuple[str, str]]]]:
    """(background_steps, {scenario: [(resolved_kind, text)]}). And inherits.

    Background (背景:) steps run before every scenario in the file, so they
    count toward pattern coverage and bound-scenario resolution.
    """
    background: list[tuple[str, str]] = []
    scenarios: dict[str, list[tuple[str, str]]] = {}
    current: str | None = None
    in_background = False
    last_kind: str | None = None
    in_docstring = False
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if '"""' in line:
            in_docstring = not in_docstring
            continue
        if in_docstring or not line or line.startswith("#") or line.startswith("@"):
            continue
        if line.startswith("|"):
            continue
        title = SCENARIO_TITLE_RE.match(line)
        if title:
            current = title.group(1)
            scenarios.setdefault(current, [])
            last_kind = None
            in_background = False
            continue
        if re.match(r"^\s*(?:功能|Feature):", raw):
            current = None
            last_kind = None
            in_background = False
            continue
        if re.match(r"^\s*(?:背景|Background):", raw):
            current = None
            last_kind = None
            in_background = True
            continue
        if re.match(r"^\s*(?:规则|Rule):", raw):
            current = None
            last_kind = None
            in_background = False
            continue
        first = re.match(r"^(\S+)\s*(.*)$", line)
        if not first:
            continue
        word, rest = first.group(1), first.group(2)
        target = background if in_background else (
            scenarios[current] if current else None
        )
        if target is None:
            continue
        if word in AND_KEYWORDS and last_kind:
            target.append((last_kind, rest))
        elif word in KEYWORD_KINDS:
            kind = KEYWORD_KINDS[word]
            target.append((kind, rest))
            last_kind = kind
    return background, scenarios


def collect_features() -> dict[Path, tuple[list[tuple[str, str]], dict[str, list[tuple[str, str]]]]]:
    features: dict[
        Path, tuple[list[tuple[str, str]], dict[str, list[tuple[str, str]]]]
    ] = {}
    for d in FEATURE_DIRS:
        for feat in sorted(d.rglob("*.feature")):
            features[feat] = parse_feature(feat)
    return features


def collect_bound_scenarios() -> list[tuple[str, str, str]]:
    """(feature_rel_path, scenario_name, binding_file:line)."""
    bound = []
    for rs in sorted(BDD_DIR.rglob("*.rs")):
        text = rs.read_text(encoding="utf-8")
        for m in SCENARIO_ATTR_RE.finditer(text):
            args = m.group("args")
            pm = re.search(r'path\s*=\s*"([^"]+)"', args)
            nm = re.search(r'name\s*=\s*"([^"]+)"', args)
            if pm and nm:
                where = (
                    str(rs.relative_to(REPO))
                    + f":{text[: m.start()].count(chr(10)) + 1}"
                )
                bound.append((pm.group(1), nm.group(1), where))
    return bound


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true", help="accepted by convention")
    ap.add_argument("--verbose", action="store_true")
    args = ap.parse_args()

    registry = collect_patterns()
    compiled = {k: pattern_to_regex(k[1]) for k in registry}
    features = collect_features()
    bound = collect_bound_scenarios()

    errors: list[str] = []

    # 1. every registered pattern is exercised somewhere (bound or unbound).
    hit: set[tuple[str, str]] = set()
    for feat, (background, scenarios) in features.items():
        all_steps = list(background)
        for steps in scenarios.values():
            all_steps.extend(steps)
        for kind, text in all_steps:
            for key, rx in compiled.items():
                if key[0] == kind and rx.fullmatch(text):
                    hit.add(key)
    for key, locs in sorted(registry.items()):
        if key not in hit:
            errors.append(
                f"orphan step pattern [{key[0]}] `{key[1]}` matches no feature"
                f" step (defined at {locs[0]})"
            )

    # 2. every bound scenario's steps resolve against registered patterns.
    unbound_steps = 0
    for rel, name, where in bound:
        feat = REPO / rel
        parsed = features.get(feat)
        if parsed is None:
            errors.append(f"binding at {where}: feature file missing: {rel}")
            continue
        background, scenarios = parsed
        if name not in scenarios:
            errors.append(f"binding at {where}: scenario `{name}` not in {rel}")
            continue
        for kind, text in background + scenarios[name]:
            if not any(key[0] == kind and compiled[key].fullmatch(text) for key in registry):
                errors.append(
                    f"{rel} scenario `{name}` step [{kind}] `{text}` has no"
                    f" registered pattern (bound at {where})"
                )
    # info only: unbound scenarios keep executable-looking steps until their
    # migration ticket lands.
    bound_keys = {(rel, name) for rel, name, _ in bound}
    for feat, (background, scenarios) in features.items():
        rel = str(feat.relative_to(REPO))
        for name, steps in scenarios.items():
            if (rel, name) not in bound_keys:
                unbound_steps += len(steps)

    if errors:
        for e in errors:
            print(f"error: {e}", file=sys.stderr)
        print(f"{len(errors)} problem(s)", file=sys.stderr)
        return 1

    if args.verbose:
        print(
            f"ok: bdd steps consistent — {len(registry)} patterns,"
            f" {len(bound)} bound scenarios resolved,"
            f" {unbound_steps} unbound-scenario steps (info)"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
