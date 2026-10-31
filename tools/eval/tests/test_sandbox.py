from xylitol_eval.sandbox import DockerSandbox, ExecResult


def test_exec_passes_env_without_value_in_argv(monkeypatch):
    sandbox = DockerSandbox()
    captured = {}

    def fake_run(args, timeout=None, env=None):
        captured["args"] = args
        captured["env"] = env
        return ExecResult(argv=args, stdout="", stderr="", exit_code=0)

    monkeypatch.setattr(sandbox, "_run", fake_run)
    sandbox.exec("container", ["echo", "ok"], timeout=1, env={"SECRET": "s3cr3t"})

    joined = " ".join(captured["args"])
    assert "s3cr3t" not in joined
    assert "-e" in captured["args"]
    assert "SECRET" in captured["args"]
    assert captured["env"]["SECRET"] == "s3cr3t"
