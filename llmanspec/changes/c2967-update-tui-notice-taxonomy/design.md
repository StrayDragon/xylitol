# Design

## Taxonomy

| Surface | Purpose | Examples | Must not |
|---|---|---|---|
| Toast notice | Blocking rejection or immediate failure that needs action now | `reloading — wait`, busy Resume switch/rename/delete refusal, link down/recovered | Carry durable outcomes; enter `UiModel.entries`; persist after TTL |
| Client notice stack | Immediate confirmation | `Copied`, `Copied Last Msg` | Explanations, heuristic provenance, navigation events |
| Scroll notice | Durable navigation/outcome/error | renamed/deleted session, import/export result, fork result, model/thinking failure, compaction/retry detail | Replace conversation content; be used for a momentary gate |
| System message | Durable information that must enter agent context | policy/mode changes that affect the next turn | Be used as a visual-only notice |

## Heuristic notice removal

Delete:

- `HEURISTIC_ESTIMATE_NOTICE` constant.
- `heuristic_notice_emitted` field.
- `UiRoot::maybe_heuristic_estimate_notice`.
- `HostSession::maybe_heuristic_estimate_notice`.
- Call sites in `effects/mod.rs` and `host/mod.rs`.
- Tests that assert the heuristic notice appears once.

The footer continues to render `used ~C tokens` and `~p%/W` for `TokenProvenance::Heuristic`.

## Call-site audit

- `RELOADING_WAIT_NOTICE`, `LINK_DOWN_NOTICE`, `LINK_RECOVERED_NOTICE`, `BUSY_SESSION_SWITCH_NOTICE` stay toast.
- Session rename/delete, import/export, fork failures and model/thinking failures stay scroll notices.
- Any `push_toast_notice` that reports a completed durable action moves to `push_scroll_notice`.

## Specs

- `app-tui-fixed-zone`:
  - `r1227` toast rule: state allowed categories and forbid durable outcomes.
  - `r1930` client stack: remove heuristic explanation; keep Copied confirmations.
  - New `notice-taxonomy` rule: one sentence per surface plus the selection rule.
- `app-tui-transcript`: durable navigation/outcome notices remain tail-inserted scroll notices.

## Tests

- Remove heuristic-notice unit/harness tests.
- Add a classification test: durable outcome → scroll notice; busy refusal → toast; copy → client stack.
- Assert footer still carries `~` for heuristic provenance.
- `just qa` and `llman-sdd validate --all --strict`.

## Task breakdown

- 1. **specs**（spec）：`app-tui-fixed-zone` 改写 `r1227` / `r1930`，新增 `notice-taxonomy`；`app-tui-transcript` 明确耐久结果走滚动提示。测：`llman-sdd validate --all --strict`。
- 2. **删除启发式通知**（app-tui）：移除 `HEURISTIC_ESTIMATE_NOTICE`、`heuristic_notice_emitted`、`maybe_heuristic_estimate_notice` 及调用点与相关测试。测：footer 仍含 `~` 且不再出现该通知。
- 3. **调用点审计**（app-tui）：按分类把耐久结果从 toast 移到 scroll notice；保留 reload 软闸、忙态拒闸、链路状态为 toast。测：分类单测。
- 4. **文档**（docs）：DESIGN.md / 端 AGENTS 记录通知分类。测：`just qa-doc-check`。
- 5. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
