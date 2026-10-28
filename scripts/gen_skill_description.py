#!/usr/bin/env python3
"""Generate the per-reference brief tail of a skill's SKILL.md description.

Each `.agents/skills/<skill>/references/*.md` declares a YAML frontmatter
`brief:` — a single, self-explanatory line saying what the reference covers
(and, incidentally, what should make the skill fire). This tool rewrites the
part of the SKILL.md `description` **after** the marker line by joining those
briefs, so the tail stays a derived index of the references (no hand-maintained
drift). The hand-written base above the marker is preserved verbatim.

Contract:
  - Only skills whose description contains the marker are touched; others are
    skipped (opt-in, safe for skills that do not use this harness).
  - Briefs are collected in filename order, normalised (NFKC) and de-duped
    (first occurrence wins), then joined with `；` (each brief may itself use
    `、` / `/` for sub-items).
  - Marker line + everything above it (the base) is kept; everything after is
    regenerated.

Usage:
  python3 scripts/gen_skill_description.py            # rewrite in place
  python3 scripts/gen_skill_description.py --check    # verify in sync (exit 1 on drift)
  python3 scripts/gen_skill_description.py --verbose
"""

from __future__ import annotations

import argparse
import sys
import unicodedata
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SKILLS = REPO / ".agents" / "skills"

MARKER = "以下相关内容也要触发"
MARKER_LINE = f"{MARKER}（脚本生成，勿手改）："
BRIEF_SEP = "；"

MAX_DESCRIPTION_CHARS = 1024


class HarnessError(RuntimeError):
    """A skill opted into the harness but its inputs are malformed."""


def _norm(text: str) -> str:
    return unicodedata.normalize("NFKC", text).strip().casefold()


def _frontmatter_span(lines: list[str]) -> tuple[int, int]:
    """Return [start, end) of the YAML frontmatter block, or raise."""
    if not lines or lines[0].strip() != "---":
        raise HarnessError("missing opening `---` frontmatter")
    for i in range(1, len(lines)):
        if lines[i].strip() == "---":
            return 1, i
    raise HarnessError("missing closing `---` frontmatter")


def _description_block(lines: list[str], fm_start: int, fm_end: int) -> tuple[int, int]:
    """Return [start, end) of the `description:` block (key line + body)."""
    for i in range(fm_start, fm_end):
        if lines[i].startswith("description:"):
            j = i + 1
            while j < fm_end and (not lines[j].strip() or lines[j][:1] in (" ", "\t")):
                j += 1
            return i, j
    raise HarnessError("missing `description:` key in frontmatter")


def _scalar_value(lines: list[str], fm_start: int, fm_end: int, key: str) -> str | None:
    """Read one frontmatter scalar without PyYAML.

    Supports the shapes this harness actually writes: plain single-line,
    quoted single-line, and `>`/`|` block scalars with chomping indicators.
    Anything else is treated as plain text joined by single spaces.
    """
    prefix = f"{key}:"
    for i in range(fm_start, fm_end):
        if not lines[i].startswith(prefix):
            continue
        head = lines[i][len(prefix):].strip()
        if head in (">", ">-", ">+", "|", "|-", "|+"):
            body: list[str] = []
            j = i + 1
            while j < fm_end and (not lines[j].strip() or lines[j][:1] in (" ", "\t")):
                body.append(lines[j])
                j += 1
            indent = next(
                (len(ln) - len(ln.lstrip()) for ln in body if ln.strip()), 0
            )
            stripped = [ln[indent:] if ln.strip() else "" for ln in body]
            if head.startswith("|"):
                text = "\n".join(stripped)
            else:
                paragraphs: list[str] = []
                current: list[str] = []
                for ln in stripped:
                    if ln.strip():
                        current.append(ln.strip())
                    elif current:
                        paragraphs.append(" ".join(current))
                        current = []
                if current:
                    paragraphs.append(" ".join(current))
                text = "\n".join(paragraphs)
            if head.endswith("+"):
                return text + "\n"
            return text.rstrip("\n")
        if head:
            if len(head) >= 2 and head[0] == head[-1] and head[0] in ('"', "'"):
                return head[1:-1]
            return head
        continuation: list[str] = []
        j = i + 1
        while j < fm_end and lines[j][:1] in (" ", "\t"):
            continuation.append(lines[j].strip())
            j += 1
        return " ".join(continuation)
    return None


def read_brief(ref_md: Path) -> str:
    """Parse the single-line `brief:` from a reference file's frontmatter."""
    lines = ref_md.read_text(encoding="utf-8").splitlines()
    try:
        fm_start, fm_end = _frontmatter_span(lines)
    except HarnessError as exc:
        raise HarnessError(f"{ref_md.name}: {exc}") from exc
    brief = _scalar_value(lines, fm_start, fm_end, "brief")
    if not isinstance(brief, str) or not brief.strip():
        raise HarnessError(f"{ref_md.name}: `brief:` must be a non-empty single line")
    if "\n" in brief.strip():
        raise HarnessError(f"{ref_md.name}: `brief:` must be a single line")
    return brief.strip()


def collect_briefs(skill_dir: Path) -> tuple[list[str], int]:
    """Filename-ordered, de-duped brief list + raw (pre-dedupe) count."""
    refs_dir = skill_dir / "references"
    refs = sorted(refs_dir.glob("*.md")) if refs_dir.is_dir() else []
    if not refs:
        raise HarnessError(f"{skill_dir.name}: no references/*.md to derive briefs from")
    seen: dict[str, str] = {}
    raw = 0
    for ref in refs:
        brief = read_brief(ref)
        raw += 1
        seen.setdefault(_norm(brief), brief)
    return list(seen.values()), raw


def render(skill_md: Path) -> tuple[str, dict[str, object]]:
    """Return the fully regenerated SKILL.md text + a small report."""
    text = skill_md.read_text(encoding="utf-8")
    if MARKER not in text:
        return text, {"skill": skill_md.parent.name, "skipped": True}
    lines = text.splitlines()
    fm_start, fm_end = _frontmatter_span(lines)
    d_start, d_end = _description_block(lines, fm_start, fm_end)
    block = lines[d_start:d_end]

    marker_at = next((k for k, ln in enumerate(block) if MARKER in ln), None)
    if marker_at is None:
        return text, {"skill": skill_md.parent.name, "skipped": True}

    indent = next((ln[: len(ln) - len(ln.lstrip())] for ln in block[1:] if ln.strip()), "  ")
    briefs, raw = collect_briefs(skill_md.parent)
    tail = BRIEF_SEP.join(briefs)

    new_block = [*block[:marker_at], f"{indent}{MARKER_LINE}{tail}"]
    new_lines = [*lines[:d_start], *new_block, *lines[d_end:]]
    new_text = "\n".join(new_lines)
    if text.endswith("\n"):
        new_text += "\n"

    desc = _scalar_value(new_lines, fm_start, fm_end, "description") or ""
    return new_text, {
        "skill": skill_md.parent.name,
        "skipped": False,
        "briefs": len(briefs),
        "raw": raw,
        "dropped": raw - len(briefs),
        "description_chars": len(desc),
        "over_limit": len(desc) > MAX_DESCRIPTION_CHARS,
    }


def iter_skill_md() -> list[Path]:
    return sorted(SKILLS.glob("*/SKILL.md"))


def run(*, check: bool, verbose: bool) -> int:
    if not SKILLS.is_dir():
        print(f"error: {SKILLS} missing", file=sys.stderr)
        return 1

    touched = 0
    problems: list[str] = []
    for skill_md in iter_skill_md():
        try:
            new_text, report = render(skill_md)
        except HarnessError as exc:
            problems.append(str(exc))
            continue
        if report.get("skipped"):
            if verbose:
                print(f"skip: {skill_md.parent.name} (no marker)")
            continue
        touched += 1
        current = skill_md.read_text(encoding="utf-8")
        if report["over_limit"]:
            problems.append(
                f"{report['skill']}: description {report['description_chars']} chars "
                f"> {MAX_DESCRIPTION_CHARS} limit"
            )
        if new_text != current:
            if check:
                problems.append(f"{report['skill']}: SKILL.md description is stale")
            else:
                skill_md.write_text(new_text, encoding="utf-8")
                print(
                    f"updated: {report['skill']} — {report['briefs']} briefs "
                    f"(dropped {report['dropped']} dup), description {report['description_chars']} chars"
                )
        elif verbose:
            print(f"ok: {report['skill']} in sync ({report['briefs']} briefs)")

    if not touched:
        print(f"error: no skill under {SKILLS} opted into the harness (marker {MARKER!r})", file=sys.stderr)
        return 1

    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    if verbose:
        print(f"ok: {touched} skill(s) in sync")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify in sync (non-mutating)")
    parser.add_argument("--verbose", action="store_true", help="print ok/skip summaries")
    args = parser.parse_args()
    return run(check=args.check, verbose=args.verbose)


if __name__ == "__main__":
    sys.exit(main())
