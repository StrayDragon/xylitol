---
depends_on: []
branch: sdd/c2510-explore-grouping
base_sha: 5df0baa5
checkpointed: false
---

# 探索簇近窗默认收起：簇头附加类目计数后缀

## Why

折叠族的既有契约（att23/att24/att26）已提供「信封⊃簇⊃块」结构、`Explored`
簇头词形与**回合末**自动收纳（keep_recent_turns=2），但**近窗与流式期间**簇内
工具块仍按 att20 tools 族默认展开逐条平铺：一轮里连环读五个文件、grep 三轮时，
真正关键的助手决策被逐条工具行挤出视口，且要求用户自己发现并手动收
（Alt+Shift+E / Ctrl+Alt+Shift+E）。缺的是一层「按语义自动聚合」的默认策略，
让默认视图天然安静。

**现状核对到 2026-09-03**（research 笔记 + 代码）：activity_fold 机械层
（segment/state/summary/degrade/settings）完备；`stream_collapse` 已覆盖
distant turn 的折叠地板（L2 同形）；近窗地板为 L0（att20 子块默认展开）。
本票增量精确为：**近窗/流式窗口内，纯探索簇的默认地板条件性降到 L2**——
即既有簇头机制的子块层延伸，不新增行类型、不动簇头基础词形。

## What Changes

- **探索簇默认收起（纯函数判定 + 既有 L2 机制）**：某簇的折叠中间段全部为
  读/检索类且总数 ≥ 阈值（默认 3）时，该簇在近窗与流式期间默认呈簇头收纳态
  （L2 同形：仅簇头行可见、子块不进渲染），簇头附加类目计数后缀
  `· 5 reads · 2 searches`；进行时用既有 `Exploring` 词形，计数随工具开始
  更新（对齐 att24 计数时机）。
- **用户意志优先**：显式展开后该簇保持展开（新段到达/流式推进不自动收起）；
  展开后子块服从既有块级折叠（att20/21）。
- **组合不变式**：不改变 att23 嵌套、att24 基础词形（后缀是 att35 的条件性
  附加）、att26 回合窗与 `stream_collapse` 折叠地板；收起簇纳入 att28 就近
  展开作用域；L2 渲染天然满足 att25（子块不进渲染）。
- **配置**：`tui.activity_fold.auto_collapse_explore`（缺省 true）；false 时
  近窗全部簇保持 att20 默认展开，块级折叠仍可用。
- **触发条件**：仅纯读/检索簇（混入写类 / bash / compaction / ask / todo /
  MCP 任一即不收起，保持逐块渲染）。
- **designing 随动**：扩展 `designing/tui/modules/activity-fold/`——intent.md
  屏上词表增补分组行条目与手动展开退组 MUST、states 增补分组固定态（默认合并 /
  展开保持 / 进行时文案 / 写类相邻反例）、draft.yaml 记录阈值取舍；chrome 词表
  G 类同步；`just gen-designing-index`。

## 非目标

- 不改 segment 落盘结构与回放合约（折叠机械层不动，只加默认地板策略）。
- 不自动收起混类簇（写类 / bash / diff / compaction / ask / todo / MCP 任一
  混入即不收起）；glob / webfetch 不进首版类别清单。
- 不动 `stream_collapse` 的 distant-turn 折叠地板语义与 att24 基础词形
  （后缀为本票新增条件性附加，att24 场景文本零改动）。
- 不新增行类型 / FoldTarget / 组 id 机制——收纳态就是既有 L2。

## Impact

- `src/app/tui/activity_fold/`：纯函数簇判定（纯探索 + 计数）；degrade 近窗
  条件地板；summary 簇头后缀格式化；settings 增 `auto_collapse_explore`。
- `llmanspec/specs/app-tui-transcript/app-tui-transcript.feature` +1 规则与
  可执行场景（att35）；`llmanspec/specs/runtime-config/runtime-config.feature`
  +1 姊妹场景（rc29，不动既有 rc28）。
- designing activity-fold 模块（intent/states/draft）随动；BDD 场景与
  designing 固定态一一对应。
- 测试双缝（已拍板）：transcript harness（HostPumpBdd 同族：合成 Driver 推
  ToolExecution 事件、断言 scrollback 渲染行）为主 + 判定函数库内单测。
