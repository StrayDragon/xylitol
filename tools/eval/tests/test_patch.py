import json

from xylitol_eval.patch import read_predictions, write_prediction


def test_prediction_roundtrip(tmp_path):
    path = tmp_path / "preds.jsonl"
    write_prediction(path, "a", "group/glm-5.3-flash", "diff --git a/x b/x\n")
    write_prediction(path, "b", "group/glm-5.3-flash", "")
    rows = read_predictions(path)
    assert rows[0]["instance_id"] == "a"
    assert rows[0]["model_name_or_path"] == "group/glm-5.3-flash"
    assert rows[1]["model_patch"] == ""
    assert json.loads(path.read_text().splitlines()[0])["instance_id"] == "a"
