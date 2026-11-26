/**
 * 会话树槽状态机（Session Tree Slot）。
 *
 * 业务：双 Esc / slash 打开 MessageHistory 活树；travel / filter / fold / fork / label 均经 Driver 取活树；
 * 树反映已持久化 turn；travel 通知附在尾部。
 *
 * @spec llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature#@req:r1325（slot-replace）
 * @spec llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature#@req:r1332（live-travel-via-driver）
 * @spec llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature#@req:r1336（tree-reflects-persisted-turn）
 * @spec llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature#@req:r1337（session-tree-filter）
 * @spec llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature#@req:r1338（session-tree-fold）
 * @spec llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature#@req:r1326（product-session-tree-fork）
 * @spec llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature#@req:r1328（session-tree-label-persist）
 * @spec llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature#@req:r1330（travel-notice-trailing）
 *
 * 实现落点：crates/xylitol-app-tui/src/（会话树槽）。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

export type TreePhase = "closed" | "open" | "traveling" | "forking";

export interface TreeNode {
  id: string;
  label: string;
  folded: boolean;
}

export interface TreeContext {
  /** 树槽是否打开。 */
  open: boolean;
  /** 节点列表（来自 Driver 活树）。 */
  nodes: TreeNode[];
  /** 当前选中节点。 */
  selected: string | null;
  /** 过滤串。 */
  filter: string;
  /** 已 travel 次数。 */
  travels: number;
  /** 已 fork 次数。 */
  forks: number;
  /** 已持久化标签数。 */
  persistedLabels: number;
  /** travel 通知位置是否为尾部。 */
  noticeAtTail: boolean;
}

export const initialContext: TreeContext = {
  open: false,
  nodes: [
    { id: "n1", label: "首轮", folded: false },
    { id: "n2", label: "次轮", folded: false },
  ],
  selected: null,
  filter: "",
  travels: 0,
  forks: 0,
  persistedLabels: 0,
  noticeAtTail: true,
};

export const meta = defineMeta({
  name: "session-tree-slot",
  pkg: "app-surface",
  purpose: "会话树槽：打开、travel、过滤、折叠、fork、标签持久",
  anchors: [
    "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature#@req:r1325",
    "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature#@req:r1332",
    "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature#@req:r1336",
    "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature#@req:r1337",
    "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature#@req:r1338",
    "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature#@req:r1326",
    "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature#@req:r1328",
    "llmanspec/specs/app-tui-session-tree/app-tui-session-tree.feature#@req:r1330",
  ],
  impl: ["crates/xylitol-app-tui/src/"],
});

export const setupMachine = setup({
  types: {
    context: {} as TreeContext,
    events: {} as
      | { type: "openTree" }
      | { type: "closeTree" }
      | { type: "select"; id: string }
      | { type: "travel"; id: string }
      | { type: "fork"; id: string }
      | { type: "filter"; text: string }
      | { type: "fold"; id: string }
      | { type: "label"; id: string; text: string }
      | { type: "persistedTurn" }
  },
  guards: {
    isClosed: ({ context }) => context.open === false,
    isOpen: ({ context }) => context.open === true,
  },
  actions: {
    markOpen: assign({ open: () => true }),
    markClosed: assign({ open: () => false }),
    selectNode: assign({ selected: ({ event }) => (event as { id: string }).id }),
    countTravel: assign({ travels: ({ context }) => context.travels + 1, selected: ({ event }) => (event as { id: string }).id }),
    countFork: assign({ forks: ({ context }) => context.forks + 1 }),
    applyFilter: assign({ filter: ({ event }) => (event as { text: string }).text }),
    toggleFold: assign({
      nodes: ({ context, event }) => {
        const id = (event as { id: string }).id;
        return context.nodes.map((n) => (n.id === id ? { ...n, folded: !n.folded } : n));
      },
    }),
    persistLabel: assign({
      nodes: ({ context, event }) => {
        const e = event as { id: string; text: string };
        return context.nodes.map((n) => (n.id === e.id ? { ...n, label: e.text } : n));
      },
      persistedLabels: ({ context }) => context.persistedLabels + 1,
    }),
    reflectTurn: assign({
      nodes: ({ context }) => [...context.nodes, { id: `n${context.nodes.length + 1}`, label: `第${context.nodes.length + 1}轮`, folded: false }],
    }),
  },
});

export const machine = setupMachine.createMachine({
  id: "session-tree-slot",
  initial: "closed",
  context: initialContext,
  states: {
    closed: {
      on: {
        openTree: { target: "open", actions: "markOpen" },
        persistedTurn: { actions: "reflectTurn" },
      },
    },
    open: {
      on: {
        closeTree: { target: "closed", actions: "markClosed" },
        select: { actions: "selectNode" },
        travel: { target: "traveling", actions: "countTravel" },
        fork: { target: "forking", actions: "countFork" },
        filter: { actions: "applyFilter" },
        fold: { actions: "toggleFold" },
        label: { actions: "persistLabel" },
        persistedTurn: { actions: "reflectTurn" },
      },
    },
    traveling: {
      always: [{ target: "open" }],
    },
    forking: {
      always: [{ target: "open" }],
    },
  },
});

export const scenarioGroups = {
  "双Esc打开并关闭树": [
    { name: "打开树", events: [{ type: "openTree" }], expectStates: ["open"], expectContext: { open: true } },
    { name: "关闭树", events: [{ type: "closeTree" }], expectStates: ["closed"], expectContext: { open: false } },
  ],
  "经Driver取活树travel": [
    { name: "打开树", events: [{ type: "openTree" }], expectStates: ["open"] },
    { name: "travel 到 n2", events: [{ type: "travel", id: "n2" }], expectStates: ["open"], expectContext: { travels: 1, selected: "n2" } },
  ],
  "树反映已持久化turn": [
    { name: "关闭态落定一轮", events: [{ type: "persistedTurn" }], expectStates: ["closed"], expect: (s) => ((s as { context: TreeContext }).context.nodes.length === 3 ? true : "期望新增一个节点") },
  ],
  "过滤与折叠": [
    { name: "打开树并过滤", events: [{ type: "openTree" }, { type: "filter", text: "首" }], expectStates: ["open"], expectContext: { filter: "首" } },
    { name: "折叠 n1", events: [{ type: "fold", id: "n1" }], expectStates: ["open"], expect: (s) => ((s as { context: TreeContext }).context.nodes[0]?.folded === true ? true : "期望 n1 已折叠") },
  ],
  "fork与标签持久": [
    { name: "打开树", events: [{ type: "openTree" }], expectStates: ["open"] },
    { name: "fork n1", events: [{ type: "fork", id: "n1" }], expectStates: ["open"], expectContext: { forks: 1 } },
    { name: "打标签", events: [{ type: "label", id: "n1", text: "修复点" }], expectStates: ["open"], expectContext: { persistedLabels: 1 } },
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