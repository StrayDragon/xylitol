/**
 * 插件装配与启停状态机（Plugin Assembly & Toggle）。
 *
 * 业务：外进程插件经 plugins/bin 发现与 --describe 清单识别；发现不等于装配，
 * 首次工具表纳入前须请求用户批准并持久化；未批准的插件零装配成本；
 * config 显式停用（plugins.disabled）优先级最高，可随时覆盖既有批准；
 * 清单声明 mcp 的插件注册为 MCP server，工具统一经 mcp__ 管线暴露；
 * 声明 commands 能力的插件注册为 plugin:<name>，委派透传并以文本回传，
 * 未注册命令、超时与非零退出须返回可观察错误；多插件争同一 config_key 可观察失败；
 * hooks 未配置时不启动脚本进程；MCP 未配置时不注册 mcp 前缀工具。
 *
 * @spec llmanspec/specs/infra-plugin/infra-plugin.feature#@req:r2990（发现与首用批准：未批准零装配、config 停用最高优先）
 * @spec llmanspec/specs/infra-plugin/infra-plugin.feature#@req:r3377（清单 mcp 声明注册：未声明不注册、停用覆盖批准）
 * @spec llmanspec/specs/infra-plugin/infra-plugin.feature#@req:r3027（plugin:<name> 命令注册与可观察错误）
 * @spec llmanspec/specs/infra-plugin/infra-plugin.feature#@req:r3074（plugins.enabled/disabled 收敛与 config_key 冲突失败）
 * @spec llmanspec/specs/infra-plugin/infra-plugin.feature#@req:r3317（run_command_capture：超时中止、非零退出报错）
 * @spec llmanspec/specs/workspace-assembly/workspace-assembly.feature#@req:r2911（未配置 MCP 不注册 mcp 前缀工具）
 * @spec llmanspec/specs/workspace-assembly/workspace-assembly.feature#@req:r2912（未配置 hooks 不启动脚本进程）
 * @spec llmanspec/specs/workspace-assembly/workspace-assembly.feature#@req:r2916（装配在 kernel：根包只选 profile）
 * @spec llmanspec/specs/cli-plugin-command/cli-plugin-command.feature#@req:r3358（plugins/bin 发现失败不阻断内置命令）
 * @spec llmanspec/specs/cli-plugin-command/cli-plugin-command.feature#@req:r3360（--describe 清单 JSON，失败只告警）
 * @spec llmanspec/specs/cli-plugin-command/cli-plugin-command.feature#@req:r3361（requires_host 时注入 host 交接环境）
 *
 * 实现落点：crates/xylitol-kernel/src/external_plugin.rs（发现/批准/停用）、crates/xylitol-kernel/src/assembly.rs（内核装配）、crates/xylitol-kernel/src/boot.rs（Kernel::boot）。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** 批准状态：none 未批准 / approved 已批准并持久化 / disabled 由 config 显式停用。 */
export type PluginApproval = "none" | "approved" | "disabled";

//** 插件清单（--describe JSON 的形状面）。 **/
export interface PluginManifest {
  name: string;
  declaresMcp: boolean;
  requiresHost: boolean;
  commands: readonly string[];
}

export interface PluginAssemblyContext {
  /** plugins/bin 发现并解析成功的清单。 */
  discovered: PluginManifest[];
  /** describe 失败仅告警的插件。 */
  describeWarnings: string[];
  /** 首用批准表（approved.json 持久化语义）。 */
  approvals: Record<string, PluginApproval>;
  /** config plugins.enabled / plugins.disabled。 */
  configEnabled: string[];
  configDisabled: string[];
  /** MCP / hooks 配置是否非空。 */
  mcpConfigured: boolean;
  hooksConfigured: boolean;
  /** config_key 认领表（key → 插件名）。 */
  configKeyClaims: Record<string, string>;
  /** 装配产物：mcp__ 前缀工具。 */
  mcpTools: string[];
  /** 装配产物：plugin:<name> 命令路由。 */
  commandRoutes: string[];
  /** 已启动的 hook 脚本进程数（空配置须为 0）。 */
  hookProcesses: number;
  /** 分发工具调用次数（结果为允许）。 */
  dispatchAllowed: number;
  /** 命令委派成功次数。 */
  delegations: number;
  /** requires_host 交接时是否注入了 host 环境事实。 */
  hostEnvInjected: boolean;
  /** 最近一次命令委派的可观察错误。 */
  lastCommandError: string | null;
  /** 装配是否由 kernel 承担（根包只选 profile）。 */
  assembledByKernel: boolean;
  /** 装配诊断与失败原因。 */
  diagnostics: string[];
  /** 装配硬失败原因；null 表示未失败。 */
  assemblyError: string | null;
}

export const initialContext: PluginAssemblyContext = {
  discovered: [],
  describeWarnings: [],
  approvals: {},
  configEnabled: [],
  configDisabled: [],
  mcpConfigured: false,
  hooksConfigured: false,
  configKeyClaims: {},
  mcpTools: [],
  commandRoutes: [],
  hookProcesses: 0,
  dispatchAllowed: 0,
  delegations: 0,
  hostEnvInjected: false,
  lastCommandError: null,
  assembledByKernel: false,
  diagnostics: [],
  assemblyError: null,
};

export const meta = defineMeta({
  name: "plugin-assembly",
  pkg: "config-meta",
  purpose: "外进程插件装配与启停：批准持久化、config 停用最高优先、mcp 声明与命令委派",
  anchors: [
    "llmanspec/specs/infra-plugin/infra-plugin.feature#@req:r2990",
    "llmanspec/specs/infra-plugin/infra-plugin.feature#@req:r3377",
    "llmanspec/specs/infra-plugin/infra-plugin.feature#@req:r3027",
    "llmanspec/specs/infra-plugin/infra-plugin.feature#@req:r3074",
    "llmanspec/specs/infra-plugin/infra-plugin.feature#@req:r3317",
    "llmanspec/specs/workspace-assembly/workspace-assembly.feature#@req:r2911",
    "llmanspec/specs/workspace-assembly/workspace-assembly.feature#@req:r2912",
    "llmanspec/specs/workspace-assembly/workspace-assembly.feature#@req:r2916",
    "llmanspec/specs/cli-plugin-command/cli-plugin-command.feature#@req:r3358",
    "llmanspec/specs/cli-plugin-command/cli-plugin-command.feature#@req:r3360",
    "llmanspec/specs/cli-plugin-command/cli-plugin-command.feature#@req:r3361",
  ],
  impl: ["crates/xylitol-kernel/src/external_plugin.rs", "crates/xylitol-kernel/src/assembly.rs", "crates/xylitol-kernel/src/boot.rs"],
});

type ManifestEvent = { type: "manifest"; name: string; declaresMcp: boolean; requiresHost: boolean; commands: readonly string[] };
type ClaimKeyEvent = { type: "claimConfigKey"; plugin: string; key: string };
type PluginConfigEvent = { type: "applyPluginConfig"; enabled: readonly string[]; disabled: readonly string[]; mcpConfigured: boolean; hooksConfigured: boolean };
type InvokeCommandEvent = { type: "invokeCommand"; plugin: string; command: string };
type CommandFailedEvent = { type: "commandFailed"; plugin: string; reason: "timeout" | "nonzero-exit" };

export type PluginAssemblyEvent =
  | ManifestEvent
  | { type: "describeFailed"; name: string }
  | ClaimKeyEvent
  | PluginConfigEvent
  | { type: "approve"; name: string }
  | { type: "finishDiscovery" }
  | { type: "reassemble" }
  | { type: "deactivate"; name: string }
  | InvokeCommandEvent
  | CommandFailedEvent
  | { type: "delegateWithHost"; plugin: string }
  | { type: "dispatchToolCall" };

export const setupMachine = setup({
  types: {
    context: {} as PluginAssemblyContext,
    events: {} as PluginAssemblyEvent,
  },
  guards: {
    // 多个插件声明同一 config_key：guard 按动作执行前的认领表求值。
    configKeyTaken: ({ context, event }) => {
      const e = event as ClaimKeyEvent;
      const owner = context.configKeyClaims[e.key];
      return owner !== undefined && owner !== e.plugin;
    },
    commandRegistered: ({ context, event }) => {
      const e = event as InvokeCommandEvent;
      const manifest = context.discovered.find((p) => p.name === e.plugin);
      return context.commandRoutes.includes(`plugin:${e.plugin}`) && (manifest?.commands.includes(e.command) ?? false);
    },
    hooksPresent: ({ context }) => context.hooksConfigured,
  },
  actions: {
    recordManifest: assign({
      discovered: ({ context, event }) => {
        const e = event as ManifestEvent;
        return [...context.discovered, { name: e.name, declaresMcp: e.declaresMcp, requiresHost: e.requiresHost, commands: [...e.commands] }];
      },
    }),
    warnDescribeFailure: assign({
      describeWarnings: ({ context, event }) => [
        ...context.describeWarnings,
        `${(event as { name: string }).name} --describe 失败：仅告警不阻断发现`,
      ],
    }),
    recordConfigKey: assign({
      configKeyClaims: ({ context, event }) => {
        const e = event as ClaimKeyEvent;
        return { ...context.configKeyClaims, [e.key]: e.plugin };
      },
    }),
    failConfigKeyConflict: assign({
      assemblyError: ({ event }) => {
        const e = event as ClaimKeyEvent;
        return `config_key ${e.key} 已由 ${e.plugin} 之外的其他插件认领：可观察失败`;
      },
    }),
    applyPluginConfig: assign({
      configEnabled: ({ event }) => [...(event as PluginConfigEvent).enabled],
      configDisabled: ({ event }) => [...(event as PluginConfigEvent).disabled],
      mcpConfigured: ({ event }) => (event as PluginConfigEvent).mcpConfigured,
      hooksConfigured: ({ event }) => (event as PluginConfigEvent).hooksConfigured,
    }),
    // 首用批准：持久化进 approvals 表；停用覆盖由 assemblePlugins / applyConfigDeactivate 体现。
    approve: assign({
      approvals: ({ context, event }) => ({ ...context.approvals, [(event as { name: string }).name]: "approved" as const }),
    }),
    // 装配决策全部从当前 context 推导：停用 > 批准 > mcp 声明。
    assemblePlugins: assign({
      mcpTools: ({ context }) => {
        if (!context.mcpConfigured) return [];
        return context.discovered
          .filter((p) => !context.configDisabled.includes(p.name))
          .filter((p) => (context.approvals[p.name] ?? "none") === "approved")
          .filter((p) => p.declaresMcp)
          .map((p) => `mcp__${p.name}`);
      },
      commandRoutes: ({ context }) =>
        context.discovered
          .filter((p) => !context.configDisabled.includes(p.name))
          .filter((p) => (context.approvals[p.name] ?? "none") === "approved")
          .filter((p) => p.commands.length > 0)
          .map((p) => `plugin:${p.name}`),
      assembledByKernel: () => true,
      diagnostics: ({ context }) => {
        const skipped = context.discovered
          .filter((p) => context.configDisabled.includes(p.name))
          .map((p) => `${p.name} 已被 config 显式停用（优先级最高，覆盖批准）`);
        const unapproved = context.discovered
          .filter((p) => !context.configDisabled.includes(p.name) && (context.approvals[p.name] ?? "none") !== "approved")
          .map((p) => `${p.name} 未批准：不注册任何工具（零装配成本）`);
        return [...context.diagnostics, ...skipped, ...unapproved];
      },
    }),
    // config 停用可随时生效：移除该插件已注册的工具与命令路由。
    applyConfigDeactivate: assign({
      configDisabled: ({ context, event }) => [...context.configDisabled, (event as { name: string }).name],
      mcpTools: ({ context, event }) => context.mcpTools.filter((t) => t !== `mcp__${(event as { name: string }).name}`),
      commandRoutes: ({ context, event }) => context.commandRoutes.filter((r) => r !== `plugin:${(event as { name: string }).name}`),
      diagnostics: ({ context, event }) => [
        ...context.diagnostics,
        `config 停用 ${(event as { name: string }).name}：覆盖既有批准`,
      ],
    }),
    delegateCommand: assign({
      delegations: ({ context }) => context.delegations + 1,
    }),
    failUnknownCommand: assign({
      lastCommandError: ({ event }) => {
        const e = event as InvokeCommandEvent;
        return `未注册命令 ${e.plugin}/${e.command}：返回可观察错误而非静默成功`;
      },
    }),
    recordCommandFailure: assign({
      lastCommandError: ({ event }) => {
        const e = event as CommandFailedEvent;
        return `${e.plugin} 命令失败（${e.reason}）：超时中止与非零退出须报错`;
      },
    }),
    injectHostEnv: assign({
      hostEnvInjected: ({ context, event }) => {
        const e = event as { plugin: string };
        return context.discovered.find((p) => p.name === e.plugin)?.requiresHost === true;
      },
    }),
    spawnHook: assign({
      hookProcesses: ({ context }) => context.hookProcesses + 1,
      dispatchAllowed: ({ context }) => context.dispatchAllowed + 1,
    }),
    // 空 hooks 配置：分发结果为允许且不启动任何脚本进程。
    allowWithoutHook: assign({
      dispatchAllowed: ({ context }) => context.dispatchAllowed + 1,
    }),
  },
});

export const machine = setupMachine.createMachine({
  id: "plugin-assembly",
  initial: "discovering",
  context: initialContext,
  states: {
    discovering: {
      on: {
        manifest: { actions: "recordManifest" },
        describeFailed: { actions: "warnDescribeFailure" },
        claimConfigKey: [
          { guard: "configKeyTaken", target: "failed", actions: "failConfigKeyConflict" },
          { actions: "recordConfigKey" },
        ],
        applyPluginConfig: { actions: "applyPluginConfig" },
        approve: { actions: "approve" },
        finishDiscovery: { target: "active", actions: "assemblePlugins" },
      },
    },
    active: {
      on: {
        approve: { actions: "approve" },
        reassemble: { actions: "assemblePlugins" },
        deactivate: { actions: "applyConfigDeactivate" },
        invokeCommand: [
          { guard: "commandRegistered", actions: "delegateCommand" },
          { actions: "failUnknownCommand" },
        ],
        commandFailed: { actions: "recordCommandFailure" },
        delegateWithHost: { actions: "injectHostEnv" },
        dispatchToolCall: [
          { guard: "hooksPresent", actions: "spawnHook" },
          { actions: "allowWithoutHook" },
        ],
      },
    },
    failed: { type: "final" },
  },
});

export const scenarioGroups = {
  "发现批准与 mcp 装配": [
    { name: "发现两清单并批准 demo", events: [{ type: "manifest", name: "demo", declaresMcp: true, requiresHost: false, commands: ["ui"] }, { type: "manifest", name: "util", declaresMcp: false, requiresHost: false, commands: [] }, { type: "approve", name: "demo" }, { type: "applyPluginConfig", enabled: [], disabled: [], mcpConfigured: true, hooksConfigured: false }, { type: "finishDiscovery" }], expectStates: ["active"], expectContext: { mcpTools: ["mcp__demo"], commandRoutes: ["plugin:demo"], assembledByKernel: true, approvals: { demo: "approved" } } },
  ],
  "未批准零装配": [
    { name: "未批准插件不注册任何工具", events: [{ type: "manifest", name: "demo", declaresMcp: true, requiresHost: false, commands: ["ui"] }, { type: "applyPluginConfig", enabled: [], disabled: [], mcpConfigured: true, hooksConfigured: false }, { type: "finishDiscovery" }], expectStates: ["active"], expectContext: { mcpTools: [], commandRoutes: [], assembledByKernel: true }, expect: (snapshot) => { const ctx = (snapshot as { context: PluginAssemblyContext }).context; return ctx.diagnostics.some((d) => d.includes("未批准")) ? true : `应留未批准诊断，实得 ${JSON.stringify(ctx.diagnostics)}`; } },
  ],
  "config 停用优先级最高": [
    { name: "装配期 disabled 拦截已批准插件", events: [{ type: "manifest", name: "demo", declaresMcp: true, requiresHost: false, commands: ["ui"] }, { type: "approve", name: "demo" }, { type: "applyPluginConfig", enabled: [], disabled: ["demo"], mcpConfigured: true, hooksConfigured: false }, { type: "finishDiscovery" }], expectStates: ["active"], expectContext: { mcpTools: [], commandRoutes: [] }, expect: (snapshot) => { const ctx = (snapshot as { context: PluginAssemblyContext }).context; return ctx.diagnostics.some((d) => d.includes("停用")) ? true : `应留停用诊断，实得 ${JSON.stringify(ctx.diagnostics)}`; } },
    { name: "运行期随时停用并移除已注册工具", events: [{ type: "manifest", name: "live", declaresMcp: true, requiresHost: false, commands: [] }, { type: "approve", name: "live" }, { type: "applyPluginConfig", enabled: [], disabled: [], mcpConfigured: true, hooksConfigured: false }, { type: "finishDiscovery" }, { type: "deactivate", name: "live" }], expectContext: { mcpTools: [], configDisabled: ["demo", "live"] } },
  ],
  "未声明 mcp 与空 hooks 零装配": [
    { name: "未声明 mcp 不注册工具但命令路由保留", events: [{ type: "manifest", name: "demo", declaresMcp: false, requiresHost: false, commands: ["ui"] }, { type: "approve", name: "demo" }, { type: "applyPluginConfig", enabled: [], disabled: [], mcpConfigured: false, hooksConfigured: false }, { type: "finishDiscovery" }], expectStates: ["active"], expectContext: { mcpTools: [], commandRoutes: ["plugin:demo"] } },
    { name: "空 hooks 分发为允许且零进程", events: [{ type: "dispatchToolCall" }], expectContext: { dispatchAllowed: 1, hookProcesses: 0 } },
  ],
  "命令委派与可观察错误": [
    { name: "已注册命令透传委派", events: [{ type: "manifest", name: "demo", declaresMcp: true, requiresHost: false, commands: ["ui"] }, { type: "approve", name: "demo" }, { type: "applyPluginConfig", enabled: [], disabled: [], mcpConfigured: true, hooksConfigured: false }, { type: "finishDiscovery" }, { type: "invokeCommand", plugin: "demo", command: "ui" }], expectContext: { delegations: 1, lastCommandError: null } },
    { name: "未注册命令返回可观察错误", events: [{ type: "invokeCommand", plugin: "demo", command: "nope" }], expect: (snapshot) => { const ctx = (snapshot as { context: PluginAssemblyContext }).context; return ctx.lastCommandError !== null && ctx.lastCommandError.includes("nope") ? true : `应报未注册命令错误，实得 ${ctx.lastCommandError}`; } },
    { name: "超时与非零退出均报错", events: [{ type: "commandFailed", plugin: "demo", reason: "timeout" }, { type: "commandFailed", plugin: "demo", reason: "nonzero-exit" }], expect: (snapshot) => { const ctx = (snapshot as { context: PluginAssemblyContext }).context; return ctx.lastCommandError !== null && ctx.lastCommandError.includes("nonzero-exit") ? true : `应报最近一次失败，实得 ${ctx.lastCommandError}`; } },
  ],
  "config_key 冲突失败": [
    { name: "两个插件争同一 config_key 可观察失败", events: [{ type: "manifest", name: "alpha", declaresMcp: false, requiresHost: false, commands: [] }, { type: "manifest", name: "beta", declaresMcp: false, requiresHost: false, commands: [] }, { type: "claimConfigKey", plugin: "alpha", key: "tools" }, { type: "claimConfigKey", plugin: "beta", key: "tools" }], expectStates: ["failed"], expect: (snapshot) => { const ctx = (snapshot as { context: PluginAssemblyContext }).context; return ctx.assemblyError !== null && ctx.assemblyError.includes("tools") ? true : `应报 config_key 冲突，实得 ${ctx.assemblyError}`; } },
  ],
  "describe 失败告警与 host 交接": [
    { name: "describe 失败仅告警不阻断", events: [{ type: "describeFailed", name: "broken" }, { type: "manifest", name: "app", declaresMcp: true, requiresHost: true, commands: [] }, { type: "approve", name: "app" }, { type: "applyPluginConfig", enabled: [], disabled: [], mcpConfigured: true, hooksConfigured: false }, { type: "finishDiscovery" }], expectStates: ["active"], expectContext: { mcpTools: ["mcp__app"], hostEnvInjected: false }, expect: (snapshot) => { const ctx = (snapshot as { context: PluginAssemblyContext }).context; return ctx.describeWarnings.some((w) => w.includes("broken")) ? true : `应告警 describe 失败，实得 ${JSON.stringify(ctx.describeWarnings)}`; } },
    { name: "requires_host 清单交接时注入 host 环境", events: [{ type: "delegateWithHost", plugin: "app" }], expectContext: { hostEnvInjected: true } },
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
