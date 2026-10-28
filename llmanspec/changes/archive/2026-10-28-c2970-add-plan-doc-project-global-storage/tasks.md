# Tasks

- [x] 1. **specs**（spec）：`agent-plan` 增 scope/优先级/文件 SSOT/原子写；`agent-session-store` 增 `plan_ref`；`domain-security` 增项目计划 trust。测：`llman-sdd validate --all --strict`。
- [x] 2. **SDK**（sdk）：新增 `PlanScope` / `PlanRef` / frontmatter 编解码；`plan_ref` CustomEntry 读写。测：路径、解析、往返。
- [x] 3. **文件后端**（plugin）：project/global 计划读写走原子写；session 仍用 CustomEntry + md 视图。测：原子写失败无残片。
- [x] 4. **trust 闸**（plugin）：project scope 未信任时 `PermissionDenied`，不降级；global 不受项目 trust 限制。测：trust 允许/拒绝。
- [x] 5. **工具参数**（plugin）：`plan_write` / `plan_read` / `plan_list` / `plan_submit` 增 `scope`；`plan_list` 跨 scope 合并。测：scope 优先级与列表标签。
- [x] 6. **会话恢复**（plugin）：`plan_ref` 解析活动计划；文件缺失明确报错。测：新会话重开项目计划。
- [x] 7. **BDD**（tests）：项目计划写入、跨会话读取、未信任拒绝。测：`cargo test --all-features --test bdd`。
- [x] 8. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
