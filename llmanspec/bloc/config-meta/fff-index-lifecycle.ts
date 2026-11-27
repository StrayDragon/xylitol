/**
 * fff 检索索引状态机（fff Index Lifecycle）。
 *
 * 业务：默认装配 ffgrep / fffind，按工作区惰性建索引；插件缺失时回落内置检索；
 * 所有模式都支持取消。
 *
 * @spec llmanspec/specs/infra-fff-search/infra-fff-search.feature#@req:r2979（fff-tool-surface）
 * @spec llmanspec/specs/infra-fff-search/infra-fff-search.feature#@req:r2980（fff-index-lifecycle）
 * @spec llmanspec/specs/infra-fff-search/infra-fff-search.feature#@req:r2981（fff-fallback）
 * @spec llmanspec/specs/infra-fff-search/infra-fff-search.feature#@req:r3350（fff-cancel-all-modes）
 * @spec llmanspec/specs/agent-tools/agent-tools.feature#@req:r1158（grep ripgrep）
 *
 * 实现落点：plugins/xylitol-official-plugin-tools-fff/。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

export type FffPhase = "unindexed" | "building" | "ready" | "searching" | "fallback" | "cancelled";

export interface FffContext {
  /** 插件二进制是否可用。 */
  pluginAvailable: boolean;
  /** 已建索引的工作区数。 */
  indexedWorkspaces: number;
  /** 已服务检索次数。 */
  servedSearches: number;
  /** 回落内置检索次数。 */
  fallbacks: number;
  /** 已取消次数。 */
  cancels: number;
  /** 当前检索模式。 */
  mode: "grep" | "find" | null;
}

export const initialContext: FffContext = {
  pluginAvailable: true,
  indexedWorkspaces: 0,
  servedSearches: 0,
  fallbacks: 0,
  cancels: 0,
  mode: null,
};

export const meta = defineMeta({
  name: "fff-index-lifecycle",
  pkg: "security-infra",
  purpose: "fff 检索：惰性建索引、回落内置、全模式可取消",
  anchors: [
    "llmanspec/specs/infra-fff-search/infra-fff-search.feature#@req:r2979",
    "llmanspec/specs/infra-fff-search/infra-fff-search.feature#@req:r2980",
    "llmanspec/specs/infra-fff-search/infra-fff-search.feature#@req:r2981",
    "llmanspec/specs/infra-fff-search/infra-fff-search.feature#@req:r3350",
    "llmanspec/specs/agent-tools/agent-tools.feature#@req:r1158",
  ],
  impl: ["plugins/xylitol-official-plugin-tools-fff/"],
});

export const setupMachine = setup({
  types: {
    context: {} as FffContext,
    events: {} as
      | { type: "search"; mode: "grep" | "find" }
      | { type: "indexBuilt" }
      | { type: "searchDone" }
      | { type: "pluginMissing" }
      | { type: "cancel" }
  },
  guards: {
    pluginUsable: ({ context }) => context.pluginAvailable === true,
    pluginAbsent: ({ context }) => context.pluginAvailable === false,
  },
  actions: {
    recordMode: assign({ mode: ({ event }) => (event as { mode: "grep" | "find" }).mode }),
    countIndex: assign({ indexedWorkspaces: ({ context }) => context.indexedWorkspaces + 1 }),
    countServed: assign({ servedSearches: ({ context }) => context.servedSearches + 1 }),
    markMissing: assign({ pluginAvailable: () => false }),
    countFallback: assign({ fallbacks: ({ context }) => context.fallbacks + 1 }),
    countCancel: assign({ cancels: ({ context }) => context.cancels + 1 }),
  },
});

export const machine = setupMachine.createMachine({
  id: "fff-index-lifecycle",
  initial: "unindexed",
  context: initialContext,
  states: {
    unindexed: {
      on: {
        search: [
          { guard: "pluginUsable", target: "building", actions: "recordMode" },
          { guard: "pluginAbsent", target: "fallback", actions: ["recordMode", "countFallback"] },
        ],
      },
    },
    building: {
      on: {
        indexBuilt: { target: "ready", actions: "countIndex" },
        cancel: { target: "cancelled", actions: "countCancel" },
        pluginMissing: { target: "fallback", actions: ["markMissing", "countFallback"] },
      },
    },
    ready: {
      on: {
        search: { target: "searching", actions: "recordMode" },
        cancel: { target: "cancelled", actions: "countCancel" },
      },
    },
    searching: {
      on: {
        searchDone: { target: "ready", actions: "countServed" },
        cancel: { target: "cancelled", actions: "countCancel" },
      },
    },
    fallback: {
      on: {
        search: { actions: "recordMode" },
        searchDone: { actions: "countServed" },
        cancel: { target: "cancelled", actions: "countCancel" },
      },
    },
    cancelled: {
      on: {
        search: { target: "searching", actions: "recordMode" },
      },
    },
  },
});

export const scenarioGroups = {
  "惰性建索引": [
    { name: "首次检索触发建索引", events: [{ type: "search", mode: "grep" }], expectStates: ["building"], expectContext: { mode: "grep" } },
    { name: "索引完成", events: [{ type: "indexBuilt" }], expectStates: ["ready"], expectContext: { indexedWorkspaces: 1 } },
    { name: "再次检索", events: [{ type: "search", mode: "find" }], expectStates: ["searching"] },
    { name: "检索完成", events: [{ type: "searchDone" }], expectStates: ["ready"], expectContext: { servedSearches: 1 } },
  ],
  "插件缺失回落内置": [
    { name: "首次检索", events: [{ type: "search", mode: "grep" }], expectStates: ["building"] },
    { name: "建索引时插件缺失", events: [{ type: "pluginMissing" }], expectStates: ["fallback"], expectContext: { pluginAvailable: false, fallbacks: 1 } },
    { name: "回落路径仍可检索", events: [{ type: "search", mode: "grep" }, { type: "searchDone" }], expectStates: ["fallback"], expectContext: { servedSearches: 1 } },
  ],
  "建索引可取消": [
    { name: "首次检索", events: [{ type: "search", mode: "find" }], expectStates: ["building"] },
    { name: "取消", events: [{ type: "cancel" }], expectStates: ["cancelled"], expectContext: { cancels: 1 } },
  ],
  "检索中可取消": [
    { name: "建索引并检索", events: [{ type: "search", mode: "grep" }, { type: "indexBuilt" }, { type: "search", mode: "grep" }], expectStates: ["searching"] },
    { name: "取消", events: [{ type: "cancel" }], expectStates: ["cancelled"], expectContext: { cancels: 1 } },
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
