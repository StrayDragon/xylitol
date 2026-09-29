#!/usr/bin/env python3
"""从 wire v3 fbs 真源重新生成 Rust 产物并(可选)校验无漂移。

维护脚本(不进 `just qa` 门禁:依赖本地 apache/fory clone)。

用法:
  just codegen-wire            # 重新生成并写回 src/protocol/wire/v3/generated.rs
  just codegen-wire --check    # 只校验 check-in 产物与真源一致(漂移即非零退出)

前置:apache/fory clone 位于本仓库同级目录(默认 ../fory,可用 FORY_ROOT 覆盖),
其 compiler 为纯 Python 零依赖,直接 PYTHONPATH 调用。
"""

from __future__ import annotations

import argparse
import difflib
import os
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
FBS = REPO_ROOT / "src/protocol/wire/v3/xy_wire_v3.fbs"
CHECKED_IN = REPO_ROOT / "src/protocol/wire/v3/generated.rs"
FORY_ROOT = Path(os.environ.get("FORY_ROOT", REPO_ROOT.parent / "fory"))
COMPILER = FORY_ROOT / "compiler"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--check", action="store_true", help="只校验不写回")
    args = ap.parse_args()

    if not COMPILER.is_dir():
        print(
            f"fory compiler 不存在:{COMPILER}\n"
            "clone apache/fory 到仓库同级(或设 FORY_ROOT),见 "
            "llmanspec/changes/c2834-update-v3-fory-flutter-poc/research/05-fbs-前端保留字实测.md"
        )
        return 2

    with tempfile.TemporaryDirectory() as tmp:
        out = Path(tmp) / "gen"
        cmd = [
            sys.executable,
            "-c",
            "import sys; from fory_compiler.cli import main; "
            f"sys.argv=['foryc', {str(FBS)!r}, '--lang', 'rust', '-o', {str(out)!r}]; main()",
        ]
        env = dict(os.environ, PYTHONPATH=str(COMPILER))
        subprocess.run(cmd, check=True, env=env, cwd=tmp)
        generated = (out / "rust/xy_wire_v3.rs").read_text()

    if args.check:
        current = CHECKED_IN.read_text()
        if generated == current:
            print("codegen-wire --check: 生成物与真源一致")
            return 0
        diff = "\n".join(
            difflib.unified_diff(
                current.splitlines(), generated.splitlines(),
                fromfile="checked-in generated.rs", tofile="regenerated", lineterm="",
            )
        )
        print("生成物漂移(真源或 compiler 变更后未同步再生成):\n" + diff[:4000])
        return 1

    CHECKED_IN.write_text(generated)
    print(f"已写回 {CHECKED_IN.relative_to(REPO_ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
