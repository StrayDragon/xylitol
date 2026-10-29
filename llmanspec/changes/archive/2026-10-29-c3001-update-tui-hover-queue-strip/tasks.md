# Tasks

## 1. 队列条命中区

- [x] 1.1 `UiRoot` 记录 `queue_strip_row`，`overlay_hits` 增加 `QUEUE_STRIP_HIT_ID`（整宽、Chrome、capturing）。
- [x] 1.2 队列条 hover 复用同一命中区，不新增背景 token。
- [x] 1.3 单测：非空队列登记命中区；空队列不登记。

## 2. 点击还原

- [x] 2.1 抽取 `HostSession::dequeue_to_editor()`，键盘 Alt+Up 与鼠标点击共用。
- [x] 2.2 `dispatch_chrome_click` 处理 `QUEUE_STRIP_HIT_ID`：还原文本、清队、聚焦 editor。
- [x] 2.3 单测 / BDD：点击后 editor 含排队文本且队列消退。

## 3. 规格与 BDD

- [x] 3.1 `app-tui-input` 新增 `queue-strip-hover-click` 规则与场景。
- [x] 3.2 补 step 与 `#[scenario]` 绑定，`cargo test --all-features --test bdd` 全绿。

## 4. 门禁与收尾

- [x] 4.1 `just qa` 全绿。
- [x] 4.2 `just qa-e2e` 全绿。
- [x] 4.3 `llman-sdd validate c3001-update-tui-hover-queue-strip --check --strict` 通过。
- [x] 4.4 verify 无 CRITICAL，finalize 归档；PR 用 rebase merge。
