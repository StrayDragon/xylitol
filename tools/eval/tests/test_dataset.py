from pathlib import Path

import pytest

from xylitol_eval.config import load_config
from xylitol_eval.dataset import load_dataset
from xylitol_eval.errors import InfraError

FIXTURES = Path(__file__).resolve().parent / "fixtures"


def _config(tmp_path, instance_ids):
    path = tmp_path / "config.yaml"
    path.write_text(
        f"""version: 1
phase: m0
dataset:
  name: {FIXTURES / 'instances.jsonl'}
  revision: test
  split: test
  instance_ids: [{', '.join(instance_ids)}]
  limit: 2
model:
  alias: "group/glm-5.3-flash"
agent:
  binary: target/eval-bin/xylitol
""",
        encoding="utf-8",
    )
    return load_config(path)


def test_load_selected_instances(tmp_path):
    cfg = _config(tmp_path, ['"astropy__astropy-12907"', '"django__django-13128"'])
    rows = load_dataset(cfg)
    assert [row["instance_id"] for row in rows] == [
        "astropy__astropy-12907",
        "django__django-13128",
    ]
    assert rows[0]["image"].startswith("swebench/sweb.eval.x86_64.astropy")


def test_missing_instance_rejected(tmp_path):
    cfg = _config(tmp_path, ['"missing"'])
    with pytest.raises(InfraError):
        load_dataset(cfg)
