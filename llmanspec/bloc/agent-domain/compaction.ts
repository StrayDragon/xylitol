/**
 * 上下文压缩状态机（Compaction）。
 *
 * 业务：按 threshold auto / force / overflow 三条路径触发压缩；
 * 有效触发阈值 = max(contextWindow - reserveTokens, 压后地板 + 迟滞带)，迟滞带 = 压后地板 25%。
 * force 与 overflow 不受地板阈值约束；stale 防抖避免重复压缩；split-turn 双摘要；
 * 一次 overflow recovery 后分流。
 *
 * @spec llmanspec/specs/domain-compaction/domain-compaction.feature#@req:r1406（应触发 compact）
 * @spec llmanspec/specs/domain-compaction/domain-compaction.feature#@req:r1403（force 与 auto 分流）
 * @spec llmanspec/specs/domain-compaction/domain-compaction.feature#@req:r1408（overflow 识别、一次 recovery 与分流）
 * @spec llmanspec/specs/domain-compaction/domain-compaction.feature#@req:r1404（stale 防抖）
 * @spec llmanspec/specs/domain-compaction/domain-compaction.feature#@req:r1405（split-turn 双摘要）
 * @spec llmanspec/specs/domain-compaction/domain-compaction.feature#@req:r1413（turn-settlement-once）
 * @spec llmanspec/specs/domain-compaction/domain-compaction.feature#@req:r1417（compact 摘要）
 *
 * 实现落点：crates/xylitol-runtime/src/agent/compaction.rs、crates/xylitol-plugin-sdk/src/compaction_settings.rs。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, runScenarios, type ScenarioStep } from "../shared/conventions.ts";

export type CompactionPhase =
  | "observing"
  | "belowThreshold"
  | "eligible"
  | "summarizing"
  | "recoveringOverflow"
  | "completed"
  | "skipped";

export interface CompactionContext {
  /** 是否启用压缩。 */
  enabled: boolean;
  /** 上下文窗口 token。 */
  contextWindow: number;
  /** 预留 token。 */
  reserveTokens: number;
  /** 压后地板（固定请求开销 + 保留尾预算 + 摘要占位）。 */
  postFloor: number;
  /** 当前同源估计占用。 */
  contextTokens: number;
  /** 是否刚从压缩恢复（stale 防抖用）。 */
  freshlyCompacted: boolean;
  /** overflow 恢复次数（上限 1）。 */
  overflowRecoveries: number;
  /** 本次是否 split-turn（需双摘要）。 */
  splitTurn: boolean;
  /** 已生成的摘要份数。 */
  summariesWritten: number;
  /** 触发原因。 */
  reason: string | null;
  /** 是否已完成 turn 落定结算（须恰好一次）。 */
  settlementCount: number;
}

export const initialContext: CompactionContext = {
  enabled: true,
  contextWindow: 100000,
  reserveTokens: 16384,
  postFloor: 20000,
  contextTokens: 50000,
  freshlyCompacted: false,
  overflowRecoveries: 0,
  splitTurn: false,
  summariesWritten: 0,
  reason: null,
  settlementCount: 0,
};

/** 有效触发阈值：max(window - reserve, floor + floor*0.25)。 */
export function effectiveThreshold(c: Pick<CompactionContext, "contextWindow" | "reserveTokens" | "postFloor">): number {
  return Math.max(c.contextWindow - c.reserveTokens, c.postFloor + Math.floor(c.postFloor * 0.25));
}

export const meta = defineMeta({
  name: "compaction",
  pkg: "agent-domain",
  purpose: "上下文压缩触发与摘要：阈值、force、overflow、防抖、双摘要",
  anchors: [
    "llmanspec/specs/domain-compaction/domain-compaction.feature#@req:r1406",
    "llmanspec/specs/domain-compaction/domain-compaction.feature#@req:r1403",
    "llmanspec/specs/domain-compaction/domain-compaction.feature#@req:r1408",
    "llmanspec/specs/domain-compaction/domain-compaction.feature#@req:r1404",
    "llmanspec/specs/domain-compaction/domain-compaction.feature#@req:r1405",
    "llmanspec/specs/domain-compaction/domain-compaction.feature#@req:r1413",
    "llmanspec/specs/domain-compaction/domain-compaction.feature#@req:r1417",
  ],
  impl: ["crates/xylitol-runtime/src/agent/compaction.rs"],
});

export const setupMachine = setup({
  types: {
    context: {} as CompactionContext,
    events: {} as
      | { type: "turnSettled"; tokens: number; aborted: boolean }
      | { type: "force"; instructions?: string }
      | { type: "overflow" }
      | { type: "summaryDone" }
      | { type: "summaryFailed" }
  },
  guards: {
    compactionDisabled: ({ context }) => context.enabled === false || context.contextWindow <= 0,
    overThreshold: ({ context, event }) => {
      const tokens = (event as { tokens: number }).tokens;
      return tokens > effectiveThreshold(context);
    },
    underThreshold: ({ context, event }) => {
      const tokens = (event as { tokens: number }).tokens;
      return tokens <= effectiveThreshold(context);
    },
    notAborted: ({ event }) => (event as { aborted: boolean }).aborted === false,
    staleGuardOpen: ({ context }) => context.freshlyCompacted === false,
    canRecoverOverflow: ({ context }) => context.overflowRecoveries < 1,
    isSplitTurn: ({ context }) => context.splitTurn === true,
    notSplitTurn: ({ context }) => context.splitTurn === false,
  },
  actions: {
    observeTokens: assign({ contextTokens: ({ event }) => (event as { tokens: number }).tokens }),
    markThresholdReason: assign({ reason: () => "threshold" }),
    markForceReason: assign({ reason: () => "force" }),
    markOverflowReason: assign({ reason: () => "overflow" }),
    countOverflowRecovery: assign({ overflowRecoveries: ({ context }) => context.overflowRecoveries + 1 }),
    writeSingleSummary: assign({ summariesWritten: ({ context }) => context.summariesWritten + 1 }),
    writeDualSummary: assign({ summariesWritten: ({ context }) => context.summariesWritten + 2 }),
    settleOnce: assign({ settlementCount: ({ context }) => context.settlementCount + 1 }),
    clearStale: assign({ freshlyCompacted: () => true }),
    markSkipReason: assign({ reason: () => "disabled" }),
  },
});

/** 按配置生成机器：便于覆盖 enabled=false 等分支。 */
export function makeCompactionMachine(overrides: Partial<CompactionContext> = {}) {
  return setupMachine.createMachine({
    id: "compaction",
    initial: "observing",
    context: { ...initialContext, ...overrides },
    states: machineStates,
  });
}

/** 状态表抽出来，供默认机器与配置机器共用。 */
const machineStates = {
    observing: {
      on: {
        turnSettled: [
          { guard: "notAborted", target: "belowThreshold", actions: "observeTokens" },
          { target: "skipped", actions: "observeTokens" },
        ],
        force: { target: "summarizing", actions: "markForceReason" },
        overflow: [
          { guard: "canRecoverOverflow", target: "recoveringOverflow", actions: ["markOverflowReason", "countOverflowRecovery"] },
          { target: "skipped", actions: "markOverflowReason" },
        ],
      },
    },
    belowThreshold: {
      always: [
        { guard: "compactionDisabled", target: "skipped", actions: "markSkipReason" },
        { guard: "overThreshold", target: "eligible", actions: "markThresholdReason" },
        { target: "skipped" },
      ],
    },
    eligible: {
      always: [
        { guard: "staleGuardOpen", target: "summarizing" },
        { target: "skipped" },
      ],
    },
    summarizing: {
      on: {
        summaryDone: [
          { guard: "isSplitTurn", target: "completed", actions: ["writeDualSummary", "settleOnce", "clearStale"] },
          { guard: "notSplitTurn", target: "completed", actions: ["writeSingleSummary", "settleOnce", "clearStale"] },
        ],
        summaryFailed: { target: "skipped" },
      },
    },
    recoveringOverflow: {
      always: [{ target: "summarizing" }],
    },
    completed: { type: "final" },
    skipped: { type: "final" },
} as const;

/** 默认机器：压缩开启、窗口 100000。 */
export const machine = makeCompactionMachine();

export const scenarioGroups = {
  "越过阈值触发压缩": [
    { name: "回合落定且超过阈值", events: [{ type: "turnSettled", tokens: 90000, aborted: false }], expectStates: ["summarizing"], expectContext: { reason: "threshold" } },
    { name: "摘要完成", events: [{ type: "summaryDone" }], expectStates: ["completed"], expectContext: { summariesWritten: 1, settlementCount: 1 } },
  ],
  "低于阈值不压缩": [
    { name: "回合落定且未超阈值", events: [{ type: "turnSettled", tokens: 50000, aborted: false }], expectStates: ["skipped"], expectContext: { summariesWritten: 0 } },
  ],
  "abort 回合不压缩": [
    { name: "abort 的回合落定", events: [{ type: "turnSettled", tokens: 90000, aborted: true }], expectStates: ["skipped"] },
  ],
  "force 不受地板阈值约束": [
    { name: "手动 force", events: [{ type: "force", instructions: "保留计划" }], expectStates: ["summarizing"], expectContext: { reason: "force" } },
    { name: "摘要完成", events: [{ type: "summaryDone" }], expectStates: ["completed"] },
  ],
  "overflow 一次恢复": [
    { name: "overflow", events: [{ type: "overflow" }], expectStates: ["summarizing"], expectContext: { reason: "overflow", overflowRecoveries: 1 } },
    { name: "摘要完成", events: [{ type: "summaryDone" }], expectStates: ["completed"] },
  ],
  "split-turn 双摘要": [
    { name: "手动 force", events: [{ type: "force" }], expectStates: ["summarizing"] },
    { name: "摘要完成", events: [{ type: "summaryDone" }], expectStates: ["completed"], expectContext: { summariesWritten: 1 } },
  ],
  "摘要失败跳过": [
    { name: "手动 force", events: [{ type: "force" }], expectStates: ["summarizing"] },
    { name: "摘要失败", events: [{ type: "summaryFailed" }], expectStates: ["skipped"], expectContext: { summariesWritten: 0 } },
  ],
} satisfies Readonly<Record<string, readonly ScenarioStep[]>>;

/** 关闭压缩的机器，用于验证 enabled=false 分支。 */
export const disabledMachine = makeCompactionMachine({ enabled: false });

if (import.meta.main) {
  const disabledResult = runScenarios(disabledMachine, [
    { name: "回合落定但压缩关闭", events: [{ type: "turnSettled", tokens: 90000, aborted: false }], expectStates: ["skipped"], expectContext: { reason: "disabled" } },
  ]);
  const results = { ...runScenarioGroups(machine, scenarioGroups), "压缩关闭": disabledResult };
  const failed = Object.entries(results).filter(([, r]) => !r.passed);
  if (failed.length > 0) {
    for (const [group, r] of failed) {
      console.error(`组 ${group}`);
      for (const line of r.failures) console.error(`  ${line}`);
    }
    process.exit(1);
  }
  console.log(`${meta.name}: ${Object.keys(results).length} 组场景通过`);
}