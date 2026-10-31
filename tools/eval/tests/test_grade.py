import json
from pathlib import Path

from xylitol_eval.grade import SwebenchGrader


def test_report_file_name_and_parse(tmp_path, monkeypatch):
    report_dir = tmp_path / "reports"
    report_dir.mkdir()
    run_id = "r1"
    model = "zhipu/glm-5.3"
    report = {
        "schema_version": 2,
        "submitted_instances": 1,
        "completed_instances": 1,
        "resolved_ids": ["a"],
        "unresolved_ids": [],
        "empty_patch_instances": 0,
        "error_ids": [],
    }
    path = report_dir / f"{model.replace('/', '__')}.{run_id}.json"
    path.write_text(json.dumps(report), encoding="utf-8")
    assert path.is_file()
    assert json.loads(path.read_text())["resolved_ids"] == ["a"]
