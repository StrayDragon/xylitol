# language: zh-CN
# capability: app-tui
# purpose: 跨切面不变量索引。产品细节见 app-tui-*；引擎/harness 见 package-tui-*；CLI 默认进 TUI 见 cli-entry。
# scope: src/, tests/

功能: app-tui

  @req:r1164
  规则: three-surfaces-coexist
    print（cli）、server（HTTP 监听器）与 tui（交互面）应用面 MUST 共存；不得以统一为名删除任一传输面。
    # verified-by: src/app/cli/mod.rs

    场景: three-app-surfaces-in-default-feature-set
      当 读取 Cargo 特性表
      那么 默认集为 cli 与 tui 与 otel 与 server
      并且 可选能力有域前缀 flag 且内置能力无条件
  @req:r1165
  规则: reuse-contract
    产品 TUI MUST 经协议契约、应用缝与 agent 公共入口导入，MUST NOT reach agent 子模块内部或 infra；MUST NOT 依赖已删除的独立 domain 顶栏。斜杠语义 MUST 复用 protocol::Command。
    # verified-by: src/AGENTS.md

    场景: tui-imports-via-public-contract-only
      当 读取库公开入口的重导出清单
      那么 清单覆盖 Xy 核心契约类型
  @req:r1166
  规则: no-skeleton-spray
    产品 TUI 面下每个新增源文件 MUST 在同一变更内由真实入口驱动；MUST NOT 用 #[allow(dead_code)] 落地未驱动骨架。死码分诊规则由架构 AGENTS 与 l8ng-audit-dead-code skill 承载。
    # verified-by: src/AGENTS.md

    场景: no-undriven-skeleton-files
      当 读取 crate 根的死代码允许写法
      那么 无全局 allow 且单项抑制带理由
  @req:r1163
  规则: capability-index
    产品 TUI 行为细节 MUST 由 app-tui-host / app-tui-bridge / app-tui-transcript / app-tui-fixed-zone / app-tui-input / app-tui-commands 各自约束；引擎与四层 harness MUST 由 package-tui-*（含 package-tui-testing）约束，不得在本索引重复实现史细节。
    # verified-by: llmanspec/specs/app-tui/app-tui.feature

    场景: index-points-to-per-capability-rules
      当 读取 tui 面 AGENTS 摘要
      那么 摘要写明先读产品代码与默认忽略应用壳
      并且 摘要写明改稿须跑 designing lint
  @req:r1162
  规则: TUI 只承担面本地
    产品 TUI MUST 只执行面本地能力（键盘、绘制、TTY、本机编辑器、剪贴板）。依赖工作区、模型、MCP 或会话生命周期的能力 MUST 由 host 角色执行。载体切分与同进程保留见 app-tui-bridge atb4。MUST NOT 把剪贴板等面本地能力交给远程 host 写入本机盘。

    场景: surface-local-clipboard-and-host-resources
      当 以主机泵注入剪贴板图片后按粘贴键
      那么 编辑器插入落盘路径文本且路径经 Driver 暂存
      当 以主机泵注入含连接态的资源快照后提交 "/mcp"
      那么 MCP 面板槽打开且列出连接态且未因开面板 abort agent
# re-review(c2826): 复审结论——本 capability 管辖行为不变；分支内改动仅测试基建与可见性再导出（2026-09-28）

# re-review(c2827): 复审结论——本 capability 管辖行为不变；分支内改动为 BDD 场景落地、BDD 测试基建（steps/bindings/驱动旋钮与探针）与可见性再导出（2026-09-28）
    # verified-by: docs/architecture/信任与项目门禁.md
# re-review(c2835): 复审结论——本 capability 管辖行为不变；v3 应答 union 与手写 JSON-RPC 分发均经对拍等价（同一 dispatch、同一组 Serialize），端上可见词表未动。（2026-09-29）

# re-review(c2837): c2837 编译隔离变更影响本 scope——agent/infra 公开化与 BDD 测试辅助面收敛（纯可见性扩张与测试基建，无行为变化）。场景映射不变量保持；已复核。（2026-10-06）

# re-review(c2838): c2838 intra-doc 链接治理触及本 scope 内源码 doc 注释（纯文档、无行为变化）。场景映射不变量保持；已复核。（2026-10-06）

# re-review(c2837): flaky-fix 分支复核——测试时序放宽与诊断增强触及本 scope；行为不变。（2026-10-06）
# re-review(c2853): 承载分支 sdd/2026-10-review-fixes 触及本 scope（BDD 步骤卫生 / 既有 codec·host 改动）；本 capability 管辖行为不变。场景映射不变量保持。（2026-10-09）
