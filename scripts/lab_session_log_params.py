#!/usr/bin/env python3
"""Lab probe: session log parameter baselines (anchor density, rotation threshold, fsync batch cost).

Parameters are fixed in code (design c3233 §4/§6). This probe records the baselines; a
parameter is only revisited if a measurement deviates by more than 2x from the recorded
baseline. Run directly: python3 scripts/lab_session_log_params.py
"""

import json
import os
import statistics
import tempfile
import time

ANCHOR_EVERY = 64
INITIAL_THRESHOLD_BYTES = 64 * 1024 * 1024
MIN_THRESHOLD_BYTES = 16 * 1024 * 1024
MAX_THRESHOLD_BYTES = 256 * 1024 * 1024
TURNS_PER_FILE = 256


def measure_fsync_batch(line_bytes: int, lines: int, batch: int) -> float:
    with tempfile.NamedTemporaryFile(delete=False) as handle:
        path = handle.name
        payload = b"x" * line_bytes
        start = time.perf_counter()
        for index in range(lines):
            handle.write(payload)
            if index % batch == batch - 1:
                handle.flush()
                os.fsync(handle.fileno())
        handle.flush()
        os.fsync(handle.fileno())
        elapsed = time.perf_counter() - start
    os.unlink(path)
    return elapsed


def anchor_scan_cost(anchor_every: int, target_line: int) -> int:
    anchor = (target_line // anchor_every) * anchor_every
    return target_line - anchor


def suggest_threshold(turn_bytes):
    if not turn_bytes:
        return INITIAL_THRESHOLD_BYTES
    median = statistics.median(turn_bytes)
    suggested = int(median * TURNS_PER_FILE)
    return max(MIN_THRESHOLD_BYTES, min(MAX_THRESHOLD_BYTES, suggested))


def main():
    line_bytes = 512
    lines = 4096
    per_line = measure_fsync_batch(line_bytes, lines, batch=1)
    per_turn = measure_fsync_batch(line_bytes, lines, batch=8)
    per_file = measure_fsync_batch(line_bytes, lines, batch=64)

    synthetic_turns = [2048, 4096, 8192, 16384, 32768, 65536, 131072]
    threshold = suggest_threshold(synthetic_turns)

    report = {
        "anchor_every": ANCHOR_EVERY,
        "max_scan_lines_without_anchor": ANCHOR_EVERY - 1,
        "anchor_scan_cost_at_4096": anchor_scan_cost(ANCHOR_EVERY, 4096),
        "rotation": {
            "initial_threshold_bytes": INITIAL_THRESHOLD_BYTES,
            "min_bytes": MIN_THRESHOLD_BYTES,
            "max_bytes": MAX_THRESHOLD_BYTES,
            "turns_per_file": TURNS_PER_FILE,
            "suggested_threshold_bytes": threshold,
        },
        "fsync_cost_seconds": {
            "per_line": round(per_line, 6),
            "per_turn_batch_8": round(per_turn, 6),
            "per_file_batch_64": round(per_file, 6),
        },
        "fsync_speedup_turn_batch_vs_per_line": round(per_line / per_turn, 2)
        if per_turn > 0
        else None,
    }
    print(json.dumps(report, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
