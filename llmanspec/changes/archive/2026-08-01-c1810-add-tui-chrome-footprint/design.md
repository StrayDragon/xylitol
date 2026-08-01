# Design: c1810 Chrome Footprint

## 调研纪要（原 docs/research · 不进长期 docs）

现象：busy + `/model`/`/theme`/`/session-resume` 等替换 editor 槽时，`Working` 不易见。

**根因**：content-end 视口贴底；高列表把 status 顶出上沿。**非** z-order 遮罩；status 仍在 render 树。

```text
loaded-resources → scrollback → queue → chrome-toast → status → editor|list → footer
viewport_top = max(0, len - term_rows)   # xylitol-tui / pi
```

约行：editor~6 OK；Models/MCP~11 多数 OK；Resume~15 在 **16 行终端易藏 Working**；树更高更险。

## 推荐形态

```text
term_rows
  − Σ lower_chrome.min_rows   # queue? toast? status(busy=2|idle=1) footer(1)
  − slot_header.min_rows      # 列表头/help（非 body）
  ─────────────────────────
  = budget → body.max_visible（≥1）
```

```text
┌─ upper（可滚出）────────────────────────┐
│ loaded-resources · scrollback · queue?  │
├─ lower chrome（manifest 保底）──────────┤
│ toast? · status                         │
│ ┌─ flex slot（预算后的 max_visible）──┐ │
│ │ Models / Resume / Tree / MCP / …    │ │
│ └─────────────────────────────────────┘ │
│ footer                                  │
└─────────────────────────────────────────┘
```

- 新 band = **加表项**，禁止各槽再写死 `10`。
- 包：动态设已有 `max_visible`；**不**为此上引擎 pin。
- 与 scrollback CPU 切片正交。

## API 意向

```text
Host → UiRoot.set_term_rows(n)   # resize / 每帧前
UiRoot::slot_body_budget() -> usize
  = term_rows.saturating_sub(reserved_lower + slot_header)

mount/render 前：
  models_list.max_visible = budget
  session_resume.set_max_visible(budget)
  tree / mcp / themes / import 同理
```

## 非目标

- Bottom-chrome pin / 三区引擎
- Footer 复述 Working 作为主方案
