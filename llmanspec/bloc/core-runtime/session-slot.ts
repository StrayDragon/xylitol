/**
 * 会话槽生命周期状态机（Session Slot Lifecycle）。
 *
 * 业务：一个 session 槽在 Host 内被占用、宽限、驱逐，并在重建时续接事件序号水位。
 *
 * @spec llmanspec/specs/server-core/server-core.feature#@req:r1793（session 写占用）
 * @spec llmanspec/specs/server-core/server-core.feature#@req:r1798（重连 journal）
 * @spec llmanspec/specs/layer-architecture/layer-architecture.feature#@req:r1514（一 session 一写者）
 * @spec llmanspec/specs/server-core/server-core.feature#@req:r1792（订阅跨回合存活）
 *
 * 实现落点：crates/xylitol-serve/src/occupancy/host.rs、crates/xylitol-serve/src/occupancy/mod.rs。
 *
 * 建模说明：evicted 不是终态。驱逐后同会话可重建（r1798 要求水位续接），
 * 因此保留出边 rebuild → occupied；终态语义由 journal 水位不变保证。
 * 架构真源：src/AGENTS.md「会话槽生命周期」。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** 槽状态：占用 → 宽限 → 驱逐。 */
export type SlotState = "occupied" | "grace" | "evicted";

/** 槽内事实。 */
export interface SlotContext {
  /** 会话 id。 */
  sessionId: string;
  /** 当前订阅者数量。 */
  subscribers: number;
  /** 已下发事件序号水位；驱逐后保留，重建时续接。 */
  seqWatermark: number;
  /** 是否由本次重建续接了旧水位。 */
  resumedFromWatermark: boolean;
  /** journal 是否为空；空 journal 不发 resync。 */
  journalEmpty: boolean;
  /** 最近一次驱逐原因。 */
  evictReason: string | null;
}

export const initialContext: SlotContext = {
  sessionId: "s1",
  subscribers: 1,
  seqWatermark: 42,
  resumedFromWatermark: false,
  journalEmpty: false,
  evictReason: null,
};

export const meta = defineMeta({
  name: "session-slot-lifecycle",
  pkg: "core-runtime",
  purpose: "会话槽的占用、宽限与驱逐，含序号水位续接",
  anchors: [
    "llmanspec/specs/server-core/server-core.feature#@req:r1793",
    "llmanspec/specs/server-core/server-core.feature#@req:r1798",
    "llmanspec/specs/layer-architecture/layer-architecture.feature#@req:r1514",
    "llmanspec/specs/server-core/server-core.feature#@req:r1792",
  ],
  impl: ["crates/xylitol-serve/src/occupancy/host.rs", "crates/xylitol-serve/src/occupancy/mod.rs"],
});

export const setupMachine = setup({
  types: {
    context: {} as SlotContext,
    events: {} as
      | { type: "subscribe" }
      | { type: "unsubscribe"; clean: boolean }
      | { type: "graceExpired" }
      | { type: "rebuild"; journalEmpty: boolean }
      | { type: "appendEvent" }
  },
  actions: {
    addSubscriber: assign({
      subscribers: ({ context }) => context.subscribers + 1,
    }),
    removeSubscriber: assign({
      subscribers: ({ context }) => Math.max(0, context.subscribers - 1),
    }),
    markEvicted: assign({
      evictReason: ({ context, event }) =>
        (event as { clean?: boolean }).clean === true ? "clean-close" : "grace-expired",
    }),
    resumeWatermark: assign({
      resumedFromWatermark: () => true,
    }),
    bumpSeq: assign({
      seqWatermark: ({ context }) => context.seqWatermark + 1,
    }),
  },
  guards: {
    lastSubscriberGoneCleanly: ({ context, event }) =>
      context.subscribers <= 1 && (event as { clean: boolean }).clean === true,
    lastSubscriberGoneDirty: ({ context, event }) =>
      context.subscribers <= 1 && (event as { clean: boolean }).clean === false,
  },
});

export const machine = setupMachine.createMachine({
  id: "session-slot",
  initial: "occupied",
  context: initialContext,
  states: {
    occupied: {
      on: {
        subscribe: { actions: "addSubscriber" },
        unsubscribe: [
          { guard: "lastSubscriberGoneCleanly", target: "evicted", actions: ["removeSubscriber", "markEvicted"] },
          { guard: "lastSubscriberGoneDirty", target: "grace", actions: "removeSubscriber" },
          { actions: "removeSubscriber" },
        ],
        appendEvent: { actions: "bumpSeq" },
      },
    },
    grace: {
      on: {
        subscribe: { target: "occupied", actions: ["addSubscriber"] },
        graceExpired: { target: "evicted", actions: "markEvicted" },
      },
    },
    evicted: {
      on: {
        rebuild: { target: "occupied", actions: ["resumeWatermark", "addSubscriber"] },
      },
    },
  },
});

/** 场景：覆盖占用、干净驱逐、脏断开宽限与水位续接。 */
export const scenarios: readonly ScenarioStep[] = [
  {
    name: "初始占用",
    expectStates: ["occupied"],
    expectContext: { subscribers: 1, seqWatermark: 42 },
  },
  {
    name: "干净断开立即驱逐",
    events: [{ type: "unsubscribe", clean: true }],
    expectStates: ["evicted"],
    expectContext: { subscribers: 0, evictReason: "clean-close", seqWatermark: 42 },
  },
  {
    name: "重建续接水位",
    events: [{ type: "rebuild", journalEmpty: false }],
    expectStates: ["occupied"],
    expectContext: { resumedFromWatermark: true, seqWatermark: 42 },
  },
];

/** 场景：脏断开走宽限，宽限内重连回占用，超时驱逐。 */
export const scenariosGrace: readonly ScenarioStep[] = [
  {
    name: "脏断开进宽限",
    events: [{ type: "unsubscribe", clean: false }],
    expectStates: ["grace"],
  },
  {
    name: "宽限内重连回占用",
    events: [{ type: "subscribe" }],
    expectStates: ["occupied"],
    expectContext: { subscribers: 1 },
  },
];

/** 场景：宽限到期驱逐且水位不重置。 */
export const scenariosGraceExpired: readonly ScenarioStep[] = [
  {
    name: "脏断开进宽限",
    events: [{ type: "unsubscribe", clean: false }],
    expectStates: ["grace"],
  },
  {
    name: "宽限到期驱逐",
    events: [{ type: "graceExpired" }],
    expectStates: ["evicted"],
    expectContext: { evictReason: "grace-expired", seqWatermark: 42 },
  },
];

/** 全部场景分组：每组从初始状态独立起跑。 */
export const scenarioGroups = {
  "干净驱逐与水位续接": scenarios,
  "脏断开宽限内重连": scenariosGrace,
  "宽限到期驱逐": scenariosGraceExpired,
} as const;

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
