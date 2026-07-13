# Design — c675-update-app-tui-status-spacer

## 对齐

| | demo | 产品（修前） | 产品（目标） |
|---|---|---|---|
| busy | Loader 前导空行 + spinner | strip 空行 → 1 行贴 transcript | keep blank + spinner |
| idle | 1 行 blank | 0 行 | 1 行 blank |

## 实现

`UiRoot::render_status_slot`：

```rust
if !self.status_busy {
    return vec![String::new()]; // idle breathing room
}
self.status_loader.render(width) // keep leading blank from Loader
```

## Non-goals

- c670 abort 停轮
