/**
 * 模型解析与回退状态机（Model Resolution & Fallback）。
 *
 * 业务：provider 注册与显式模型条目登记驱动 ModelRegistry 构建；
 * 显式条目独立于鉴权注册（缺 key 不跳过）；内置默认模型 ID 仅两族、只作解析辅助，
 * 不构成「用户已配置默认模型」的选中态；零显式条目时经 bootstrap 的表面硬失败，
 * 不得仅凭 provider 环境变量注册或选中默认模型；thinking 档位按支持集精确 clamp，
 * 任务级解析失败回退当前会话模型且可观测；/reload 重建注册表并按规则回落。
 *
 * @spec llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1742（register_provider 与鉴权检查 API）
 * @spec llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1751（get_available 按 provider 优先级排序）
 * @spec llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1752（内置默认仅两族且不构成选中态）
 * @spec llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1753（resolve_model 支持 provider/model、裸 id、别名优先）
 * @spec llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1755（回退保留意图：同 provider 基础模型）
 * @spec llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1743（thinking 档位精确 clamp：拒绝集外值且不改当前值）
 * @spec llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1744（generate 须携带当前档位影响请求组装）
 * @spec llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1745（no-silent-env-select：无显式配置注册表为空且硬失败）
 * @spec llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1749（显式条目独立于 auth 注册，不回落 kind 级 env）
 * @spec llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1750（任务级解析失败回退当前会话模型且可观测）
 * @spec llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r3030（reload 重建注册表：失效回落、空则保留旧表报诊断、档位回落）
 *
 * 实现落点：crates/xylitol-runtime/src/agent/model/registry.rs（注册表）、crates/xylitol-runtime/src/agent/model/resolver.rs（解析与回退）、crates/xylitol-runtime/src/agent/model/task_model.rs（任务级解析）。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** 换模/精确选模的末项策略：可调模型取支持集配置顺序末项；仅精确 off 或不可调归 off。 */
function adoptOnSwitch(supported: readonly string[]): string {
  if (supported.length <= 1) return "off";
  return supported[supported.length - 1] ?? "off";
}

/** reload 档位回落：新模型不支持当前档位时按末项策略回落，支持则保留。 */
function clampOnReload(supported: readonly string[], current: string): string {
  return supported.includes(current) ? current : adoptOnSwitch(supported);
}

export interface ModelRegistryContext {
  /** 已注册 provider（按优先级序）。 */
  providers: string[];
  /** 显式模型条目数（models.entries）。 */
  explicitEntries: number;
  /** 其中鉴权已解析的条目数。 */
  authResolvedEntries: number;
  /** 当前选中模型（显式解析结果；内置默认不得直接写入）。 */
  selected: string | null;
  /** 当前 thinking 档位。 */
  thinkingLevel: string;
  /** 当前模型支持集（配置声明顺序）。 */
  supportedLevels: string[];
  /** 内置默认模型 ID（仅两族；不构成选中态）。 */
  builtinDefaults: string[];
  /** 回退次数（解析回退 + 任务回退 + reload 回落）。 */
  fallbacks: number;
  /** 最近一次回退原因（可观测归因）。 */
  fallbackReason: string | null;
  /** 任务级解析是否走了可观测回退。 */
  taskFallbackObserved: boolean;
  /** 任务级解析产出的独立模型实例标识（不进注册表）。 */
  taskModel: string | null;
  /** 已按当前档位组装的请求数。 */
  requestsWithLevel: number;
  /** 拒绝与 reload 的可观察诊断。 */
  diagnostics: string[];
  /** 硬失败原因；null 表示未失败。 */
  error: string | null;
}

export const initialContext: ModelRegistryContext = {
  providers: [],
  explicitEntries: 0,
  authResolvedEntries: 0,
  selected: null,
  thinkingLevel: "off",
  supportedLevels: ["off"],
  builtinDefaults: [],
  fallbacks: 0,
  fallbackReason: null,
  taskFallbackObserved: false,
  taskModel: null,
  requestsWithLevel: 0,
  diagnostics: [],
  error: null,
};

export const meta = defineMeta({
  name: "model-resolution",
  pkg: "config-meta",
  purpose: "模型解析与回退：显式条目注册、档位 clamp、任务级回退、reload 重建",
  anchors: [
    "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1742",
    "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1751",
    "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1752",
    "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1753",
    "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1755",
    "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1743",
    "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1744",
    "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1745",
    "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1749",
    "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r1750",
    "llmanspec/specs/runtime-model-registry/runtime-model-registry.feature#@req:r3030",
  ],
  impl: ["crates/xylitol-runtime/src/agent/model/registry.rs", "crates/xylitol-runtime/src/agent/model/resolver.rs", "crates/xylitol-runtime/src/agent/model/task_model.rs"],
});

type RegisterEntryEvent = { type: "registerModelEntry"; modelId: string; authResolved: boolean };
type ResolveModelEvent = { type: "resolveModel"; found: boolean; modelId: string; fallbackTo: string };
type ResolveTaskEvent = { type: "resolveTaskModel"; entryValid: boolean; modelId: string };
type SetLevelEvent = { type: "setThinkingLevel"; level: string };
type SwitchModelEvent = { type: "switchModel"; modelId: string; supportedLevels: readonly string[] };
type ReloadEvent = { type: "reload"; empty: boolean; newEntryCount: number; currentValid: boolean; currentSupportedLevels: readonly string[]; defaultModel: string };

export type ModelRegistryEvent =
  | { type: "registerProvider"; name: string }
  | RegisterEntryEvent
  | { type: "finishBootstrap" }
  | ResolveModelEvent
  | ResolveTaskEvent
  | SetLevelEvent
  | SwitchModelEvent
  | { type: "generateStep" }
  | ReloadEvent;

export const setupMachine = setup({
  types: {
    context: {} as ModelRegistryContext,
    events: {} as ModelRegistryEvent,
  },
  guards: {
    // 零显式条目即硬失败（no-silent-env-select）。
    hasExplicitEntries: ({ context }) => context.explicitEntries > 0,
    // 档位匹配为精确字符串相等：不 trim、不大小写折叠，guard 按动作执行前的支持集求值。
    levelSupported: ({ context, event }) => context.supportedLevels.includes((event as SetLevelEvent).level),
    taskEntryValid: ({ event }) => (event as ResolveTaskEvent).entryValid === true,
    newRegistryEmpty: ({ event }) => (event as ReloadEvent).empty === true,
  },
  actions: {
    addProvider: assign({
      providers: ({ context, event }) => [...context.providers, (event as { name: string }).name],
    }),
    // 显式条目独立于鉴权注册：缺 key 不跳过，也不回落 kind 级环境变量。
    addModelEntry: assign({
      explicitEntries: ({ context }) => context.explicitEntries + 1,
      authResolvedEntries: ({ context, event }) => {
        const e = event as RegisterEntryEvent;
        return context.authResolvedEntries + (e.authResolved ? 1 : 0);
      },
    }),
    // 内置默认 ID 只登记为解析辅助；不得写入 selected（不构成选中态）。
    enterReady: assign({ builtinDefaults: () => ["gpt-4o", "claude-sonnet-4-20250514"] }),
    markBootstrapFailed: assign({
      error: () => "无显式模型条目：不得仅凭 provider 环境变量注册或选中默认模型（no-silent-env-select）",
    }),
    selectResolved: assign({
      selected: ({ event }) => {
        const e = event as ResolveModelEvent;
        return e.found ? e.modelId : e.fallbackTo;
      },
      fallbacks: ({ context, event }) => {
        const e = event as ResolveModelEvent;
        return e.found ? context.fallbacks : context.fallbacks + 1;
      },
      fallbackReason: ({ context, event }) => {
        const e = event as ResolveModelEvent;
        if (e.found) return context.fallbackReason;
        return "目标模型不在 provider 可用列表：回退同 provider 基础模型并保留用户意图";
      },
    }),
    adoptTaskModel: assign({
      taskModel: ({ event }) => (event as ResolveTaskEvent).modelId,
    }),
    // 任务条目不进注册表：explicitEntries 不变，回退当前会话模型且归因可观测。
    fallbackTaskModel: assign({
      taskModel: () => null,
      taskFallbackObserved: () => true,
      fallbacks: ({ context }) => context.fallbacks + 1,
      fallbackReason: () => "任务模型条目构建失败：回退当前会话模型（归因标注 + 受控频率通知）",
    }),
    applyLevel: assign({
      thinkingLevel: ({ event }) => (event as SetLevelEvent).level,
    }),
    recordLevelRejection: assign({
      diagnostics: ({ context, event }) => [
        ...context.diagnostics,
        `档位被拒绝：${(event as SetLevelEvent).level} 不在当前模型支持集（精确字符串匹配）`,
      ],
    }),
    countRequest: assign({
      requestsWithLevel: ({ context }) => context.requestsWithLevel + 1,
    }),
    switchToModel: assign({
      selected: ({ event }) => (event as SwitchModelEvent).modelId,
      supportedLevels: ({ event }) => [...(event as SwitchModelEvent).supportedLevels],
      thinkingLevel: ({ event }) => {
        const e = event as SwitchModelEvent;
        return adoptOnSwitch(e.supportedLevels);
      },
    }),
    // reload 后新注册表为空：保留旧注册表并报可观察诊断。
    keepOldRegistryWithDiagnostic: assign({
      diagnostics: ({ context }) => [...context.diagnostics, "reload 后新注册表为空：保留旧注册表并报告诊断"],
    }),
    rebuildRegistry: assign({
      explicitEntries: ({ event }) => (event as ReloadEvent).newEntryCount,
      supportedLevels: ({ event }) => [...(event as ReloadEvent).currentSupportedLevels],
      selected: ({ context, event }) => {
        const e = event as ReloadEvent;
        return e.currentValid ? context.selected : e.defaultModel;
      },
      fallbacks: ({ context, event }) => {
        const e = event as ReloadEvent;
        return e.currentValid ? context.fallbacks : context.fallbacks + 1;
      },
      fallbackReason: ({ context, event }) => {
        const e = event as ReloadEvent;
        if (e.currentValid) return context.fallbackReason;
        return "当前模型在新注册表失效：回落默认模型（无默认则首个可用模型）";
      },
      thinkingLevel: ({ context, event }) => {
        const e = event as ReloadEvent;
        return clampOnReload(e.currentSupportedLevels, context.thinkingLevel);
      },
    }),
  },
});

export const machine = setupMachine.createMachine({
  id: "model-resolution",
  initial: "bootstrapping",
  context: initialContext,
  states: {
    bootstrapping: {
      on: {
        registerProvider: { actions: "addProvider" },
        registerModelEntry: { actions: "addModelEntry" },
        finishBootstrap: [
          { guard: "hasExplicitEntries", target: "ready", actions: "enterReady" },
          { target: "failed", actions: "markBootstrapFailed" },
        ],
      },
    },
    ready: {
      on: {
        resolveModel: { actions: "selectResolved" },
        resolveTaskModel: [
          { guard: "taskEntryValid", actions: "adoptTaskModel" },
          { actions: "fallbackTaskModel" },
        ],
        setThinkingLevel: [
          { guard: "levelSupported", actions: "applyLevel" },
          { actions: "recordLevelRejection" },
        ],
        switchModel: { actions: "switchToModel" },
        generateStep: { actions: "countRequest" },
        reload: [
          { guard: "newRegistryEmpty", actions: "keepOldRegistryWithDiagnostic" },
          { actions: "rebuildRegistry" },
        ],
      },
    },
    failed: { type: "final" },
  },
});

/** 各组公用的 bootstrap 前缀：两族 provider + 两条显式条目（一条缺鉴权）。 */
const bootReady: readonly unknown[] = [
  { type: "registerProvider", name: "openai" },
  { type: "registerProvider", name: "anthropic" },
  { type: "registerModelEntry", modelId: "fake-1", authResolved: true },
  { type: "registerModelEntry", modelId: "fake-2", authResolved: false },
  { type: "finishBootstrap" },
];

export const scenarioGroups = {
  "两族注册与显式条目登记": [
    { name: "显式条目注册完成进入 ready", events: bootReady, expectStates: ["ready"], expectContext: { providers: ["openai", "anthropic"], explicitEntries: 2, authResolvedEntries: 1, selected: null }, expect: (snapshot) => { const ctx = (snapshot as { context: ModelRegistryContext }).context; return ctx.builtinDefaults.length === 2 ? true : "内置默认应仅两族"; } },
  ],
  "解析命中与回退保留意图": [
    { name: "精确解析命中显式条目", events: [...bootReady, { type: "resolveModel", found: true, modelId: "fake-1", fallbackTo: "" }], expectContext: { selected: "fake-1", fallbacks: 0 } },
    { name: "目标不在可用列表时回退基础模型", events: [...bootReady, { type: "resolveModel", found: false, modelId: "fake-1-20260101", fallbackTo: "fake-base" }], expectContext: { selected: "fake-base", fallbacks: 1 }, expect: (snapshot) => { const ctx = (snapshot as { context: ModelRegistryContext }).context; return ctx.fallbackReason !== null && ctx.fallbackReason.includes("保留用户意图") ? true : `回退应可观测并保留意图，实得 ${ctx.fallbackReason}`; } },
  ],
  "thinking 档位精确 clamp": [
    { name: "换模采用支持集末项", events: [...bootReady, { type: "switchModel", modelId: "fake-1", supportedLevels: ["off", "high", "max"] }], expectContext: { selected: "fake-1", thinkingLevel: "max" } },
    { name: "大小写变体被拒绝且档位不变", events: [{ type: "setThinkingLevel", level: "HIGH" }], expectContext: { thinkingLevel: "max" }, expect: (snapshot) => { const ctx = (snapshot as { context: ModelRegistryContext }).context; return ctx.diagnostics.some((d) => d.includes("HIGH")) ? true : `拒绝应留下可观察诊断，实得 ${JSON.stringify(ctx.diagnostics)}`; } },
    { name: "首尾空白变体被拒绝", events: [{ type: "setThinkingLevel", level: " high " }], expectContext: { thinkingLevel: "max" } },
    { name: "支持集内档位生效并进入请求组装", events: [{ type: "setThinkingLevel", level: "high" }, { type: "generateStep" }], expectContext: { thinkingLevel: "high", requestsWithLevel: 1 } },
  ],
  "零显式条目硬失败": [
    { name: "仅 provider 环境变量时注册表为空且硬失败", events: [{ type: "registerProvider", name: "openai" }, { type: "registerProvider", name: "anthropic" }, { type: "finishBootstrap" }], expectStates: ["failed"], expectContext: { explicitEntries: 0, selected: null }, expect: (snapshot) => { const ctx = (snapshot as { context: ModelRegistryContext }).context; return ctx.error !== null && ctx.error.includes("环境变量") ? true : `错误应指向环境变量回退禁令，实得 ${ctx.error}`; } },
  ],
  "任务级解析失败可观测回退": [
    { name: "任务条目可构建时返回独立实例", events: [...bootReady, { type: "switchModel", modelId: "fake-1", supportedLevels: ["off", "high"] }, { type: "resolveTaskModel", entryValid: true, modelId: "task-model" }], expectContext: { taskModel: "task-model", taskFallbackObserved: false, selected: "fake-1", explicitEntries: 2 } },
    { name: "任务条目构建失败回退当前会话模型", events: [{ type: "resolveTaskModel", entryValid: false, modelId: "" }], expectContext: { taskModel: null, taskFallbackObserved: true, selected: "fake-1", explicitEntries: 2 }, expect: (snapshot) => { const ctx = (snapshot as { context: ModelRegistryContext }).context; return ctx.fallbackReason !== null && ctx.fallbackReason.includes("当前会话模型") ? true : `回退归因应可观测，实得 ${ctx.fallbackReason}`; } },
  ],
  "reload 空注册表保留旧表": [
    { name: "换到 fake-1 并采用末项档位", events: [...bootReady, { type: "switchModel", modelId: "fake-1", supportedLevels: ["off", "high"] }], expectContext: { selected: "fake-1", thinkingLevel: "high" } },
    { name: "reload 后新表为空则保留旧表并报诊断", events: [{ type: "reload", empty: true, newEntryCount: 0, currentValid: true, currentSupportedLevels: ["off"], defaultModel: "" }], expectContext: { explicitEntries: 2, selected: "fake-1", thinkingLevel: "high" }, expect: (snapshot) => { const ctx = (snapshot as { context: ModelRegistryContext }).context; return ctx.diagnostics.some((d) => d.includes("保留旧注册表")) ? true : `应报保留旧注册表诊断，实得 ${JSON.stringify(ctx.diagnostics)}`; } },
  ],
  "reload 当前模型失效回落默认": [
    { name: "新注册表重建且当前模型失效", events: [...bootReady, { type: "switchModel", modelId: "fake-2", supportedLevels: ["off", "low"] }, { type: "reload", empty: false, newEntryCount: 3, currentValid: false, currentSupportedLevels: ["off", "high", "max"], defaultModel: "fake-1" }], expectContext: { selected: "fake-1", explicitEntries: 3, fallbacks: 1, thinkingLevel: "max" }, expect: (snapshot) => { const ctx = (snapshot as { context: ModelRegistryContext }).context; return ctx.fallbackReason !== null && ctx.fallbackReason.includes("回落默认模型") ? true : `回落归因应可观测，实得 ${ctx.fallbackReason}`; } },
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
