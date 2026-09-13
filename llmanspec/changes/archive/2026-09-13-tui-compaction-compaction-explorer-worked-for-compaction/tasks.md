# Tasks — compaction 切簇独立成块

## 测试 seam（复用既有 harness，不新发明）

- `segment.rs` 模块单测（partition 纯函数）。
- BDD `@executable`：`render_plain`（SceneBuilder → 80 宽纯文本帧）+ `TranscriptBdd.frames`，
  绑定经 `tests/bdd/bindings_app_tui_transcript.rs` 注册。

## Tasks

### T1 specs landing：att34 修订

- [x] `llmanspec/specs/app-tui-transcript/app-tui-transcript.feature` att34：
  「Thinking、工具、Ask、Diff、Todo、Compaction MUST NOT 单独切簇」改为
  「Thinking、工具、Ask、Diff、Todo MUST NOT 单独切簇；Compaction MUST 封口当前打开簇并
  自成单例簇独立成块，其后中间活动 MUST 另起新簇」。
- [x] 新增 `@req:att34 @executable` 场景 `compaction-seals-cluster-headless`。
- [x] 校验：`llman sdd validate <id> --strict`。

### T2 SceneBuilder compaction 构造 + BDD 步骤/绑定

`blocked-by: T1`

- [x] `activity_fold/scene.rs`：`pub fn compaction(&mut self, summary: &str, tokens_before: u64)`
  （Start 占位 + End 完成的产品投影路径）。
- [x] `steps_app_tui_transcript.rs`：when「以场景构建器回放工具后压缩再工具序列」+
  then「压缩块独立成块且前后工具簇各自成簇」+ then「全帧不出现 Worked for 与 Compaction 簇摘要头」。
- [x] `bindings_app_tui_transcript.rs`：注册场景 `test_att34_compaction_seals`。

### T3 partition 切簇 + 单测

`blocked-by: T1`

- [x] `segment.rs`：match 循环增 `UiEntry::Compaction` 封口分支（先 seal_open(None)、push、再 seal_open(None)）。
- [x] 单测三形态：tools→compaction→tools（三簇）；连续两条 compaction（两单例簇）；
  纯 compaction 轮（与现状同构，`partition_includes_compaction_skips_empty_and_scrollnotice` 保持绿）。

### T4 门禁与校准

`blocked-by: T2, T3`

- [x] `just fmt` / `just lint` / `cargo test --lib`（1917 passed，含 att23/att26/att29/att35/att36 回归）。
- [x] BDD 步骤断言按实际帧校准（第二簇头为打开词形 Exploring 1 file）；`llman sdd review` 通过。
