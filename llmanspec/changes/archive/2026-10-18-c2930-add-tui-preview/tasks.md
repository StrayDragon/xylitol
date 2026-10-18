# c2930 实施任务

Seam：CLI 子命令解析（debug 构建 `xylitol tui designing` / `--help`）；产品 slash 解析；预览组合主机泵（场景槽 + 注入）；既有 session-tree 夹具场景改绑预览注入。不新增脱离 `.feature` 的平行 harness。

- [x] 1. `app-tui-preview.feature`：规则覆盖 tui designing（debug 叶）、场景槽、隔离注入、不进信封
- [x] 2. `cli-entry.feature`：表面动词仅 tui / print；tui designing 不适用 attach 未在听失败
- [x] 3. `app-tui-commands.feature`：废止 `/debug` 解析与补全；场景改走预览
- [x] 4. `app-tui-session-tree.feature`：夹具装载步骤改为预览注入（不再提交 `/debug …`）
- [x] 5. CLI + 组合根：debug 构建 `xylitol tui designing` 打开预览组合（假 Host，不 attach）；release 不列出该叶；产品 TUI 路径不变
- [x] 6. 场景槽：列出 id/描述；选中注入画布；未知场景短错误且不崩溃
- [x] 7. 删除产品 slash `/debug` 全路径（解析、补全、busy 策略、effects）
- [x] 8. BDD / harness / DESIGN：步骤与文案改预览词；`cargo test --test bdd` 相关场景绿
- [x] 9. `just qa`（本 change 触及 TUI 手测路径时再加 `just qa-e2e`）
