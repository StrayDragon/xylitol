## 设计笔记

### D1 按块状态模型：全局默认 + 覆盖表（选定）

沿用本文件已验证两次的模式（`tools_overrides` / `thinking_overrides`，att7 / att20 / att21）：

```rust
// ScrollbackFold 新增
pub output_overrides: HashMap<String, bool>,
pub fn output_effective(&self, id: &str) -> bool  // 覆盖缺失回退 tools_output_expanded
pub fn toggle_output(&mut self, id: &str)
```

- `FoldTarget::OutputViewport` → `OutputViewport(String)`（携带块 id）。
- 备选「完全去掉全局布尔、每块独立」被否：键盘 Ctrl+O 无目标块概念，失去「全部展开/折叠」逃生口；且破坏 Alt+E / Ctrl+T 已建立的「默认+覆盖」心智模型。
- `clear_output_overrides()` 在 Ctrl+O 键路径调用（同构 Alt+E 的 `clear_tools_overrides()`）。

### D2 块 id 来源

| 块 | id | 稳定性 |
|---|---|---|
| Tool | 既有 `toolCallId`（att12） | 直播/rebuild 一致 |
| Diff（独立块与 write/edit 正文） | 既有 `diff_fold_key`（内容哈希） | 同既有 L1 三角键，流式期间键变化丢失覆盖为已接受语义 |
| Bash | **新增** `UiEntry::Bash.id`：直播 `begin_bash_block` 分配序号 id（参照 `allocate_thinking_id` 先例）；rebuild 取 JSONL `bashId` | 单次渲染会话内稳定即可——覆盖表是内存态，resume 重建后本就清空 |

不选「命令文本哈希当 Bash 键」：同命令两次执行会互相串扰，违背按块直觉。

### D3 折叠提示行条件（包层 peo3）

- 仅当 `expanded == true` 且**实际视觉行数** > `max_preview_lines`（即 collapsed 态本会折叠的块）才追加 `... (expanded, {fold_hint})` dim 行；短内容、空文本不追加（保持「expanded 无冗余提示」旧直觉不被滥用）。
- 硬截断块（att16）app 侧永不传 `viewport_full=true`，天然无此行。
- `hint_style` 沿用既有选项；`fold_hint` 默认 `"ctrl+o to fold"`。
- 命中注册：`push_expandable_with_viewport_hit` 对 collapsed hint 行与 expanded fold 行都注册 `FoldTarget::OutputViewport(id)`，命中几何同 att30（文案可点带）。

### D4 缓存与重绘（ath25）

entry fingerprint 现哈希全局 `tools_output_expanded`（cache.rs 三处）→ 改为哈希该块 `output_effective(id)`。全局 Ctrl+O 切换改变所有相关块的有效值 → 自然全量相关块重绘；单块点击只改一个键 → 只重绘该块。`ScrollbackFoldDefaultsKey`（`(thinking, tools)`）保持不含 output 值。
