# Tasks: c1450-add-config-vars-home

- [x] 1. live `runtime-config`：新增 rc23（`{{ vars.home }}`）；feature:false 场景注明单测覆盖
- [x] 2. `infra/config/template.rs`：注入 `vars.home`（`dirs::home_dir`）；未知 `vars.*` / home 不可解析 → strict 失败；单测
- [x] 3. 更新配置注释 / `configs/example.yaml` / `.xylitol/config.yaml`（lspz 用 `{{ vars.home }}`）
- [x] 4. `cargo test -p xylitol --lib infra::config::template`（或等价）+ `llman sdd validate c1450-add-config-vars-home --strict --no-check`
- [x] 5. `just lint`（或相关闸）绿
