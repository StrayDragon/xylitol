/**
 * 工具权限门控状态机（Permission Gate）。
 *
 * 业务：ReAct 工具分发前的建议性 permission 门控：文件系统路径、bash 命令
 * 网络域、MCP server/tool 允许列表、run_code 进程工作区。任一列表非空时
 * 未列出者默认拒绝；配置合并仅收紧，不得放宽。
 *
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r64（PermissionConfig 结构）
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r65（文件系统读写检查）
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r66（bash 命令网络域检查）
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r44（仅收紧）
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r50（file_path 与 path 统一检查）
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r53（MCP 专用策略与缺省拒绝）
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r1423（XyPermission trait 与裁决形状）
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r1424（默认拒绝未列出）
 * @spec llmanspec/specs/domain-security/domain-security.feature#@req:r3258（run_code 映射 check_process，嵌套逐次过闸）
 *
 * 实现落点：crates/xylitol-runtime/src/permission/policy.rs（path/domain 匹配语义）、
 * crates/xylitol-runtime/src/permission/mod.rs（trait 与裁决）。
 *
 * 建模说明：guard 在动作执行前求值，callDenied 直接用纯函数 decide 对
 * context.config 与事件负载求裁决，动作只落盘结果；mergeTightens 自行推算
 * 「放宽与否」，不依赖同一步 assign 先生效。loadConfig 表示基础配置载入，
 * mergeConfig 表示覆盖层合并（r44 仅收紧的约束只作用于合并，patch 语义为
 * 整列表替换）。路径匹配对齐 BDD 场景口径：字面模式可命中全路径相等、
 * 目录前缀与 basename；含通配符的模式按 glob 匹配。状态保持扁平
 * （ready/allowed/blocked），与共享 conventions 的状态路径断言兼容。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

/** 翻译 glob 片段：** 跨路径段、* 单段内通配，其余字符按字面转义。 */
function translateGlob(pattern: string): string {
  let out = "";
  let i = 0;
  while (i < pattern.length) {
    const c = pattern.charAt(i);
    if (c === "*" && pattern.charAt(i + 1) === "*") {
      if (pattern.charAt(i + 2) === "/") {
        out += "(?:.*/)?";
        i += 3;
        continue;
      }
      out += ".*";
      i += 2;
      continue;
    }
    if (c === "*") {
      out += "[^/]*";
      i += 1;
      continue;
    }
    out += c.replace(/[.+?^${}()|[\]\\]/g, "\\$&");
    i += 1;
  }
  return out;
}

function basename(path: string): string {
  const idx = path.lastIndexOf("/");
  return idx === -1 ? path : path.slice(idx + 1);
}

/**
 * 单模式路径匹配，对齐 BDD 场景口径（r64/r65）：
 * - 含通配符：按 glob 匹配全路径（模式无法编译时退化为包含判断，同 policy.rs）；
 * - 字面模式：全路径相等、目录前缀（pattern/ 开头）或 basename 相等。
 */
export function pathMatches(path: string, pattern: string): boolean {
  if (pattern.includes("*")) {
    try {
      return new RegExp("^" + translateGlob(pattern) + "$").test(path);
    } catch {
      return path.includes(pattern);
    }
  }
  const dirPrefix = pattern.endsWith("/") ? pattern : pattern + "/";
  return path === pattern || path.startsWith(dirPrefix) || basename(path) === pattern;
}

/** 任一模式命中即真；空列表恒假（与 policy.rs 的 path_matches_any 对齐）。 */
export function pathMatchesAny(path: string, patterns: readonly string[]): boolean {
  if (patterns.length === 0) return false;
  return patterns.some((p) => pathMatches(path, p));
}

/** 域匹配：精确、*、*.后缀（子域）与前缀.*（TLD 组），与 policy.rs 对齐。 */
export function domainMatches(domain: string, pattern: string): boolean {
  const d = domain.trim().toLowerCase();
  const p = pattern.trim().toLowerCase();
  if (p === "*") return true;
  if (d === p) return true;
  if (p.startsWith("*.")) {
    const rest = p.slice(2);
    return d.endsWith(rest) && d.length > rest.length && d.charAt(d.length - rest.length - 1) === ".";
  }
  if (p.endsWith(".*")) {
    const prefix = p.slice(0, -2);
    return d.startsWith(prefix) && d.length > prefix.length && d.charAt(prefix.length) === ".";
  }
  return false;
}

export function domainMatchesAny(domain: string, patterns: readonly string[]): boolean {
  if (patterns.length === 0) return false;
  return patterns.some((p) => domainMatches(domain, p));
}

/** 权限配置结构（r64）：filesystem / network / process / mcp 四组列表。 */
export interface PermissionConfig {
  readAllowed: string[];
  writeAllowed: string[];
  writeDenied: string[];
  netAllowedDomains: string[];
  netDeniedDomains: string[];
  processAllowedPaths: string[];
  /** MCP 允许列表：条目为 server 或 server/tool；空列表缺省拒绝（r53）。 */
  mcpAllowed: string[];
}

/** 配置补丁：整列表替换语义；只提供要改的字段。 */
export interface ConfigPatch {
  readAllowed?: readonly string[];
  writeAllowed?: readonly string[];
  writeDenied?: readonly string[];
  netAllowedDomains?: readonly string[];
  netDeniedDomains?: readonly string[];
  processAllowedPaths?: readonly string[];
  mcpAllowed?: readonly string[];
}

export type ToolCall =
  | { tool: "read" | "write" | "edit"; filePath?: string; path?: string }
  | { tool: "bash"; command: string; domains: readonly string[] }
  | { tool: "mcp"; server: string; name: string }
  | { tool: "run_code"; workspace: string };

export interface Verdict {
  allowed: boolean;
  reason: string;
}

/** 纯函数裁决：guard 与 action 共用同一实现，保证 guard 先行语义一致。 */
export function decide(config: PermissionConfig, call: ToolCall): Verdict {
  switch (call.tool) {
    case "read":
    case "write":
    case "edit": {
      // r50：file_path 与 path 两个字段统一检查，任一字段命中即按该路径裁决。
      const paths = [call.filePath, call.path].filter((p): p is string => typeof p === "string");
      if (paths.length === 0) return { allowed: false, reason: "missing-path" };
      if (call.tool === "read") {
        // r1424：read_allowed 非空时未列出即拒绝。
        if (config.readAllowed.length > 0 && !paths.every((p) => pathMatchesAny(p, config.readAllowed))) {
          return { allowed: false, reason: "read-denied:default-deny" };
        }
        return { allowed: true, reason: "read-allowed" };
      }
      // r65：路径命中 write_denied 即拒绝。
      if (paths.some((p) => pathMatchesAny(p, config.writeDenied))) {
        return { allowed: false, reason: "write-denied" };
      }
      if (config.writeAllowed.length > 0 && !paths.every((p) => pathMatchesAny(p, config.writeAllowed))) {
        return { allowed: false, reason: "write-denied:default-deny" };
      }
      return { allowed: true, reason: "write-allowed" };
    }
    case "bash": {
      // r66：域名从命令文本 URL 提取，建模为事件负载，逐个裁决并默认拒绝未列出。
      for (const d of call.domains) {
        if (domainMatchesAny(d, config.netDeniedDomains)) {
          return { allowed: false, reason: "network:denied-domain:" + d };
        }
        if (config.netAllowedDomains.length > 0 && !domainMatchesAny(d, config.netAllowedDomains)) {
          return { allowed: false, reason: "network:default-deny:" + d };
        }
      }
      return { allowed: true, reason: "network-allowed" };
    }
    case "mcp": {
      // r53：MCP 专用分支，显式 server/tool 允许列表，缺省拒绝。
      if (config.mcpAllowed.length === 0) {
        return { allowed: false, reason: "mcp:default-deny:no-allowlist" };
      }
      const hit = config.mcpAllowed.some(
          (entry) => entry === call.server || entry === `${call.server}/${call.name}`);
      return hit
          ? { allowed: true, reason: "mcp:allowlist" }
          : { allowed: false, reason: "mcp:default-deny" };
    }
    case "run_code": {
      // r3258：映射 check_process(workspace)；嵌套调用作为独立事件逐次过闸。
      if (pathMatchesAny(call.workspace, config.processAllowedPaths)) {
        return { allowed: true, reason: "process:workspace-allowed" };
      }
      return { allowed: false, reason: "process:default-deny" };
    }
  }
}

/** 合并是否放宽（r44）：允许列表只可删、拒绝列表只可增；放宽即拒绝合并。 */
export function mergeWouldWiden(config: PermissionConfig, patch: ConfigPatch): boolean {
  const addedAllow = (current: readonly string[], next?: readonly string[]): boolean =>
      next !== undefined && next.some((item) => !current.includes(item));
  const droppedDeny = (current: readonly string[], next?: readonly string[]): boolean => {
    if (next === undefined) return false;
    return current.some((item) => !next.includes(item));
  };
  if (addedAllow(config.readAllowed, patch.readAllowed)) return true;
  if (addedAllow(config.writeAllowed, patch.writeAllowed)) return true;
  if (addedAllow(config.netAllowedDomains, patch.netAllowedDomains)) return true;
  if (addedAllow(config.processAllowedPaths, patch.processAllowedPaths)) return true;
  if (addedAllow(config.mcpAllowed, patch.mcpAllowed)) return true;
  if (droppedDeny(config.writeDenied, patch.writeDenied)) return true;
  if (droppedDeny(config.netDeniedDomains, patch.netDeniedDomains)) return true;
  return false;
}

function overlay(base: PermissionConfig, patch: ConfigPatch): PermissionConfig {
  const take = (current: string[], next?: readonly string[]): string[] =>
      next === undefined ? current : [...next];
  return {
    readAllowed: take(base.readAllowed, patch.readAllowed),
    writeAllowed: take(base.writeAllowed, patch.writeAllowed),
    writeDenied: take(base.writeDenied, patch.writeDenied),
    netAllowedDomains: take(base.netAllowedDomains, patch.netAllowedDomains),
    netDeniedDomains: take(base.netDeniedDomains, patch.netDeniedDomains),
    processAllowedPaths: take(base.processAllowedPaths, patch.processAllowedPaths),
    mcpAllowed: take(base.mcpAllowed, patch.mcpAllowed),
  };
}

/** 仅收紧合并：允许列表取交集、拒绝列表取并集。 */
function tightenOverlay(base: PermissionConfig, patch: ConfigPatch): PermissionConfig {
  const inter = (current: string[], next?: readonly string[]): string[] =>
      next === undefined ? current : current.filter((item) => next.includes(item));
  const union = (current: string[], next?: readonly string[]): string[] =>
      next === undefined ? current : [...new Set([...current, ...next])];
  return {
    readAllowed: inter(base.readAllowed, patch.readAllowed),
    writeAllowed: inter(base.writeAllowed, patch.writeAllowed),
    writeDenied: union(base.writeDenied, patch.writeDenied),
    netAllowedDomains: inter(base.netAllowedDomains, patch.netAllowedDomains),
    netDeniedDomains: union(base.netDeniedDomains, patch.netDeniedDomains),
    processAllowedPaths: inter(base.processAllowedPaths, patch.processAllowedPaths),
    mcpAllowed: inter(base.mcpAllowed, patch.mcpAllowed),
  };
}

export interface GateContext {
  config: PermissionConfig;
  lastVerdict: "allowed" | "denied" | null;
  lastReason: string | null;
  checks: number;
  mergeViolations: number;
}

export const initialContext: GateContext = {
  config: {
    readAllowed: [],
    writeAllowed: [],
    writeDenied: [],
    netAllowedDomains: [],
    netDeniedDomains: [],
    processAllowedPaths: [],
    mcpAllowed: [],
  },
  lastVerdict: null,
  lastReason: null,
  checks: 0,
  mergeViolations: 0,
};

export const meta = defineMeta({
  name: "permission-gate",
  pkg: "security-infra",
  purpose: "工具分发前 permission 裁决：路径、网络域、MCP 允许列表、进程工作区，默认拒绝未列出",
  anchors: [
    "llmanspec/specs/domain-security/domain-security.feature#@req:r64",
    "llmanspec/specs/domain-security/domain-security.feature#@req:r65",
    "llmanspec/specs/domain-security/domain-security.feature#@req:r66",
    "llmanspec/specs/domain-security/domain-security.feature#@req:r44",
    "llmanspec/specs/domain-security/domain-security.feature#@req:r50",
    "llmanspec/specs/domain-security/domain-security.feature#@req:r53",
    "llmanspec/specs/domain-security/domain-security.feature#@req:r1423",
    "llmanspec/specs/domain-security/domain-security.feature#@req:r1424",
    "llmanspec/specs/domain-security/domain-security.feature#@req:r3258",
  ],
  impl: ["crates/xylitol-runtime/src/permission/policy.rs", "crates/xylitol-runtime/src/permission/mod.rs"],
});

/** 裁决与合并的公共转移：三个状态共用同一事件语义（状态只表达最近裁决）。 */
const gateHandlers = {
  loadConfig: { actions: "replaceConfig" },
  mergeConfig: [
    { guard: "mergeTightens", actions: "applyMerge" },
    { actions: "rejectMerge" },
  ],
  check: [
    { guard: "callDenied", target: "blocked", actions: "applyVerdict" },
    { target: "allowed", actions: "applyVerdict" },
  ],
} as const;

export const setupMachine = setup({
  types: {
    context: {} as GateContext,
    events: {} as
      | { type: "loadConfig"; config: ConfigPatch }
      | { type: "mergeConfig"; patch: ConfigPatch }
      | { type: "check"; call: ToolCall },
  },
  guards: {
    callDenied: ({ context, event }) =>
        !decide(context.config, (event as { call: ToolCall }).call).allowed,
    mergeTightens: ({ context, event }) =>
        !mergeWouldWiden(context.config, (event as { patch: ConfigPatch }).patch),
  },
  actions: {
    replaceConfig: assign(({ context, event }) => ({
      config: overlay(context.config, (event as { config: ConfigPatch }).config),
    })),
    applyVerdict: assign(({ context, event }) => {
      const verdict = decide(context.config, (event as { call: ToolCall }).call);
      return {
        lastVerdict: verdict.allowed ? ("allowed" as const) : ("denied" as const),
        lastReason: verdict.reason,
        checks: context.checks + 1,
      };
    }),
    applyMerge: assign(({ context, event }) => ({
      config: tightenOverlay(context.config, (event as { patch: ConfigPatch }).patch),
    })),
    rejectMerge: assign(({ context }) => ({
      mergeViolations: context.mergeViolations + 1,
    })),
  },
});

export const machine = setupMachine.createMachine({
  id: "permission-gate",
  initial: "ready",
  context: initialContext,
  states: {
    ready: { on: gateHandlers },
    allowed: { on: gateHandlers },
    blocked: { on: gateHandlers },
  },
});

export const scenarioGroups = {
  "写拒绝与允许": [
    {
      name: "载入基础配置 write_denied=[.env]",
      events: [
        { type: "loadConfig", config: { writeDenied: [".env"] } },
      ],
      expectStates: ["ready"],
    },
    {
      name: "写入 .env 被 basename 命中拒绝（r65）",
      events: [{ type: "check", call: { tool: "write", filePath: "/project/.env" } }],
      expectStates: ["blocked"],
      expectContext: { lastReason: "write-denied" },
    },
    {
      name: "写入源码允许（r65）",
      events: [{ type: "check", call: { tool: "write", filePath: "/project/src/main.rs" } }],
      expectStates: ["allowed"],
      expectContext: { lastReason: "write-allowed" },
    },
  ],
  "读取默认拒绝未列出": [
    {
      name: "载入 read_allowed=/home/user/project",
      events: [
        { type: "loadConfig", config: { readAllowed: ["/home/user/project"] } },
      ],
      expectStates: ["ready"],
    },
    {
      name: "读取列表外路径默认拒绝（r1424）",
      events: [{ type: "check", call: { tool: "read", filePath: "/etc/passwd" } }],
      expectStates: ["blocked"],
      expectContext: { lastReason: "read-denied:default-deny" },
    },
    {
      name: "读取列表内路径按目录前缀放行",
      events: [{ type: "check", call: { tool: "read", filePath: "/home/user/project/src/main.rs" } }],
      expectStates: ["allowed"],
      expectContext: { lastReason: "read-allowed" },
    },
  ],
  "双路径字段统一检查": [
    {
      name: "只带 path 字段的调用同样被列表裁决（r50）",
      events: [
        { type: "loadConfig", config: { readAllowed: ["/project"] } },
        { type: "check", call: { tool: "read", path: "/etc/passwd" } },
      ],
      expectStates: ["blocked"],
      expectContext: { lastReason: "read-denied:default-deny" },
    },
    {
      name: "缺路径字段的调用直接拒绝",
      events: [{ type: "check", call: { tool: "write" } }],
      expectStates: ["blocked"],
      expectContext: { lastReason: "missing-path" },
    },
  ],
  "bash 网络域裁决": [
    {
      name: "载入网络配置：拒绝 evil.com，允许 *.github.com",
      events: [
        {
          type: "loadConfig",
          config: { netDeniedDomains: ["evil.com"], netAllowedDomains: ["*.github.com"] },
        },
      ],
      expectStates: ["ready"],
    },
    {
      name: "命中拒绝域阻止（r66）",
      events: [
        {
          type: "check",
          call: { tool: "bash", command: "curl https://evil.com/x", domains: ["evil.com"] },
        },
      ],
      expectStates: ["blocked"],
      expectContext: { lastReason: "network:denied-domain:evil.com" },
    },
    {
      name: "通配子域命中允许列表",
      events: [
        {
          type: "check",
          call: { tool: "bash", command: "curl https://api.github.com", domains: ["api.github.com"] },
        },
      ],
      expectStates: ["allowed"],
      expectContext: { lastReason: "network-allowed" },
    },
    {
      name: "允许列表非空时未列出域默认拒绝（r1424）",
      events: [
        {
          type: "check",
          call: { tool: "bash", command: "curl https://unlisted.com", domains: ["unlisted.com"] },
        },
      ],
      expectStates: ["blocked"],
      expectContext: { lastReason: "network:default-deny:unlisted.com" },
    },
  ],
  "MCP 缺省拒绝": [
    {
      name: "无允许列表时 MCP 工具缺省拒绝（r53）",
      events: [
        { type: "loadConfig", config: { mcpAllowed: [] } },
        { type: "check", call: { tool: "mcp", server: "github", name: "ping" } },
      ],
      expectStates: ["blocked"],
      expectContext: { lastReason: "mcp:default-deny:no-allowlist" },
    },
    {
      name: "server 命中允许列表后放行",
      events: [
        { type: "loadConfig", config: { mcpAllowed: ["github"] } },
        { type: "check", call: { tool: "mcp", server: "github", name: "ping" } },
      ],
      expectStates: ["allowed"],
      expectContext: { lastReason: "mcp:allowlist" },
    },
    {
      name: "未列出 server 仍拒绝",
      events: [{ type: "check", call: { tool: "mcp", server: "other", name: "ping" } }],
      expectStates: ["blocked"],
      expectContext: { lastReason: "mcp:default-deny" },
    },
  ],
  "run_code 进程逐次过闸": [
    {
      name: "工作区命中进程允许列表放行（r3258）",
      events: [
        { type: "loadConfig", config: { processAllowedPaths: ["/home/user/project"] } },
        { type: "check", call: { tool: "run_code", workspace: "/home/user/project" } },
      ],
      expectStates: ["allowed"],
      expectContext: { lastReason: "process:workspace-allowed" },
    },
    {
      name: "嵌套调用换到列表外工作区被拒（逐次过闸）",
      events: [{ type: "check", call: { tool: "run_code", workspace: "/tmp/other" } }],
      expectStates: ["blocked"],
      expectContext: { lastReason: "process:default-deny", checks: 2 },
    },
  ],
  "仅收紧合并": [
    {
      name: "载入基础配置：允许读项目、拒绝 secrets 目录",
      events: [
        {
          type: "loadConfig",
          config: {
            readAllowed: ["/home/user/project"],
            writeDenied: ["/home/user/project/secrets/**"],
          },
        },
      ],
      expectStates: ["ready"],
    },
    {
      name: "试图把 /etc/** 加进允许列表被拒（r44）",
      events: [
        {
          type: "mergeConfig",
          patch: { readAllowed: ["/home/user/project", "/etc/**"] },
        },
      ],
      expectStates: ["ready"],
      expectContext: { mergeViolations: 1 },
    },
    {
      name: "放宽未生效，列表外读取仍被阻止",
      events: [{ type: "check", call: { tool: "read", filePath: "/etc/passwd" } }],
      expectStates: ["blocked"],
      expectContext: { lastReason: "read-denied:default-deny" },
    },
    {
      name: "拒绝列表整表替换且保留旧条目属收紧，可合并",
      events: [
        {
          type: "mergeConfig",
          patch: { writeDenied: ["/home/user/project/secrets/**", "**/*.pem"] },
        },
      ],
      expectStates: ["blocked"],
      expectContext: { mergeViolations: 1 },
    },
    {
      name: "合并后新拒绝模式按 basename 生效",
      events: [{ type: "check", call: { tool: "write", filePath: "/project/secret.pem" } }],
      expectStates: ["blocked"],
      expectContext: { lastReason: "write-denied" },
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
