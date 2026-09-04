import { parse } from "yaml";
import type { ModulePreview } from "./types";

export type FrameSpan = {
  text: string;
  token?: string | null;
  fg: string;
  bg?: string | null;
  bgToken?: string | null;
  rev?: boolean;
};

export type ShellFrame = {
  id: string;
  rows: number;
  lines: FrameSpan[][];
};

export type ShellTheme = "dark" | "light";

type ThemeDoc = { surface: string; tokens?: Record<string, string> };

export type ShellFrameDoc = {
  cols: number;
  generated_by: string;
  themes: { dark: ThemeDoc; light: ThemeDoc & { tokens: Record<string, string> } };
  frames: ShellFrame[];
};

export type RegionDef = {
  module: string;
  contains: string;
  rows: number;
  /// 区域显示名（同一模块可拆多个条目级区域，如 transcript 的用户消息/助手回复）。
  label?: string;
};

export type PinInfo = {
  color: string;
  pos: string;
};

export type ResolvedRegion = {
  module: string;
  label?: string;
  start: number;
  end: number;
};

import shellFrameJson from "../../generated/shell-frame.json";
import regionsRaw from "../../tui/shell.regions.yaml?raw";

const frameDoc = shellFrameJson as unknown as ShellFrameDoc;

export const shellFrames = frameDoc.frames;
export const shellCols = frameDoc.cols;
export const shellThemes = frameDoc.themes;

const regionFrames = (parse(regionsRaw) as { frames: Record<string, RegionDef[]> }).frames;

export function regionsForFrame(frameId: string): RegionDef[] {
  return regionFrames[frameId] ?? [];
}

/// 把锚点定义解析成行区间；锚点未命中的区域跳过（返回 warnings 供侧栏提示）。
export function resolveRegions(frame: ShellFrame): {
  regions: ResolvedRegion[];
  warnings: string[];
} {
  const regions: ResolvedRegion[] = [];
  const warnings: string[] = [];
  for (const def of regionsForFrame(frame.id)) {
    // 锚点按整行拼接文本匹配：跨色 span（如 spinner+短词）也能命中。
    const index = frame.lines.findIndex((line) =>
      line.map((span) => span.text).join("").includes(def.contains),
    );
    if (index < 0) {
      warnings.push(`区域 ${def.module} 锚点未命中：${def.contains}`);
      continue;
    }
    regions.push({
      module: def.module,
      label: def.label,
      start: index,
      end: Math.min(index + def.rows, frame.lines.length),
    });
  }
  regions.sort((a, b) => a.start - b.start);
  return { regions, warnings };
}

function spanColor(
  span: FrameSpan,
  theme: ShellTheme,
  kind: "fg" | "bg",
): string | null {
  if (theme === "dark") return kind === "bg" ? (span.bg ?? null) : span.fg;
  // light：优先按 token 反查亮色（fg 用 token，bg 用 bgToken），无 token 保留原 hex。
  const token = kind === "bg" ? span.bgToken : span.token;
  if (token) {
    const mapped = shellThemes.light.tokens[token];
    if (mapped) return mapped;
  }
  return kind === "bg" ? (span.bg ?? null) : span.fg;
}

/// 帧渲染：按行区间包裹 data-region（opencode session-v2 的块包裹模式，
/// 命中测试走事件委托）。画布纯展示：无点击选中（选择在右栏完成），
/// 文本可自由拖选复制。
export function renderShellFrame(
  frame: ShellFrame,
  theme: ShellTheme,
  regions: ResolvedRegion[],
): HTMLElement {
  const host = document.createElement("div");
  host.className = "shell-frame";
  host.dataset.theme = theme;
  host.style.setProperty("--term-bg", shellThemes[theme].surface);

  const lineToRegion = new Map<number, ResolvedRegion>();
  for (const region of regions) {
    for (let i = region.start; i < region.end; i += 1) {
      if (!lineToRegion.has(i)) lineToRegion.set(i, region);
    }
  }

  let currentRegion: ResolvedRegion | null = null;
  let regionEl: HTMLElement | null = null;

  frame.lines.forEach((line, i) => {
    const region = lineToRegion.get(i) ?? null;
    if (region !== currentRegion) {
      currentRegion = region;
      regionEl = null;
      if (region) {
        regionEl = document.createElement("div");
        regionEl.className = "shell-region";
        regionEl.dataset.region = region.module;
        host.appendChild(regionEl);
      }
    }
    const row = document.createElement("div");
    row.className = "shell-line";
    for (const span of line) {
      const el = document.createElement("span");
      el.textContent = span.text;
      const fg = spanColor(span, theme, "fg");
      const bg = spanColor(span, theme, "bg");
      if (span.rev) {
        el.style.color = bg ?? shellThemes[theme].surface;
        el.style.backgroundColor = fg ?? shellThemes[theme].surface;
      } else {
        el.style.color = fg ?? shellThemes[theme].surface;
        if (bg) el.style.backgroundColor = bg;
      }
      row.appendChild(el);
    }
    if (regionEl) regionEl.appendChild(row);
    else host.appendChild(row);
  });

  return host;
}

/// 同步右栏 → 画布的状态：钉住（虚线彩框 + 小字标签，位置可选）与悬停高亮。
export function applyRegionState(
  host: HTMLElement,
  modules: ModulePreview[],
  pins: Map<string, PinInfo>,
  hoverModule: string | null,
): void {
  for (const el of host.querySelectorAll("[data-region]")) {
    const el2 = el as HTMLElement;
    const id = el2.dataset.region as string;
    const pin = pins.get(id);
    el2.classList.toggle("pinned", Boolean(pin));
    el2.classList.toggle("hover-sync", id === hoverModule);
    if (pin) el2.style.setProperty("--pin-color", pin.color);
    else el2.style.removeProperty("--pin-color");
    const existing = el2.querySelector(":scope > .region-label") as HTMLElement | null;
    if (pin) {
      const mod = modules.find((m) => m.id === id);
      const label = existing ?? document.createElement("span");
      label.className = `region-label pos-${pin.pos}`;
      label.textContent = `${id}${mod ? ` · ${mod.title}` : ""}`;
      if (!existing) el2.appendChild(label);
    } else if (existing) existing.remove();
  }
}

/// 尺寸策略：固定长宽比（等宽字 + em 排版，改 font-size 即等比缩放，布局盒真实收缩）。
/// 宽度尽量平铺 stage 列；放大封顶 1.2×（不要过大）；高度超出容器时交给滚动。
const SHELL_FRAME_BASE_FONT_PX = 13; // 与 .shell-frame 的 font-size 保持一致

export function fitShellFrame(host: HTMLElement): void {
  const wrap = host.parentElement;
  if (!wrap) return;
  host.style.fontSize = "";
  const w = host.offsetWidth;
  if (!w) return;
  const scale = Math.min(wrap.clientWidth / w, 1.2);
  if (scale === 1) return;
  host.style.fontSize = `${SHELL_FRAME_BASE_FONT_PX * scale}px`;
  // 字号换算与像素取整有偏差：复测一次，超宽则按比例回调。
  const w2 = host.offsetWidth;
  if (w2 > wrap.clientWidth) {
    host.style.fontSize = `${SHELL_FRAME_BASE_FONT_PX * scale * (wrap.clientWidth / w2)}px`;
  }
}

export function moduleById(modules: ModulePreview[], id: string): ModulePreview | undefined {
  return modules.find((m) => m.surface === "tui" && m.id === id);
}
