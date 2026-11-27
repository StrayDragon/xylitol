/**
 * 会话存储写入状态机（Session Store Append）。
 *
 * 业务：基于 JSONL 文件的追加日志存储；写路径串行化与 fsync；
 * 格式严格演进、条目驼峰、tool-result 关联 tool-call-id；
 * 稀疏锚点索引与投影视图；CWD 校验与回退；加载时跳过并告警而非整树失败。
 *
 * @spec llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1090（追加日志存储）
 * @spec llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1093（写路径与 fsync）
 * @spec llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1104（格式严格演进）
 * @spec llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1095（日志条目驼峰）
 * @spec llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1098（session-load-skip-warn）
 * @spec llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1094（CWD 校验与回退）
 * @spec llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1109（追加写入安全）
 * @spec llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1900（稀疏锚点索引）
 *
 * 实现落点：crates/xylitol-runtime/src/driver/in_process/session.rs。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

export type StorePhase = "opened" | "appending" | "fsynced" | "loadDegraded" | "formatRejected";

export interface StoreContext {
  /** 已追加条目数。 */
  entries: number;
  /** 已 fsync 次数。 */
  fsyncs: number;
  /** 加载时跳过并告警的条目数。 */
  skippedOnLoad: number;
  /** 是否检测到未知 schema 版本。 */
  unknownSchema: boolean;
  /** CWD 是否与记录一致。 */
  cwdMatches: boolean;
  /** 是否触发 CWD 回退。 */
  cwdFallback: boolean;
  /** 索引中的稀疏锚点数。 */
  anchorCount: number;
  /** 写入是否串行（并发写冲突计数）。 */
  concurrentWriteConflicts: number;
}

export const initialContext: StoreContext = {
  entries: 0,
  fsyncs: 0,
  skippedOnLoad: 0,
  unknownSchema: false,
  cwdMatches: true,
  cwdFallback: false,
  anchorCount: 0,
  concurrentWriteConflicts: 0,
};

export const meta = defineMeta({
  name: "session-store-append",
  pkg: "core-runtime",
  purpose: "会话 JSONL 追加存储：串行写、fsync、跳过告警、CWD 回退",
  anchors: [
    "llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1090",
    "llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1093",
    "llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1104",
    "llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1095",
    "llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1098",
    "llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1094",
    "llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1109",
    "llmanspec/specs/agent-session-store/agent-session-store.feature#@req:r1900",
  ],
  impl: ["crates/xylitol-runtime/src/driver/in_process/session.rs"],
});

export const setupMachine = setup({
  types: {
    context: {} as StoreContext,
    events: {} as
      | { type: "append"; count: number; schemaVersion: number }
      | { type: "fsync" }
      | { type: "load"; skipped: number }
      | { type: "cwdCheck"; matches: boolean }
      | { type: "indexAnchor"; anchors: number }
      | { type: "concurrentWrite" }
  },
  guards: {
    schemaKnown: ({ event }) => (event as { schemaVersion: number }).schemaVersion <= 1,
    schemaUnknown: ({ event }) => (event as { schemaVersion: number }).schemaVersion > 1,
    cwdMismatch: ({ event }) => (event as { matches: boolean }).matches === false,
    cwdMatch: ({ event }) => (event as { matches: boolean }).matches === true,
    loadClean: ({ event }) => (event as { skipped: number }).skipped === 0,
    loadHadSkips: ({ event }) => (event as { skipped: number }).skipped > 0,
  },
  actions: {
    countEntries: assign({ entries: ({ context, event }) => context.entries + (event as { count: number }).count }),
    countFsync: assign({ fsyncs: ({ context }) => context.fsyncs + 1 }),
    recordSkips: assign({ skippedOnLoad: ({ event }) => (event as { skipped: number }).skipped }),
    flagUnknownSchema: assign({ unknownSchema: () => true }),
    enableCwdFallback: assign({ cwdMatches: () => false, cwdFallback: () => true }),
    confirmCwd: assign({ cwdMatches: () => true }),
    addAnchors: assign({ anchorCount: ({ context, event }) => context.anchorCount + (event as { anchors: number }).anchors }),
    countConflict: assign({ concurrentWriteConflicts: ({ context }) => context.concurrentWriteConflicts + 1 }),
  },
});

export const machine = setupMachine.createMachine({
  id: "session-store-append",
  initial: "opened",
  context: initialContext,
  states: {
    opened: {
      on: {
        append: [
          { guard: "schemaKnown", target: "appending", actions: "countEntries" },
          { guard: "schemaUnknown", target: "formatRejected", actions: "flagUnknownSchema" },
        ],
        load: [
          { guard: "loadClean" },
          { guard: "loadHadSkips", target: "loadDegraded", actions: "recordSkips" },
        ],
        cwdCheck: [
          { guard: "cwdMatch", actions: "confirmCwd" },
          { guard: "cwdMismatch", actions: "enableCwdFallback" },
        ],
        indexAnchor: { actions: "addAnchors" },
        concurrentWrite: { actions: "countConflict" },
      },
    },
    appending: {
      on: {
        fsync: { target: "fsynced", actions: "countFsync" },
        concurrentWrite: { actions: "countConflict" },
      },
    },
    fsynced: {
      on: {
        append: [
          { guard: "schemaKnown", target: "appending", actions: "countEntries" },
          { guard: "schemaUnknown", target: "formatRejected", actions: "flagUnknownSchema" },
        ],
        indexAnchor: { actions: "addAnchors" },
      },
    },
    loadDegraded: {
      on: {
        append: { target: "appending", actions: "countEntries" },
      },
    },
    formatRejected: { type: "final" },
  },
});

export const scenarioGroups = {
  "追加并 fsync": [
    { name: "追加两条", events: [{ type: "append", count: 2, schemaVersion: 1 }], expectStates: ["appending"], expectContext: { entries: 2 } },
    { name: "落盘", events: [{ type: "fsync" }], expectStates: ["fsynced"], expectContext: { fsyncs: 1 } },
  ],
  "未知 schema 版本拒绝": [
    { name: "追加高版本条目", events: [{ type: "append", count: 1, schemaVersion: 2 }], expectStates: ["formatRejected"], expectContext: { unknownSchema: true } },
  ],
  "加载跳过并告警": [
    { name: "加载含坏条目", events: [{ type: "load", skipped: 3 }], expectStates: ["loadDegraded"], expectContext: { skippedOnLoad: 3 } },
    { name: "降级后仍可追加", events: [{ type: "append", count: 1, schemaVersion: 1 }], expectStates: ["appending"], expectContext: { entries: 1 } },
  ],
  "CWD 不匹配回退": [
    { name: "CWD 校验失败", events: [{ type: "cwdCheck", matches: false }], expectStates: ["opened"], expectContext: { cwdFallback: true } },
  ],
  "稀疏锚点索引": [
    { name: "登记锚点", events: [{ type: "indexAnchor", anchors: 5 }], expectStates: ["opened"], expectContext: { anchorCount: 5 } },
  ],
  "并发写冲突计数": [
    { name: "并发写", events: [{ type: "concurrentWrite" }], expectStates: ["opened"], expectContext: { concurrentWriteConflicts: 1 } },
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
