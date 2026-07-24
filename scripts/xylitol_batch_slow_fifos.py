#!/usr/bin/env python3
"""Prepare FIFO-backed slow reads for tool_batch parallel wall-clock experiment.

Starts three writers that each block until `read` opens the FIFO, then sleep
DELAY_SEC and write a marker. Parallel reads ≈ DELAY_SEC; sequential ≈ 3×DELAY.

Usage:
  python3 scripts/xylitol_batch_slow_fifos.py          # foreground, Ctrl+C to stop
  python3 scripts/xylitol_batch_slow_fifos.py --delay 2

Then in xylitol (same-message 3× read), see experiments.md Experiment 1.
"""
from __future__ import annotations

import argparse
import os
import signal
import sys
import threading
import time
from pathlib import Path

DEFAULT_DIR = Path("/tmp/xylitol-batch-demo/slow")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dir", type=Path, default=DEFAULT_DIR)
    ap.add_argument("--delay", type=float, default=2.0)
    args = ap.parse_args()
    d: Path = args.dir
    d.mkdir(parents=True, exist_ok=True)
    names = ("a", "b", "c")
    paths = []
    for name in names:
        p = d / f"{name}.txt"
        if p.exists() or p.is_fifo():
            p.unlink()
        os.mkfifo(p)
        paths.append((p, name))

    stop = threading.Event()

    def writer(path: Path, tag: str) -> None:
        while not stop.is_set():
            try:
                # Blocks until a reader opens the FIFO.
                with open(path, "w", encoding="utf-8") as f:
                    time.sleep(args.delay)
                    f.write(f"MARK_{tag}\n")
                    f.flush()
            except FileNotFoundError:
                break
            except OSError as e:
                if stop.is_set():
                    break
                print(f"writer {tag}: {e}", file=sys.stderr)
                time.sleep(0.2)

    threads = [
        threading.Thread(target=writer, args=(p, tag), daemon=True) for p, tag in paths
    ]
    for t in threads:
        t.start()

    def _stop(*_args: object) -> None:
        stop.set()
        for p, _ in paths:
            try:
                p.unlink(missing_ok=True)
            except OSError:
                pass
        sys.exit(0)

    signal.signal(signal.SIGINT, _stop)
    signal.signal(signal.SIGTERM, _stop)

    print(f"slow FIFOs ready under {d} (delay={args.delay}s)")
    print("Issue in ONE assistant message:")
    for p, _ in paths:
        print(f"  read {p}")
    print("Expect ~{:.1f}s wall if parallel; ~{:.1f}s if serial.".format(
        args.delay, args.delay * 3
    ))
    while not stop.is_set():
        time.sleep(1)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
