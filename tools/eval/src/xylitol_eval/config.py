"""Eval configuration loading, validation and hashing."""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import yaml

from .errors import ConfigError

PHASES = ("m0", "m1", "m2", "m3", "m4", "m5")
SANDBOX_BACKENDS = ("docker",)
GRADER_BACKENDS = ("swebench", "harbor")


@dataclass(frozen=True)
class DatasetConfig:
    name: str
    revision: str
    split: str
    instance_ids: list[str] = field(default_factory=list)
    limit: int = 0


@dataclass(frozen=True)
class ModelConfig:
    alias: str
    provider: str = "openai"
    thinking_level: str = "high"
    api: str = "openai-responses"
    context_window: int = 1000000
    max_output_tokens: int = 131072
    base_url_env: str = "MAGPIE_GATEWAY_URL"
    api_key_env: str = "MAGPIE_GATEWAY_KEY"


@dataclass(frozen=True)
class AgentConfig:
    binary: str
    max_turns: int = 80
    wall_seconds: int = 1800
    trust: bool = False


@dataclass(frozen=True)
class SandboxConfig:
    backend: str = "docker"
    image: str = "dataset"
    network: str = "model_only"
    cpus: int = 2
    memory: str = "4g"


@dataclass(frozen=True)
class GraderConfig:
    backend: str = "swebench"
    version: str = "5.0.2"
    run_id: str = ""
    max_workers: int = 2


@dataclass(frozen=True)
class ReportConfig:
    formats: list[str] = field(default_factory=lambda: ["json", "markdown"])
    run_dir: str = "target/eval-runs"


@dataclass(frozen=True)
class EvalConfig:
    version: int
    phase: str
    dataset: DatasetConfig
    model: ModelConfig
    agent: AgentConfig
    sandbox: SandboxConfig
    grader: GraderConfig
    report: ReportConfig
    path: Path
    sha256: str

    def to_dict(self) -> dict[str, Any]:
        return {
            "version": self.version,
            "phase": self.phase,
            "dataset": {
                "name": self.dataset.name,
                "revision": self.dataset.revision,
                "split": self.dataset.split,
                "instance_ids": list(self.dataset.instance_ids),
                "limit": self.dataset.limit,
            },
            "model": {
                "alias": self.model.alias,
                "provider": self.model.provider,
                "thinking_level": self.model.thinking_level,
                "api": self.model.api,
                "context_window": self.model.context_window,
                "max_output_tokens": self.model.max_output_tokens,
                "base_url_env": self.model.base_url_env,
                "api_key_env": self.model.api_key_env,
            },
            "agent": {
                "binary": self.agent.binary,
                "max_turns": self.agent.max_turns,
                "wall_seconds": self.agent.wall_seconds,
                "trust": self.agent.trust,
            },
            "sandbox": {
                "backend": self.sandbox.backend,
                "image": self.sandbox.image,
                "network": self.sandbox.network,
                "cpus": self.sandbox.cpus,
                "memory": self.sandbox.memory,
            },
            "grader": {
                "backend": self.grader.backend,
                "version": self.grader.version,
                "run_id": self.grader.run_id,
                "max_workers": self.grader.max_workers,
            },
            "report": {
                "formats": list(self.report.formats),
                "run_dir": self.report.run_dir,
            },
        }


def _require(mapping: dict[str, Any], key: str, where: str) -> Any:
    if key not in mapping or mapping[key] in (None, ""):
        raise ConfigError(f"{where}.{key} is required")
    return mapping[key]


def _positive_int(value: Any, where: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
        raise ConfigError(f"{where} must be a positive integer")
    return value


def _string_list(value: Any, where: str) -> list[str]:
    if not isinstance(value, list) or any(not isinstance(item, str) or not item for item in value):
        raise ConfigError(f"{where} must be a list of non-empty strings")
    return list(value)


def load_config(path: Path) -> EvalConfig:
    if not path.is_file():
        raise ConfigError(f"config not found: {path}")
    try:
        raw = yaml.safe_load(path.read_text(encoding="utf-8"))
    except yaml.YAMLError as exc:
        raise ConfigError(f"invalid YAML: {exc}") from exc
    if not isinstance(raw, dict):
        raise ConfigError("config root must be a mapping")

    version = raw.get("version")
    if version != 1:
        raise ConfigError("version must be 1")
    phase = str(_require(raw, "phase", "config"))
    if phase not in PHASES:
        raise ConfigError(f"phase must be one of {', '.join(PHASES)}")

    ds_raw = raw.get("dataset")
    if not isinstance(ds_raw, dict):
        raise ConfigError("dataset must be a mapping")
    dataset = DatasetConfig(
        name=str(_require(ds_raw, "name", "dataset")),
        revision=str(_require(ds_raw, "revision", "dataset")),
        split=str(ds_raw.get("split", "test")),
        instance_ids=_string_list(ds_raw.get("instance_ids", []), "dataset.instance_ids"),
        limit=_positive_int(ds_raw.get("limit", 0), "dataset.limit") if ds_raw.get("limit", 0) else 0,
    )
    if not dataset.instance_ids and dataset.limit <= 0:
        raise ConfigError("dataset.instance_ids or dataset.limit is required")

    model_raw = raw.get("model")
    if not isinstance(model_raw, dict):
        raise ConfigError("model must be a mapping")
    raw_level = model_raw.get("thinking_level", "high")
    if isinstance(raw_level, bool):
        thinking_level = "off" if not raw_level else "high"
    else:
        thinking_level = str(raw_level)
    model = ModelConfig(
        alias=str(_require(model_raw, "alias", "model")),
        provider=str(model_raw.get("provider", "openai")),
        thinking_level=thinking_level,
        api=str(model_raw.get("api", "openai-responses")),
        context_window=_positive_int(model_raw.get("context_window", 1000000), "model.context_window"),
        max_output_tokens=_positive_int(
            model_raw.get("max_output_tokens", 131072), "model.max_output_tokens"
        ),
        base_url_env=str(model_raw.get("base_url_env", "MAGPIE_GATEWAY_URL")),
        api_key_env=str(model_raw.get("api_key_env", "MAGPIE_GATEWAY_KEY")),
    )

    agent_raw = raw.get("agent")
    if not isinstance(agent_raw, dict):
        raise ConfigError("agent must be a mapping")
    agent = AgentConfig(
        binary=str(_require(agent_raw, "binary", "agent")),
        max_turns=_positive_int(agent_raw.get("max_turns", 80), "agent.max_turns"),
        wall_seconds=_positive_int(agent_raw.get("wall_seconds", 1800), "agent.wall_seconds"),
        trust=bool(agent_raw.get("trust", False)),
    )

    sandbox_raw = raw.get("sandbox", {})
    if not isinstance(sandbox_raw, dict):
        raise ConfigError("sandbox must be a mapping")
    sandbox = SandboxConfig(
        backend=str(sandbox_raw.get("backend", "docker")),
        image=str(sandbox_raw.get("image", "dataset")),
        network=str(sandbox_raw.get("network", "model_only")),
        cpus=_positive_int(sandbox_raw.get("cpus", 2), "sandbox.cpus"),
        memory=str(sandbox_raw.get("memory", "4g")),
    )
    if sandbox.backend not in SANDBOX_BACKENDS:
        raise ConfigError(f"sandbox.backend must be one of {', '.join(SANDBOX_BACKENDS)}")

    grader_raw = raw.get("grader", {})
    if not isinstance(grader_raw, dict):
        raise ConfigError("grader must be a mapping")
    grader = GraderConfig(
        backend=str(grader_raw.get("backend", "swebench")),
        version=str(grader_raw.get("version", "5.0.2")),
        run_id=str(grader_raw.get("run_id", "")),
        max_workers=_positive_int(grader_raw.get("max_workers", 2), "grader.max_workers"),
    )
    if grader.backend not in GRADER_BACKENDS:
        raise ConfigError(f"grader.backend must be one of {', '.join(GRADER_BACKENDS)}")

    report_raw = raw.get("report", {})
    if not isinstance(report_raw, dict):
        raise ConfigError("report must be a mapping")
    report = ReportConfig(
        formats=_string_list(report_raw.get("formats", ["json", "markdown"]), "report.formats"),
        run_dir=str(report_raw.get("run_dir", "target/eval-runs")),
    )

    normalized = {
        "version": version,
        "phase": phase,
        "dataset": {
            "name": dataset.name,
            "revision": dataset.revision,
            "split": dataset.split,
            "instance_ids": dataset.instance_ids,
            "limit": dataset.limit,
        },
        "model": {
            "alias": model.alias,
            "provider": model.provider,
            "thinking_level": model.thinking_level,
            "api": model.api,
            "context_window": model.context_window,
            "max_output_tokens": model.max_output_tokens,
            "base_url_env": model.base_url_env,
            "api_key_env": model.api_key_env,
        },
        "agent": {
            "binary": agent.binary,
            "max_turns": agent.max_turns,
            "wall_seconds": agent.wall_seconds,
            "trust": agent.trust,
        },
        "sandbox": {
            "backend": sandbox.backend,
            "image": sandbox.image,
            "network": sandbox.network,
            "cpus": sandbox.cpus,
            "memory": sandbox.memory,
        },
        "grader": {
            "backend": grader.backend,
            "version": grader.version,
            "run_id": grader.run_id,
            "max_workers": grader.max_workers,
        },
        "report": {"formats": report.formats, "run_dir": report.run_dir},
    }
    digest = hashlib.sha256(
        json.dumps(normalized, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    return EvalConfig(
        version=version,
        phase=phase,
        dataset=dataset,
        model=model,
        agent=agent,
        sandbox=sandbox,
        grader=grader,
        report=report,
        path=path,
        sha256=digest,
    )
