---
brief: "临时：cccc-rs 与 cargo-crap 对比实验（不进仓库 scripts/、不进 just）"
---

# cccc vs cargo-crap（临时参考）

**不进 `just qa`、不进 `scripts/`**。需要复跑对比时用本目录附带脚本；决策收口后删除本 reference 与 `lab_complexity_compare.py`。

## 退役结论（当前）

| 保留 | 理由 |
|---|---|
| `just qa-check-complexity`（cccc） | **cognitive** 硬闸；cargo-crap 无认知复杂度 |
| CI `crap.yaml` / `just crap` | 覆盖 × 圈复杂度回归 |
| `just complexity` / `just crap-duplicates` | 本地软雷达 |

**不得**用 cargo-crap 单独替代 cccc 硬闸。

## 复跑对比

在仓库根目录：

```bash
python3 .claude/skills/xylitol-dev/references/lab_complexity_compare.py
python3 .claude/skills/xylitol-dev/references/lab_complexity_compare.py --top 40 --write
```

依赖：`just _cccc-bin`、`cargo-crap`、在 git 工作区内执行（脚本用 `git rev-parse --show-toplevel` 定位根）。

本地软雷达（无本脚本）：`just complexity` → `just crap-duplicates` →（可选）`just coverage-lcov` + `just crap`。
