---
depends_on: []
branch: sdd/c2510-explore-grouping
base_sha: 5df0baa5
checkpointed: false
---

# 探索分组：近窗连续检索段默认聚合为一行计数摘要

## Why

折叠族的既有契约（att23/att24/att26）已提供「信封⊃簇⊃块」结构、`Explored`
簇头词形与**回合末**自动收纳（keep_recent_turns=2），但**近窗与流式期间**簇内
工具块仍按 att20 tools 族默认展开逐条平铺：一轮里连环读五个文件、grep 三轮时，
真正关键的助手决策被逐条工具行挤出视口，且要求用户自己发现并手动收
（Alt+Shift+E / Ctrl+Alt+Shift+E）。缺的是一层「按语义自动聚合」的默认策略，
让默认视图天然安静。

**现状核对到 2026-09-03**（research 笔记 + 代码）：activity_fold 机械层
（segment/state/summary/degrade/settings）完备；`stream_collapse` 已覆盖
distant turn 的折叠地板——本票不与之重叠，增量精确为**近窗/流式窗口内**的
聚合显示层。

## What Changes

- **分组策略（显示层，纯函数分类器）**：相邻连续同类检索段（读文件 / 搜索；
  中间无助手正文或其它类段隔断）达到阈值（默认 ≥3）时，默认渲染为单行分组
  摘要——类目计数式 `✱ Explored — 3 reads · 2 searches`；进行时复用既有
  `Exploring` 词形，计数随工具开始更新（对齐 att24 计数时机）。不足阈值或
  类别断开保持既有逐块渲染。
- **用户意志优先**：手动展开某分组后，该组保持展开、不再被自动收起；组 id
  稳定，live 与 travel/fork/resume 重建同构。
- **组合不变式**：分组不改变 att23 嵌套结构、att24 簇头词形（文件计数式）、
  att26 回合窗收纳——分组行与簇头以 `✱` 标记 + 类目计数格式在词表区隔；
  组折叠时子块不进渲染（att25 同构）；展开后子块服从既有块级折叠（att20/21）。
- **配置**：`tui.activity_fold.auto_group`（缺省 true）；false 回到全细账
  逐块渲染，块级折叠仍可用。
- **类别清单**：首版仅 {读文件, 检索}；写类 / bash / diff / compaction /
  ask 段 MUST NOT 进分组。
- **designing 随动**：扩展 `designing/tui/modules/activity-fold/`——intent.md
  屏上词表增补分组行条目与手动展开退组 MUST、states 增补分组固定态（默认合并 /
  展开保持 / 进行时文案 / 写类相邻反例）、draft.yaml 记录阈值取舍；chrome 词表
  G 类同步；`just gen-designing-index`。

## 非目标

- 不改 segment 落盘结构与回放合约（折叠机械层不动，只加策略层）。
- 不合并写类 / bash / diff / compaction / ask；glob / webfetch 不进首版类别。
- 不动 `stream_collapse` 的 distant-turn 折叠地板语义。
- 不动纵向槽高预算与 att28 就近折叠键位语义（分组行纳入其作用域为随动细节，
  不另开键位）。

## Impact

- `src/app/tui/activity_fold/` 策略层新增纯函数分类器；settings 增
  `auto_group`；scrollback 渲染行数下降。
- `llmanspec/specs/app-tui-transcript/app-tui-transcript.feature` +1 规则与
  可执行场景（att35 起）；`llmanspec/specs/runtime-config/runtime-config.feature`
  +1 姊妹场景（rc29，不动既有 rc28）。
- designing activity-fold 模块（intent/states/draft）与 chrome 词表 G 类随动；
  BDD 场景与 designing 固定态一一对应。
- 测试双缝（已拍板）：transcript harness（HostPumpBdd 同族：合成 Driver 推
  ToolExecution 事件、断言 scrollback 渲染行）为主 + 分类器纯函数库内单测。
