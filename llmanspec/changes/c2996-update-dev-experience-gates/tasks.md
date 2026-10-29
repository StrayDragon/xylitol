# Tasks

- [ ] 1. **CRAP**（Q1）：`.cargo-crap.toml` 增加 `epsilon = 0.25` 与说明；更新 `qa-gates.md`。测：`cargo crap --help` 配置可读 + CI 下轮观察。
- [ ] 2. **test-changed**（Q2）：新增 `scripts/dev_test_changed.py` 与 `just test-changed`；根级/规格/测试目录回退全量。测：`just test-changed`。
- [ ] 3. **crap-package**（Q3）：新增 `just crap-package <pkg>`；单包 coverage + crap。测：`just crap-package xylitol-tui`。
- [ ] 4. **E2E CI**（Q4）：新增 `.github/workflows/tui-e2e.yaml`（nightly + label + dispatch，非阻塞）；更新 `qa-gates.md`。测：YAML 解析 + `gh workflow view`。
- [ ] 5. **validate 分层**（Q5）：`qa-spec-validate` 改非 strict；`just sdd` 保持 strict；更新 `qa-gates.md`。测：`just qa`。
- [ ] 6. **门禁**（repo）：`llman-sdd validate --strict`、`just qa` 全绿。测：`just qa`。
