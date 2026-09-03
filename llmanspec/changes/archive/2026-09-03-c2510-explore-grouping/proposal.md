---
depends_on: []
branch: sdd/c2510-explore-grouping
base_sha: 5df0baa5
checkpointed: true
checkpoint_sha: 5df0baa5
---

# 探索簇头类目计数后缀：Explored N files · N reads · M searches

## Why

折叠族的既有契约（att23/att24/att26）已把近窗与流式的簇内子块默认收纳在簇头
之后（点击簇头 / att28 就近展开才见细账）——原调研笔记担心的「逐条平铺」已被
c1760/c2045 簇机制解决。**现状核对到 2026-09-03 代码**（`cluster_is_expanded`：
L0/L2 均需 `cluster_open` 才渲染子块；live 窗同样）后，真实剩余缺口只有一个：
簇头词形只有文件计数（`Explored 3 files`，去重 path），**没有调用类目细分**——
用户无法从簇头得知「这轮探索到底是粗读三个文件，还是深挖了五轮检索」。

## What Changes

- `ActivityCounts` 增加探索调用次数维度（`read_calls` / `search_calls`，
  按 `ExploreKind::{File, Search}` 细分，无路径搜索计入 searches）；
- 簇头在 att24 文件层词形之后附加类目计数后缀 `· 5 reads · 2 searches`
  （仅列非零类目；进行时 `Exploring` 词形同样携带，计数随工具开始更新，
  对齐 att24 计数时机）；后缀只附着于探索分句——Edited 头不附，沿用 att24
  既有文件层互斥精神；
- 簇内子块的既有默认收纳 / 展开语义（`cluster_kids_visible` / att28）保持
  不变——本票是纯信息密度增强，不新增行类型、不新增设置键、不动任何既有
  折叠行为条款（att23/24/26 零改动）。

## 非目标

- 不改 segment 落盘结构与回放合约；不动簇内子块的默认收纳机制（现状即收起，
  不新增开关去「放开」它）。
- 不给 Edited 头附加探索计数（att24 文件层互斥的自然延伸）。
- glob / webfetch 的类目细分不在首版（Search 类内不再细分）。

## Impact

- `src/app/tui/activity_fold/summary.rs`：`ActivityCounts` +2 字段、
  `counts_from_atoms` 计数、`format_cluster_body` 后缀格式化；库内单测。
- `llmanspec/specs/app-tui-transcript/app-tui-transcript.feature`：att35 规则
  与可执行场景（本票新增）。
- designing activity-fold 模块（intent 词表 + states + draft）随动；BDD 场景
  与 designing 固定态一一对应。
- 测试双缝（已拍板）：transcript harness（`SceneBuilder` 回放 XyEvent → 无头
  帧断言，att24 可执行场景同缝）为主 + summary 纯函数库内单测。
