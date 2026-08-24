#!/usr/bin/env python3
"""PTY CPU sample for product TUI + restored session (lab; not a qa gate)."""

from __future__ import annotations

import os
import pty
import select
import signal
import time


def cpu_jiffies(pid: int) -> int:
    with open(f"/proc/{pid}/stat", encoding="utf-8") as f:
        parts = f.read().split()
    return int(parts[13]) + int(parts[14])


def sample(pid: int, label: str, secs: float = 4.0) -> float:
    t0 = time.time()
    c0 = cpu_jiffies(pid)
    time.sleep(secs)
    c1 = cpu_jiffies(pid)
    dt = max(time.time() - t0, 1e-6)
    hz = os.sysconf(os.sysconf_names["SC_CLK_TCK"])
    pct = 100.0 * (c1 - c0) / hz / dt
    print(f"{label}: ~{pct:.1f}% CPU over {secs:.1f}s (delta_jiffies={c1 - c0})", flush=True)
    return pct


def drain(fd: int) -> None:
    while select.select([fd], [], [], 0.05)[0]:
        try:
            os.read(fd, 8192)
        except OSError:
            break


def main() -> None:
    session = os.environ.get(
        "XYLITOL_LAB_SESSION", "460ad16e-874f-418f-8ca0-dabc58f89320"
    )
    model = os.environ.get("XYLITOL_LAB_MODEL", "Ornith-1.5-35B-APEX/I-Quality")
    target = os.environ.get("CARGO_TARGET_DIR")
    if not target:
        raise SystemExit("CARGO_TARGET_DIR unset — run: eval \"$(just cargo-wt-env)\"")
    binary = os.path.join(target, "debug", "xylitol")
    if not os.path.isfile(binary):
        raise SystemExit(f"missing binary: {binary}")

    env = os.environ.copy()
    pid, fd = pty.fork()
    if pid == 0:
        os.chdir("/home/l8ng/Projects/__straydragon__/xylitol")
        os.execve(
            binary,
            [binary, "tui", "--trust", "--session", session, "--model", model],
            env,
        )

    time.sleep(7.0)
    drain(fd)
    print(f"child pid={pid} session={session}", flush=True)
    sample(pid, "IDLE", 5.0)
    for _ in range(80):
        os.write(fd, b"\x1b[<64;20;10M")
    sample(pid, "WHEEL", 3.0)
    os.write(fd, b"\x1b[<0;5;5M")
    for row in range(5, 25):
        os.write(fd, f"\x1b[<32;8;{row}M".encode())
    os.write(fd, b"\x1b[<0;8;24m")
    sample(pid, "DRAG", 3.0)
    os.write(fd, b"/exit\r")
    time.sleep(1.5)
    try:
        os.kill(pid, signal.SIGTERM)
    except OSError:
        pass
    try:
        os.waitpid(pid, 0)
    except OSError:
        pass
    print("done", flush=True)


if __name__ == "__main__":
    main()
