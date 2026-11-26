/**
 * 启动装配状态机（Boot Assembly：argv → profile → Kernel::boot）。
 *
 * 业务：CLI 统一入口区分表面动词（tui / print）与管理动词（serve）：
 * TTY 裸跑默认进入产品 TUI；print 为 app 独立表面的子模式；
 * 表面旗标只挂表面动词下，顶层旗标解析失败；--trust 与 --no-trust 同传显式报错非零退出；
 * 配置加载失败 fail-closed（非零退出，不回退环境变量默认）；
 * 零显式模型条目时硬失败（no-silent-fallback）；
 * cli 与 Host 监听器须经共享 Kernel::boot 装配（同一 profile 路径），
 * 组合根端口（SessionStore / EventSink）从 infra 注入；
 * 产品 slash 与 GetCommands 内建表同源同一产品命令 SSOT，废弃 pi 短名不解析，
 * idle slash 经 commands → effects → Driver 共享派发路径。
 *
 * @spec llmanspec/specs/cli-entry/cli-entry.feature#@req:r1381（TTY 且无表面子命令默认进产品 TUI）
 * @spec llmanspec/specs/cli-entry/cli-entry.feature#@req:r1385（表面动词与管理动词区分，serve 顶层）
 * @spec llmanspec/specs/cli-entry/cli-entry.feature#@req:r1379（CLI 独立应用表面，print 为子模式）
 * @spec llmanspec/specs/cli-entry/cli-entry.feature#@req:r1386（config-load-fail-closed）
 * @spec llmanspec/specs/cli-entry/cli-entry.feature#@req:r1389（no-silent-fallback：零显式模型硬失败）
 * @spec llmanspec/specs/cli-entry/cli-entry.feature#@req:r1392（组合根端口从 infra 构造注入）
 * @spec llmanspec/specs/cli-entry/cli-entry.feature#@req:r1394（cli 与 Host 须经共享 Kernel::boot，不内联第二套装配）
 * @spec llmanspec/specs/cli-entry/cli-entry.feature#@req:r1388（表面旗标挂在表面动词下，顶层旗标解析失败）
 * @spec llmanspec/specs/cli-entry/cli-entry.feature#@req:r3352（trust 旗标同传显式报错非零退出）
 * @spec llmanspec/specs/cli-entry/cli-entry.feature#@req:r67（产品 slash 与内建表同源同一 SSOT）
 * @spec llmanspec/specs/cli-entry/cli-entry.feature#@req:r69（废弃 pi 短名不解析）
 * @spec llmanspec/specs/cli-entry/cli-entry.feature#@req:r70（idle slash 经共享派发路径）
 *
 * 实现落点：src/cli.rs（argv 与 profile 选择）、crates/xylitol-kernel/src/boot.rs（Kernel::boot）、crates/xylitol-kernel/src/composition.rs（组合根端口注入）。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

export type CliSurface = "tui" | "print" | "serve";

export interface BootContext {
  /** 解析出的动词（tui / print / serve）。 */
  verb: string | null;
  /** 解析出的表面与管理动词。 */
  surface: CliSurface | null;
  /** 是否 TTY。 */
  tty: boolean;
  /** trust 旗标冲突。 */
  trustConflict: boolean;
  /** 配置加载是否成功。 */
  configOk: boolean;
  /** 显式模型条目数。 */
  explicitModels: number;
  /** boot 装配路径。 */
  bootMode: "kernel-boot" | "inline" | null;
  /** 组合根端口（SessionStore / EventSink）是否已注入。 */
  portsInjected: boolean;
  /** 产品命令 SSOT 内建表（与 GetCommands 同源）。 */
  builtinCommands: string[];
  /** 待发命令（共享派发路径产出）。 */
  pendingCommand: string | null;
  /** 共享路径派发次数。 */
  dispatches: number;
  /** 废弃短名拒绝次数。 */
  rejectedSlash: number;
  /** 硬失败原因；null 表示未失败。 */
  error: string | null;
  /** 进程退出码（null = 未退出）。 */
  exitCode: number | null;
}

export const initialContext: BootContext = {
  verb: null,
  surface: null,
  tty: true,
  trustConflict: false,
  configOk: false,
  explicitModels: 0,
  bootMode: null,
  portsInjected: false,
  builtinCommands: ["/model", "/exit", "/session-tree", "/session-resume", "/session-new"],
  pendingCommand: null,
  dispatches: 0,
  rejectedSlash: 0,
  error: null,
  exitCode: null,
};

export const meta = defineMeta({
  name: "boot-assembly",
  pkg: "config-meta",
  purpose: "启动装配：argv → profile → Kernel::boot、表面与 ops 区分、no-silent-fallback",
  anchors: [
    "llmanspec/specs/cli-entry/cli-entry.feature#@req:r1381",
    "llmanspec/specs/cli-entry/cli-entry.feature#@req:r1385",
    "llmanspec/specs/cli-entry/cli-entry.feature#@req:r1379",
    "llmanspec/specs/cli-entry/cli-entry.feature#@req:r1386",
    "llmanspec/specs/cli-entry/cli-entry.feature#@req:r1389",
    "llmanspec/specs/cli-entry/cli-entry.feature#@req:r1392",
    "llmanspec/specs/cli-entry/cli-entry.feature#@req:r1394",
    "llmanspec/specs/cli-entry/cli-entry.feature#@req:r1388",
    "llmanspec/specs/cli-entry/cli-entry.feature#@req:r3352",
    "llmanspec/specs/cli-entry/cli-entry.feature#@req:r67",
    "llmanspec/specs/cli-entry/cli-entry.feature#@req:r69",
    "llmanspec/specs/cli-entry/cli-entry.feature#@req:r70",
  ],
  impl: ["src/cli.rs", "crates/xylitol-kernel/src/boot.rs", "crates/xylitol-kernel/src/composition.rs"],
});

type ParseArgvEvent = { type: "parseArgv"; verb: string | null; tty: boolean; topLevelSurfaceFlag: boolean; trust: boolean; noTrust: boolean };

export type BootEvent =
  | ParseArgvEvent
  | { type: "loadConfig"; ok: boolean }
  | { type: "checkModels"; explicitEntries: number }
  | { type: "bootKernel"; mode: "kernel-boot" | "inline" }
  | { type: "slashInput"; name: string };

const SURFACE_VERBS: readonly string[] = ["tui", "print", "serve"];

export const setupMachine = setup({
  types: {
    context: {} as BootContext,
    events: {} as BootEvent,
  },
  guards: {
    // r3352：同传 --trust 与 --no-trust（含父子命令合并后同真）须显式报错。
    trustFlagsConflict: ({ event }) => (event as ParseArgvEvent).trust === true && (event as ParseArgvEvent).noTrust === true,
    // r1388：顶层不得提供表面旗标，解析须失败。
    topLevelSurfaceFlag: ({ event }) => (event as ParseArgvEvent).topLevelSurfaceFlag === true,
    // r1385：不得提供 tokenizer / resources / preview / designing 等顶层动词。
    forbiddenVerb: ({ event }) => {
      const e = event as ParseArgvEvent;
      return e.verb !== null && !SURFACE_VERBS.includes(e.verb);
    },
    // r1381：有动词或 TTY 裸跑均可解析出表面。
    resolvesToSurface: ({ event }) => {
      const e = event as ParseArgvEvent;
      return e.verb !== null || e.tty === true;
    },
    configLoadFailed: ({ event }) => (event as { ok: boolean }).ok === false,
    // r1389：零显式模型条目即硬失败，不得凭 env 默认继续。
    noExplicitModels: ({ event }) => (event as { explicitEntries: number }).explicitEntries === 0,
    // r1394：不得内联第二套装配。
    inlineAssembly: ({ event }) => (event as { mode: string }).mode === "inline",
    // guard 读动作执行前的 surface（bootKernel 的动作尚未执行）。
    surfaceIsTui: ({ context }) => context.surface === "tui",
    surfaceIsPrint: ({ context }) => context.surface === "print",
    surfaceIsServe: ({ context }) => context.surface === "serve",
    // r67/r69：内建表同一产品命令 SSOT，废弃短名不在表内。
    inBuiltinTable: ({ context, event }) => context.builtinCommands.includes((event as { name: string }).name),
  },
  actions: {
    resolveVerb: assign({
      verb: ({ event }) => {
        const e = event as ParseArgvEvent;
        return e.verb ?? (e.tty ? "tui" : null);
      },
      surface: ({ event }) => {
        const e = event as ParseArgvEvent;
        if (e.verb !== null && SURFACE_VERBS.includes(e.verb)) {
          return e.verb as CliSurface;
        }
        return e.verb === null && e.tty ? "tui" : null;
      },
      tty: ({ event }) => (event as ParseArgvEvent).tty,
    }),
    failNoSurface: assign({
      error: () => "非 TTY 且无表面动词：须显式 tui / print / serve",
      exitCode: () => 2,
    }),
    failTrustConflict: assign({
      trustConflict: () => true,
      error: () => "--trust 与 --no-trust 同传：显式报错退出（未进入 trust 门禁）",
      exitCode: () => 2,
    }),
    failTopLevelFlag: assign({
      error: () => "表面旗标不得挂在顶层：请挂在 tui / print 子命令下",
      exitCode: () => 2,
    }),
    failForbiddenVerb: assign({
      error: ({ event }) => {
        const e = event as ParseArgvEvent;
        return `未知顶层动词 ${e.verb ?? ""}：不得提供 tokenizer / resources / preview 等顶层动词`;
      },
      exitCode: () => 2,
    }),
    markConfigLoaded: assign({
      configOk: () => true,
    }),
    failConfigLoad: assign({
      error: () => "配置加载失败：fail-closed，非零退出，不回退环境变量默认",
      exitCode: () => 1,
    }),
    recordExplicitModels: assign({
      explicitModels: ({ event }) => (event as { explicitEntries: number }).explicitEntries,
    }),
    failNoSilentFallback: assign({
      error: () => "零显式模型条目：不得凭 provider 环境变量注册或选中默认模型，请检查 models.entries",
      exitCode: () => 1,
    }),
    // 共享 Kernel::boot：组合根端口从 infra 注入（r1392 / r1394）。
    bootViaKernel: assign({
      bootMode: () => "kernel-boot" as const,
      portsInjected: () => true,
    }),
    failInlineAssembly: assign({
      error: () => "不得内联第二套装配：cli 与 Host 须经共享 Kernel::boot",
      exitCode: () => 1,
    }),
    failNoSurfaceBoot: assign({
      error: () => "无表面可进入：boot 前须解析出 tui / print / serve",
      exitCode: () => 1,
    }),
    // r70：idle slash 经 commands → effects → Driver 共享派发路径。
    resolvePendingCommand: assign({
      pendingCommand: ({ event }) => (event as { name: string }).name,
      dispatches: ({ context }) => context.dispatches + 1,
    }),
    // r67/r69：废弃 pi 短名不解析为有效命令入口。
    rejectDeprecatedShortName: assign({
      rejectedSlash: ({ context }) => context.rejectedSlash + 1,
    }),
  },
});

export const machine = setupMachine.createMachine({
  id: "boot-assembly",
  initial: "parsing",
  context: initialContext,
  states: {
    parsing: {
      on: {
        parseArgv: [
          { guard: "trustFlagsConflict", target: "failed", actions: "failTrustConflict" },
          { guard: "topLevelSurfaceFlag", target: "failed", actions: "failTopLevelFlag" },
          { guard: "forbiddenVerb", target: "failed", actions: "failForbiddenVerb" },
          { guard: "resolvesToSurface", target: "loading", actions: "resolveVerb" },
          { target: "failed", actions: "failNoSurface" },
        ],
      },
    },
    loading: {
      on: {
        loadConfig: [
          { guard: "configLoadFailed", target: "failed", actions: "failConfigLoad" },
          { target: "assembling", actions: "markConfigLoaded" },
        ],
      },
    },
    assembling: {
      on: {
        checkModels: [
          { guard: "noExplicitModels", target: "failed", actions: "failNoSilentFallback" },
          { actions: "recordExplicitModels" },
        ],
        bootKernel: [
          { guard: "inlineAssembly", target: "failed", actions: "failInlineAssembly" },
          { guard: "surfaceIsTui", target: "runningTui", actions: "bootViaKernel" },
          { guard: "surfaceIsPrint", target: "runningPrint", actions: "bootViaKernel" },
          { guard: "surfaceIsServe", target: "runningServe", actions: "bootViaKernel" },
          { target: "failed", actions: "failNoSurfaceBoot" },
        ],
      },
    },
    runningTui: {
      on: {
        slashInput: [
          { guard: "inBuiltinTable", actions: "resolvePendingCommand" },
          { actions: "rejectDeprecatedShortName" },
        ],
      },
    },
    runningPrint: {},
    runningServe: {},
    failed: { type: "final" },
  },
});

export const scenarioGroups = {
  "TTY 默认 TUI 与共享派发": [
    { name: "TTY 裸跑默认解析为 tui", events: [{ type: "parseArgv", verb: null, tty: true, topLevelSurfaceFlag: false, trust: false, noTrust: false }], expectStates: ["loading"], expectContext: { surface: "tui", verb: "tui" } },
    { name: "配置与模型检查通过", events: [{ type: "loadConfig", ok: true }, { type: "checkModels", explicitEntries: 2 }], expectContext: { configOk: true, explicitModels: 2 } },
    { name: "经共享 Kernel::boot 进入 TUI", events: [{ type: "bootKernel", mode: "kernel-boot" }], expectStates: ["runningTui"], expectContext: { bootMode: "kernel-boot", portsInjected: true } },
    { name: "产品 slash 走共享派发路径", events: [{ type: "slashInput", name: "/model" }], expectContext: { pendingCommand: "/model", dispatches: 1 } },
    { name: "废弃 pi 短名不解析", events: [{ type: "slashInput", name: "/tree" }], expectContext: { rejectedSlash: 1, pendingCommand: "/model" } },
  ],
  "serve 顶层管理动词": [
    { name: "serve 保持顶层 ops 动词", events: [{ type: "parseArgv", verb: "serve", tty: true, topLevelSurfaceFlag: false, trust: false, noTrust: false }, { type: "loadConfig", ok: true }, { type: "checkModels", explicitEntries: 1 }, { type: "bootKernel", mode: "kernel-boot" }], expectStates: ["runningServe"], expectContext: { surface: "serve", portsInjected: true } },
  ],
  "print 子模式": [
    { name: "print 作为 app 子模式启动", events: [{ type: "parseArgv", verb: "print", tty: true, topLevelSurfaceFlag: false, trust: false, noTrust: false }, { type: "loadConfig", ok: true }, { type: "checkModels", explicitEntries: 1 }, { type: "bootKernel", mode: "kernel-boot" }], expectStates: ["runningPrint"], expectContext: { surface: "print", portsInjected: true } },
  ],
  "顶层表面旗标拒绝": [
    { name: "顶层 --session 解析失败", events: [{ type: "parseArgv", verb: "tui", tty: true, topLevelSurfaceFlag: true, trust: false, noTrust: false }], expectStates: ["failed"], expectContext: { exitCode: 2 }, expect: (snapshot) => { const ctx = (snapshot as { context: BootContext }).context; return ctx.error !== null && ctx.error.includes("顶层") ? true : `错误应指向顶层旗标，实得 ${ctx.error}`; } },
  ],
  "trust 旗标冲突硬失败": [
    { name: "同传 --trust 与 --no-trust 非零退出", events: [{ type: "parseArgv", verb: "tui", tty: true, topLevelSurfaceFlag: false, trust: true, noTrust: true }], expectStates: ["failed"], expectContext: { trustConflict: true, exitCode: 2 }, expect: (snapshot) => { const ctx = (snapshot as { context: BootContext }).context; return ctx.error !== null && ctx.error.includes("未进入 trust 门禁") ? true : `错误应写明未进入 trust 门禁，实得 ${ctx.error}`; } },
  ],
  "损坏配置 fail-closed": [
    { name: "YAML 损坏硬错误而非警告后继续", events: [{ type: "parseArgv", verb: "tui", tty: true, topLevelSurfaceFlag: false, trust: false, noTrust: false }, { type: "loadConfig", ok: false }], expectStates: ["failed"], expectContext: { configOk: false, exitCode: 1 }, expect: (snapshot) => { const ctx = (snapshot as { context: BootContext }).context; return ctx.error !== null && ctx.error.includes("fail-closed") ? true : `错误应写明 fail-closed，实得 ${ctx.error}`; } },
  ],
  "零显式模型 no-silent-fallback": [
    { name: "零显式条目硬失败不回退 env 默认", events: [{ type: "parseArgv", verb: "tui", tty: true, topLevelSurfaceFlag: false, trust: false, noTrust: false }, { type: "loadConfig", ok: true }, { type: "checkModels", explicitEntries: 0 }], expectStates: ["failed"], expectContext: { explicitModels: 0, exitCode: 1 }, expect: (snapshot) => { const ctx = (snapshot as { context: BootContext }).context; return ctx.error !== null && ctx.error.includes("models.entries") ? true : `错误应指向 models.entries，实得 ${ctx.error}`; } },
  ],
  "未知顶层动词拒绝": [
    { name: "tokenizer 不得作为顶层动词", events: [{ type: "parseArgv", verb: "tokenizer", tty: true, topLevelSurfaceFlag: false, trust: false, noTrust: false }], expectStates: ["failed"], expectContext: { exitCode: 2 }, expect: (snapshot) => { const ctx = (snapshot as { context: BootContext }).context; return ctx.error !== null && ctx.error.includes("tokenizer") ? true : `错误应点名动词，实得 ${ctx.error}`; } },
  ],
  "内联装配拒绝": [
    { name: "内联第二套装配硬失败", events: [{ type: "parseArgv", verb: "serve", tty: true, topLevelSurfaceFlag: false, trust: false, noTrust: false }, { type: "loadConfig", ok: true }, { type: "checkModels", explicitEntries: 1 }, { type: "bootKernel", mode: "inline" }], expectStates: ["failed"], expectContext: { exitCode: 1 }, expect: (snapshot) => { const ctx = (snapshot as { context: BootContext }).context; return ctx.error !== null && ctx.error.includes("Kernel::boot") ? true : `错误应指向共享 Kernel::boot，实得 ${ctx.error}`; } },
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
