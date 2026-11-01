import json

from xylitol_eval.report import InstanceResult, RunReport, write_run_json, write_summary


def _report():
    return RunReport(
        run_id="r1",
        phase="m0",
        started_at="2026-11-01T00:00:00Z",
        finished_at="2026-11-01T00:01:00Z",
        xylitol={"git_commit": "abc", "binary_sha256": "def"},
        harness={"name": "swebench", "version": "5.0.2"},
        dataset={"name": "SWE-bench/SWE-bench_Verified", "revision": "rev"},
        model={"alias": "group/glm-5.3-flash"},
        config={"sha256": "cfg"},
        limits={"max_turns": 80, "wall_seconds": 1800},
        instances=[
            InstanceResult(instance_id="a", graded=True, resolved=True),
            InstanceResult(instance_id="b", empty_patch=True),
        ],
    )


def test_run_json_roundtrip(tmp_path):
    report = _report()
    path = write_run_json(tmp_path, report)
    data = json.loads(path.read_text(encoding="utf-8"))
    assert data["schema_version"] == 1
    assert data["summary"]["submitted"] == 2
    assert data["summary"]["completed"] == 1
    assert data["summary"]["empty_patches"] == 1
    assert data["summary"]["resolved"] == 1


def test_summary_markdown(tmp_path):
    report = _report()
    path = write_summary(tmp_path, report)
    text = path.read_text(encoding="utf-8")
    assert "# Eval run r1" in text
    assert "| a |" in text
    assert "submitted: 2" in text
