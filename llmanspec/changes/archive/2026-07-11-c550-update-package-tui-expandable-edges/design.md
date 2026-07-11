# Design — c550-update-package-tui-expandable-edges

## Head vs Tail

| 模式 | 可见窗口 | 提示 |
|---|---|---|
| Tail（默认） | 末 N 行 | 上方 earlier + expand_hint |
| Head | 首 N 行 | 下方 more lines + expand_hint |

## 零宽

`render(0)` / 空串：返回空或单空行，禁止 panic。
