# c2922 实施任务

## T1 孤儿场景清理

- [x] 1.1 `test-hooks-wiring.feature`：删 `model-select` / `thinking-select` 场景句柄；r1820 规则正文改为指向 `wiring-observe-matrix` case 4/5
- [x] 1.2 补 `app-tui::surface-local-clipboard-and-host-resources` 绑定（步骤已在 `steps_app_tui_host.rs`，sync fn）
- [x] 1.3 补 `app-tui-transcat::product-gutter-click-jumps` 产品侧步骤壳与绑定（复用包侧 gutter 断言语义）
- [x] 1.4 `packages/xylitol-ai-bridge/.../package-ai-bridge-accounting.feature`：`estimate-fallback-heuristic` 加 `# verified-by` 指主仓绑定（镜像声明，不删）
- [x] 1.5 对账：绑定数 == 场景句柄数（包仓镜像经 verified-by 豁免计数）

## T2 check_transitions 接入

- [x] 2.1 `llmanspec/config.yaml` `specs` 段加 `check_transitions: true`
- [x] 2.2 `llman-sdd validate --specs --strict` 全绿（现有 4 outline 不误伤，已预验）

## T3 server-core r1786 就绪状态机迁移表

- [x] 3.1 r1786 规则下新增 `场景大纲`：from/event/to 三列，覆盖 starting→ready（装配完成）、starting→failed（装配失败）、ready→stopping（优雅停机）、stopping→failed 等转移；既有 3 离散场景（starting-window-503 / ready-flips-healthz / failed-nonretryable）语义并入或保留为观测断言场景，以 E1-E4 全过为准
- [x] 3.2 BDD 侧：迁移表行展开所需 step 复用既有 server 步骤（`服务端在空闲端口上启动`族），不足则补参数化 step；binding sync fn
- [x] 3.3 `cargo test --test bdd` 对账：新增 case 数 == 表行数

## T4 hooks 域过渡态标注

- [x] 4.1 `agent-hooks.feature` 头部注释：声明 r1-r3 为 rutis P1（hook 层替换）重审对象，机制锚定场景（stdin JSON/block-allow/三层合并）冻结新增
- [x] 4.2 change research 目录落一份 hooks↔rutis 事件模型映射备忘（一行指针级，不展开设计）

## T5 门禁与收口

- [x] 5.1 `cargo test --test bdd` 全绿（对账净变化：−2 句柄 +3 绑定 +迁移表 case）
- [x] 5.2 `just fmt` && `just lint`
- [x] 5.3 `llman-sdd validate --all --strict`
- [x] 5.4 tick 全部 checkbox，`llman-sdd change finalize c2922-refactor-bdd-orphans-outline-wave2`
