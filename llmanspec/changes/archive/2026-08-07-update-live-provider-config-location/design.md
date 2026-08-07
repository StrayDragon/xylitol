---
depends_on: []
---

# 设计：live-provider 配置迁移到全局共享目录

## 决策 1：位置 `dev/live-provider.yaml`（而非直接并入 `config.yaml`）

qg07 原「MUST NOT 默认合并 `~/.config/xylitol`」的意图是**测试条件固定**：live-provider 是 prompt-cache 反例验证，model/base_url 必须由专用文件钉死，不受用户全局 AppConfig 漂移影响。

迁移后配置仍保持「专用文件」语义，只是位置挪到共享目录的 `dev/` 子目录：

- 与全局 AppConfig `config.yaml` 物理隔离——测试解析器**只读** `dev/live-provider.yaml`，绝不读 `config.yaml` 的 models/agents 段；
- 享受 dotxylitol 多机同步，新机器 clone 即开箱；
- `dev/` 命名暗示「开发向测试闸配置」，与产品运行时配置分层清晰。

## 决策 2：global_dir 解析与主 crate 同源

测试二进制位于 `packages/xylitol-ai-bridge`（不依赖主 crate），自行实现与 `src/infra/config/paths.rs::resolve_global_dir` 相同的优先级：

1. `XYLITOL_CONFIG_DIR`（非空）
2. `$XDG_CONFIG_HOME/xylitol`
3. `~/.config/xylitol`（`dirs::home_dir` 兜底；无 home 则放弃 → skip）

避免同一用户在两处配置目录时行为分裂。

## 决策 3：env escape hatch 保留

`XYLITOL_LIVE_PROVIDER_CONFIG` 仍是最高优先级（显式路径）；`XYLITOL_LIVE_PROVIDER` / `XYLITOL_LIVE_BASE_URL` 等字段级覆盖不变。CI 或临时切换网关不需要动共享文件。

## 决策 4：example 自动生成

仓库只留 example，且 example 由维护脚本生成（不进 qa，符合 scripts/ 维护脚本惯例）：

- `scripts/gen_live_provider_example.py`：内嵌与 `LiveProviderFile` 字段对齐的 YAML 模板（enabled/base_url/model/api_key/max_output_tokens/serial + 注释说明 copy 目标与 env 覆盖），生成/覆盖 `configs/testing/live-provider.example.yaml`，并自校验产物可被 yaml 解析；
- `just gen-live-provider-example` 接线；
- 字段演进时改脚本模板一处，example 不漂移。

## 决策 5：删除与提示

- 本机 `configs/testing/live-provider.local.yaml` 删除；`.gitignore` 的 `configs/testing/live-provider.local.yaml` 条目删除（`*.local` 通配保留）；
- SKIP 提示文案改为：copy 全局 example → 全局路径，或运行 `just gen-live-provider-example` 生成仓库示例后自行放置；
- 文档指针（AGENTS.md / research doc / justfile 注释）同步新路径。
