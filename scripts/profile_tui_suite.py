#!/usr/bin/env python3
"""Automated xylitol TUI CPU profile suite (c1520 MVP).

Preconditions: samply, tmux, perf_event_paranoid<=1, release xylitol with debuginfo.

Scenarios (Fake model; no real LLM / MCP tools):
  A-idle        — attach + idle
  B-scroll      — long seeded session + PageUp/Down
  C-stream      — empty session + XYLITOL_FAKE_SLOW_STREAM + submit
  D-resume      — many sessions + /session-resume scroll + Ctrl+U
  E-hist-stream — seeded long history + stream (c1505 T0d; bumps upper_gen)

Examples:
  python3 scripts/profile_tui_suite.py --build
  python3 scripts/profile_tui_suite.py --scenarios A,D
  python3 scripts/profile_tui_suite.py --scenarios E --b-pairs 400 --duration 20
  python3 scripts/profile_tui_suite.py --all
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import signal
import subprocess
import sys
import time
import uuid
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_BIN = REPO / "target" / "release" / "xylitol"
PROFILE_ROOT = REPO / "target" / "profile"


def die(msg: str, code: int = 1) -> None:
    print(f"error: {msg}", file=sys.stderr)
    raise SystemExit(code)


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess[str]:
    print("+", " ".join(cmd), flush=True)
    return subprocess.run(cmd, text=True, **kw)


def require_tools() -> None:
    for tool in ("tmux", "samply"):
        if shutil.which(tool) is None:
            die(f"{tool} not on PATH")
    paranoid = Path("/proc/sys/kernel/perf_event_paranoid")
    if paranoid.is_file():
        val = int(paranoid.read_text().strip())
        if val > 1:
            die(
                f"perf_event_paranoid={val} (need <=1). "
                "Run: echo 1 | sudo tee /proc/sys/kernel/perf_event_paranoid"
            )


def profile_build(bin_path: Path) -> None:
    env = os.environ.copy()
    env["CARGO_PROFILE_RELEASE_STRIP"] = "none"
    env["CARGO_PROFILE_RELEASE_DEBUG"] = "line-tables-only"
    r = run(
        ["cargo", "build", "--release", "-p", "xylitol"],
        cwd=REPO,
        env=env,
        check=False,
    )
    if r.returncode != 0:
        die("cargo build --release failed")
    if not bin_path.is_file():
        die(f"missing binary {bin_path}")


def write_fake_config(project: Path) -> None:
    d = project / ".xylitol"
    d.mkdir(parents=True, exist_ok=True)
    (d / "config.yaml").write_text(
        "models:\n"
        "  default_model: fake\n"
        "  models:\n"
        "    fake:\n"
        "      provider: fake\n"
        "      model: fake-model\n",
        encoding="utf-8",
    )


def ts() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.000Z")


def seed_long_session(
    sessions_dir: Path,
    cwd: str,
    n_pairs: int = 80,
    *,
    sid: str = "profile-scroll-b",
) -> str:
    """Linear user/assistant jsonl for scrollback pressure (CJK + long lines)."""
    path = sessions_dir / f"{sid}.jsonl"
    lines: list[str] = []
    lines.append(
        json.dumps(
            {
                "type": "session",
                "version": 5,
                "id": sid,
                "timestamp": ts(),
                "cwd": cwd,
            },
            ensure_ascii=False,
        )
    )
    parent = None
    for i in range(n_pairs):
        for role, text in (
            (
                "user",
                f"请看第{i}轮：{'测宽与排版' * 20} path=/tmp/x/{i}.rs",
            ),
            (
                "assistant",
                f"回复{i}：" + ("这是一段用于 scrollback 压力的中文与 `code` 混合正文。" * 8),
            ),
        ):
            mid = f"m-{i}-{role}"
            entry = {
                "type": "message",
                "id": mid,
                "parentId": parent,
                "timestamp": ts(),
                "message": {
                    "role": role,
                    "content": [{"type": "text", "text": text}],
                    "timestamp": 0,
                },
            }
            lines.append(json.dumps(entry, ensure_ascii=False))
            parent = mid
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return sid


def seed_resume_sessions(sessions_dir: Path, cwd: str, n: int = 40) -> None:
    for i in range(n):
        sid = str(uuid.uuid4())
        path = sessions_dir / f"{sid}.jsonl"
        preview = f"profile resume row {i} " + ("预览标题" * 6)
        lines = [
            json.dumps(
                {
                    "type": "session",
                    "version": 5,
                    "id": sid,
                    "timestamp": ts(),
                    "cwd": cwd,
                },
                ensure_ascii=False,
            ),
            json.dumps(
                {
                    "type": "message",
                    "id": f"{sid}-u",
                    "parentId": None,
                    "timestamp": ts(),
                    "message": {
                        "role": "user",
                        "content": [{"type": "text", "text": preview}],
                        "timestamp": 0,
                    },
                },
                ensure_ascii=False,
            ),
            json.dumps(
                {
                    "type": "message",
                    "id": f"{sid}-a",
                    "parentId": f"{sid}-u",
                    "timestamp": ts(),
                    "message": {
                        "role": "assistant",
                        "content": [{"type": "text", "text": f"ok-{i}"}],
                        "timestamp": 0,
                    },
                },
                ensure_ascii=False,
            ),
        ]
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def tmux(args: list[str]) -> subprocess.CompletedProcess[str]:
    return run(["tmux", *args], check=False, capture_output=True)


def tmux_ok(args: list[str]) -> None:
    r = tmux(args)
    if r.returncode != 0:
        die(f"tmux {' '.join(args)} failed: {r.stderr}")


def wait_pane(name: str, needle: str, timeout: float = 45.0) -> None:
    deadline = time.time() + timeout
    last = ""
    while time.time() < deadline:
        r = tmux(["capture-pane", "-t", name, "-p", "-J"])
        last = r.stdout or ""
        if needle in last:
            return
        time.sleep(0.2)
    die(f"timeout waiting for {needle!r} in tmux {name}; last:\n{last[-2000:]}")


def find_xylitol_pid(bin_path: Path, home: Path) -> int:
    # Prefer exact binary path match.
    r = run(["pgrep", "-af", str(bin_path)], capture_output=True, check=False)
    lines = [ln for ln in (r.stdout or "").splitlines() if str(bin_path) in ln]
    # Prefer processes with our HOME in environ via cwd heuristic: list /proc
    candidates: list[int] = []
    for ln in lines:
        try:
            pid = int(ln.split(None, 1)[0])
        except ValueError:
            continue
        environ = Path(f"/proc/{pid}/environ")
        if environ.is_file():
            try:
                env = environ.read_bytes()
            except OSError:
                continue
            if f"HOME={home}".encode() in env or str(home).encode() in env:
                candidates.append(pid)
        else:
            candidates.append(pid)
    if not candidates and lines:
        candidates = [int(lines[0].split(None, 1)[0])]
    if not candidates:
        die("could not find xylitol PID")
    return candidates[0]


def send_keys(name: str, *keys: str) -> None:
    tmux_ok(["send-keys", "-t", name, *keys])


def send_literal(name: str, text: str) -> None:
    tmux_ok(["send-keys", "-t", name, "-l", text])


def run_scenario(
    scenario: str,
    *,
    bin_path: Path,
    out_dir: Path,
    duration: int,
    cols: int,
    rows: int,
    b_pairs: int = 80,
) -> Path:
    sandbox = out_dir / f"sandbox-{scenario}"
    if sandbox.exists():
        shutil.rmtree(sandbox)
    project = sandbox / "project"
    home = sandbox / "home"
    config = sandbox / "config"
    sessions = home / ".xylitol" / "sessions"
    for p in (project, home, config, sessions):
        p.mkdir(parents=True, exist_ok=True)
    write_fake_config(project)
    cwd = str(project.resolve())

    session_arg: list[str] = []
    extra_env: dict[str, str] = {}
    if scenario == "B-scroll":
        sid = seed_long_session(sessions, cwd, n_pairs=b_pairs, sid="profile-scroll-b")
        session_arg = ["--session", sid]
    elif scenario == "C-stream":
        # ~40 chunks * 15ms + long grapheme payload
        extra_env["XYLITOL_FAKE_SLOW_STREAM"] = "50,12,64"
    elif scenario == "E-hist-stream":
        # Long history already loaded; TextDelta bumps upper_gen → warm flatten path.
        # Stream MUST fill most of the samply window — short C-style streams (~0.6s)
        # leave idle that dilutes scroll_render% and hides n-scaling.
        sid = seed_long_session(
            sessions, cwd, n_pairs=b_pairs, sid="profile-hist-stream-e"
        )
        session_arg = ["--session", sid]
        chunks = max(80, duration * 8)
        delay_ms = max(20, (duration * 1000) // chunks)
        extra_env["XYLITOL_FAKE_SLOW_STREAM"] = f"{chunks},{delay_ms},48"
        print(
            f"E-hist-stream stream_spec={extra_env['XYLITOL_FAKE_SLOW_STREAM']} "
            f"(~{chunks * delay_ms}ms)",
            flush=True,
        )
    elif scenario == "D-resume":
        seed_resume_sessions(sessions, cwd, n=45)

    name = f"xyl_prof_{scenario.replace('-', '_')}_{os.getpid()}"
    # Kill leftover
    tmux(["kill-session", "-t", name])

    env_exports = " ".join(
        [
            f"HOME={home}",
            f"XYLITOL_CONFIG_DIR={config}",
            f"XYLITOL_PROJECT_DIR={project}",
            "TERM=xterm-256color",
            *[f"{k}={v}" for k, v in extra_env.items()],
        ]
    )
    cmd = (
        f"cd {project} && {env_exports} {bin_path} --trust --model fake "
        f"{' '.join(session_arg)} tui; echo EXIT:$?; sleep 2"
    )
    r = tmux(
        [
            "new-session",
            "-d",
            "-s",
            name,
            "-x",
            str(cols),
            "-y",
            str(rows),
            cmd,
        ]
    )
    if r.returncode != 0:
        die(f"tmux new-session failed: {r.stderr}")

    try:
        # Product chrome usually shows model id or path-ish footer.
        # E with large seed may need longer first paint before "fake" appears.
        boot_timeout = 120.0 if scenario == "E-hist-stream" and b_pairs >= 200 else 60.0
        wait_pane(name, "fake", timeout=boot_timeout)
        time.sleep(0.5)

        # Scenario-specific prep before/during sampling
        if scenario == "D-resume":
            send_literal(name, "/session-resume")
            send_keys(name, "Enter")
            # list_sessions → Loading N/N paint, then picker paint can be slow
            # (width/ANSI on many rows); do not require "Resume" too early.
            wait_pane(name, "Loading sessions", timeout=30.0)
            wait_pane(name, "Resume Session", timeout=30.0)

        pid = find_xylitol_pid(bin_path, home)
        out_gz = out_dir / f"{scenario}.json.gz"
        err = out_dir / f"samply-{scenario}.err"
        # Attach samply; drive keys while recording via background thread timing.
        samply_cmd = [
            "samply",
            "record",
            "--save-only",
            "-n",
            "-d",
            str(duration),
            "-o",
            str(out_gz),
            "-p",
            str(pid),
        ]
        print("+", " ".join(samply_cmd), flush=True)
        with err.open("w", encoding="utf-8") as ef:
            proc = subprocess.Popen(samply_cmd, stdout=ef, stderr=ef)

        # Actions during recording window
        t0 = time.time()
        if scenario == "A-idle":
            while time.time() - t0 < duration - 1:
                time.sleep(0.5)
        elif scenario == "B-scroll":
            while time.time() - t0 < duration - 1:
                send_keys(name, "PPage")
                time.sleep(0.15)
                send_keys(name, "NPage")
                time.sleep(0.15)
        elif scenario in ("C-stream", "E-hist-stream"):
            send_literal(name, "ping stream profile")
            send_keys(name, "Enter")
            while time.time() - t0 < duration - 1:
                time.sleep(0.2)
        elif scenario == "D-resume":
            while time.time() - t0 < duration - 2:
                send_keys(name, "Down")
                time.sleep(0.08)
            send_keys(name, "C-u")
            time.sleep(0.4)
            send_keys(name, "C-u")
            time.sleep(0.3)
            for _ in range(8):
                send_keys(name, "Down")
                time.sleep(0.08)

        # samply -p sometimes ignores -d; stop explicitly after the window.
        deadline = t0 + duration
        while time.time() < deadline:
            if proc.poll() is not None:
                break
            time.sleep(0.2)
        if proc.poll() is None:
            proc.send_signal(signal.SIGINT)
            try:
                proc.wait(timeout=45)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=10)
                die(f"samply hung; see {err}")
        rc = proc.returncode
        # SIGINT exit may be non-zero; accept if profile exists.
        if not out_gz.is_file():
            die(f"samply failed (rc={rc}); missing {out_gz}; see {err}")

        # Summarize
        summary = out_dir / f"{scenario}.summary.txt"
        run(
            [
                sys.executable,
                str(REPO / "scripts" / "summarize_samply_profile.py"),
                str(out_gz),
                "--addr2line",
                str(bin_path),
                "-o",
                str(summary),
            ],
            check=False,
        )
        print(f"OK {scenario} -> {out_gz} / {summary}", flush=True)
        return out_gz
    finally:
        tmux(["kill-session", "-t", name])


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--build", action="store_true", help="release build with symbols")
    ap.add_argument("--all", action="store_true", help="run A,B,C,D,E")
    ap.add_argument(
        "--scenarios",
        default="",
        help="comma list: A..E / A-idle,B-scroll,C-stream,D-resume,E-hist-stream",
    )
    ap.add_argument("--bin", type=Path, default=DEFAULT_BIN)
    ap.add_argument("--duration", type=int, default=20, help="samply seconds per scenario")
    ap.add_argument(
        "--b-pairs",
        type=int,
        default=80,
        help="seed pair count for B-scroll and E-hist-stream (default 80; 400+ for c1505)",
    )
    ap.add_argument("--cols", type=int, default=120)
    ap.add_argument("--rows", type=int, default=40)
    ap.add_argument(
        "--run-id",
        default="",
        help="output dir name under target/profile/ (default: timestamp)",
    )
    args = ap.parse_args()

    alias = {
        "A": "A-idle",
        "B": "B-scroll",
        "C": "C-stream",
        "D": "D-resume",
        "E": "E-hist-stream",
        "A-idle": "A-idle",
        "B-scroll": "B-scroll",
        "C-stream": "C-stream",
        "D-resume": "D-resume",
        "E-hist-stream": "E-hist-stream",
    }
    scenarios: list[str] = []
    if args.all:
        scenarios = [
            "A-idle",
            "B-scroll",
            "C-stream",
            "D-resume",
            "E-hist-stream",
        ]
    elif args.scenarios.strip():
        for part in args.scenarios.split(","):
            part = part.strip()
            if not part:
                continue
            if part not in alias:
                die(f"unknown scenario {part!r}")
            scenarios.append(alias[part])
    elif args.build and not args.scenarios:
        profile_build(args.bin)
        print("build ok:", args.bin)
        return 0
    else:
        die("pass --all, --scenarios, or --build alone")

    require_tools()
    if args.build or not args.bin.is_file():
        profile_build(args.bin)

    run_id = args.run_id or datetime.now().strftime("%Y%m%d-%H%M%S")
    out_dir = PROFILE_ROOT / run_id
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "README.txt").write_text(
        f"xylitol profile suite run {run_id}\n"
        f"bin={args.bin}\n"
        f"scenarios={scenarios}\n"
        f"duration={args.duration}s\n"
        f"b_pairs={args.b_pairs}\n"
        "note: E-hist-stream auto-sizes FAKE_SLOW_STREAM to ~cover duration\n",
        encoding="utf-8",
    )

    for sc in scenarios:
        run_scenario(
            sc,
            bin_path=args.bin.resolve(),
            out_dir=out_dir,
            duration=args.duration,
            cols=args.cols,
            rows=args.rows,
            b_pairs=args.b_pairs,
        )

    print(f"\nAll done: {out_dir}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
