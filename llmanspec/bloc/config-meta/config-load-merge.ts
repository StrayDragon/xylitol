/**
 * 配置分层加载与深合并状态机（Config Load & Merge）。
 *
 * 业务：产品配置仅以 config.yaml（可分享）与 secret.env（密钥）为配置面；
 * 加载链为 global config.yaml → project config.yaml → 可选 --config，
 * 每层 config.yaml 之后按文件名升序合并同层 conf.d/*.yaml 片段；
 * 未知顶层键两阶段校验（加载期收集、插件认领、剩余硬失败）；
 * 加载失败一律 fail-closed，非零退出，不回退环境变量默认。
 *
 * @spec llmanspec/specs/runtime-config/runtime-config.feature#@req:r1731（config.yaml + secret.env 分层加载链）
 * @spec llmanspec/specs/runtime-config/runtime-config.feature#@req:r1732（config.local 不支持，忽略不迁移）
 * @spec llmanspec/specs/runtime-config/runtime-config.feature#@req:r3076（conf.d drop-in 片段升序合并）
 * @spec llmanspec/specs/runtime-config/runtime-config.feature#@req:r3072（yaml merge key 父键优先，非法值失败）
 * @spec llmanspec/specs/runtime-config/runtime-config.feature#@req:r3070（未知键两阶段校验与 warn 降级）
 * @spec llmanspec/specs/runtime-config/runtime-config.feature#@req:r3071（xylitol 引导段：schema_version 高于当前硬失败）
 * @spec llmanspec/specs/runtime-config/runtime-config.feature#@req:r3069（顶层 include 已移除，出现即硬失败）
 * @spec llmanspec/specs/runtime-config/runtime-config.feature#@req:r3311（context_inject 同层重复 id 报错，跨层后层覆盖）
 * @spec llmanspec/specs/runtime-config/runtime-config.feature#@req:r1723（mode 字段缺省 one-at-a-time）
 * @spec llmanspec/specs/runtime-config/runtime-config.feature#@req:r1740（配置边界承载 JsonSchema 快照）
 * @spec llmanspec/specs/cli-entry/cli-entry.feature#@req:r1386（config-load-fail-closed：失败硬错误非零退出）
 * @spec llmanspec/specs/user-experience/user-experience.feature#@req:r3075（未知键错误指出键名与迁移方向）
 *
 * 实现落点：crates/xylitol-kernel/src/bootstrap.rs（load_app_config 加载链）、crates/xylitol-kernel/src/config_sections.rs（节注册表聚合）。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** 配置层：global → project → --config。 */
export type ConfigLayer = "global" | "project" | "cli";

/** xylitol.unknown_keys 策略，缺省 error。 */
export type UnknownKeyPolicy = "error" | "warn";

export interface ConfigLoadContext {
  /** 已按序合并的层（每层 config.yaml 各记录一次）。 */
  layersApplied: ConfigLayer[];
  /** 生效配置（按顶层节覆盖；真实实现为逐键深合并，此处取后层覆盖面）。 */
  effective: Record<string, unknown>;
  /** secret.env 注入的密钥数。 */
  secretCount: number;
  /** 是否发现并忽略了 config.local.yaml。 */
  localIgnored: boolean;
  /** 已合并的 conf.d 片段路径（升序到达序）。 */
  fragments: string[];
  /** 顶层键全集（两阶段校验第一阶段收集）。 */
  topKeys: string[];
  /** 插件认领的顶层节（第二阶段）。 */
  claimed: string[];
  /** context_inject 各层注入器 id（同层重复须报错）。 */
  injectorIdsByLayer: Record<string, string[]>;
  /** 未知键策略。 */
  unknownPolicy: UnknownKeyPolicy;
  /** 忽略与 warn 降级产生的警告。 */
  warnings: string[];
  /** 配置 JSON Schema 是否已在配置边界快照导出。 */
  schemaExported: boolean;
  /** 硬失败原因；null 表示未失败。 */
  error: string | null;
  /** 失败是否 fail-closed（非零退出、不回退 env 默认）。 */
  failClosed: boolean;
}

export const initialContext: ConfigLoadContext = {
  layersApplied: [],
  effective: {},
  secretCount: 0,
  localIgnored: false,
  fragments: [],
  topKeys: [],
  claimed: [],
  injectorIdsByLayer: {},
  unknownPolicy: "error",
  warnings: [],
  schemaExported: false,
  error: null,
  failClosed: false,
};

export const meta = defineMeta({
  name: "config-load-merge",
  pkg: "config-meta",
  purpose: "配置分层加载与深合并：三层链、conf.d 片段、两阶段未知键校验、fail-closed",
  anchors: [
    "llmanspec/specs/runtime-config/runtime-config.feature#@req:r1731",
    "llmanspec/specs/runtime-config/runtime-config.feature#@req:r1732",
    "llmanspec/specs/runtime-config/runtime-config.feature#@req:r3076",
    "llmanspec/specs/runtime-config/runtime-config.feature#@req:r3072",
    "llmanspec/specs/runtime-config/runtime-config.feature#@req:r3070",
    "llmanspec/specs/runtime-config/runtime-config.feature#@req:r3071",
    "llmanspec/specs/runtime-config/runtime-config.feature#@req:r3069",
    "llmanspec/specs/runtime-config/runtime-config.feature#@req:r3311",
    "llmanspec/specs/runtime-config/runtime-config.feature#@req:r1723",
    "llmanspec/specs/runtime-config/runtime-config.feature#@req:r1740",
    "llmanspec/specs/cli-entry/cli-entry.feature#@req:r1386",
    "llmanspec/specs/user-experience/user-experience.feature#@req:r3075",
  ],
  impl: ["crates/xylitol-kernel/src/bootstrap.rs", "crates/xylitol-kernel/src/config_sections.rs"],
});

type LoadLayerEvent = { type: "loadLayer"; layer: ConfigLayer; section: string; value: unknown; injectorIds?: readonly string[] };
type MergeFragmentEvent = { type: "mergeFragment"; path: string; broken: boolean; section?: string; value?: unknown };
type ExpandMergeKeyEvent = { type: "expandMergeKey"; key: string; parentValue: unknown; invalidScalar: boolean };
type ApplyBootstrapEvent = { type: "applyBootstrap"; schemaVersionHigher: boolean; unknownKeys: UnknownKeyPolicy };
type ClaimSectionsEvent = { type: "claimSections"; keys: readonly string[] };
type LoadFailureEvent = { type: "loadFailure"; stage: "yaml-parse" | "io" | "template-render" };

export type ConfigLoadEvent =
  | LoadLayerEvent
  | { type: "readLocalOverride" }
  | { type: "applySecretEnv"; keys: number }
  | MergeFragmentEvent
  | ExpandMergeKeyEvent
  | ApplyBootstrapEvent
  | { type: "includeKeyDetected" }
  | { type: "beginValidate" }
  | ClaimSectionsEvent
  | { type: "finishValidate" }
  | LoadFailureEvent;

export const setupMachine = setup({
  types: {
    context: {} as ConfigLoadContext,
    events: {} as ConfigLoadEvent,
  },
  guards: {
    // guard 按动作执行前的 context 与事件负载求值：同层重复 id 在合并前即拒绝。
    injectorDupInLayer: ({ context, event }) => {
      const e = event as LoadLayerEvent;
      const existing = context.injectorIdsByLayer[e.layer] ?? [];
      const incoming = e.injectorIds ?? [];
      const hasInternalDup = new Set(incoming).size !== incoming.length;
      return hasInternalDup || incoming.some((id) => existing.includes(id));
    },
    fragmentBroken: ({ event }) => (event as MergeFragmentEvent).broken === true,
    mergeKeyInvalid: ({ event }) => (event as ExpandMergeKeyEvent).invalidScalar === true,
    schemaVersionHigherThanCurrent: ({ event }) => (event as ApplyBootstrapEvent).schemaVersionHigher === true,
    // 剩余未知键是否全部被插件认领（guard 自行推算，不依赖同步 assign）。
    unknownCovered: ({ context }) => context.topKeys.every((k) => context.claimed.includes(k)),
    warnPolicy: ({ context }) => context.unknownPolicy === "warn",
  },
  actions: {
    mergeLayer: assign({
      layersApplied: ({ context, event }) => {
        const e = event as LoadLayerEvent;
        return [...context.layersApplied, e.layer];
      },
      effective: ({ context, event }) => {
        const e = event as LoadLayerEvent;
        return { ...context.effective, [e.section]: e.value };
      },
      topKeys: ({ context, event }) => {
        const e = event as LoadLayerEvent;
        return context.topKeys.includes(e.section) ? context.topKeys : [...context.topKeys, e.section];
      },
      injectorIdsByLayer: ({ context, event }) => {
        const e = event as LoadLayerEvent;
        if (!e.injectorIds) return context.injectorIdsByLayer;
        const prev = context.injectorIdsByLayer[e.layer] ?? [];
        return { ...context.injectorIdsByLayer, [e.layer]: [...prev, ...e.injectorIds] };
      },
    }),
    ignoreLocalOverride: assign({
      localIgnored: () => true,
      warnings: ({ context }) => [...context.warnings, "config.local.yaml 已忽略（产品配置仅 config.yaml + secret.env）"],
    }),
    injectSecrets: assign({
      secretCount: ({ context, event }) => context.secretCount + (event as { keys: number }).keys,
    }),
    mergeFragment: assign({
      fragments: ({ context, event }) => {
        const e = event as MergeFragmentEvent;
        return [...context.fragments, e.path];
      },
      effective: ({ context, event }) => {
        const e = event as MergeFragmentEvent;
        if (e.section === undefined) return context.effective;
        return { ...context.effective, [e.section]: e.value };
      },
    }),
    failFragment: assign({
      error: ({ event }) => {
        const e = event as MergeFragmentEvent;
        return `片段解析或校验失败：${e.path}`;
      },
      failClosed: () => true,
    }),
    // merge key 父 mapping 键优先于合并来源。
    applyMergeKey: assign({
      effective: ({ context, event }) => {
        const e = event as ExpandMergeKeyEvent;
        return { ...context.effective, [e.key]: e.parentValue };
      },
    }),
    failMergeKey: assign({
      error: () => "merge key 值须为 mapping 或 mapping 列表，非法类型加载失败",
      failClosed: () => true,
    }),
    applyBootstrap: assign({
      unknownPolicy: ({ event }) => (event as ApplyBootstrapEvent).unknownKeys,
    }),
    failSchemaVersion: assign({
      error: () => "xylitol.schema_version 显式版本高于当前支持版本，加载失败",
      failClosed: () => true,
    }),
    failIncludeKey: assign({
      error: () => "顶层 include 已移除：请迁移到同层 conf.d/*.yaml 片段",
      failClosed: () => true,
    }),
    failDuplicateInjector: assign({
      error: ({ event }) => {
        const e = event as LoadLayerEvent;
        return `context_inject 同层注入器 id 重复：${(e.injectorIds ?? []).join("、")}`;
      },
      failClosed: () => true,
    }),
    snapshotSchema: assign({ schemaExported: () => true }),
    claimSections: assign({
      claimed: ({ context, event }) => [...new Set([...context.claimed, ...(event as ClaimSectionsEvent).keys])],
    }),
    // 进入 ready 前补齐 Settings 缺省：mode 字段缺省 one-at-a-time。
    applyDefaults: assign({
      effective: ({ context }) => ({
        ...context.effective,
        steering_mode: (context.effective["steering_mode"] as string | undefined) ?? "one-at-a-time",
        follow_up_mode: (context.effective["follow_up_mode"] as string | undefined) ?? "one-at-a-time",
      }),
    }),
    downgradeUnknownToWarnings: assign({
      warnings: ({ context }) => [
        ...context.warnings,
        ...context.topKeys
          .filter((k) => !context.claimed.includes(k))
          .map((k) => `未知顶层键 ${k}：未由任何插件认领，已按 warn 策略降级为警告`),
      ],
    }),
    failUnknownKeys: assign({
      error: ({ context }) => {
        const unknown = context.topKeys.filter((k) => !context.claimed.includes(k));
        return `未知顶层键 ${unknown.join("、")}：未由插件注册表认领，请迁移到对应配置节`;
      },
      failClosed: () => true,
    }),
    failLoad: assign({
      error: ({ event }) => {
        const e = event as LoadFailureEvent;
        return `配置加载失败（${e.stage}）：fail-closed，不回退环境变量默认`;
      },
      failClosed: () => true,
    }),
  },
});

export const machine = setupMachine.createMachine({
  id: "config-load-merge",
  initial: "unloaded",
  context: initialContext,
  states: {
    unloaded: {
      on: {
        loadLayer: [
          { guard: "injectorDupInLayer", target: "failed", actions: "failDuplicateInjector" },
          { target: "merging", actions: "mergeLayer" },
        ],
        loadFailure: { target: "failed", actions: "failLoad" },
      },
    },
    merging: {
      on: {
        loadLayer: [
          { guard: "injectorDupInLayer", target: "failed", actions: "failDuplicateInjector" },
          { actions: "mergeLayer" },
        ],
        readLocalOverride: { actions: "ignoreLocalOverride" },
        applySecretEnv: { actions: "injectSecrets" },
        mergeFragment: [
          { guard: "fragmentBroken", target: "failed", actions: "failFragment" },
          { actions: "mergeFragment" },
        ],
        expandMergeKey: [
          { guard: "mergeKeyInvalid", target: "failed", actions: "failMergeKey" },
          { actions: "applyMergeKey" },
        ],
        applyBootstrap: [
          { guard: "schemaVersionHigherThanCurrent", target: "failed", actions: "failSchemaVersion" },
          { actions: "applyBootstrap" },
        ],
        includeKeyDetected: { target: "failed", actions: "failIncludeKey" },
        beginValidate: { target: "validating", actions: "snapshotSchema" },
        loadFailure: { target: "failed", actions: "failLoad" },
      },
    },
    validating: {
      on: {
        claimSections: { actions: "claimSections" },
        finishValidate: [
          { guard: "unknownCovered", target: "ready", actions: "applyDefaults" },
          { guard: "warnPolicy", target: "ready", actions: "downgradeUnknownToWarnings" },
          { target: "failed", actions: "failUnknownKeys" },
        ],
      },
    },
    ready: { type: "final" },
    failed: { type: "final" },
  },
});

export const scenarioGroups = {
  "三层顺序深合并与片段": [
    { name: "global 层 models 节", events: [{ type: "loadLayer", layer: "global", section: "models", value: { family: "anthropic" } }] },
    { name: "global conf.d 片段升序合并并覆盖层值", events: [{ type: "mergeFragment", path: "~/.config/xylitol/conf.d/10-models.yaml", broken: false, section: "models", value: { family: "openai" } }] },
    { name: "注入 secret.env 密钥", events: [{ type: "applySecretEnv", keys: 2 }], expectContext: { secretCount: 2 } },
    { name: "config.local.yaml 被忽略", events: [{ type: "readLocalOverride" }], expectContext: { localIgnored: true } },
    { name: "project 层 context_inject 与 cli 层 tools", events: [{ type: "loadLayer", layer: "project", section: "context_inject", value: { injectors: [{ id: "drift" }] }, injectorIds: ["drift"] }, { type: "loadLayer", layer: "cli", section: "tools", value: ["read"] }] },
    { name: "project conf.d 片段", events: [{ type: "mergeFragment", path: ".xylitol/conf.d/20-inject.yaml", broken: false }] },
    { name: "引导段缺省按 error 策略", events: [{ type: "applyBootstrap", schemaVersionHigher: false, unknownKeys: "error" }] },
    { name: "进入两阶段校验并快照 schema", events: [{ type: "beginValidate" }, { type: "claimSections", keys: ["models", "context_inject", "tools", "xylitol"] }, { type: "finishValidate" }], expectStates: ["ready"], expectContext: { schemaExported: true, fragments: ["~/.config/xylitol/conf.d/10-models.yaml", ".xylitol/conf.d/20-inject.yaml"], layersApplied: ["global", "project", "cli"] }, expect: (snapshot) => { const ctx = (snapshot as { context: ConfigLoadContext }).context; const models = ctx.effective["models"] as { family?: string } | undefined; if (models?.family !== "openai") return "conf.d 片段应覆盖 global 层 models.family"; if (ctx.effective["steering_mode"] !== "one-at-a-time") return "mode 字段缺省应为 one-at-a-time"; return true; } },
  ],
  "yaml merge key 父键优先": [
    { name: "父 mapping 键覆盖合并来源", events: [{ type: "loadLayer", layer: "global", section: "models", value: { family: "openai" } }, { type: "expandMergeKey", key: "models", parentValue: { family: "anthropic" }, invalidScalar: false }], expectStates: ["merging"], expect: (snapshot) => { const ctx = (snapshot as { context: ConfigLoadContext }).context; const models = ctx.effective["models"] as { family?: string } | undefined; return models?.family === "anthropic" ? true : "merge key 父键应优先于合并来源"; } },
    { name: "非法标量值硬失败", events: [{ type: "expandMergeKey", key: "models", parentValue: 1, invalidScalar: true }], expectStates: ["failed"], expectContext: { failClosed: true }, expect: (snapshot) => { const ctx = (snapshot as { context: ConfigLoadContext }).context; return ctx.error !== null && ctx.error.includes("merge key") ? true : `错误应含 merge key，实得 ${ctx.error}`; } },
  ],
  "未知键两阶段校验硬失败": [
    { name: "加载期收集未知顶层键", events: [{ type: "loadLayer", layer: "global", section: "models", value: {} }, { type: "loadLayer", layer: "global", section: "plugin_zoo", value: {} }] },
    { name: "无插件认领时校验硬失败", events: [{ type: "applyBootstrap", schemaVersionHigher: false, unknownKeys: "error" }, { type: "beginValidate" }, { type: "claimSections", keys: ["models", "xylitol"] }, { type: "finishValidate" }], expectStates: ["failed"], expectContext: { failClosed: true }, expect: (snapshot) => { const ctx = (snapshot as { context: ConfigLoadContext }).context; return ctx.error !== null && ctx.error.includes("plugin_zoo") && ctx.error.includes("迁移") ? true : `错误应含键名与迁移提示（r3075），实得 ${ctx.error}`; } },
  ],
  "未知键 warn 降级为警告": [
    { name: "warn 策略下装配成功", events: [{ type: "loadLayer", layer: "global", section: "models", value: {} }, { type: "loadLayer", layer: "global", section: "plugin_zoo", value: {} }, { type: "applyBootstrap", schemaVersionHigher: false, unknownKeys: "warn" }, { type: "beginValidate" }, { type: "claimSections", keys: ["models", "xylitol"] }, { type: "finishValidate" }], expectStates: ["ready"], expect: (snapshot) => { const ctx = (snapshot as { context: ConfigLoadContext }).context; return ctx.warnings.some((w) => w.includes("plugin_zoo")) ? true : `警告应列出 plugin_zoo，实得 ${JSON.stringify(ctx.warnings)}`; } },
  ],
  "context_inject 同层重复 id 报错": [
    { name: "同层两条同 id 注入器校验报错", events: [{ type: "loadLayer", layer: "project", section: "context_inject", value: {}, injectorIds: ["drift", "drift"] }], expectStates: ["failed"], expectContext: { failClosed: true }, expect: (snapshot) => { const ctx = (snapshot as { context: ConfigLoadContext }).context; return ctx.error !== null && ctx.error.includes("重复") ? true : `错误应指出重复 id，实得 ${ctx.error}`; } },
  ],
  "include 键出现即硬失败": [
    { name: "顶层 include 键拒绝并给迁移提示", events: [{ type: "loadLayer", layer: "global", section: "models", value: {} }, { type: "includeKeyDetected" }], expectStates: ["failed"], expect: (snapshot) => { const ctx = (snapshot as { context: ConfigLoadContext }).context; return ctx.error !== null && ctx.error.includes("include") && ctx.error.includes("conf.d") ? true : `错误应含 include 与迁移方向，实得 ${ctx.error}`; } },
  ],
  "配置损坏 fail-closed": [
    { name: "YAML 解析失败硬错误", events: [{ type: "loadLayer", layer: "global", section: "models", value: {} }, { type: "loadFailure", stage: "yaml-parse" }], expectStates: ["failed"], expectContext: { failClosed: true }, expect: (snapshot) => { const ctx = (snapshot as { context: ConfigLoadContext }).context; return ctx.error !== null && ctx.error.includes("fail-closed") ? true : `错误应写明 fail-closed，实得 ${ctx.error}`; } },
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
