import json
from pathlib import Path

from xylitol_eval.cli import main

EXAMPLES = Path(__file__).resolve().parents[1] / "examples"


def test_dry_run_json(capsys):
    code = main(["dry-run", "--config", str(EXAMPLES / "eval-swebench-mini.yaml"), "--json"])
    assert code == 0
    out = json.loads(capsys.readouterr().out)
    assert out["phase"] == "m0"
    assert out["dataset"]["name"] == "SWE-bench/SWE-bench_Verified"
    assert out["model"]["alias"] == "group/glm-5.3-flash"
    assert out["commands"]["agent"].startswith("xylitol print")


def test_run_requires_binary(tmp_path, capsys):
    config = tmp_path / "config.yaml"
    config.write_text(
        """version: 1
phase: m0
dataset:
  name: x.jsonl
  revision: r
  instance_ids: ["a"]
model:
  alias: "group/glm-5.3-flash"
agent:
  binary: target/eval-bin/missing
""",
        encoding="utf-8",
    )
    code = main(["run", "--config", str(config)])
    assert code == 2
    assert "agent binary not found" in capsys.readouterr().err
