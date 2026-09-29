---
depends_on: []
branch: sdd/c2831-add-post-compact-floor-seam
base_branch: main
base_sha: 0cec4b9a3bfd0c556766947323c7263ed15fff4d
---

## Why

批 3 遗留：r1415（压后地板一次性诊断）判为 (c) missing-surface。落实时核查发现**缝与满窗注入场景其实已存在**：orchestrator 的 `floor_notice_emitted` 旗标参数就是注入缝，单测 `floor_notice_fires_once_and_manual_exempt` 已用「window 3000 / overhead 3000」构造压后仍满窗并断言诊断恰发一次。真正缺的只有两件：测试名承诺的 **manual 豁免断言未实现**（`compact()` manual 路径确实不接 floor 旗标，但无断言钉住），以及规则**无 verified-by 锚**（check_spec_anchors 视为 naked）。

## What Changes

- 扩展 `floor_notice_fires_once_and_manual_exempt`：在同一满窗会话上追加一次 manual `compact()`，断言其 CompactionEnd notice 不携带地板诊断文案（manual 豁免落地为断言）。
- r1415 加 `# verified-by: fn floor_notice_fires_once_and_manual_exempt`（组件编排语义，单测承载比 BDD 场景更诚实——BDD 侧 c2826 场景已证明 CompactionEnd 事件面可达）。

## Evidence

- llmanspec/specs/domain-compaction r1415；orchestrator.rs:441-450（floor 判定）、:541（文案）、:865（既有单测）。
- 矩阵预期：with-anchor 406→407、naked 2→1（余 r1790 归 c2832）。
