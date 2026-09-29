---
depends_on: []
branch: sdd/bridge-rootside-rules
base_branch: main
base_sha: 2b76f1f59b86139f0ae8eb423e6acf9be42ab2bf
---

## Why

c2833 子项目迁移时 bridge 族整体先迁入 `packages/xylitol-ai-bridge/llmanspec/`，把「根侧规则拆回根」留成两处 `c2833 TODO(split)` 注释。欠账点：

- `package-ai-bridge` 有 2 条规则本体在根侧代码：r1543（session-vs-llm-vocab，锚 `src/protocol/message.rs`）、r1559（主仓映射层，锚 `src/agent/llm_project.rs`）；其 scope 仍跨进 `src/infra/provider/`。
- `package-ai-bridge-accounting` 的 scope 仍跨进 `src/agent/compaction/`（规则本体全部是 bridge 库行为，仅锚点是 covered-by 引用）。

子根 specs 声明根侧目录与上游「路径单一归属」的实例根基准不符；TODO 注释长期挂在 live specs 上也是债。

## What Changes

- 新建根 capability `llmanspec/specs/agent-llm-projection/`（scope `src/protocol/, src/agent/`），承接 r1543、r1559 **原文平移、保留原 id**（id 随规则走，不重编号：两 id 在 bridge 侧删除后全局仍唯一，零撞号风险，也绕开上游 next-req-id 跨根盲区）。
- `package-ai-bridge.feature`：删除上述 2 条规则、scope 收窄为 `packages/xylitol-ai-bridge/`、删 TODO 注释。
- `package-ai-bridge-accounting.feature`：scope 收窄为 `packages/xylitol-ai-bridge/`、删 TODO 注释；规则不动。
- 两条规则均无场景，BDD bindings 零影响；迁移后核对矩阵守恒（879 规则 / 879 唯一 id / naked 6 不变）。

## Evidence

- c2833 proposal §3 与 tasks T3（拆分留后续 change）。
- `scripts/check_spec_anchors.py` 跨根唯一性门禁（599756f4）兜底取号纪律。
