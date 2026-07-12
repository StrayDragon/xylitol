# Tasks — c477-update-app-tui-demo-visual-parity

## 1. 空闲操作区

- [ ] 1.1 对照 `just demo-tui` 空闲底部高度；确定产品空草稿目标可见行数（建议 1 内容行 + 上下 `─`）
- [ ] 1.2 实现空草稿紧凑渲染（优先 app 层；若需包 API 再最小改 `Editor`）
- [ ] 1.3 单测：空闲短操作区；多行草稿可长高

## 2. Busy spinner

- [ ] 2.1 status 槽接入包 `Loader`（或等价帧）+ 短词；主题用 accent
- [ ] 2.2 idle 仍 0 行；host tick 驱动动画
- [ ] 2.3 单测：busy 有独立 spinner 行；footer 无 Working/spinner

## 3. User message bg

- [ ] 3.1 `scrollback` 用户行应用 `user-message-bg`（`theme` 暴露 paint 或 bg 闭包）
- [ ] 3.2 单测或快照可观测背景应用

## 4. 校验

- [ ] 4.1 `just qa`
- [ ] 4.2 人工对照：`cargo run`（已 trust）vs `just demo-tui` 空闲/忙碌各一帧
- [ ] 4.3 `llman sdd validate c477-update-app-tui-demo-visual-parity --strict --no-interactive`
