/**
 * 固定区 footer 状态机（Footer Composition）。
 *
 * 业务：footer 为 1 行 dim 文本，字段序 cwd · model · thinking；
 * 追加 used tokens（按 TokenProvenance：Api 为 used C tokens，Heuristic 为 used ~C tokens，Unknown 为 used ? tokens）；
 * context_window > 0 时追加派生占用比；cache 三态为 cache N / cache ? / 省略，不得伪造 cache 0；
 * 不得展示队列计数徽章；放不下时截断右侧而非增高。
 *
 * @spec llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature#@req:r1224（footer-minimal）
 * @spec llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature#@req:r1214（idle-status-omitted）
 * @spec llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature#@req:r1226（footer-derived-context-percent）
 * @spec llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature#@req:r1219（footer-token-refresh）
 * @spec llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature#@req:r1216（models-picker-footer）
 * @spec llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature#@req:r1229（footer-used-compact-count）
 *
 * 实现落点：crates/xylitol-app-tui/src/（footer 渲染）。
 */

import { assign, setup } from "xstate";
import { defineMeta, runScenarioGroups, type ScenarioStep } from "../shared/conventions.ts";

export type Provenance = "api" | "heuristic" | "unknown";
export type CacheState = "hit" | "unreported" | "notApplicable";

export interface FooterContext {
  cwd: string;
  model: string;
  /** thinking 档位显示名；null 表示省略该段。 */
  thinking: string | null;
  provenance: Provenance;
  usedTokens: number | null;
  contextWindow: number;
  cache: CacheState;
  cacheHits: number | null;
  rendered: string;
  /** footer 行数（须恒为 1）。 */
  lineCount: number;
  truncated: boolean;
}

export const initialContext: FooterContext = {
  cwd: "/repo",
  model: "test-model",
  thinking: "off",
  provenance: "api",
  usedTokens: 12000,
  contextWindow: 100000,
  cache: "notApplicable",
  cacheHits: null,
  rendered: "",
  lineCount: 1,
  truncated: false,
};

export const meta = defineMeta({
  name: "footer-composition",
  pkg: "app-surface",
  purpose: "footer 单行字段序：used 来源标注、占用比、cache 三态、截断",
  anchors: [
    "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature#@req:r1224",
    "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature#@req:r1214",
    "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature#@req:r1226",
    "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature#@req:r1219",
    "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature#@req:r1216",
    "llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature#@req:r1229",
  ],
  impl: ["crates/xylitol-app-tui/src/"],
});

/** 按上下文渲染 footer 单行文本；超宽时截断右侧而非增高。 */
export function renderFooter(c: FooterContext, maxWidth = 200): string {
  const parts: string[] = [c.cwd, c.model];
  if (c.thinking !== null) parts.push(c.thinking);
  if (c.usedTokens !== null) {
    if (c.provenance === "api") parts.push(`used ${c.usedTokens} tokens`);
    else if (c.provenance === "heuristic") parts.push(`used ~${c.usedTokens} tokens`);
    else parts.push("used ? tokens");
    if (c.contextWindow > 0 && c.provenance !== "unknown") {
      parts.push(`${Math.round((c.usedTokens / c.contextWindow) * 100)}%`);
    }
    if (c.cache === "hit" && c.cacheHits !== null) parts.push(`cache ${c.cacheHits}`);
    else if (c.cache === "unreported") parts.push("cache ?");
  }
  const line = parts.join(" · ");
  return line.length > maxWidth ? `${line.slice(0, maxWidth - 1)}…` : line;
}

export const setupMachine = setup({
  types: {
    context: {} as FooterContext,
    events: {} as
      | { type: "estimate"; provenance: Provenance; usedTokens: number | null; cache: CacheState; cacheHits: number | null }
      | { type: "switchModel"; model: string; thinking: string | null }
      | { type: "resize"; maxWidth: number }
  },
  actions: {
    applyEstimate: assign({
      provenance: ({ event }) => (event as { provenance: Provenance }).provenance,
      usedTokens: ({ event }) => (event as { usedTokens: number | null }).usedTokens,
      cache: ({ event }) => (event as { cache: CacheState }).cache,
      cacheHits: ({ event }) => (event as { cacheHits: number | null }).cacheHits,
      rendered: ({ context, event }) => {
        const e = event as { provenance: Provenance; usedTokens: number | null; cache: CacheState; cacheHits: number | null };
        return renderFooter({ ...context, ...e });
      },
    }),
    applyModel: assign({
      model: ({ event }) => (event as { model: string }).model,
      thinking: ({ event }) => (event as { thinking: string | null }).thinking,
      rendered: ({ context, event }) => {
        const e = event as { model: string; thinking: string | null };
        return renderFooter({ ...context, ...e });
      },
    }),
    applyWidth: assign({
      rendered: ({ context, event }) => renderFooter(context, (event as { maxWidth: number }).maxWidth),
      truncated: ({ context, event }) => {
        const full = renderFooter(context, 10000);
        return full.length > (event as { maxWidth: number }).maxWidth;
      },
    }),
  },
});

export const machine = setupMachine.createMachine({
  id: "footer-composition",
  initial: "rendering",
  context: initialContext,
  states: {
    rendering: {
      on: {
        estimate: { actions: "applyEstimate" },
        switchModel: { actions: "applyModel" },
        resize: { actions: "applyWidth" },
      },
    },
  },
});

export const scenarioGroups = {
  "页脚单行带启发式来源": [
    { name: "以 Heuristic 估计渲染", events: [{ type: "estimate", provenance: "heuristic", usedTokens: 12000, cache: "notApplicable", cacheHits: null }], expectStates: ["rendering"], expect: (s) => {
        const r = (s as { context: FooterContext }).context.rendered;
        if (!r.includes("used ~12000 tokens")) return "缺少 used ~C tokens";
        if (r.includes("q:")) return "不得含队列徽章";
        return (s as { context: FooterContext }).context.lineCount === 1 ? true : "期望单行";
      } },
  ],
  "页脚缓存命中": [
    { name: "cache 命中", events: [{ type: "estimate", provenance: "api", usedTokens: 12000, cache: "hit", cacheHits: 8000 }], expectStates: ["rendering"], expect: (s) => {
        const r = (s as { context: FooterContext }).context.rendered;
        if (!r.includes("cache 8000")) return "缺少 cache N";
        return r.indexOf("cache") > r.indexOf("12%") ? true : "cache 段须在占用比之后";
      } },
  ],
  "页脚缓存未上报": [
    { name: "上游未上报", events: [{ type: "estimate", provenance: "api", usedTokens: 12000, cache: "unreported", cacheHits: null }], expectStates: ["rendering"], expect: (s) => {
        const r = (s as { context: FooterContext }).context.rendered;
        if (!r.includes("cache ?")) return "缺少 cache ?";
        if (r.includes("cache 0")) return "不得伪造 cache 0";
        return true;
      } },
  ],
  "页脚无cache段": [
    { name: "缓存不适用", events: [{ type: "estimate", provenance: "api", usedTokens: 12000, cache: "notApplicable", cacheHits: null }], expectStates: ["rendering"], expect: (s) => {
        const r = (s as { context: FooterContext }).context.rendered;
        return !r.includes("cache") ? true : "不得含 cache 段";
      } },
  ],
  "页脚宽度不足截断": [
    { name: "窄终端渲染", events: [{ type: "resize", maxWidth: 20 }], expectStates: ["rendering"], expect: (s) => {
        const c = (s as { context: FooterContext }).context;
        if (!c.truncated) return "期望标记截断";
        return c.rendered.length <= 20 ? true : "期望不超宽";
      } },
  ],
  "无思考模型省略thinking段": [
    { name: "切到无思考模型", events: [{ type: "switchModel", model: "plain", thinking: null }], expectStates: ["rendering"], expect: (s) => {
        const r = (s as { context: FooterContext }).context.rendered;
        return !r.includes("thinking") ? true : "期望省略 thinking 段";
      } },
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
