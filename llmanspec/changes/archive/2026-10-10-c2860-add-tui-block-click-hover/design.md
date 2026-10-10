# 设计：产品折叠命中与悬停

## 1. 决策表

| # | 决策 | 理由 |
|---|---|---|
| D1 | 叶块（Tool / Diff / Ask / Thinking）收起点 **块头全宽** 展开；展开后 **空选点 painted 正文** 也折 | 只加宽块头方便展开、不方便折回；opencode 空选守卫 = `UpIfEmptySelection`。body **不** capturing，否则划词死 |
| D2 | ActivityFold 信封/簇 **仅块头行**（含折叠标记，全宽），不登记展开后的体 | 体是嵌套块与夹心助手正文；整信封点会点穿 |
| D3 | Compaction **保持三角列-only**；summary 不得进可点体 | 全局 `compaction_expanded`；视觉分割线交给延后 c2892 |
| D4 | 待办栏本票不改 | 固定区（输入坞），不是对话条目 |
| D5 | att30 输出视口提示带后登记，只折视口 | 与 L1 分层；hint 比工具体更具体 |
| D6 | 悬停用引擎 dim，不新开 hover-bg、不复活 `tool-*-bg` | DESIGN 默认不整行洗底；c2858 已和选区反色分开 |
| D7 | 悬停画 **块头 / 可见左边轨 / 提示带**，不 dim 输出正文 | 点击区可以大于悬停区；满屏 dim 违反「避免满屏闪」 |
| D8 | 产品只走 `mark_hit` + `take_pointer_click`；摘掉 `install_fold_triangle_hit_priority` | 双表会和 Up 相位、悬停 id 不同步。包测 `HitPriorityFn` 可留 |
| D9 | ApplicationOwned begin 后武装探针；fan-in 仅在武装时停丢 Moved | 否则悬停到不了引擎；未武装仍守 r1258 无态 Moved 不整帧 |
| D10 | 晋级 `designing/tui-lab/modules/hover-highlight/` → `tui/modules/` | SOP：定案即晋级；intent 从整块淡底改为 dim + 块头/轨 |

## 2. 命中 z-order（后登记赢）

```
信封块头 → 簇块头 → 叶块头 → 叶块展开正文 → 输出视口提示带
```

点工具只动该工具。点 `ctrl+o to fold` 只折视口。夹心助手正文无折叠命中。

## 3. 产品接线

每帧 paint 后 `replace_hits`：叶块 `Hit::on_line(id, row).cols(..).tall(height)`（展开正文 height>1）；ActivityFold / Compaction 三角或块头一行。hover 另登记或 paint 时只对块头/轨/hint 调 `apply_hover_tint` 的几何（与 click 矩形可不同）。`set_pointer_click_phase(UpIfEmptySelection)`。`take_pointer_click` → 既有 `FoldTarget` toggle。

## 4. spec 映射

| @req | 改什么 |
|---|---|
| r1352 | 三角列-only → 叶块块头+空选正文；ActivityFold 仅块头；Compaction 仍三角列 |
| r1263 | `set_transcript_hit_priority` → `mark_hit` + Up 空选 toggle；拖选中不消费 |
| r1258 | Moved 可丢 → AO 探针武装时 MUST 送达；同 id 不整帧 |
| r1361 | att30 hint 仍只折视口（句可不变，场景补「点正文折 L1 / 点 hint 折视口」） |
| r1359 | Compaction 三角列 **不改句** |
| r1362 | 信封/簇定点仍禁止 collapseNearest 代替；块头可全宽（放宽「仅三角列」） |

## 5. 测试缝

复用 `src/app/tui/tests.rs` 合成 Mouse、`tests/bdd` 既有「左键单击折叠命中表」步骤（扩几何，不新开 harness crate）。包测 `HitPriorityFn` 不必为本票转绿产品路径。
