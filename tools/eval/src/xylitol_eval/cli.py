"""Command line entry point."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from .config import EvalConfig, load_config
from .errors import ConfigError, EXIT_INFRA, EXIT_OK, EXIT_USAGE, InfraError, PhaseNotImplemented
from .paths import default_run_root, resolve_path, run_dir as make_run_dir


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="xylitol-eval")
    sub = parser.add_subparsers(dest="command", required=True)

    dry = sub.add_parser("dry-run", help="print the planned run without Docker or model calls")
    dry.add_argument("--config", required=True)
    dry.add_argument("--json", action="store_true")

    doctor = sub.add_parser("doctor", help="validate config and local prerequisites")
    doctor.add_argument("--config", required=True)

    run = sub.add_parser("run", help="run the eval")
    run.add_argument("--config", required=True)

    grade = sub.add_parser("grade", help="grade existing predictions")
    grade.add_argument("--config", required=True)

    report = sub.add_parser("report", help="render an existing run report")
    report.add_argument("--run-dir", required=True)

    return parser


def _plan(config: EvalConfig) -> dict[str, Any]:
    dataset = config.dataset
    images = []
    if config.sandbox.image != "dataset":
        images = [config.sandbox.image] * max(len(dataset.instance_ids), 1)
    run_id = config.grader.run_id or "<run_id>"
    return {
        "phase": config.phase,
        "config_path": str(config.path),
        "config_sha256": config.sha256,
        "run_id": run_id,
        "dataset": {
            "name": dataset.name,
            "revision": dataset.revision,
            "split": dataset.split,
            "instance_ids": dataset.instance_ids,
            "limit": dataset.limit,
        },
        "model": {"alias": config.model.alias, "thinking_level": config.model.thinking_level},
        "agent": {
            "binary": config.agent.binary,
            "max_turns": config.agent.max_turns,
            "wall_seconds": config.agent.wall_seconds,
            "trust": config.agent.trust,
        },
        "sandbox": {
            "backend": config.sandbox.backend,
            "image": config.sandbox.image,
            "network": config.sandbox.network,
            "cpus": config.sandbox.cpus,
            "memory": config.sandbox.memory,
        },
        "images": images,
        "grader": {
            "backend": config.grader.backend,
            "version": config.grader.version,
            "run_id": run_id,
            "max_workers": config.grader.max_workers,
        },
        "commands": {
            "agent": "xylitol print --no-trust --config <generated> --model {alias} \"<problem_statement>\"".format(
                alias=config.model.alias
            ),
            "grader": "python -m swebench.harness.run_evaluation --dataset_name <parquet> --predictions_path preds.jsonl --run_id {run_id} --max_workers {workers}".format(
                run_id=run_id, workers=config.grader.max_workers
            ),
        },
    }


def _cmd_dry_run(args: argparse.Namespace) -> int:
    config = load_config(Path(args.config))
    plan = _plan(config)
    if args.json:
        print(json.dumps(plan, ensure_ascii=False, indent=2))
    else:
        print(f"phase: {plan['phase']}")
        print(f"config: {plan['config_path']} sha256={plan['config_sha256']}")
        print(f"run_id: {plan['run_id']}")
        print(f"dataset: {plan['dataset']['name']} @ {plan['dataset']['revision']} ({plan['dataset']['split']})")
        print(f"instances: {len(plan['dataset']['instance_ids'])} explicit, limit={plan['dataset']['limit']}")
        for iid in plan["dataset"]["instance_ids"]:
            print(f"  - {iid}")
        print(f"model: {plan['model']['alias']} thinking={plan['model']['thinking_level']}")
        print(f"agent: {plan['agent']['binary']} max_turns={plan['agent']['max_turns']} wall={plan['agent']['wall_seconds']}s")
        print(f"sandbox: {plan['sandbox']['backend']} image={plan['sandbox']['image']} network={plan['sandbox']['network']}")
        print(f"grader: {plan['grader']['backend']} {plan['grader']['version']}")
        print(f"agent command: {plan['commands']['agent']}")
        print(f"grader command: {plan['commands']['grader']}")
    return EXIT_OK


def _cmd_doctor(args: argparse.Namespace) -> int:
    config = load_config(Path(args.config))
    binary = resolve_path(config.agent.binary)
    print(f"config: ok ({config.sha256})")
    if binary.is_file():
        print(f"binary: ok ({binary})")
    else:
        print(f"binary: missing ({binary})")
    print(f"run_dir: {resolve_path(config.report.run_dir)}")
    print(f"default_run_root: {default_run_root()}")
    print(f"run_dir_for_config: {make_run_dir(resolve_path(config.report.run_dir), config.grader.run_id or '<run_id>')}")
    return EXIT_OK


def _cmd_run(args: argparse.Namespace) -> int:
    config = load_config(Path(args.config))
    from .runner import run_eval

    report = run_eval(config)
    print(f"run: {report.run_id}")
    print(f"summary: {report.summary}")
    return EXIT_OK


def _cmd_grade(args: argparse.Namespace) -> int:
    config = load_config(Path(args.config))
    run_id = config.grader.run_id or "<run_id>"
    run_dir = make_run_dir(resolve_path(config.report.run_dir), run_id)
    predictions = run_dir / "preds.jsonl"
    instances = run_dir / "instances.jsonl"
    if not predictions.is_file():
        raise InfraError(f"predictions not found: {predictions}")
    if not instances.is_file():
        raise InfraError(f"instances not found: {instances}")
    from .grade import SwebenchGrader

    result = SwebenchGrader().grade(
        predictions_path=predictions,
        run_id=run_id,
        instances_path=instances,
        model_name=config.model.alias,
        report_dir=run_dir / "reports",
        max_workers=config.grader.max_workers,
    )
    print(f"report: {result.report_path}")
    return EXIT_OK


def _cmd_report(args: argparse.Namespace) -> int:
    path = Path(args.run_dir) / "run.json"
    if not path.is_file():
        raise InfraError(f"run.json not found: {path}")
    data = json.loads(path.read_text(encoding="utf-8"))
    summary = data.get("summary", {})
    print(f"run: {data.get('run_id', '')}")
    for key in ("submitted", "completed", "empty_patches", "resolved", "errors"):
        print(f"{key}: {summary.get(key, 0)}")
    return EXIT_OK


def main(argv: list[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        if args.command == "dry-run":
            return _cmd_dry_run(args)
        if args.command == "doctor":
            return _cmd_doctor(args)
        if args.command == "run":
            return _cmd_run(args)
        if args.command == "grade":
            return _cmd_grade(args)
        if args.command == "report":
            return _cmd_report(args)
        return EXIT_USAGE
    except ConfigError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return EXIT_USAGE
    except InfraError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return EXIT_INFRA
    except PhaseNotImplemented as exc:
        print(f"error: {exc}", file=sys.stderr)
        return EXIT_USAGE


if __name__ == "__main__":
    raise SystemExit(main())
