# Tasks

- [x] 1. **specs**（spec）：`agent-workflow` 增嵌套模型、递归校验、执行展开、replan/resume 与边界；`agent-session-store` 增嵌套条目与路径 cursor。测：`llman-sdd validate --all --strict`。
- [x] 2. **SDK 模型**（sdk）：`WorkflowStep.sub_workflow`；递归 `validate_workflow`；`max_depth` / `max_total_steps`。测：深度/总数/环/重复 id。
- [x] 3. **中间件展开**（plugin）：容器步骤进入子 workflow；子完成标记父完成；子失败向上 replan；blocked 上浮。测：完成/失败/阻塞传播。
- [x] 4. **恢复**（plugin/sdk）：`resume_cursor` 路径解析与回退；已完成容器不重跑。测：路径恢复与缺失回退。
- [x] 5. **配置**（config）：`workflow.max_depth` / `max_total_steps`；schema/example 同步。测：schema 漂移。
- [x] 6. **BDD**（tests）：嵌套 workflow 完成后父步骤为 completed。测：`cargo test --all-features --test bdd`。
- [x] 7. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
