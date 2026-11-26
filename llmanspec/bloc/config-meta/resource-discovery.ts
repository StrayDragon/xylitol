/**
 * 资源发现与热重载状态机（Resource Discovery & Reload）。
 *
 * 业务：统一资源加载器列出 skills、themes 与 context（AGENTS.md / SYSTEM），
 * 每条资源带 SourceInfo（path / source / scope / origin / base_dir）；
 * SourceScope 支持 user / project / temporary；项目资源受 Trust 门禁，
 * 未信任不列入项目条目；reload 清缓存后重扫并返回 report（names / count），
 * 失败诊断可观察；prompts/*.md 不列为可展开 prompt 模板资源；
 * reload 不得修改会话历史文件。
 *
 * @spec llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature#@req:r1759（资源列表：skills/themes + scope，不含 prompts）
 * @spec llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature#@req:r1760（热重载：清缓存重扫、失败诊断可观察、不改会话历史）
 * @spec llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature#@req:r1761（主题名发现：Trust 下列 stem，未信任不列项目 themes）
 * @spec llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature#@req:r1762（skills 发现与 reload 报告，未信任不列项目 skills）
 * @spec llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature#@req:r1767（统一 SourceInfo 类型形状）
 * @spec llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature#@req:r1768（Scope 枚举 user/project/temporary）
 *
 * 实现落点：crates/xylitol-runtime/src/resource/loader.rs（DefaultResourceLoader 与 reload）。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** 资源作用域：对齐 SourceScope 枚举。 */
export type ResourceScope = "user" | "project" | "temporary";

/** 资源种类：统一发现面不含 prompts。 */
export type ResourceKind = "skill" | "theme" | "context";

/** 单条资源的来源信息（SourceInfo 简面：path / scope / kind）。 */
export interface ResourceEntry {
  name: string;
  kind: ResourceKind;
  scope: ResourceScope;
  path: string;
}

/** 最近一次 reload 报告：names 与 count。 */
export interface ReloadReport {
  names: string[];
  count: number;
}

export interface ResourceDiscoveryContext {
  /** Trust 门禁：项目本地资源仅信任时加载。 */
  trusted: boolean;
  /** 已发现资源（到达序）。 */
  entries: ResourceEntry[];
  /** 是否把 prompts 列为资源（须恒为 false）。 */
  promptsListed: boolean;
  /** reload 是否修改过会话历史文件（须恒为 false）。 */
  historyModified: boolean;
  /** 最近一次 reload / 扫描报告。 */
  lastReport: ReloadReport | null;
  /** reload 失败标记（诊断可观察）。 */
  lastReloadFailed: boolean;
  /** 可观察诊断。 */
  diagnostics: string[];
}

export const initialContext: ResourceDiscoveryContext = {
  trusted: false,
  entries: [],
  promptsListed: false,
  historyModified: false,
  lastReport: null,
  lastReloadFailed: false,
  diagnostics: [],
};

export const meta = defineMeta({
  name: "resource-discovery",
  pkg: "config-meta",
  purpose: "统一资源发现与热重载：skills/themes/context、Trust 门禁、reload 报告",
  anchors: [
    "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature#@req:r1759",
    "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature#@req:r1760",
    "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature#@req:r1761",
    "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature#@req:r1762",
    "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature#@req:r1767",
    "llmanspec/specs/runtime-resource-discovery/runtime-resource-discovery.feature#@req:r1768",
  ],
  impl: ["crates/xylitol-runtime/src/resource/loader.rs"],
});

type DiscoverEvent = { type: "discover"; kind: ResourceKind; name: string; scope: ResourceScope; path: string };
type FinishReloadEvent = { type: "finishReload"; failed: boolean };

export type ResourceDiscoveryEvent =
  | { type: "setTrust"; trusted: boolean }
  | { type: "beginScan" }
  | DiscoverEvent
  | { type: "offerPromptsDir"; present: boolean }
  | { type: "finishScan" }
  | { type: "startReload" }
  | FinishReloadEvent;

export const setupMachine = setup({
  types: {
    context: {} as ResourceDiscoveryContext,
    events: {} as ResourceDiscoveryEvent,
  },
  guards: {
    // 项目资源受 Trust 门禁：未信任时 project scope 一律跳过（guard 读动作执行前的信任态）。
    projectEntryBlocked: ({ context, event }) => (event as DiscoverEvent).scope === "project" && !context.trusted,
    reloadFailed: ({ event }) => (event as FinishReloadEvent).failed === true,
  },
  actions: {
    applyTrust: assign({
      trusted: ({ event }) => (event as { trusted: boolean }).trusted,
    }),
    addEntry: assign({
      entries: ({ context, event }) => {
        const e = event as DiscoverEvent;
        return [...context.entries, { name: e.name, kind: e.kind, scope: e.scope, path: e.path }];
      },
    }),
    skipProjectEntry: assign({
      diagnostics: ({ context, event }) => [
        ...context.diagnostics,
        `${(event as DiscoverEvent).name}（project scope）未信任跳过，不列入已发现资源`,
      ],
    }),
    // prompts 目录存在也不列为可展开 prompt 模板资源。
    refusePrompts: assign({
      promptsListed: () => false,
      diagnostics: ({ context }) => [...context.diagnostics, "prompts/*.md 不列为可展开 prompt 模板资源"],
    }),
    buildReport: assign({
      lastReport: ({ context }) => ({ names: context.entries.map((e) => e.name), count: context.entries.length }),
      lastReloadFailed: () => false,
    }),
    // reload 先清缓存再重扫；不触碰会话历史文件。
    clearCache: assign({
      entries: () => [],
      lastReloadFailed: () => false,
    }),
    recordReloadFailure: assign({
      lastReloadFailed: () => true,
      diagnostics: ({ context }) => [...context.diagnostics, "reload 失败：已重建部分缓存，失败诊断可观察"],
      lastReport: ({ context }) => ({ names: context.entries.map((e) => e.name), count: context.entries.length }),
    }),
  },
});

export const machine = setupMachine.createMachine({
  id: "resource-discovery",
  initial: "unscanned",
  context: initialContext,
  states: {
    unscanned: {
      on: {
        setTrust: { actions: "applyTrust" },
        beginScan: { target: "scanning" },
      },
    },
    scanning: {
      on: {
        setTrust: { actions: "applyTrust" },
        discover: [
          { guard: "projectEntryBlocked", actions: "skipProjectEntry" },
          { actions: "addEntry" },
        ],
        offerPromptsDir: { actions: "refusePrompts" },
        finishScan: { target: "discovered", actions: "buildReport" },
      },
    },
    discovered: {
      on: {
        setTrust: { actions: "applyTrust" },
        startReload: { target: "reloading", actions: "clearCache" },
      },
    },
    reloading: {
      on: {
        discover: [
          { guard: "projectEntryBlocked", actions: "skipProjectEntry" },
          { actions: "addEntry" },
        ],
        offerPromptsDir: { actions: "refusePrompts" },
        finishReload: [
          { guard: "reloadFailed", target: "discovered", actions: "recordReloadFailure" },
          { target: "discovered", actions: "buildReport" },
        ],
      },
    },
  },
});

export const scenarioGroups = {
  "全量发现与来源形状": [
    { name: "信任后开始扫描", events: [{ type: "setTrust", trusted: true }, { type: "beginScan" }] },
    { name: "发现 skills/themes/context 覆盖三种 scope", events: [{ type: "discover", kind: "skill", name: "demo", scope: "user", path: "~/.xylitol/skills/demo" }, { type: "discover", kind: "skill", name: "review", scope: "project", path: ".xylitol/skills/review" }, { type: "discover", kind: "theme", name: "dark", scope: "project", path: ".xylitol/themes/dark.toml" }, { type: "discover", kind: "theme", name: "light", scope: "user", path: "~/.xylitol/themes/light.toml" }, { type: "discover", kind: "context", name: "AGENTS.md", scope: "project", path: "AGENTS.md" }, { type: "discover", kind: "skill", name: "tmp-note", scope: "temporary", path: "/tmp/skills/tmp-note" }] },
    { name: "扫描完成出报告", events: [{ type: "finishScan" }], expectStates: ["discovered"], expectContext: { promptsListed: false, historyModified: false, lastReport: { names: ["demo", "review", "dark", "light", "AGENTS.md", "tmp-note"], count: 6 } }, expect: (snapshot) => { const ctx = (snapshot as { context: ResourceDiscoveryContext }).context; const scopes = new Set(ctx.entries.map((e) => e.scope)); if (!(scopes.has("user") && scopes.has("project") && scopes.has("temporary"))) return "scope 应覆盖 user/project/temporary"; const shaped = ctx.entries.every((e) => e.path.length > 0 && e.kind !== undefined && e.scope !== undefined); return shaped ? true : "每条资源应带 path/scope/kind（SourceInfo 形状）"; } },
  ],
  "未信任跳过项目资源": [
    { name: "未信任开始扫描", events: [{ type: "setTrust", trusted: false }, { type: "beginScan" }] },
    { name: "项目 skill 被跳过而 user 主题保留", events: [{ type: "discover", kind: "skill", name: "review", scope: "project", path: ".xylitol/skills/review" }, { type: "discover", kind: "theme", name: "dark", scope: "user", path: "~/.xylitol/themes/dark.toml" }, { type: "finishScan" }], expectStates: ["discovered"], expectContext: { lastReport: { names: ["dark"], count: 1 } }, expect: (snapshot) => { const ctx = (snapshot as { context: ResourceDiscoveryContext }).context; return ctx.diagnostics.some((d) => d.includes("review") && d.includes("跳过")) ? true : `应留下跳过诊断，实得 ${JSON.stringify(ctx.diagnostics)}`; } },
  ],
  "prompts 目录不列为资源": [
    { name: "prompts 目录存在也不入列表", events: [{ type: "setTrust", trusted: true }, { type: "beginScan" }, { type: "discover", kind: "skill", name: "demo", scope: "user", path: "~/.xylitol/skills/demo" }, { type: "offerPromptsDir", present: true }, { type: "finishScan" }], expectStates: ["discovered"], expectContext: { promptsListed: false }, expect: (snapshot) => { const ctx = (snapshot as { context: ResourceDiscoveryContext }).context; const noPrompts = ctx.entries.every((e) => e.kind !== "context" || !e.name.endsWith(".md") || e.name === "AGENTS.md"); return noPrompts && ctx.diagnostics.some((d) => d.includes("prompts")) ? true : "prompts 应被拒绝并留诊断"; } },
  ],
  "热重载清缓存并出报告": [
    { name: "首轮扫描两条资源", events: [{ type: "setTrust", trusted: true }, { type: "beginScan" }, { type: "discover", kind: "skill", name: "demo", scope: "user", path: "~/.xylitol/skills/demo" }, { type: "discover", kind: "theme", name: "dark", scope: "project", path: ".xylitol/themes/dark.toml" }, { type: "finishScan" }] },
    { name: "reload 清空缓存", events: [{ type: "startReload" }], expectStates: ["reloading"], expectContext: { historyModified: false } },
    { name: "重扫后报告新集合", events: [{ type: "discover", kind: "theme", name: "dark", scope: "project", path: ".xylitol/themes/dark.toml" }, { type: "discover", kind: "skill", name: "demo2", scope: "user", path: "~/.xylitol/skills/demo2" }, { type: "finishReload", failed: false }], expectStates: ["discovered"], expectContext: { historyModified: false, lastReloadFailed: false, lastReport: { names: ["dark", "demo2"], count: 2 } } },
  ],
  "重载失败诊断可观察": [
    { name: "reload 中部分重扫", events: [{ type: "setTrust", trusted: true }, { type: "beginScan" }, { type: "discover", kind: "skill", name: "demo", scope: "user", path: "~/.xylitol/skills/demo" }, { type: "finishScan" }, { type: "startReload" }, { type: "discover", kind: "skill", name: "demo", scope: "user", path: "~/.xylitol/skills/demo" }] },
    { name: "失败仍返回报告并留诊断", events: [{ type: "finishReload", failed: true }], expectStates: ["discovered"], expectContext: { lastReloadFailed: true, lastReport: { names: ["demo"], count: 1 } }, expect: (snapshot) => { const ctx = (snapshot as { context: ResourceDiscoveryContext }).context; return ctx.diagnostics.some((d) => d.includes("reload 失败")) && ctx.historyModified === false ? true : "失败诊断应可观察且不改会话历史"; } },
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
