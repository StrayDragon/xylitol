# Tasks

- [x] 1. 新建 `src/agent/provider/attribution.rs`，定义 `merge_provider_attribution_headers()` 函数
- [x] 2. 实现 OpenRouter attribution（`HTTP-Referer` + `X-OpenRouter-Title` + `X-OpenRouter-Categories`）
- [x] 3. 实现 NVIDIA NIM attribution（`X-BILLING-INVOKE-ORIGIN`）
- [x] 4. 实现 Cloudflare attribution（`User-Agent: pi-coding-agent`）
- [x] 5. 实现 Vercel AI Gateway attribution（`http-referer` + `x-title`）
- [x] 6. 实现 OpenCode session headers（`x-opencode-session` + `x-opencode-client`）
- [x] 7. 定义 `BUILT_IN_PROVIDER_DISPLAY_NAMES` 静态表（27+ 个 provider）
- [x] 8. 实现 `provider_display_name()` 函数
- [x] 9. 集成到 `agent/resolver.rs`：模型请求时注入 attribution headers
- [x] 10. 集成到 CLI/RPC 输出格式化使用显示名称
- [x] 11. 编写单元测试覆盖各 provider 的 header 注入
- [x] 12. `cargo test --lib` 全绿（393 passed）
- [x] 13. `cargo clippy` 无新增警告
