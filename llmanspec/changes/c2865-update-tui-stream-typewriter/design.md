# 设计：流式半截闭合围栏

## 1. 决策表

| # | 决策 | 理由 |
|---|---|---|
| D1 | 修剪放引擎 `Markdown::render`（parse **前**剥源字符串） | pulldown-cmark 事件流不能像 pi `marked` 那样改 token；pi-oxidized 也是剥原文。产品 / `agent_demo` 共用同一把尺子 |
| D2 | 只剥 **未闭合围栏** 末行上、长度短于 opener 的同字符 `` ` `` / `~` 前缀 | 对齐 pi `trimPartialClosingFences`：半截 `` ` `` / `` `` `` 不当代码行画出来，补全 ``` 时高度不收缩 |
| D3 | 本票 **不做** thinking 尾只 wrap 最后视觉行 | thinking 是 plain wrap，无围栏；另开抖动源，塞进来会糊切片 |
| D4 | **不**扩 `find_stable_markdown_prefix_end` 到列表/表边界 | r1255 已禁止切开未闭合围栏；列表长高是另一类问题，不本票 |
| D5 | 不引入 `StyledLine` / 降帧率 / 下沉 `UiModel` 流缓冲 | D12；CPU% 仍 park 在 delayed c1535 |
| D6 | mermaid 仍默认 `final`（c2890）；本票不画半图 | 流式围栏稳定与 ASCII 图解耦 |

## 2. 抖动机制

流式补全 ``` 时：末行 `` 先被当成代码内容，第三颗 ` 变成闭合围栏后该行消失（r1645 不画围栏铬）→ 可见高度 −1 → 跟底 vis 跳。

```
```rust          ```rust
print(1)   →     print(1)
``               ```          ← 高度不应变
```

产品前缀缓存只在围栏外 `\n\n` 处切开，**救不了**围栏内部这一行。

## 3. 算法（源字符串）

扫描行首 ≥3 的 `` ` `` / `~` 围栏，记下 opener 字符与长度、是否仍打开。若 EOF 时围栏开着，且最后一行全是该字符、长度严格小于 opener，则去掉该行（及前面的换行）。已闭合、或末行是正经代码，不动。

## 4. spec 映射

| @req | 改什么 |
|---|---|
| 新 r1936（`package-tui-markdown`） | 半截闭合围栏修剪；补全后可见行数不变 |
| 新 r1937（`app-tui-host`） | 流式助手走同一把 Markdown 尺子；半截 ``` 序列高度不跳 |
| r1255 | **不改句**（仍是前缀复用 / 全量解析上界） |
| r1645 | **不改句**（仍禁止画出围栏行） |

## 5. 测试缝

- 包：`packages/xylitol-tui/src/components/markdown.rs` 既有 `Markdown::render` 快照式单测（半截 `` ` `` / `` `` `` / 完整 ``` / `~~~`）。
- 产品：`src/app/tui/tests.rs` 已有 `StreamingAssistantPaint` / `streaming_assistant_reuses_stable_prefix_under_deltas`；加围栏序列高度断言。
- BDD：新场景绑 `tests/bdd`（包 capability 同 c2858 的 `bindings_package_tui_*`；产品走既有 host 交互面）。不新开 harness crate。
