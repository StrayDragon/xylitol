/**
 * 会话 journal 重放状态机（Journal Replay）。
 *
 * 业务：Host 为每会话维护单调事件 seq 与事件 journal；客户端携带 last_seq 重连时
 * 从 last_seq+1 重放遗漏事实事件；journal 截断覆盖 last_seq 时发
 * session/resync_required。journal 只承载事实事件，流式 tape（TextDelta 等）
 * 不得写入 journal、不得消耗 seq。中段重连（last_seq>0 且回合成中）在订阅应答
 * 携带 partial 快照，冷订阅（last_seq=0）不得携带。空 journal 冷订阅不发 resync；
 * 客户端 last_seq 超过水位时不得让客户端误以为无遗漏，同样走 resync。
 * 稀疏锚点索引损坏时回退线性扫描，重放不产生 false negative；
 * 重放跨压缩边界时以摘要作前缀折叠。
 *
 * @spec llmanspec/specs/server-core/server-core.feature#@req:r1798（重连 journal：last_seq 重放、截断 resync、partial）
 * @spec llmanspec/specs/server-core/server-core.feature#@req:r1806（单调 seq：水位续接，空 journal 不得误判无遗漏）
 * @spec llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1090（追加式 JSONL 日志：每行 entry 含 seq）
 * @spec llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1900（稀疏锚点索引：损坏回退线性扫描）
 * @spec llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1103（投影视图：碰 Compaction 截断、摘要作前缀）
 *
 * 实现落点：crates/xylitol-serve/src/occupancy/ws.rs（EventJournal）、
 * crates/xylitol-runtime/src/driver/in_process/mod.rs（JSONL 会话日志）。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** journal 重放上下文：序号水位、tape 不变量与重放事实。 */
export interface JournalContext {
  /** journal 内最小事件 seq（截断推进下界）。 */
  minSeq: number;
  /** journal 内最大事件 seq（事实事件水位）。 */
  maxSeq: number;
  /** 压缩边界 seq：边界及之前条目以摘要折叠（null 表示无压缩）。 */
  boundarySeq: number | null;
  /** 流式 tape 帧计数（不消耗 seq）。 */
  tapeFrames: number;
  /** tape 是否消耗过 seq（不变量：恒 false）。 */
  tapeConsumedSeq: boolean;
  /** 最近一次订阅重放的事件数。 */
  replayed: number;
  /** 中段重连附带 partial 快照次数。 */
  partialAttached: number;
  /** 已发 session/resync_required 次数。 */
  resyncSent: number;
  /** 稀疏索引损坏后回退线性扫描次数。 */
  indexFallbacks: number;
  /** 以压缩摘要作前缀完成的重放次数（r1103）。 */
  summaryPrefixReplays: number;
}

export const initialContext: JournalContext = {
  minSeq: 0,
  maxSeq: 0,
  boundarySeq: null,
  tapeFrames: 0,
  tapeConsumedSeq: false,
  replayed: 0,
  partialAttached: 0,
  resyncSent: 0,
  indexFallbacks: 0,
  summaryPrefixReplays: 0,
};

export const meta = defineMeta({
  name: "journal-replay",
  pkg: "protocol-transport",
  purpose: "会话 journal 重放：序号水位、partial 补齐、截断 resync、seq 不变量",
  anchors: [
    "llmanspec/specs/server-core/server-core.feature#@req:r1798",
    "llmanspec/specs/server-core/server-core.feature#@req:r1806",
    "llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1090",
    "llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1900",
    "llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1103",
  ],
  impl: [
    "crates/xylitol-serve/src/occupancy/ws.rs",
    "crates/xylitol-runtime/src/driver/in_process/mod.rs",
  ],
});

export const setupMachine = setup({
  types: {
    context: {} as JournalContext,
    events: {} as
      | { type: "appendFacts"; count: number }
      | { type: "tapeFrame" }
      | { type: "truncateOld"; newMinSeq: number }
      | { type: "compactionBoundary" }
      | { type: "indexCorrupted" }
      | { type: "subscribe"; lastSeq: number; turnActive: boolean }
      | { type: "replayCompleted" }
      | { type: "resubscribe"; totalFacts: number },
  },
  guards: {
    journalTruncated: ({ context, event }) =>
      event.type === "subscribe" && event.lastSeq > 0 && event.lastSeq < context.minSeq,
    clientAheadOfWatermark: ({ context, event }) =>
      event.type === "subscribe" && event.lastSeq > context.maxSeq,
  },
  actions: {
    appendFacts: assign({
      maxSeq: ({ context, event }) =>
        context.maxSeq + (event.type === "appendFacts" ? event.count : 0),
    }),
    addTapeFrame: assign({
      tapeFrames: ({ context }) => context.tapeFrames + 1,
      tapeConsumedSeq: () => false,
    }),
    advanceMinSeq: assign({
      minSeq: ({ context, event }) =>
        event.type === "truncateOld" ? event.newMinSeq : context.minSeq,
    }),
    markCompactionBoundary: assign({
      boundarySeq: ({ context }) => context.maxSeq,
    }),
    corruptIndex: assign({
      indexFallbacks: ({ context }) => context.indexFallbacks + 1,
    }),
    computeReplay: assign({
      replayed: ({ context, event }) => {
        if (event.type !== "subscribe") return context.replayed;
        const from = Math.max(event.lastSeq, context.boundarySeq ?? 0);
        return Math.max(0, context.maxSeq - from);
      },
      partialAttached: ({ context, event }) =>
        context.partialAttached +
        (event.type === "subscribe" && event.turnActive && event.lastSeq > 0 ? 1 : 0),
      summaryPrefixReplays: ({ context, event }) =>
        context.summaryPrefixReplays +
        (event.type === "subscribe" && context.boundarySeq !== null && event.lastSeq < context.boundarySeq
          ? 1
          : 0),
    }),
    requestResync: assign({
      resyncSent: ({ context }) => context.resyncSent + 1,
      replayed: () => 0,
    }),
    completeResyncReplay: assign({
      replayed: ({ context, event }) =>
        event.type === "resubscribe" ? event.totalFacts : context.replayed,
    }),
  },
});

export const machine = setupMachine.createMachine({
  id: "journal-replay",
  initial: "collecting",
  context: initialContext,
  states: {
    collecting: {
      on: {
        appendFacts: { actions: "appendFacts" },
        tapeFrame: { actions: "addTapeFrame" },
        truncateOld: { actions: "advanceMinSeq" },
        compactionBoundary: { actions: "markCompactionBoundary" },
        indexCorrupted: { actions: "corruptIndex" },
        subscribe: [
          { guard: "journalTruncated", target: "resyncRequired", actions: "requestResync" },
          { guard: "clientAheadOfWatermark", target: "resyncRequired", actions: "requestResync" },
          { target: "replaying", actions: "computeReplay" },
        ],
      },
    },
    replaying: {
      on: {
        replayCompleted: { target: "collecting" },
        appendFacts: { actions: "appendFacts" },
        tapeFrame: { actions: "addTapeFrame" },
      },
    },
    resyncRequired: {
      on: {
        resubscribe: { target: "replaying", actions: "completeResyncReplay" },
      },
    },
  },
});

export const scenarioGroups = {
  "事实追加与 tape 不占序号": [
    { name: "事实事件逐条追加消耗 seq", events: [{ type: "appendFacts", count: 3 }], expectStates: ["collecting"], expectContext: { maxSeq: 3, minSeq: 0 } },
    { name: "tape 帧不消耗 seq", events: [{ type: "tapeFrame" }, { type: "tapeFrame" }], expectStates: ["collecting"], expectContext: { tapeFrames: 2, tapeConsumedSeq: false, maxSeq: 3 } },
    { name: "中段重连从 last_seq+1 重放并带 partial", events: [{ type: "subscribe", lastSeq: 1, turnActive: true }], expectStates: ["replaying"], expectContext: { replayed: 2, partialAttached: 1, resyncSent: 0 } },
    { name: "重放完成回到采集", events: [{ type: "replayCompleted" }], expectStates: ["collecting"] },
  ],
  "journal 截断与客户端超前发 resync": [
    { name: "追加 3 条事实", events: [{ type: "appendFacts", count: 3 }], expectContext: { maxSeq: 3, minSeq: 0 } },
    { name: "journal wrap 截断推进下界", events: [{ type: "truncateOld", newMinSeq: 3 }], expectContext: { minSeq: 3, maxSeq: 3 } },
    { name: "last_seq 被截断覆盖即发 resync", events: [{ type: "subscribe", lastSeq: 1, turnActive: false }], expectStates: ["resyncRequired"], expectContext: { resyncSent: 1, replayed: 0 } },
    { name: "全量重订阅重放全部事实", events: [{ type: "resubscribe", totalFacts: 3 }], expectStates: ["replaying"], expectContext: { replayed: 3 } },
    { name: "重放完成", events: [{ type: "replayCompleted" }], expectStates: ["collecting"] },
    { name: "客户端 last_seq 超前水位同样 resync", events: [{ type: "subscribe", lastSeq: 9, turnActive: false }], expectStates: ["resyncRequired"], expectContext: { resyncSent: 2 } },
  ],
  "空 journal 冷订阅不发 resync": [
    { name: "空 journal 冷订阅重放零条且不发 resync", events: [{ type: "subscribe", lastSeq: 0, turnActive: false }], expectStates: ["replaying"], expectContext: { replayed: 0, partialAttached: 0, resyncSent: 0 } },
    { name: "冷订阅完成", events: [{ type: "replayCompleted" }], expectStates: ["collecting"] },
    { name: "空 journal 携带更大 last_seq 不得误判无遗漏", events: [{ type: "subscribe", lastSeq: 5, turnActive: false }], expectStates: ["resyncRequired"], expectContext: { resyncSent: 1 } },
  ],
  "索引损坏回退线性扫描": [
    { name: "追加 4 条事实", events: [{ type: "appendFacts", count: 4 }], expectContext: { maxSeq: 4 } },
    { name: "稀疏索引损坏回退线性扫描", events: [{ type: "indexCorrupted" }], expectStates: ["collecting"], expectContext: { indexFallbacks: 1 } },
    { name: "回退扫描不产生 false negative", events: [{ type: "subscribe", lastSeq: 2, turnActive: false }], expectStates: ["replaying"], expectContext: { replayed: 2, resyncSent: 0 } },
    { name: "重放完成", events: [{ type: "replayCompleted" }], expectStates: ["collecting"] },
  ],
  "压缩边界摘要前缀": [
    { name: "压缩前追加 3 条", events: [{ type: "appendFacts", count: 3 }], expectContext: { maxSeq: 3 } },
    { name: "压缩条目标记边界", events: [{ type: "compactionBoundary" }], expectContext: { boundarySeq: 3 } },
    { name: "压缩后追加 2 条", events: [{ type: "appendFacts", count: 2 }], expectContext: { maxSeq: 5, boundarySeq: 3 } },
    { name: "跨边界重放以摘要为前缀", events: [{ type: "subscribe", lastSeq: 1, turnActive: false }], expectStates: ["replaying"], expectContext: { replayed: 2, summaryPrefixReplays: 1, resyncSent: 0 } },
  ],
} satisfies Readonly<Record<string, readonly ScenarioStep[]>>;

if (import.meta.main) {
  const results = runScenarioGroups(machine, scenarioGroups);
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