"""Model/config lock loading and validation."""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import yaml

from .config import EvalConfig
from .errors import ConfigError


@dataclass(frozen=True)
class LockProfile:
    max_steps_per_turn: int
    wall_seconds: int


@dataclass(frozen=True)
class ModelLock:
    model_id: str
    provider: str
    api: str
    base_url_env: str
    api_key_env: str
    context_window: int
    max_output_tokens: int
    thinking_levels: list[str]
    thinking_level: str
    profiles: dict[str, LockProfile]
    benchmarks: dict[str, dict[str, Any]]
    sha256: str

    def to_dict(self) -> dict[str, Any]:
        return {
            "model": {
                "id": self.model_id,
                "provider": self.provider,
                "api": self.api,
                "base_url_env": self.base_url_env,
                "api_key_env": self.api_key_env,
                "context_window": self.context_window,
                "max_output_tokens": self.max_output_tokens,
                "thinking_levels": list(self.thinking_levels),
                "thinking_level": self.thinking_level,
            },
            "profiles": {
                name: {"max_steps_per_turn": p.max_steps_per_turn, "wall_seconds": p.wall_seconds}
                for name, p in self.profiles.items()
            },
            "benchmarks": self.benchmarks,
        }


def load_lock(path: Path) -> ModelLock:
    if not path.is_file():
        raise ConfigError(f"model lock not found: {path}")
    raw = yaml.safe_load(path.read_text(encoding="utf-8"))
    if not isinstance(raw, dict):
        raise ConfigError("model lock root must be a mapping")
    if raw.get("schema_version") != 1:
        raise ConfigError("model lock schema_version must be 1")

    model = raw.get("model")
    if not isinstance(model, dict):
        raise ConfigError("model lock model must be a mapping")
    required = (
        "id",
        "provider",
        "api",
        "context_window",
        "max_output_tokens",
        "thinking_levels",
        "thinking_level",
    )
    for key in required:
        if model.get(key) in (None, "", []):
            raise ConfigError(f"model lock model.{key} is required")
    levels = model["thinking_levels"]
    if not isinstance(levels, list) or not all(isinstance(x, str) and x for x in levels):
        raise ConfigError("model lock model.thinking_levels must be a string list")
    if model["thinking_level"] not in levels:
        raise ConfigError("model lock model.thinking_level must be in thinking_levels")

    profiles_raw = raw.get("profiles")
    if not isinstance(profiles_raw, dict) or not profiles_raw:
        raise ConfigError("model lock profiles must be a non-empty mapping")
    profiles: dict[str, LockProfile] = {}
    for name, item in profiles_raw.items():
        if not isinstance(item, dict):
            raise ConfigError(f"model lock profiles.{name} must be a mapping")
        try:
            profiles[name] = LockProfile(
                max_steps_per_turn=int(item["max_steps_per_turn"]),
                wall_seconds=int(item["wall_seconds"]),
            )
        except (KeyError, TypeError, ValueError) as exc:
            raise ConfigError(f"model lock profiles.{name} is invalid") from exc

    benchmarks = raw.get("benchmarks", {})
    if not isinstance(benchmarks, dict):
        raise ConfigError("model lock benchmarks must be a mapping")

    normalized = {
        "model": {
            "id": model["id"],
            "provider": model["provider"],
            "api": model["api"],
            "base_url_env": model.get("base_url_env", "MAGPIE_GATEWAY_URL"),
            "api_key_env": model.get("api_key_env", "MAGPIE_GATEWAY_KEY"),
            "context_window": int(model["context_window"]),
            "max_output_tokens": int(model["max_output_tokens"]),
            "thinking_levels": levels,
            "thinking_level": model["thinking_level"],
        },
        "profiles": {
            name: {"max_steps_per_turn": p.max_steps_per_turn, "wall_seconds": p.wall_seconds}
            for name, p in profiles.items()
        },
        "benchmarks": benchmarks,
    }
    digest = hashlib.sha256(
        json.dumps(normalized, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    return ModelLock(
        model_id=str(model["id"]),
        provider=str(model["provider"]),
        api=str(model["api"]),
        base_url_env=str(model.get("base_url_env", "MAGPIE_GATEWAY_URL")),
        api_key_env=str(model.get("api_key_env", "MAGPIE_GATEWAY_KEY")),
        context_window=int(model["context_window"]),
        max_output_tokens=int(model["max_output_tokens"]),
        thinking_levels=list(levels),
        thinking_level=str(model["thinking_level"]),
        profiles=profiles,
        benchmarks=benchmarks,
        sha256=digest,
    )


def validate_eval_config(config: EvalConfig, lock: ModelLock, profile: str = "compare") -> None:
    if profile not in lock.profiles:
        raise ConfigError(f"unknown lock profile: {profile}")
    checks = [
        ("model.alias", config.model.alias, lock.model_id),
        ("model.provider", config.model.provider, lock.provider),
        ("model.api", config.model.api, lock.api),
        ("model.context_window", config.model.context_window, lock.context_window),
        ("model.max_output_tokens", config.model.max_output_tokens, lock.max_output_tokens),
        ("model.thinking_levels", config.model.thinking_levels, lock.thinking_levels),
        ("model.thinking_level", config.model.thinking_level, lock.thinking_level),
        ("model.base_url_env", config.model.base_url_env, lock.base_url_env),
        ("model.api_key_env", config.model.api_key_env, lock.api_key_env),
        ("agent.max_steps_per_turn", config.agent.max_steps_per_turn, lock.profiles[profile].max_steps_per_turn),
        ("agent.wall_seconds", config.agent.wall_seconds, lock.profiles[profile].wall_seconds),
    ]
    for field, actual, expected in checks:
        if actual != expected:
            raise ConfigError(
                f"{field} mismatch: config={actual!r}, lock={expected!r} ({profile})"
            )
