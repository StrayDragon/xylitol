---
depends_on: []
rules_edit_acked: true
branch: sdd/c2540-replace-cluster-head-basename-with-count
base_sha: 28e72e2d
checkpointed: true
checkpoint_sha: 28e72e2d
---

# 簇头文件分句统一计数式：撤销 basename 回显

## Why

att24 规定单路径簇头写可用 basename（`Explored old.rs`）。人工实测（c2510 手动
验收反馈）：basename 是「内容回显」而非动作+指标——文件名一闪而过读不到价值，
用户只关心规模指标，具体文件展开簇内子块即可见；带空格 / 长名字时观感更差。
与 c2510 刚落成的类目计数后缀（`· N reads · M searches`）并列时，
`Explored Cargo.toml · 2 reads` 的「名词 + 两类数字」混排进一步放大了违和感。

## What Changes

- **att24 文件分句改为纯计数式**：恰一个去重 path 写 `1 file`、多个写
  `N files`（Edited / Explored 同规则）；撤销 basename / 上一路径分量回退等
  特例（`.` / `..` 处理随 basename 一起消亡，天然不可能再输出 `Explored .`）。
- 进行时词形同构：`Editing 1 file` / `Exploring 3 files`。
- att24 可执行场景（cluster-head-wording-exclusivity）措辞随动
  （`Explored old.rs` → `Explored 1 file`、`Editing a.rs` → `Editing 1 file`）。
- 其余词形零改动：`Ran N commands`、`Used N tools`（单次仍写 `Used {短名}`，
  工具短名不在本票范围）、探索计数后缀（c2510/att35）、att23/26 嵌套与回合窗。

## 非目标

- 不动 `Used {短名}`（工具短名单头）——若也要计数式另行开票。
- 不动 designing 固定态之外的视觉细节（折行、配色）。

## Impact

- `src/app/tui/activity_fold/summary.rs`：`file_clause` 删除 basename 分支与
  `basename()` 助手（真死即删）；summary.rs 既有词形单测随动。
- `llmanspec/specs/app-tui-transcript/app-tui-transcript.feature`：att24 规则
  文件分句句改（rules_edit_acked）+ 同文件 att24 可执行场景措辞随动。
- tests/bdd/steps_app_tui_transcript.rs：att24 场景断言随动；c2510 att35 场景
  单文件帧断言随动（`Exploring 1 file · 1 read`）。
- designing activity-fold states 若含 basename 词形则随动（ 初查无）。
