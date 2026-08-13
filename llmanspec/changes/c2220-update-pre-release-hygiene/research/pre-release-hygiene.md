# Pre-0.0.1 卫生：无未发布兼容债

> Change `c2220`。2026-08-14。以本仓代码与 AGENTS 为真源。

## 1. 问题

未发布二进制没有外部 SemVer 客户。为「进行中的 worktree / 旧会话 JSONL / 旧配置键」留双写，会把迭代税变成默认。根 `AGENTS.md` 已有「提交不加 co-author」等卫生，**没有**「0.0.1 前禁止 unpublished 兼容层」。

`src/AGENTS.md` 已禁：为未交付能力预挖空 `Xy*`、「已是统一口再包一层」。`la14` 禁 domain shim。缺口是：**应用面与 TUI 标识符改名时仍想留别名**；`#[allow(dead_code)]` 压预留。

## 2. 本仓已有工具

- Skill `audit-dead-code`：真死 / 逻辑死 / 预留。预留须注释落地条件，六个月无消费降级删除。
- 体量：软 ~1200 / 硬 ~2000 行；**默认不为行数大拆**。
- 实测痛点：`src/app/tui/harness.rs` 6321、`widgets/scrollback.rs` 2321（硬顶附近/以上）、playground HTML 2664。
- `allow(dead_code)`：`app/core/driver/remote.rs`、`packages/xylitol-tui/src/terminal.rs`、infra bash/truncate、测试 support 等。须分诊，不是一律删（remote driver 可能是嵌入/server 预留）。

## 3. 拟写入根 AGENTS 的规则（确认后）

在 **第一个 tagged 0.0.1 之前**：

1. 禁止为未发布的公开 API、YAML 键、slash、UI 字符串保留兼容别名、双解析路径、deprecated 转发。改名改调用点。
2. Session JSONL：未知字段可忽略（serde 已如此）≠ 在产品代码里永久读旧键。需要读旧会话时写 **一次性迁移** 或文档「旧会话不保证」，不要双语义永久并存。
3. 死码按 skill 分诊；禁止新的无理由 `#[allow(dead_code)]`。
4. 优先穷举类型与注册表，避免字符串 magics（与 c2200 工具名表交叉）。
5. 不为满足本条而写「兼容 shim 以便以后再删」。

0.0.1 **之后** 再谈 SemVer / 弃用窗。

## 4. LSP 友好（降低开发困境）

- 标识符可跳转：角色 enum 优于 `"edit" | "write"`。
- 少用宏隐藏类型；theme 闭包保留（已是包契约）。
- 文件过大时 rust-analyzer 变慢：按 **编辑痛点** 拆 harness/scrollback，不按行数 KPI。
- 测试 helper 放 `tests/support`，避免每个文件复制 `Rc<RefCell>`（tt02 意图对，但不要把行号写进 spec）。

## 5. 与 A/B 的边界

- 清 `_ => {}` 与工具名表 → **c2200** 实现，本票只定卫生原则。
- spec 里的防复活清单 → **c2210** 删。
- 本票默认 skip live specs。
