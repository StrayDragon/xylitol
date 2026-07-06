# TUI 渲染调研报告：为何当前输出「不符合预期」

> 调研日期：2026-07-05
> 范围：c395 自研 renderer 落地后，用户反馈渲染效果仍明显不如 codex「好看舒畅」。
> 结论先行：差距分两层——**样式表残缺（表层）**与**流式架构耦合（根因）**。后者放大前者。

---

## 一、症状对照（用户反馈 + 代码事实）

| 维度 | codex（用户认可） | xylitol 当前（用户不满） | 证据 |
|---|---|---|---|
| 标题 H1–H6 | 6 级分级（H1 下划线 / H2 粗 / H3 粗斜 / H4–6 斜）+ `# ` 前缀 | **6 级共用一个粗体**，**无 `#` 前缀** | xylitol `markdown_render.rs:39, 149-165`；codex `markdown_render.rs:89-94, 547` |
| 引用块 | 每行绿色 `> ` 前缀，与正文清晰区分 | **仅灰斜体段落**，无任何前缀，**看不出是引用** | xylitol `markdown_render.rs:41-44, 262-265`；测试 `markdown_render.rs:496-505` 显式断言「no pipe prefix」 |
| 有序列表 | `1. 2. 3.`（light_blue），自动递增 | **数字 marker 完全丢失**——`Tag::List(Some(_))` 落入 `_ => {}` | xylitol `markdown_render.rs:266, 288` |
| 嵌套列表 | 每级缩进 4 格 + 悬挂缩进 | **不处理嵌套**，无 indent_stack，续行无悬挂缩进 | xylitol `markdown_render.rs:266-271` |
| 代码块留白 | `needs_newline` 精确控制，前后留白一致 | **无条件 push 空行**（前后各一行），段落独立渲染无法感知前文 | xylitol `markdown_render.rs:253, 359` |
| 高亮主题 | 按终端明暗**自适应** catppuccin-latte/mocha + 32 主题 | **写死 CatppuccinMocha**，亮色终端刺眼 | xylitol `syntect_highlight.rs:74-78` |
| 表格 | Unicode 边框 + 列宽自适应收缩 + 窄屏 fallback | 仅空格对齐，无边框 | xylitol `markdown_render.rs:365-404` |

**最显眼的三个**：标题层级塌平、引用块无标识、有序列表丢数字。这三个直接造成「长文档视觉骨架崩塌」。

---

## 二、根因：渲染粒度与 commit 粒度耦合

样式问题修起来不难（补样式表即可），但**留白不一致**这一项不是样式问题，是架构问题。

### 2.1 xylitol 当前模型

```
TextDelta → StreamBuffer.push
         → drain_complete_paragraphs()          [app.rs:71-119]
              fence 外：逐行 commit
              fence 内：累积到 ``` 闭合，整块 commit
         → 每个 commit 单元独立 render_markdown  [render.rs:119-127]
         → insert_before（不可逆，进 scrollback） [terminal.rs:64-74]
```

**关键限制**：commit 单元 = 渲染单元 = 段落。每个段落是**独立**的 `render_markdown` 调用，渲染器**看不到前文**。这就是 `markdown_render.rs:249-253` 那段「无条件加空行」注释的由来——它无法判断前面有没有内容，只能强制 padding，结果就是空行过多或不一致。

### 2.2 codex 的解耦模型

```
delta → MarkdownStreamCollector（新行门控，markdown_stream.rs:87-96）
      → raw_source 累积（append-only）           [controller.rs:76]
      → recompute_streaming_render：对【完整 raw_source】整体 render   [controller.rs:292-294]
      → sync_stable_queue：把新增稳定行切片入 FIFO 队列（粒度=行）     [controller.rs:335-359]
      → 独立的 commit-tick 动画逐行 dequeue → insert_before           [controller.rs:167-171]

tail 区域 = rendered_lines[enqueued..]（可擦除重画，不进 scrollback）
finalize 时：ConsolidateAgentMessage 把临时行替换成单个 source-backed cell（resize 可重渲）
```

**核心差异**：

| 维度 | xylitol | codex |
|---|---|---|
| commit 粒度 | 段落 / 行 | 单 rendered Line |
| **渲染粒度** | **= commit 粒度（段落）** | **整条消息 raw_source** |
| 跨段落上下文 | **丢失**（被迫 hack 空行） | **免费保留**（渲染器永远看全貌） |
| mutable region | **固定 6 行**（mutable 2 + status 1 + panel 3） | **动态**，desired_height 按内容决定，理论无上限 |
| commit 可逆性 | 不可逆 | stable 不可逆 + tail 可重画 + finalize 二次 canonicalize |
| 生产/消费 | 同步（drain 即渲染即 commit） | 解耦（delta→enqueue，tick→dequeue） |

codex 的测试 `controller_loose_vs_tight_with_commit_ticks_matches_full`（controller.rs:1085-1200）直接断言：**流式逐 delta + tick 的输出 == 一次性渲染整条源**。这是 codex 的不变量，xylitol 无法满足。

### 2.3 为什么架构差异会放大样式问题

- **留白**：codex 的 `needs_newline` 机制（markdown_render.rs:510-582, 1702-1710）依赖渲染器看到完整文档结构，才能在每个 block 边界精确 push 一个空行。xylitol 按段落切分，`needs_newline` 无法跨 commit 单元工作，只能退化为「无条件空行」。
- **列表/引用的连续性**：codex 的 indent_stack + prefix_spans（markdown_render.rs:1716-1747）在每行重放前缀，前提是渲染器一次处理整个列表/引用。xylitol 段落切分会打断这种连续性。

---

## 三、最小可行改进路径（按 ROI 排序，非实施计划）

### 第一梯队：纯样式修复（低风险，不改架构）— ✅ 已完成（c396）

这一组改动在现有「按段落渲染」模型下即可完成，能立即改善观感：

1. **标题分级** ✅：`current_style` 按 `HeadingLevel` 返回不同样式（H1 粗+下划线，H2 粗，H3 粗斜，H4-6 斜）；加 `# ` 前缀。
2. **引用块加 `> ` 前缀** ✅：删除 `blockquote_italic_no_pipe` 测试的「no pipe」断言，改成断言「每行有 `> ` 前缀」。保留 quote 样式（灰斜）+ `> ` 前缀，嵌套 `>>` 产生 `> > `。
3. **有序列表数字** ✅：处理 `Tag::List(Some(start))`，按 `start` 递增输出 `{n}. `（light_blue）。
4. **高亮主题自适应** ✅：按 `COLORFGBS` 探测终端背景明暗选 CatppuccinLatte(亮)/CatppuccinMocha(暗)；OSC 11 因 inline viewport 竞态降级为 future。

这一梯队覆盖了「症状对照表」里的 6/7 项，工作量集中在 `markdown_render.rs` 单文件。

### 第二梯队：渲染粒度解耦（中风险，改流式管线）

要让留白和列表连续性达到 codex 水平，必须解耦「渲染粒度」与「commit 粒度」：

5. **StreamBuffer 改为累积 raw_source**：维护 `committed_rendered_line_count`，每次 delta 对**完整累积源**调 `render_markdown`，把差值行（`rendered[committed..]`）送 commit。commit 粒度仍是行（甚至段落），但**渲染粒度升级为整条消息**，跨段落上下文自动恢复。
6. **mutable region 动态高度**：放弃固定 6 行，改成「未 commit 行数 + 状态行 + 输入面板」按需高度。codex 的 `desired_height` 模型可参考，但实现可大幅简化（xylitol 不需要 codex 的动画/finalize canonicalize）。

这一梯队是「从根上消除视觉断裂」的必要改动，但触及 `app.rs` 的 drain 逻辑 + `terminal.rs` 的 viewport 模型，需要单独一个 SDD 变更承载。

### 第三梯队：可选增强（高成本）

7. **stable 区可替换**：借鉴 codex 的 `AgentMarkdownCell`，流式期 commit 临时行，turn 结束后从源整体重渲一次。需要 scrollback 支持「替换」，改动最大，但换来 resize 自适应。
8. **CustomTerminal fork**：完全照搬 codex 的动态 viewport_area。成本最高，除非要支持复杂布局，否则不值。

---

## 四、与 codex 的关键代码索引

### xylitol 限制点
- `src/app/tui/app.rs:71-119` — 段落切分 drain（fence 外逐行 / fence 内整块）
- `src/app/tui/terminal.rs:27` — `TAIL_HEIGHT: u16 = 6` 固定 mutable region
- `src/app/tui/terminal.rs:64-74` — insert_before 不可逆 commit
- `src/app/tui/components/markdown_render.rs:249-253` — 被迫无条件加空行的注释（架构限制的症状）
- `src/app/tui/render.rs:119-127` — 每个 commit 单元独立 render_markdown
- `src/app/tui/components/syntect_highlight.rs:74-78` — 写死 CatppuccinMocha

### codex 参考
- `codex-rs/tui/src/markdown_render.rs:69-105` — 完整 MarkdownStyles 样式表
- `codex-rs/tui/src/markdown_render.rs:510-582, 1702-1710` — needs_newline + push_blank_line 留白机制
- `codex-rs/tui/src/markdown_render.rs:723-784` — 列表（有序/无序/嵌套/悬挂缩进）
- `codex-rs/tui/src/markdown_render.rs:561-582` — 引用块 `> ` 前缀 + 绿色
- `codex-rs/tui/src/streaming/controller.rs:71-90, 127-184, 292-403` — 双区域 + 渲染/commit 解耦
- `codex-rs/tui/src/chatwidget/rendering.rs:6-40, 93-96` — desired_height 动态布局
- `codex-rs/tui/src/custom_terminal.rs:82-97, 309-313` — 动态 viewport_area

---

## 五、给下一个会话的建议

1. **先做第一梯队**（样式修复）：单文件改动，立即改善，可在一个 SDD 变更里完成。建议变更号 c395 的 follow-up 或新建 c396。
2. **第二梯队单独立项**：渲染粒度解耦是「治本」，但要改流式管线 + viewport，风险更高，需要独立的 design.md 论证。
3. **不要边改样式边改架构**：样式问题修完后，如果用户仍觉得留白/连续性有问题，再确认是架构根因，启动第二梯队。
4. **测试策略**：第一梯队用现有 `render_to_buf` + `row_text` 测试框架（markdown_render.rs:424-452）直接断言样式。第二梯队需要类似 codex 的 `controller_loose_vs_tight_with_commit_ticks_matches_full` 不变量测试（流式 == 整体渲染）。

---

## 附：当前代码事实校正

调研中子代理报告与实际代码的出入（已核对）：
- 代码块**前**空行是无条件的（`markdown_render.rs:253` `self.lines.push(Line::raw(""))`，无 `if !self.lines.is_empty()` 守卫）。
- 代码块**后**空行同样无条件（`markdown_render.rs:359`）。
- `finish()` 里 `if self.lines.is_empty()` 守卫只针对「整个渲染结果为空」兜底（markdown_render.rs:408-410），不影响代码块逻辑。
