/**
 * WS 会话通道状态机（WS Session Channel）。
 *
 * 业务：WS /rpc 是产品入口之一，升级后只承载 text 帧内的 JSON-RPC 文本；
 * binary 帧以关闭码 1003 拒绝且不执行业务，超过 4 MiB 的 text 帧以 1009 收口。
 * 订阅以 session_id 与 last_seq 建立，订阅确认以 result 表达确认 seq；
 * 中段重连（last_seq>0 且回合成中）附带 partial 快照，冷订阅（last_seq=0）不带；
 * journal 截断覆盖 last_seq 时发 session/resync_required。
 * 订阅跨回合存活：单次回合 AgentEnd 不结束订阅，后续事件继续送达；
 * 压缩结束事件按载荷完整性三分支处理，不得把丢载荷重建为伪成功完成态。
 *
 * @spec llmanspec/specs/protocol-app/protocol-app.feature#@req:r1709（产品通道：text 帧 JSON-RPC、id 原样回显）
 * @spec llmanspec/specs/protocol-app/protocol-app.feature#@req:r1697（Subscribe 命令：session_id + last_seq，result 含 seq）
 * @spec llmanspec/specs/protocol-app/protocol-app.feature#@req:r1721（CompactionEnd 载荷下行：全载荷保真、旧形态解码缺省）
 * @spec llmanspec/specs/server-core/server-core.feature#@req:r1804（WS 只承载 JSON-RPC 文本：1003/1009 关闭码）
 * @spec llmanspec/specs/server-core/server-core.feature#@req:r1805（WS 升级挂载：缺 Origin 原生客户端可升级）
 * @spec llmanspec/specs/server-core/server-core.feature#@req:r1792（订阅跨回合存活：AgentEnd 不停订阅）
 * @spec llmanspec/specs/server-core/server-core.feature#@req:r1798（重连 journal：last_seq 重放、截断 resync、partial）
 *
 * 实现落点：crates/xylitol-serve/src/occupancy/ws.rs、crates/xylitol-serve/src/occupancy/envelope.rs、
 * crates/xylitol-serve/src/listen.rs。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** binary 帧关闭码：不支持的帧类型（r1804）。 */
export const WS_CLOSE_UNSUPPORTED_DATA = 1003;

/** 超限 text 帧关闭码：超过 4 MiB（r1804）。 */
export const WS_CLOSE_TOO_BIG = 1009;

/** WS 通道上下文：订阅水位、重放计数与关闭事实。 */
export interface WsChannelContext {
  /** 当前订阅会话 id；未订阅为 null。 */
  sessionId: string | null;
  /** 客户端最近一次订阅携带的 last_seq。 */
  lastSeq: number;
  /** 订阅确认 seq（r1697 result 语义）。 */
  confirmedSeq: number | null;
  /** 最近一次订阅从 last_seq+1 重放的遗漏事件数。 */
  replayed: number;
  /** 已发 session/resync_required 次数。 */
  resyncCount: number;
  /** 中段重连附带 partial 快照次数。 */
  partialAttached: number;
  /** 订阅跨过的回合数（AgentEnd 后仍存活）。 */
  survivedTurns: number;
  /** 已受理的上行 text 帧请求数。 */
  uplinkFrames: number;
  /** 已下行的 text 帧数。 */
  downlinkFrames: number;
  /** 最近一次 CompactionEnd 载荷处置结果。 */
  compactionOutcome: "delivered" | "default-decoded" | "rejected" | null;
  /** 关闭码；未关闭为 null。 */
  closeCode: number | null;
  /** 最近一次原样回显的请求 id。 */
  lastEchoedId: string | number | null;
}

export const initialContext: WsChannelContext = {
  sessionId: null,
  lastSeq: 0,
  confirmedSeq: null,
  replayed: 0,
  resyncCount: 0,
  partialAttached: 0,
  survivedTurns: 0,
  uplinkFrames: 0,
  downlinkFrames: 0,
  compactionOutcome: null,
  closeCode: null,
  lastEchoedId: null,
};

export const meta = defineMeta({
  name: "ws-session-channel",
  pkg: "protocol-transport",
  purpose: "WS /rpc 通道：升级、纯 JSON-RPC 文本、订阅跨回合存活、重连 resync",
  anchors: [
    "llmanspec/specs/protocol-app/protocol-app.feature#@req:r1709",
    "llmanspec/specs/protocol-app/protocol-app.feature#@req:r1697",
    "llmanspec/specs/protocol-app/protocol-app.feature#@req:r1721",
    "llmanspec/specs/server-core/server-core.feature#@req:r1804",
    "llmanspec/specs/server-core/server-core.feature#@req:r1805",
    "llmanspec/specs/server-core/server-core.feature#@req:r1792",
    "llmanspec/specs/server-core/server-core.feature#@req:r1798",
  ],
  impl: [
    "crates/xylitol-serve/src/occupancy/ws.rs",
    "crates/xylitol-serve/src/occupancy/envelope.rs",
    "crates/xylitol-serve/src/listen.rs",
  ],
});

export const setupMachine = setup({
  types: {
    context: {} as WsChannelContext,
    events: {} as
      | { type: "upgrade"; originPresent: boolean }
      | { type: "textFrame"; method: string; id?: string | number }
      | { type: "binaryFrame" }
      | { type: "oversizeTextFrame" }
      | {
          type: "subscribe";
          sessionId: string;
          lastSeq: number;
          journalMinSeq: number;
          journalMaxSeq: number;
          turnActive: boolean;
        }
      | { type: "agentEnd" }
      | { type: "connectionLost" }
      | { type: "compactionEndFrame"; payloadComplete: boolean; legacyForm: boolean }
      | { type: "resubscribe"; journalMaxSeq: number },
  },
  guards: {
    journalTruncated: ({ event }) =>
      event.type === "subscribe" && event.lastSeq > 0 && event.lastSeq < event.journalMinSeq,
    payloadComplete: ({ event }) => event.type === "compactionEndFrame" && event.payloadComplete,
    legacyNoPayload: ({ event }) =>
      event.type === "compactionEndFrame" && !event.payloadComplete && event.legacyForm,
  },
  actions: {
    mountSubscription: assign({
      sessionId: ({ event }) => (event.type === "subscribe" ? event.sessionId : null),
      lastSeq: ({ event }) => (event.type === "subscribe" ? event.lastSeq : 0),
      confirmedSeq: ({ event }) => (event.type === "subscribe" ? event.journalMaxSeq : null),
      replayed: ({ event }) =>
        event.type === "subscribe" ? Math.max(0, event.journalMaxSeq - event.lastSeq) : 0,
      partialAttached: ({ context, event }) =>
        context.partialAttached +
        (event.type === "subscribe" && event.turnActive && event.lastSeq > 0 ? 1 : 0),
    }),
    requestResync: assign({
      resyncCount: ({ context }) => context.resyncCount + 1,
      replayed: () => 0,
    }),
    completeFullResub: assign({
      lastSeq: ({ event }) => (event.type === "resubscribe" ? event.journalMaxSeq : 0),
      confirmedSeq: ({ event }) => (event.type === "resubscribe" ? event.journalMaxSeq : null),
      replayed: ({ event }) => (event.type === "resubscribe" ? event.journalMaxSeq : 0),
    }),
    echoUplinkRequest: assign({
      uplinkFrames: ({ context }) => context.uplinkFrames + 1,
      lastEchoedId: ({ event }) =>
        event.type === "textFrame" && event.id !== undefined ? event.id : null,
    }),
    surviveAgentEnd: assign({
      survivedTurns: ({ context }) => context.survivedTurns + 1,
    }),
    deliverCompaction: assign({
      compactionOutcome: () => "delivered" as const,
      downlinkFrames: ({ context }) => context.downlinkFrames + 1,
    }),
    decodeLegacyCompaction: assign({
      compactionOutcome: () => "default-decoded" as const,
      downlinkFrames: ({ context }) => context.downlinkFrames + 1,
    }),
    rejectCompactionPayload: assign({
      compactionOutcome: () => "rejected" as const,
    }),
    markProtocolClose: assign({
      closeCode: () => WS_CLOSE_UNSUPPORTED_DATA,
    }),
    markSizeClose: assign({
      closeCode: () => WS_CLOSE_TOO_BIG,
    }),
  },
});

export const machine = setupMachine.createMachine({
  id: "ws-session-channel",
  initial: "upgrading",
  context: initialContext,
  states: {
    upgrading: {
      on: {
        upgrade: { target: "open" },
      },
    },
    open: {
      on: {
        textFrame: { actions: "echoUplinkRequest" },
        binaryFrame: { target: "protocolClosed", actions: "markProtocolClose" },
        oversizeTextFrame: { target: "sizeClosed", actions: "markSizeClose" },
        subscribe: [
          { guard: "journalTruncated", target: "resyncRequired", actions: "requestResync" },
          { target: "streaming", actions: "mountSubscription" },
        ],
      },
    },
    streaming: {
      on: {
        agentEnd: { actions: "surviveAgentEnd" },
        textFrame: { actions: "echoUplinkRequest" },
        connectionLost: { target: "disconnected" },
        compactionEndFrame: [
          { guard: "payloadComplete", actions: "deliverCompaction" },
          { guard: "legacyNoPayload", actions: "decodeLegacyCompaction" },
          { actions: "rejectCompactionPayload" },
        ],
        binaryFrame: { target: "protocolClosed", actions: "markProtocolClose" },
        oversizeTextFrame: { target: "sizeClosed", actions: "markSizeClose" },
        subscribe: [
          { guard: "journalTruncated", target: "resyncRequired", actions: "requestResync" },
          { target: "streaming", actions: "mountSubscription" },
        ],
      },
    },
    disconnected: {
      on: {
        subscribe: [
          { guard: "journalTruncated", target: "resyncRequired", actions: "requestResync" },
          { target: "streaming", actions: "mountSubscription" },
        ],
      },
    },
    resyncRequired: {
      on: {
        resubscribe: { target: "streaming", actions: "completeFullResub" },
      },
    },
    protocolClosed: { type: "final" },
    sizeClosed: { type: "final" },
  },
});

export const scenarioGroups = {
  "升级订阅并跨回合存活": [
    { name: "升级受理（缺 Origin 亦可）", events: [{ type: "upgrade", originPresent: false }], expectStates: ["open"] },
    { name: "以 last_seq 5 订阅并重放遗漏", events: [{ type: "subscribe", sessionId: "s0", lastSeq: 5, journalMinSeq: 1, journalMaxSeq: 9, turnActive: true }], expectStates: ["streaming"], expectContext: { sessionId: "s0", replayed: 4, confirmedSeq: 9, partialAttached: 1 } },
    { name: "AgentEnd 不结束订阅", events: [{ type: "agentEnd" }], expectStates: ["streaming"], expectContext: { survivedTurns: 1 } },
    { name: "回合结束后事件继续送达", events: [{ type: "agentEnd" }], expectStates: ["streaming"], expectContext: { survivedTurns: 2 } },
    { name: "text 帧请求原样回显 id", events: [{ type: "textFrame", method: "host.describe", id: 7 }], expectStates: ["streaming"], expectContext: { uplinkFrames: 1, lastEchoedId: 7 } },
  ],
  "二进制帧以 1003 拒绝": [
    { name: "升级受理", events: [{ type: "upgrade", originPresent: true }], expectStates: ["open"] },
    { name: "binary 帧触发 1003 关闭且不执行业务", events: [{ type: "binaryFrame" }], expectStates: ["protocolClosed"], expectContext: { closeCode: 1003, uplinkFrames: 0 } },
    { name: "关闭后订阅不再受理", events: [{ type: "subscribe", sessionId: "s0", lastSeq: 0, journalMinSeq: 0, journalMaxSeq: 0, turnActive: false }], expectStates: ["protocolClosed"], expectContext: { sessionId: null } },
  ],
  "超限文本帧以 1009 收口": [
    { name: "升级受理", events: [{ type: "upgrade", originPresent: true }], expectStates: ["open"] },
    { name: "超过 4 MiB 的 text 帧触发 1009", events: [{ type: "oversizeTextFrame" }], expectStates: ["sizeClosed"], expectContext: { closeCode: 1009 } },
  ],
  "journal 截断走 resync": [
    { name: "升级受理", events: [{ type: "upgrade", originPresent: true }], expectStates: ["open"] },
    { name: "last_seq 已被截断覆盖即发 resync", events: [{ type: "subscribe", sessionId: "s0", lastSeq: 2, journalMinSeq: 5, journalMaxSeq: 7, turnActive: false }], expectStates: ["resyncRequired"], expectContext: { resyncCount: 1, replayed: 0 } },
    { name: "全量重订阅后重放至最新水位", events: [{ type: "resubscribe", journalMaxSeq: 7 }], expectStates: ["streaming"], expectContext: { replayed: 7, lastSeq: 7, confirmedSeq: 7 } },
  ],
  "冷订阅不带 partial 与压缩结束三分支": [
    { name: "升级受理", events: [{ type: "upgrade", originPresent: true }], expectStates: ["open"] },
    { name: "冷订阅 last_seq=0 不带 partial", events: [{ type: "subscribe", sessionId: "s0", lastSeq: 0, journalMinSeq: 0, journalMaxSeq: 0, turnActive: false }], expectStates: ["streaming"], expectContext: { replayed: 0, partialAttached: 0 } },
    { name: "全载荷 CompactionEnd 原样下行", events: [{ type: "compactionEndFrame", payloadComplete: true, legacyForm: false }], expectContext: { compactionOutcome: "delivered", downlinkFrames: 1 } },
    { name: "旧无载荷形态解码为缺省载荷", events: [{ type: "compactionEndFrame", payloadComplete: false, legacyForm: true }], expectContext: { compactionOutcome: "default-decoded", downlinkFrames: 2 } },
    { name: "半缺载荷不得重建为伪成功", events: [{ type: "compactionEndFrame", payloadComplete: false, legacyForm: false }], expectContext: { compactionOutcome: "rejected", downlinkFrames: 2 } },
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
