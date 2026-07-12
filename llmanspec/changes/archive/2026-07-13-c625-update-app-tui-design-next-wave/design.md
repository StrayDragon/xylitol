# Design notes — c625 next-wave gate

本 change 是**设计闸**：形状已写入应用面 SSOT，实现分属依赖本 change 的 A/B 提案。

## 固定形状入口

| 形状 | DESIGN | playground |
|---|---|---|
| `/models` fuzzy 槽 | `design/models-picker.md` | `?slot=models` |
| 树 filter/fold/fork | `design/session-tree.md` + `keybindings.md` | `?slot=tree-power` |
| 真 `$EDITOR` | `design/bash-mode.md` | （行为以 demo/TTY 为准；静图不模拟 spawn） |
| footer context% | `design/footer.md` | Full shell / Layout 壳 footer 行 |
| abort 反馈 | `design/status.md` / `errors.md` | busy Esc 语义见 Keybindings |

## 明确裁剪

- **不做** Settings / Plate 运行时改 YAML。
- **不做** computer-use（本波）。
- **移除** `/model` 作为产品主路径（由 `/models` 取代）。

## 依赖图（宽松）

```text
c625 (design gate)
├── c626 playground lint L1+L2   ← 防静图↔DESIGN 漂移
├── c630 /models
├── c635 tree filter ──┬── c640 fold
│                      └── c645 fork
├── c650 external editor
└── c655 footer context%

c660 abort process ── c665 abort UI   (并行于轨 A)
```

## 静图一致性：L1 + L2（格式 SSOT）

目标：不靠人眼发现「不完全遵循 DESIGN」。像素主观不进闸；**已知漂移类**进 `just qa`。

### L1 — 机械 lint（`scripts/check_tui_design_playground.py`）

对 `playground/index.html`（及约定片段）失败即红：

| 规则 id | 断言 |
|---|---|
| `no-impl-noise` | 禁止 `c\d{3}`、`demo 已有`、`(包)`、`secondary` 置灰、顶栏快捷键墙 |
| `no-raw-hex` | mock 色只用 CSS var / class，禁止裸 `#rrggbb` |
| `required-slots` | 存在 `panel-models`、`panel-tree-power` 等 Next wave 槽 |
| `term-slot` | Models / Tree power 使用 `term-slot`（禁空撑大 `min-height`） |
| `tree-status-footer` | 树 mock 中 `(i/n)` 出现在选中行之后（底栏） |
| `rev-no-kind-fg` | `.rev` / `row rev` 选中块内不得残留会穿透的 kind 色用法；CSS 须 `.rev * { color: inherit }` |
| `tokens-from` | `design/*.md` 声明 `tokens_from` |

接线：`just check-tui-tokens` 之后跑本脚本；进 `check-scripts` / `qa`。

### L2 — 形状夹具（fixture）

**格式**（放 `src/app/tui/design/fixtures/`，UTF-8）：

```yaml
# session-tree.filter.yaml
id: session-tree.filter
design: ../session-tree.md
slot: tree-power   # playground ?slot=
state: filter      # 对应 ctl / JS state
must_contain:
  - "(2/5) [user]"
  - "› user:"
must_not_contain:
  - "c635"
  - "fg-user"      # 选中行 HTML 片段内
assert_html:
  selected_selector: ".row.rev, span.rev"
  selected_forbids_classes: ["fg-user", "fg-success", "fg-tool"]
  status_after_selected: true   # (i/n) 在 selected 之后
```

Playground 侧：每个受检 mock 根节点带 `data-design-fixture="session-tree.filter"`，lint 加载 YAML 对 **该状态渲染后的 HTML 字符串**（或静态模板源）做断言。

改 DESIGN 形状 → 先改 fixture → lint 红 → 再改 HTML。反向同理。

### 不做（L3 边界）

- 截图 / AI 视觉评分不当主闸。
- 与 Rust `TreeSelector` 逐字符一致：交给 `agent_demo` / 包测；静图只保结构契约。
