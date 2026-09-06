---
depends_on: []
---

# 疑似项清欠：adapter 层塌缩、词表常量化、超长函数深拆与死接线处置

## Why

2026-09 二轮代码体检（死码 / slop / 测试盲区三线审计，落地为 e93f81ae..f01f3ce6 四轮清淤提交）之后，以下项目被判定为「疑似」：要么是架构判断（塌缩后是否损害未来开闭），要么是行为面广的高风险重构（需要逐段回归），不适合混进机械清理提交。本 change 把它们集中存档避免散失；正式化（propose）时按项 triage，可整单落地也可拆单。

## What Changes

- **LlmAdapter → AdapterXyModel 双层 1:1 包装塌缩**：`src/infra/provider/adapter/mod.rs` 三个零调用命名包装壳已随 e93f81ae 删除；此后 `LlmAdapter` trait 仅剩 `MappedBridgeAdapter` 一个实现、唯一消费者 `AdapterXyModel`。评估把 to_bridge_tools / to_xy_error / to_xy_stream 映射内联进 AdapterXyModel、塌缩为一层。决策点：domain-facing seam 的存废主张（见 `src/AGENTS.md`「扩展开闭」）——第二实现出现前保留 trait 是否仍有价值。
- **Fake 模型双轨收拢**：`src/infra/provider/fake.rs` 手写 to_bridge_tools + map_err(to_xy_error) + to_xy_stream，与 `MappedBridgeAdapter::generate_stream` 同构；bridge 侧 `AiBridgeModel`（单实现、唯一消费者）若改实现 `AiBridgeLlmAdapter`，可整条复用 MappedBridgeAdapter 映射并删除一个平行 trait。
- **thinking-level 字符串词表**：`src/app/core/dispatch.rs` 的 `level == "high"/"off"/"vendor-max"` 散落比较 + bridge `canonical_known_level` 的规范化词表，两处无单一来源类型。在 protocol 暴露词表常量或类型，消散落字面量（与已落地的 `METHOD_*` / `NODE_KIND_*` 同思路，见 1d332279）。
- **超长函数深拆**（机械部分已做之外）：`ensure_downlink`（`src/app/core/driver/remote.rs`，≈240 行，按 downlink 类型拆臂函数）、`ChoicePrompt::render`（`packages/xylitol-tui/src/components/choice_prompt.rs`，≈240 行）、`cli::run`（`src/app/cli/mod.rs`，≈223 行，子命令分派 + 配置装配 + surface 启动内联）。
- **navigate_tree 退役**：`SessionManager::navigate_tree` 生产零调用，唯一消费者是内部实现型 BDD 场景 tree-nav（agent-session-store.feature @req:s5）。删除需动 live specs（场景 + steps 成组退役），quick 路径禁止，须在本 change 的绑定分支上做；或确认保留为内部 API 并写落地条件。
- **XyEvent 未发射变体处置**：`ThinkingLevelChanged` / `SessionInfoChanged` 消费者侧（bridge / print / wire 投影）已实现并有测试，但生产者从不发射——多面 attach 场景下是跨面同步缺口。**方向已定（2026-09-07 用户拍板）：后续补发射，不退役**——判定为迁移遗留（消费侧先落、生产侧没跟上），不是死码。待办：在 driver / capabilities 层补发射点（设计点：状态归属与事件去重）。同族 `AutoRetryStart/End` 的生产者接线已在四轮清淤后补齐（react 重试环内联 yield），可作实现参照。
