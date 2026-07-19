# Tasks: c1410-add-app-tui-tokenizer-consent

> purpose-draft：下列在升 full / attach 后执行。

## 0. 钉产品

- [ ] 0.1 同意粒度（本机 / 每模型 / 每会话）
- [ ] 0.2 UI 落点（overlay vs Choice 窄解冻）
- [ ] 0.3 触发时机（选模型 / estimate / slash）

## 1. 合约与实现（升 full 后）

- [ ] 1.1 live specs + feature 场景
- [ ] 1.2 Driver / seam 端口（若需）
- [ ] 1.3 TUI pending + effects 委托 download
- [ ] 1.4 harness 同意 / 跳过
- [ ] 1.5 validate → 脏树 finalize → 一次 commit → merge

## 显式不在本 change

- Web M3
- CLI 动词树（c1390）
- HF revision / token / nested list 修补
