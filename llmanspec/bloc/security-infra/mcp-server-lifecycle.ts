/**
 * MCP 服务器生命周期状态机（MCP Server Lifecycle）。
 *
 * 业务：未配置零装配；有配置才按 transport 连接并注册 mcp__server__tool；
 * 配置校验失败只诊断不致命；进程内重载；视图忙/重连时返回上一份已知快照；
 * resume 重建 claims 并释放未用 server；switch/resume 不重置连接池。
 *
 * @spec llmanspec/specs/infra-mcp/infra-mcp.feature#@req:r1445（无配置零装配）
 * @spec llmanspec/specs/infra-mcp/infra-mcp.feature#@req:r1447（有配置装配为 XyTool）
 * @spec llmanspec/specs/infra-mcp/infra-mcp.feature#@req:r1448（动态配置重载）
 * @spec llmanspec/specs/infra-mcp/infra-mcp.feature#@req:r1449（配置校验与失败可观测）
 * @spec llmanspec/specs/infra-mcp/infra-mcp.feature#@req:r1450（已连接列表只读）
 * @spec llmanspec/specs/infra-mcp/infra-mcp.feature#@req:r3228（switch/resume 不重置，仅 reload 清空重连）
 * @spec llmanspec/specs/infra-mcp/infra-mcp.feature#@req:r3229（忙/重连期返回上份已知快照）
 * @spec llmanspec/specs/infra-mcp/infra-mcp.feature#@req:r3234（resume 重建 claims 释放未用）
 *
 * 实现落点：plugins/xylitol-official-plugin-mcp/src/hub.rs、
 * plugins/xylitol-official-plugin-mcp/src/client.rs、
 * plugins/xylitol-official-plugin-mcp/src/assemble.rs。
 *
 * 建模说明：guard 在动作执行前求值——allSettled 用「合并本事件结果后的
 * results」自行推算最后一个结果是否收口，不指望同一步 assign 先生效；
 * serverDown 不改 connected（last-known 保留在册列表，r3229）；
 * r1447 的公开名约定 mcp__{server_id}__{name} 以工具数聚合表达。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

export interface McpServerEntry {
  id: string;
  transport: "stdio" | "http";
  command?: string;
  url?: string;
  tools: readonly string[];
}

/** 配置校验（r1449）：stdio 需 command，http 需 url；server 名须安全（r3273 同口径）。 */
export function isValidEntry(entry: McpServerEntry): boolean {
  if (!/^[A-Za-z0-9_-]+$/.test(entry.id)) return false;
  if (entry.transport === "stdio") {
    return typeof entry.command === "string" && entry.command.length > 0;
  }
  return typeof entry.url === "string" && entry.url.length > 0;
}

export interface ConfigureEvent {
  type: "configure" | "reload";
  servers: readonly McpServerEntry[];
}

export interface ConnectResultEvent {
  type: "connectResult";
  id: string;
  ok: boolean;
}

export interface ServerDownEvent {
  type: "serverDown";
  id: string;
}

export interface ResumeEvent {
  type: "resume";
  usedServers: readonly string[];
}

export interface McpContext {
  configured: boolean;
  servers: readonly McpServerEntry[];
  results: Readonly<Record<string, "ok" | "fail">>;
  connected: readonly string[];
  failed: readonly string[];
  diagnostics: readonly string[];
  /** 已武装的 mcp__ 工具数（有效且已连接 server 的工具合计）。 */
  armedTools: number;
  /** 内置非 mcp 工具数；重载不得删除（r1448）。 */
  builtinTools: number;
  lastKnown: { configured: number; connected: readonly string[] } | null;
  snapshotSource: "live" | "last-known" | null;
  reloads: number;
  reconnects: number;
  releasedClaims: number;
}

export const BUILTIN_TOOLS = 5;

export const initialContext: McpContext = {
  configured: false,
  servers: [],
  results: {},
  connected: [],
  failed: [],
  diagnostics: [],
  armedTools: 0,
  builtinTools: BUILTIN_TOOLS,
  lastKnown: null,
  snapshotSource: null,
  reloads: 0,
  reconnects: 0,
  releasedClaims: 0,
};

export const meta = defineMeta({
  name: "mcp-server-lifecycle",
  pkg: "security-infra",
  purpose: "MCP 服务器装配与连接：零配置零装配、失败诊断、重载、last-known 快照、resume 释放",
  anchors: [
    "llmanspec/specs/infra-mcp/infra-mcp.feature#@req:r1445",
    "llmanspec/specs/infra-mcp/infra-mcp.feature#@req:r1447",
    "llmanspec/specs/infra-mcp/infra-mcp.feature#@req:r1448",
    "llmanspec/specs/infra-mcp/infra-mcp.feature#@req:r1449",
    "llmanspec/specs/infra-mcp/infra-mcp.feature#@req:r1450",
    "llmanspec/specs/infra-mcp/infra-mcp.feature#@req:r3228",
    "llmanspec/specs/infra-mcp/infra-mcp.feature#@req:r3229",
    "llmanspec/specs/infra-mcp/infra-mcp.feature#@req:r3234",
  ],
  impl: [
    "plugins/xylitol-official-plugin-mcp/src/hub.rs",
    "plugins/xylitol-official-plugin-mcp/src/client.rs",
    "plugins/xylitol-official-plugin-mcp/src/assemble.rs",
  ],
});

function toolCount(servers: readonly McpServerEntry[], ids: readonly string[]): number {
  return servers
      .filter((s) => isValidEntry(s) && ids.includes(s.id))
      .reduce((n, s) => n + s.tools.length, 0);
}

function invalidIds(servers: readonly McpServerEntry[]): readonly string[] {
  return servers.filter((s) => !isValidEntry(s)).map((s) => "invalid-entry:" + s.id);
}

function viewOf(context: McpContext): { configured: number; connected: readonly string[] } {
  return {
    configured: context.servers.filter(isValidEntry).length,
    connected: [...context.connected],
  };
}

export const setupMachine = setup({
  types: {
    context: {} as McpContext,
    events: {} as
      | ConfigureEvent
      | ConnectResultEvent
      | ServerDownEvent
      | ResumeEvent
      | { type: "switchSession" }
      | { type: "viewSnapshot" },
  },
  guards: {
    anyValidServer: ({ event }) =>
        (event as ConfigureEvent).servers.some((s) => isValidEntry(s)),
    reloadToEmpty: ({ event }) => (event as ConfigureEvent).servers.length === 0,
    allSettled: ({ context, event }) => {
      // guard 先于动作求值：把本事件的结果合并进 results 后再判断是否收口。
      const e = event as ConnectResultEvent;
      const merged = { ...context.results, [e.id]: e.ok ? ("ok" as const) : ("fail" as const) };
      return context.servers.filter((s) => isValidEntry(s))
          .every((s) => merged[s.id] !== undefined);
    },
    reconnectOk: ({ event }) => (event as ConnectResultEvent).ok === true,
  },
  actions: {
    acceptConfig: assign(({ context, event }) => {
      const servers = (event as ConfigureEvent).servers;
      return {
        configured: true,
        servers,
        results: {},
        connected: [],
        failed: [],
        diagnostics: invalidIds(servers),
        armedTools: 0,
        lastKnown: null,
        snapshotSource: null,
      };
    }),
    diagnoseNoValidEntry: assign(({ context, event }) => ({
      configured: false,
      servers: [],
      diagnostics: [...context.diagnostics, ...invalidIds((event as ConfigureEvent).servers), "no-valid-mcp-entry"],
      armedTools: 0,
    })),
    applyResult: assign(({ context, event }) => {
      const e = event as ConnectResultEvent;
      const entry = context.servers.find((s) => s.id === e.id);
      if (entry === undefined || !isValidEntry(entry)) {
        return { diagnostics: [...context.diagnostics, "result-for-unknown:" + e.id] };
      }
      const results = { ...context.results, [e.id]: e.ok ? ("ok" as const) : ("fail" as const) };
      const connected = e.ok && !context.connected.includes(e.id)
          ? [...context.connected, e.id]
          : context.connected;
      const failed = !e.ok && !context.failed.includes(e.id)
          ? [...context.failed, e.id]
          : context.failed;
      const diagnostics = e.ok
          ? context.diagnostics
          : [...context.diagnostics, "connect-failed:" + e.id];
      return { results, connected, failed, diagnostics, armedTools: toolCount(context.servers, connected) };
    }),
    takeLastKnownView: assign(({ context }) => ({
      lastKnown: viewOf(context),
      snapshotSource: "last-known" as const,
    })),
    takeLiveView: assign(({ context }) => ({
      lastKnown: viewOf(context),
      snapshotSource: "live" as const,
    })),
    takeEmptyView: assign(({ context }) => ({
      lastKnown: { configured: 0, connected: [] as readonly string[] },
      snapshotSource: "live" as const,
      diagnostics: context.diagnostics,
    })),
    resetForReload: assign(({ context, event }) => ({
      servers: (event as ConfigureEvent).servers,
      results: {},
      connected: [],
      failed: [],
      diagnostics: invalidIds((event as ConfigureEvent).servers),
      armedTools: 0,
      lastKnown: null,
      snapshotSource: null,
      reloads: context.reloads + 1,
    })),
    teardownForReload: assign(({ context }) => ({
      configured: false,
      servers: [],
      results: {},
      connected: [],
      failed: [],
      armedTools: 0,
      lastKnown: null,
      snapshotSource: null,
      reloads: context.reloads + 1,
      diagnostics: [...context.diagnostics, "mcp-clients-shutdown"],
    })),
    diagnoseReloadInvalid: assign(({ context }) => ({
      diagnostics: [...context.diagnostics, "reload-invalid-entry"],
    })),
    reconcileClaims: assign(({ context, event }) => {
      const used = (event as ResumeEvent).usedServers;
      const kept = context.connected.filter((id) => used.includes(id));
      const released = context.connected.length - kept.length;
      return {
        connected: kept,
        releasedClaims: context.releasedClaims + released,
        armedTools: toolCount(context.servers, kept),
        lastKnown: viewOf({ ...context, connected: kept }),
        snapshotSource: "live" as const,
      };
    }),
    markDown: assign(({ context, event }) => {
      const id = (event as ServerDownEvent).id;
      return {
        failed: context.failed.includes(id) ? context.failed : [...context.failed, id],
        diagnostics: [...context.diagnostics, "server-down:" + id],
      };
    }),
    applyReconnect: assign(({ context, event }) => {
      const e = event as ConnectResultEvent;
      if (e.ok) {
        const failed = context.failed.filter((f) => f !== e.id);
        const connected = context.connected.includes(e.id)
            ? context.connected
            : [...context.connected, e.id];
        return {
          reconnects: context.reconnects + 1,
          failed,
          connected,
          diagnostics: [...context.diagnostics, "reconnect-ok:" + e.id],
        };
      }
      return {
        reconnects: context.reconnects + 1,
        diagnostics: [...context.diagnostics, "reconnect-failed:" + e.id],
      };
    }),
  },
});

export const machine = setupMachine.createMachine({
  id: "mcp-server-lifecycle",
  initial: "unconfigured",
  context: initialContext,
  states: {
    unconfigured: {
      on: {
        configure: [
          { guard: "anyValidServer", target: "connecting", actions: "acceptConfig" },
          { actions: "diagnoseNoValidEntry" },
        ],
        viewSnapshot: { actions: "takeEmptyView" },
      },
    },
    connecting: {
      on: {
        connectResult: [
          { guard: "allSettled", target: "armed", actions: "applyResult" },
          { actions: "applyResult" },
        ],
        viewSnapshot: { actions: "takeLastKnownView" },
      },
    },
    armed: {
      on: {
        reload: [
          { guard: "reloadToEmpty", target: "unconfigured", actions: "teardownForReload" },
          { guard: "anyValidServer", target: "connecting", actions: "resetForReload" },
          { actions: "diagnoseReloadInvalid" },
        ],
        // r3228：switch 不重置连接池，只记录事件、context 不变。
        switchSession: {},
        resume: { actions: "reconcileClaims" },
        serverDown: { target: "reconnecting", actions: "markDown" },
        viewSnapshot: { actions: "takeLiveView" },
      },
    },
    reconnecting: {
      on: {
        connectResult: [
          { guard: "reconnectOk", target: "armed", actions: "applyReconnect" },
          { target: "armed", actions: "applyReconnect" },
        ],
        viewSnapshot: { actions: "takeLastKnownView" },
      },
    },
  },
});

const S1: McpServerEntry = { id: "s1", transport: "stdio", command: "mcp-ping", tools: ["ping"] };
const S2: McpServerEntry = {
  id: "s2",
  transport: "http",
  url: "http://127.0.0.1:9999/mcp",
  tools: ["files_list", "files_read"],
};

export const scenarioGroups = {
  "未配置零装配": [
    {
      name: "无配置时不产工具、不建连接，视图为空但可观测（r1445）",
      events: [{ type: "viewSnapshot" }],
      expectStates: ["unconfigured"],
      expectContext: { configured: false, armedTools: 0, snapshotSource: "live" },
      expect: (snapshot) => {
        const ctx = (snapshot as { context: McpContext }).context;
        return ctx.lastKnown !== null && ctx.lastKnown.configured === 0
            ? true
            : "视图未返回 configured=0 空快照";
      },
    },
  ],
  "配置装配注册工具": [
    {
      name: "两个有效 server 进入连接期",
      events: [
        { type: "configure", servers: [S1, S2] },
      ],
      expectStates: ["connecting"],
      expectContext: { configured: true, armedTools: 0 },
    },
    {
      name: "首个 server 连上但尚未收口",
      events: [{ type: "connectResult", id: "s1", ok: true }],
      expectStates: ["connecting"],
      expectContext: { connected: ["s1"], armedTools: 1 },
    },
    {
      name: "全部收口后 armed，工具按 server 合计注册（r1447）",
      events: [{ type: "connectResult", id: "s2", ok: true }],
      expectStates: ["armed"],
      expectContext: { connected: ["s1", "s2"], armedTools: 3 },
    },
    {
      name: "只读视图返回在册摘要且不改变连接（r1450）",
      events: [{ type: "viewSnapshot" }],
      expectStates: ["armed"],
      expectContext: { connected: ["s1", "s2"], snapshotSource: "live" },
    },
  ],
  "无效条目诊断不致命": [
    {
      name: "无效条目进入时立即产生诊断",
      events: [
        {
          type: "configure",
          servers: [
            S1,
            { id: "bad", transport: "stdio", tools: [] },
          ],
        },
      ],
      expectStates: ["connecting"],
      expect: (snapshot) => {
        const ctx = (snapshot as { context: McpContext }).context;
        return ctx.diagnostics.some((d) => d === "invalid-entry:bad")
            ? true
            : `诊断缺 invalid-entry:bad，实得 ${JSON.stringify(ctx.diagnostics)}`;
      },
    },
    {
      name: "有效 server 仍装配，无效条目不产工具（r1449）",
      events: [{ type: "connectResult", id: "s1", ok: true }],
      expectStates: ["armed"],
      expectContext: { connected: ["s1"], armedTools: 1, failed: [] },
    },
  ],
  "全部无效零装配": [
    {
      name: "全部条目无效时保持零装配并诊断（r1445/r1449 拒绝路径）",
      events: [
        {
          type: "configure",
          servers: [{ id: "bad", transport: "stdio", tools: [] }],
        },
      ],
      expectStates: ["unconfigured"],
      expectContext: { configured: false, armedTools: 0, connected: [] },
    },
  ],
  "重载工具集跟随配置": [
    {
      name: "初始装配 s1 一个工具",
      events: [
        { type: "configure", servers: [S1] },
        { type: "connectResult", id: "s1", ok: true },
      ],
      expectStates: ["armed"],
      expectContext: { armedTools: 1, builtinTools: BUILTIN_TOOLS },
    },
    {
      name: "重载进连接期并递增 reload 计数（r1448）",
      events: [
        {
          type: "reload",
          servers: [{ id: "s1", transport: "stdio", command: "mcp-ping", tools: ["ping", "pong"] }],
        },
      ],
      expectStates: ["connecting"],
      expectContext: { connected: [], reloads: 1, builtinTools: BUILTIN_TOOLS },
    },
    {
      name: "重连后工具集更新且内置工具保留",
      events: [{ type: "connectResult", id: "s1", ok: true }],
      expectStates: ["armed"],
      expectContext: { armedTools: 2, builtinTools: BUILTIN_TOOLS, connected: ["s1"] },
    },
  ],
  "重载清空配置": [
    {
      name: "重载到空配置时先 shutdown 旧 client 再回零装配（r1452）",
      events: [
        { type: "configure", servers: [S1] },
        { type: "connectResult", id: "s1", ok: true },
        { type: "reload", servers: [] },
      ],
      expectStates: ["unconfigured"],
      expect: (snapshot) => {
        const ctx = (snapshot as { context: McpContext }).context;
        return ctx.diagnostics.includes("mcp-clients-shutdown")
            ? true
            : "诊断缺 mcp-clients-shutdown";
      },
    },
  ],
  "单服务器失败不拖死": [
    {
      name: "一个 server 连接失败仍可观测且其余继续（r1449/r3228）",
      events: [
        { type: "configure", servers: [S1, S2] },
        { type: "connectResult", id: "s1", ok: false },
      ],
      expectStates: ["connecting"],
      expectContext: { failed: ["s1"], connected: [] },
    },
    {
      name: "另一 server 成功后照常 armed",
      events: [{ type: "connectResult", id: "s2", ok: true }],
      expectStates: ["armed"],
      expectContext: { connected: ["s2"], failed: ["s1"], armedTools: 2 },
    },
  ],
  "重连期返回上份快照": [
    {
      name: "armed 后 server 挂掉进入重连期",
      events: [
        { type: "configure", servers: [S1, S2] },
        { type: "connectResult", id: "s1", ok: true },
        { type: "connectResult", id: "s2", ok: true },
        { type: "serverDown", id: "s1" },
      ],
      expectStates: ["reconnecting"],
      expectContext: { connected: ["s1", "s2"], failed: ["s1"] },
    },
    {
      name: "重连期视图返回上一份已知快照，未知不当 configured=0（r3229）",
      events: [{ type: "viewSnapshot" }],
      expectStates: ["reconnecting"],
      expectContext: { snapshotSource: "last-known", connected: ["s1", "s2"] },
      expect: (snapshot) => {
        const ctx = (snapshot as { context: McpContext }).context;
        return ctx.lastKnown !== null && ctx.lastKnown.configured === 2 && ctx.lastKnown.connected.length === 2
            ? true
            : "上份快照不完整";
      },
    },
    {
      name: "按 server 重连成功后回 armed（r3228）",
      events: [{ type: "connectResult", id: "s1", ok: true }],
      expectStates: ["armed"],
      expectContext: { reconnects: 1, failed: [], connected: ["s1", "s2"] },
    },
  ],
  "resume 释放未用 server": [
    {
      name: "两个 server 在册",
      events: [
        { type: "configure", servers: [S1, S2] },
        { type: "connectResult", id: "s1", ok: true },
        { type: "connectResult", id: "s2", ok: true },
      ],
      expectStates: ["armed"],
      expectContext: { connected: ["s1", "s2"], armedTools: 3 },
    },
    {
      name: "resume 只用 s1 时释放 s2 的 claim（r3234）",
      events: [{ type: "resume", usedServers: ["s1"] }],
      expectStates: ["armed"],
      expectContext: { connected: ["s1"], releasedClaims: 1, armedTools: 1 },
    },
    {
      name: "switch 不重置连接池，context 保持（r3228）",
      events: [{ type: "switchSession" }],
      expectStates: ["armed"],
      expectContext: { connected: ["s1"], releasedClaims: 1, armedTools: 1 },
    },
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
