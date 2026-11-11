from pathlib import Path

import pytest

from xylitol_eval.config import load_config
from xylitol_eval.errors import ConfigError
from xylitol_eval.lock import load_lock, validate_eval_config

ROOT = Path(__file__).resolve().parents[1]


def test_lock_matches_example():
    lock = load_lock(ROOT / "model-lock.yaml")
    config = load_config(ROOT / "examples" / "eval-terminal-bench.yaml")
    validate_eval_config(config, lock, "compare")
    assert lock.model_id == "group/glm-5.3-flash"
    assert len(lock.sha256) == 64


def test_lock_hash_is_stable():
    first = load_lock(ROOT / "model-lock.yaml")
    second = load_lock(ROOT / "model-lock.yaml")
    assert first.sha256 == second.sha256


def test_lock_rejects_mismatch(tmp_path):
    lock = load_lock(ROOT / "model-lock.yaml")
    config = load_config(ROOT / "examples" / "eval-terminal-bench.yaml")
    object.__setattr__(config, "agent", config.agent.__class__(
        binary=config.agent.binary,
        max_steps_per_turn=1,
        wall_seconds=config.agent.wall_seconds,
        trust=config.agent.trust,
    ))
    with pytest.raises(ConfigError):
        validate_eval_config(config, lock, "compare")


def test_lock_rejects_unknown_profile():
    lock = load_lock(ROOT / "model-lock.yaml")
    config = load_config(ROOT / "examples" / "eval-terminal-bench.yaml")
    with pytest.raises(ConfigError):
        validate_eval_config(config, lock, "missing")
