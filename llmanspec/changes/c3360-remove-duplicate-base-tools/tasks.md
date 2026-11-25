# Tasks: c3360-remove-duplicate-base-tools

## 1. 删掉重复写入

- [ ] 1.1 在 `install_product_tool_set` 里删掉第二次 `set_base_tools`。`compose_tools` 的结果仍先写入基座，再交给 `try_set_tools_inline`。

## 2. 现有装配测试

seam：`xylitol-kernel` 里已经走过工具装配的测试。不新增 BDD。

- [ ] 2.1 `cargo test -p xylitol-kernel -- composition factory` 保持通过。

## 3. 临时评判（deslop-please）

不进 `just qa`。基线是 `.tmp/deslop-please.allow`。测试路径诊断见 `.tmp/deslop-please-deferred-tests.md`，本 change 不改。

- [ ] 3.1 在本 change 分支上跑 `deslop-please xylitol-kernel`。`XyInProcessDriver::base_tools` 在 `install_product_tool_set` 里只剩一处调用。剩下的那一处 setter 留在基线里。
- [ ] 3.2 `deslop-please xylitol-kernel --allow .tmp/deslop-please.allow` 没有基线以外的新诊断。
