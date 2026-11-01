from pathlib import Path

import pytest

from xylitol_eval.config import load_config
from xylitol_eval.errors import ConfigError

EXAMPLES = Path(__file__).resolve().parents[1] / "examples"


def test_load_mini_config():
    cfg = load_config(EXAMPLES / "eval-swebench-mini.yaml")
    assert cfg.phase == "m0"
    assert cfg.dataset.name == "SWE-bench/SWE-bench_Verified"
    assert len(cfg.dataset.instance_ids) == 5
    assert cfg.model.alias == "group/glm-5.3-flash"
    assert cfg.model.thinking_levels == ["low", "high"]
    assert len(cfg.sha256) == 64


def test_config_hash_is_stable():
    first = load_config(EXAMPLES / "eval-swebench-mini.yaml")
    second = load_config(EXAMPLES / "eval-swebench-mini.yaml")
    assert first.sha256 == second.sha256


def test_invalid_phase_rejected(tmp_path):
    path = tmp_path / "bad.yaml"
    path.write_text("version: 1\nphase: m9\n", encoding="utf-8")
    with pytest.raises(ConfigError):
        load_config(path)


def test_off_thinking_level_normalized(tmp_path):
    path = tmp_path / "off.yaml"
    path.write_text(
        """version: 1
phase: m0
dataset:
  name: x.jsonl
  revision: r
  instance_ids: ["a"]
model:
  alias: "group/glm-5.3-flash"
  thinking_level: off
agent:
  binary: target/eval-bin/xylitol
""",
        encoding="utf-8",
    )
    cfg = load_config(path)
    assert cfg.model.thinking_level == "off"


def test_thinking_level_must_be_declared(tmp_path):
    path = tmp_path / "level.yaml"
    path.write_text(
        """version: 1
phase: m0
dataset:
  name: x.jsonl
  revision: r
  instance_ids: ["a"]
model:
  alias: "group/glm-5.3-flash"
  thinking_level: max
  thinking_levels: ["low", "high"]
agent:
  binary: target/eval-bin/xylitol
""",
        encoding="utf-8",
    )
    with pytest.raises(ConfigError):
        load_config(path)


def test_missing_dataset_rejected(tmp_path):
    path = tmp_path / "bad.yaml"
    path.write_text(
        "version: 1\nphase: m0\nmodel:\n  alias: x\nagent:\n  binary: x\n",
        encoding="utf-8",
    )
    with pytest.raises(ConfigError):
        load_config(path)
