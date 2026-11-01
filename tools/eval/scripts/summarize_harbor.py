#!/usr/bin/env python3
"""Summarize Harbor job directories into pass@1 / flip / cross-scaffold tables."""

from __future__ import annotations

import argparse
import json
import statistics
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass
class Trial:
    label: str
    benchmark: str
    scaffold: str
    task: str
    reward: float | None
    wall_seconds: float | None
    cost_usd: float | None
    input_tokens: int | None
    output_tokens: int | None
    error: str | None


def _seconds(start: str | None, end: str | None) -> float | None:
    if not start or not end:
        return None
    from datetime import datetime

    def parse(value: str) -> datetime:
        return datetime.fromisoformat(value.replace("Z", "+00:00"))

    try:
        return max(0.0, (parse(end) - parse(start)).total_seconds())
    except ValueError:
        return None


def _load_trial(label: str, path: Path) -> Trial | None:
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    if "task_name" not in raw or "trial_name" not in raw:
        return None
    benchmark, _, scaffold = label.partition(":")
    if not scaffold:
        scaffold = "unknown"
    task = str(raw.get("task_name", "")).split("/")[-1] or str(raw.get("trial_name"))
    rewards = (raw.get("verifier_result") or {}).get("rewards") or {}
    reward = rewards.get("reward")
    reward = float(reward) if isinstance(reward, (int, float)) else None
    agent_result = raw.get("agent_result") or {}
    execution = raw.get("agent_execution") or {}
    wall = _seconds(execution.get("started_at"), execution.get("finished_at"))
    exception = raw.get("exception_info")
    error = None
    if isinstance(exception, dict):
        error = str(exception.get("message") or exception.get("type") or "exception")
    elif exception:
        error = str(exception)
    return Trial(
        label=label,
        benchmark=benchmark,
        scaffold=scaffold,
        task=task,
        reward=reward,
        wall_seconds=wall,
        cost_usd=agent_result.get("cost_usd"),
        input_tokens=agent_result.get("n_input_tokens"),
        output_tokens=agent_result.get("n_output_tokens"),
        error=error,
    )


def load_jobs(specs: list[str]) -> list[Trial]:
    trials: list[Trial] = []
    for spec in specs:
        label, sep, raw_path = spec.partition("=")
        if not sep or not label or not raw_path:
            raise SystemExit(f"invalid --job {spec!r}; expected label=path")
        root = Path(raw_path)
        if not root.is_dir():
            raise SystemExit(f"job dir not found: {root}")
        for result in sorted(root.glob("*/result.json")):
            trial = _load_trial(label, result)
            if trial is not None:
                trials.append(trial)
    return trials


def _fmt(value: float | None, digits: int = 3) -> str:
    return "n/a" if value is None else f"{value:.{digits}f}"


def _mean(values: list[float]) -> float | None:
    return statistics.fmean(values) if values else None


def summarize(trials: list[Trial]) -> dict[str, Any]:
    by_label: dict[str, list[Trial]] = defaultdict(list)
    for trial in trials:
        by_label[trial.label].append(trial)

    rows: list[dict[str, Any]] = []
    for label, items in sorted(by_label.items()):
        by_task: dict[str, list[Trial]] = defaultdict(list)
        for item in items:
            by_task[item.task].append(item)
        task_rows = []
        for task, runs in sorted(by_task.items()):
            rewards = [r.reward for r in runs if r.reward is not None]
            passes = [1.0 if r is not None and r >= 1.0 else 0.0 for r in rewards]
            task_rows.append(
                {
                    "task": task,
                    "runs": len(runs),
                    "rewards": rewards,
                    "pass_at_1": _mean(passes),
                    "flip": len(set(passes)) > 1 if passes else False,
                    "errors": sum(1 for r in runs if r.error),
                }
            )
        all_passes = [row["pass_at_1"] for row in task_rows if row["pass_at_1"] is not None]
        rows.append(
            {
                "label": label,
                "benchmark": items[0].benchmark,
                "scaffold": items[0].scaffold,
                "tasks": task_rows,
                "pass_at_1": _mean(all_passes),
                "flip_rate": (
                    sum(1 for row in task_rows if row["flip"]) / len(task_rows)
                    if task_rows
                    else None
                ),
                "errors": sum(row["errors"] for row in task_rows),
                "wall_seconds": _mean([t.wall_seconds for t in items if t.wall_seconds is not None]),
                "cost_usd": sum(t.cost_usd for t in items if t.cost_usd is not None) or None,
            }
        )

    cross: list[dict[str, Any]] = []
    by_benchmark: dict[str, dict[str, dict[str, float]]] = defaultdict(dict)
    for row in rows:
        by_benchmark[row["benchmark"]][row["scaffold"]] = {
            task["task"]: task["pass_at_1"]
            for task in row["tasks"]
            if task["pass_at_1"] is not None
        }
    for benchmark, scaffolds in sorted(by_benchmark.items()):
        names = sorted(scaffolds)
        if len(names) < 2:
            continue
        tasks = sorted(set().union(*(set(scaffolds[name]) for name in names)))
        gaps = []
        disagree = 0
        for task in tasks:
            values = [scaffolds[name].get(task) for name in names]
            if any(value is None for value in values):
                continue
            gaps.append(max(values) - min(values))
            if len(set(values)) > 1:
                disagree += 1
        cross.append(
            {
                "benchmark": benchmark,
                "scaffolds": names,
                "tasks": len(gaps),
                "mean_gap": _mean(gaps),
                "disagree_tasks": disagree,
                "disagree_rate": (disagree / len(gaps)) if gaps else None,
            }
        )
    return {"rows": rows, "cross": cross}


def render_markdown(summary: dict[str, Any]) -> str:
    lines = ["# Harbor eval summary", "", "## Per scaffold", ""]
    lines.append("| benchmark | scaffold | tasks | trials | pass@1 | flip rate | errors | wall (s) | cost |")
    lines.append("|---|---|---:|---:|---:|---:|---:|---:|---:|")
    for row in summary["rows"]:
        trials = sum(task["runs"] for task in row["tasks"])
        lines.append(
            f"| {row['benchmark']} | {row['scaffold']} | {len(row['tasks'])} | {trials} | "
            f"{_fmt(row['pass_at_1'])} | {_fmt(row['flip_rate'])} | {row['errors']} | "
            f"{_fmt(row['wall_seconds'], 1)} | {_fmt(row['cost_usd'], 4)} |"
        )
    lines.extend(["", "## Per task", ""])
    lines.append("| benchmark | scaffold | task | runs | rewards | pass@1 | flip |")
    lines.append("|---|---|---|---:|---|---:|---|")
    for row in summary["rows"]:
        for task in row["tasks"]:
            rewards = ", ".join("n/a" if r is None else f"{r:g}" for r in task["rewards"])
            lines.append(
                f"| {row['benchmark']} | {row['scaffold']} | {task['task']} | {task['runs']} | "
                f"{rewards} | {_fmt(task['pass_at_1'])} | {'yes' if task['flip'] else 'no'} |"
            )
    lines.extend(["", "## Cross-scaffold", ""])
    if not summary["cross"]:
        lines.append("No benchmark has two or more scaffolds.")
    else:
        lines.append("| benchmark | scaffolds | tasks | mean gap | disagree tasks | disagree rate |")
        lines.append("|---|---|---:|---:|---:|---:|")
        for row in summary["cross"]:
            lines.append(
                f"| {row['benchmark']} | {', '.join(row['scaffolds'])} | {row['tasks']} | "
                f"{_fmt(row['mean_gap'])} | {row['disagree_tasks']} | {_fmt(row['disagree_rate'])} |"
            )
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--job", action="append", default=[], help="label=path; repeatable")
    parser.add_argument("--out", default="", help="write markdown here")
    parser.add_argument("--json", default="", help="write raw summary JSON here")
    args = parser.parse_args()
    if not args.job:
        raise SystemExit("at least one --job is required")
    trials = load_jobs(args.job)
    summary = summarize(trials)
    markdown = render_markdown(summary)
    if args.out:
        Path(args.out).write_text(markdown, encoding="utf-8")
    else:
        print(markdown)
    if args.json:
        Path(args.json).write_text(
            json.dumps(summary, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
