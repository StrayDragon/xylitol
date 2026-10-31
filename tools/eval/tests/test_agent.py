from pathlib import Path

from xylitol_eval.agent import render_xylitol_config
from xylitol_eval.config import load_config

EXAMPLES = Path(__file__).resolve().parents[1] / "examples"


def test_render_config_uses_env_placeholders():
    cfg = load_config(EXAMPLES / "eval-swebench-mini.yaml")
    text = render_xylitol_config(cfg)
    assert 'default_model: "zhipu/glm-5.3"' in text
    assert "{{ env.MAGPIE_GATEWAY_URL }}/v1" in text
    assert "{{ env.MAGPIE_GATEWAY_KEY }}" in text
    assert "thinking_levels: [off, low, high]" in text
    assert "max_turns: 80" in text
    assert "MAGPIE_GATEWAY_KEY" in text
    assert "sk-" not in text


def test_render_fake_provider(tmp_path):
    path = tmp_path / "fake.yaml"
    path.write_text(
        """version: 1
phase: m0
dataset:
  name: fixture.jsonl
  revision: test
  instance_ids: ["a"]
model:
  alias: "fake"
  provider: fake
agent:
  binary: target/eval-bin/xylitol
""",
        encoding="utf-8",
    )
    text = render_xylitol_config(load_config(path))
    assert "provider: fake" in text
    assert "MAGPIE_GATEWAY_KEY" not in text
