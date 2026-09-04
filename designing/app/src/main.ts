import "../../generated/tokens.css";
import "./shell.css";
import { renderGrid } from "./cell-grid";
import { applyFrame, hasSpin, spinFrames, spinMs } from "./animate";
import { loadModules } from "./catalog";
import { handoffCopy, handoffMarkdown } from "./handoff";
import { renderMarkdown } from "./markdown";
import {
  formatPath,
  formatUrl,
  parseLocation,
  writeLocation,
  type Route,
} from "./route";
import {
  applyRegionState,
  fitShellFrame,
  moduleById,
  renderShellFrame,
  resolveRegions,
  shellFrames,
  type ResolvedRegion,
  type ShellTheme,
} from "./shell";
import type { ModulePreview, TodoItem } from "./types";

const modules = loadModules();
const labModules = modules.filter((m) => m.surface === "tui-lab");
const titleEl = document.getElementById("module-title") as HTMLElement;
const specEl = document.getElementById("module-spec") as HTMLElement;
const chipsEl = document.getElementById("state-chips") as HTMLElement;
const stageBarEl = document.getElementById("stage-bar") as HTMLElement;
const stageEl = document.getElementById("stage") as HTMLElement;
const detailEl = document.getElementById("detail") as HTMLElement;
const endpointEl = document.getElementById("endpoint") as HTMLElement;
const copyBtn = document.getElementById("copy-handoff") as HTMLButtonElement;
const handoffMdEl = document.getElementById("handoff-md") as HTMLElement;
const handoffToggle = document.getElementById("handoff-toggle") as HTMLButtonElement;
const themeToggle = document.getElementById("theme-toggle") as HTMLButtonElement;

const SCHEME: Route["scheme"] = "light";
/// 钉住轮廓的可选颜色（截图沟通用）。
const PIN_COLORS = ["#e5534b", "#daaa3f", "#57ab5a", "#539bf5", "#b083f0", "#f778ba"];

let lastAction = "opened";

// --- shell 视图（tui 面：单一一致视图；画布纯展示，选择与钉住在右栏） ---
let shellFrameId = shellFrames[0]?.id ?? "";
let termTheme: ShellTheme = "dark";
let shellDetail: string | null = null;
let shellHover: string | null = null;
const shellPins = new Map<string, { color: string; pos: string }>();
let shellFrameHost: HTMLElement | null = null;
let shellResizeObserver: ResizeObserver | null = null;

// --- lab 视图（tui-lab 面：固定态 chips，URL 直达） ---
let lab: ModulePreview | null = null;
let labStateId = "";
let frame = 0;
let playing = false;
let playTimer = 0;

function isLabMode(): boolean {
  return Boolean(lab);
}

function shellDetailModule(): ModulePreview | null {
  return shellDetail ? (moduleById(modules, shellDetail) ?? null) : null;
}

function shellRegions(): ResolvedRegion[] {
  const frame = shellFrames.find((f) => f.id === shellFrameId);
  if (!frame) return [];
  return resolveRegions(frame).regions;
}

function routeNow(): Route | null {
  if (isLabMode() && lab) {
    if (!labStateId) return null;
    return {
      surface: lab.surface || "tui-lab",
      id: lab.id,
      state: labStateId,
      frame: playing ? 0 : frame,
      play: playing,
      scheme: SCHEME,
    };
  }
  return {
    surface: "tui",
    id: shellDetail ?? "",
    state: "",
    frame: 0,
    play: false,
    scheme: termTheme,
    view: shellFrameId,
  };
}

function applyRoute(partial: ReturnType<typeof parseLocation>): void {
  if ((partial.surface ?? "tui") === "tui-lab") {
    const hit =
      labModules.find((m) => m.id === partial.id) ??
      labModules.find((m) => m.surface === "tui-lab") ??
      null;
    lab = hit;
    const keys = hit ? Object.keys(hit.states) : [];
    labStateId =
      partial.state && keys.includes(partial.state) ? partial.state : (keys[0] ?? "");
    const st = hit?.states[labStateId];
    const n = st ? spinFrames(st).length : 0;
    frame = n ? (partial.frame ?? 0) % n : 0;
    playing = Boolean(partial.play) && n > 1;
    return;
  }
  // shell 视图：深链 /tui/<module> 直接进入该区域详情；?view= 帧与 ?scheme= 主题同样可达
  lab = null;
  const requested = partial.id ?? "";
  shellDetail = requested && moduleById(modules, requested) ? requested : null;
  if (partial.view && shellFrames.some((f) => f.id === partial.view)) {
    shellFrameId = partial.view;
  }
  if (partial.scheme === "dark" || partial.scheme === "light") {
    termTheme = partial.scheme;
  }
}

function commit(mode: "replace" | "push"): void {
  const route = routeNow();
  if (!route) return;
  writeLocation(route, mode);
  endpointEl.textContent = formatUrl(route);
}

function stopPlay(): void {
  if (playTimer) {
    window.clearInterval(playTimer);
    playTimer = 0;
  }
}

function startPlay(): void {
  stopPlay();
  const st = lab?.states[labStateId];
  if (!st || !hasSpin(st) || spinFrames(st).length < 2) {
    playing = false;
    return;
  }
  playing = true;
  playTimer = window.setInterval(() => {
    const live = lab?.states[labStateId];
    const n = live ? spinFrames(live).length : 0;
    if (!n) return;
    frame = (frame + 1) % n;
    paintLabStage();
    const label = document.getElementById("frame-label");
    if (label) label.textContent = `frame ${frame + 1}/${n}`;
  }, spinMs(st));
}

function setPlaying(next: boolean): void {
  const st = lab?.states[labStateId];
  if (!st || !hasSpin(st)) {
    playing = false;
    stopPlay();
    return;
  }
  playing = next;
  if (playing) startPlay();
  else stopPlay();
}

function stepFrame(dir: 1 | -1): void {
  const st = lab?.states[labStateId];
  const n = st ? spinFrames(st).length : 0;
  if (!n) return;
  setPlaying(false);
  frame = (frame + dir + n) % n;
  paint("replace");
}

// --- shell 视图渲染 ---

function paintShell(): void {
  const detailModule = shellDetailModule();
  // 总览标题「场景」：chips 是不同场景时刻（busy/idle）；选中区域时显示模块名。
  titleEl.textContent = detailModule ? detailModule.title : "场景";
  themeToggle.hidden = false;
  const path = formatPath({ surface: "tui", id: shellDetail ?? "", state: "" });
  specEl.textContent = `${path} · 设计对照 · 运行时真值在产品代码`;
  paintShellChips();
  paintShellStage();
  paintShellDetail();
}

function paintShellChips(): void {
  chipsEl.replaceChildren();
  for (const frame of shellFrames) {
    // 短终端是视口变体而非场景时刻：不出 chips，仅经 ?view=short 深链。
    if (frame.id === "short") continue;
    const btn = document.createElement("button");
    btn.type = "button";
    btn.textContent = frame.id;
    btn.setAttribute("aria-pressed", String(frame.id === shellFrameId));
    btn.addEventListener("click", () => {
      lastAction = `switch frame → ${frame.id}`;
      shellFrameId = frame.id;
      paint("push");
    });
    chipsEl.appendChild(btn);
  }
  syncThemeToggle();
  const hint = document.createElement("p");
  hint.className = "stage-hint";
  hint.textContent =
    "终端为产品真实渲染（just export-design-frame 再生成）；帧/亮暗可经 URL 直达（?view=&scheme=）；在右栏悬停/点击区域，钉住后可配虚线彩框截图沟通";
  stageBarEl.replaceChildren(hint);
}

function syncThemeToggle(): void {
  // 按钮显示当前终端背景模式；点击切换到另一模式。
  themeToggle.textContent = termTheme === "light" ? "亮色背景" : "暗色背景";
}

function paintShellStage(): void {
  stageEl.replaceChildren();
  const frame = shellFrames.find((f) => f.id === shellFrameId);
  if (!frame) {
    stageEl.textContent = "缺少帧数据：跑 just export-design-frame";
    return;
  }
  const { regions, warnings } = resolveRegions(frame);
  const canvas = renderShellFrame(frame, termTheme, regions);
  const wrap = document.createElement("div");
  wrap.className = "shell-frame-wrap";
  wrap.appendChild(canvas);
  stageEl.appendChild(wrap);
  shellFrameHost = canvas;
  applyRegionState(canvas, modules, shellPins, shellHover);
  const refit = () => fitShellFrame(canvas);
  requestAnimationFrame(refit);
  if (shellResizeObserver) shellResizeObserver.disconnect();
  shellResizeObserver = new ResizeObserver(refit);
  shellResizeObserver.observe(wrap);
  if (warnings.length) {
    const warn = document.createElement("p");
    warn.className = "stage-hint";
    warn.textContent = warnings.join("；");
    stageEl.appendChild(warn);
  }
}

function syncCanvasRegionState(): void {
  if (shellFrameHost) {
    applyRegionState(shellFrameHost, modules, shellPins, shellHover);
  }
}

function paintShellDetail(): void {
  detailEl.replaceChildren();
  const detailModule = shellDetailModule();
  if (detailModule) {
    const back = document.createElement("button");
    back.type = "button";
    back.className = "back-btn";
    back.textContent = "← 返回总览";
    back.addEventListener("click", () => {
      lastAction = "back to shell overview";
      shellDetail = null;
      paint("push");
    });
    detailEl.appendChild(back);

    const title = document.createElement("p");
    title.className = "summary";
    title.textContent = `${detailModule.id} | ${detailModule.title}`;
    detailEl.appendChild(title);

    paintModuleDetail(detailModule);

    const pinRow = document.createElement("div");
    pinRow.className = "pin-row";
    pinRow.appendChild(pinButton(detailModule.id));
    const pin = shellPins.get(detailModule.id);
    if (pin) {
      const dot = document.createElement("span");
      dot.className = "pin-dot";
      dot.style.background = pin.color;
      pinRow.appendChild(dot);
      pinRow.appendChild(posGrid(detailModule.id, pin));
      const swatches = document.createElement("span");
      swatches.className = "swatches";
      for (const color of PIN_COLORS) {
        const sw = document.createElement("button");
        sw.type = "button";
        sw.className = "swatch";
        sw.style.background = color;
        sw.setAttribute("aria-label", `框色 ${color}`);
        if (color === pin.color) sw.classList.add("active");
        sw.addEventListener("click", () => {
          shellPins.set(detailModule.id, { color, pos: pin.pos });
          lastAction = `pin color ${detailModule.id} → ${color}`;
          syncCanvasRegionState();
          paint("replace");
        });
        swatches.appendChild(sw);
      }
      pinRow.appendChild(swatches);
      const copyOne = document.createElement("button");
      copyOne.type = "button";
      copyOne.className = "copy-pinned";
      copyOne.textContent = "复制该区域（Markdown）";
      copyOne.addEventListener("click", () => {
        void navigator.clipboard.writeText(pinMarkdown(detailModule.id)).catch(() => undefined);
        lastAction = `copied region ${detailModule.id}`;
      });
      pinRow.appendChild(copyOne);
    }
    detailEl.appendChild(pinRow);
    return;
  }
  const help = document.createElement("p");
  help.className = "summary";
  help.textContent =
    "这是一帧真实产品渲染（由 just export-design-frame 从产品代码导出）。悬停下方条目会在中间高亮对应区域；点击展开设计稿；钉住可叠加虚线彩框方便截图沟通。";
  detailEl.appendChild(section("使用", help));
  detailEl.appendChild(section("区域", paintRegionIndex()));
  const pinned = paintPinnedIndex();
  if (pinned) detailEl.appendChild(section("已钉住", pinned));
  detailEl.appendChild(section("实验原型（URL 直达）", paintLabIndex()));
}

function paintLabIndex(): HTMLElement {
  // 与上方「区域」条目同构：同一 list + region-item 按钮视觉；点击打开 tui-lab 预览。
  const ul = document.createElement("ul");
  ul.className = "region-index";
  for (const mod of labModules) {
    const li = document.createElement("li");
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "region-item";
    btn.textContent = `${mod.id} | ${mod.title}`;
    btn.addEventListener("click", () => {
      lastAction = `open lab ${mod.id}`;
      lab = mod;
      labStateId = Object.keys(mod.states)[0] ?? "";
      frame = 0;
      setPlaying(false);
      paint("push");
    });
    li.appendChild(btn);
    ul.appendChild(li);
  }
  return ul;
}

function paintRegionIndex(): HTMLElement {
  const ul = document.createElement("ul");
  ul.className = "region-index";
  const seen = new Set<string>();
  for (const region of shellRegions()) {
    const mod = moduleById(modules, region.module);
    const key = region.label ?? region.module;
    if (seen.has(key)) continue;
    seen.add(key);
    const li = document.createElement("li");
    li.dataset.regionItem = region.module;
    li.dataset.regionLabel = region.label ?? "";
    const main = document.createElement("button");
    main.type = "button";
    main.className = "region-item";
    main.textContent = region.label
      ? `${region.label} · ${mod?.title ?? region.module}`
      : `${region.module} | ${mod?.title ?? "？"}`;
    main.addEventListener("click", () => {
      lastAction = `region detail → ${region.module}`;
      shellDetail = region.module;
      paint("push");
    });
    li.appendChild(main);
    li.appendChild(pinButton(region.module));
    ul.appendChild(li);
  }
  wireRegionHover(ul);
  return ul;
}

function pinButton(moduleId: string): HTMLButtonElement {
  const pinned = shellPins.has(moduleId);
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = "pin-btn";
  btn.textContent = pinned ? "已钉" : "钉住";
  btn.title = pinned ? "取消钉住" : "钉住（画布叠加虚线彩框）";
  btn.addEventListener("click", (ev) => {
    ev.stopPropagation();
    if (shellPins.has(moduleId)) {
      shellPins.delete(moduleId);
      lastAction = `unpin ${moduleId}`;
    } else {
      shellPins.set(moduleId, {
        color: PIN_COLORS[shellPins.size % PIN_COLORS.length],
        pos: "tr",
      });
      lastAction = `pin ${moduleId}`;
    }
    syncCanvasRegionState();
    paint("replace");
  });
  return btn;
}

function posGrid(moduleId: string, pin: { color: string; pos: string }): HTMLElement {
  const grid = document.createElement("span");
  grid.className = "pos-grid";
  grid.title = "标签位置";
  const positions: { id: string; css: string }[] = [
    { id: "tl", css: "top:2px;left:2px;" },
    { id: "tc", css: "top:2px;left:50%;margin-left:-2px;" },
    { id: "tr", css: "top:2px;right:2px;" },
    { id: "bl", css: "bottom:2px;left:2px;" },
    { id: "bc", css: "bottom:2px;left:50%;margin-left:-2px;" },
    { id: "br", css: "bottom:2px;right:2px;" },
  ];
  for (const pos of positions) {
    const cell = document.createElement("button");
    cell.type = "button";
    cell.className = `pos-cell${pin.pos === pos.id ? " active" : ""}`;
    cell.setAttribute("aria-label", `标签 ${pos.id}`);
    const dot = document.createElement("i");
    dot.className = "pos-dot";
    dot.style.cssText = pos.css;
    cell.appendChild(dot);
    cell.addEventListener("click", () => {
      shellPins.set(moduleId, { color: pin.color, pos: pos.id });
      lastAction = `label pos ${moduleId} → ${pos.id}`;
      syncCanvasRegionState();
      paint("replace");
    });
    grid.appendChild(cell);
  }
  return grid;
}

function paintPinnedIndex(): HTMLElement | null {
  if (!shellPins.size) return null;
  const wrap = document.createElement("div");
  wrap.className = "pinned-list";
  for (const [moduleId, pin] of shellPins) {
    const mod = moduleById(modules, moduleId);
    const card = document.createElement("div");
    card.className = "pinned-card";
    card.dataset.regionItem = moduleId;

    const head = document.createElement("div");
    head.className = "pinned-head";
    const dot = document.createElement("span");
    dot.className = "pin-dot";
    dot.style.background = pin.color;
    const title = document.createElement("button");
    title.type = "button";
    title.className = "pinned-title";
    title.textContent = mod?.title ?? moduleId;
    title.title = `展开 ${moduleId} 设计稿`;
    title.addEventListener("click", () => {
      shellDetail = moduleId;
      paint("push");
    });
    head.append(dot, title, pinButton(moduleId));
    card.appendChild(head);

    if (mod) {
      const brief = document.createElement("p");
      brief.className = "pinned-brief";
      brief.textContent = mod.summary;
      card.appendChild(brief);
    }

    const controls = document.createElement("div");
    controls.className = "pinned-controls";
    controls.appendChild(posGrid(moduleId, pin));
    const swatches = document.createElement("span");
    swatches.className = "swatches";
    for (const color of PIN_COLORS) {
      const sw = document.createElement("button");
      sw.type = "button";
      sw.className = "swatch";
      sw.style.background = color;
      sw.setAttribute("aria-label", `框色 ${color}`);
      if (color === pin.color) sw.classList.add("active");
      sw.addEventListener("click", () => {
        shellPins.set(moduleId, { color, pos: pin.pos });
        lastAction = `pin color ${moduleId} → ${color}`;
        syncCanvasRegionState();
        paint("replace");
      });
      swatches.appendChild(sw);
    }
    controls.appendChild(swatches);
    card.appendChild(controls);
    wrap.appendChild(card);
  }
  const copy = document.createElement("button");
  copy.type = "button";
  copy.className = "copy-pinned";
  copy.textContent = "复制钉住内容（Markdown）";
  copy.addEventListener("click", () => {
    void copyPinned();
  });
  wrap.appendChild(copy);
  wireRegionHover(wrap);
  return wrap;
}

function wireRegionHover(root: HTMLElement): void {
  for (const el of root.querySelectorAll("[data-region-item]")) {
    const id = (el as HTMLElement).dataset.regionItem as string;
    el.addEventListener("mouseenter", () => {
      shellHover = id;
      syncCanvasRegionState();
    });
    el.addEventListener("mouseleave", () => {
      if (shellHover === id) shellHover = null;
      syncCanvasRegionState();
    });
  }
}

function pinMarkdown(moduleId: string): string {
  const mod = moduleById(modules, moduleId);
  if (!mod) return "";
  const lines = [`## ${mod.id} | ${mod.title}`, "", mod.summary, ""];
  const al = mod.alignment;
  if (al.fixed || al.item || al["todo-bar"]) {
    lines.push(`- 位置：${al.fixed ?? "—"}`);
    if (al.item) lines.push(`- 与对话条目：${al.item}`);
    if (al["todo-bar"]) lines.push(`- 与待办栏：${al["todo-bar"]}`);
  }
  if (mod.notes) lines.push("", "```", mod.notes.trim(), "```");
  lines.push("", `文档：designing/tui/modules/${mod.id}/intent.md`);
  return lines.join("\n");
}

async function copyPinned(): Promise<void> {
  const text = [...shellPins.keys()].map(pinMarkdown).join("\n\n---\n\n");
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const ta = document.createElement("textarea");
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand("copy");
    ta.remove();
  }
  lastAction = "copied pinned regions";
}

// --- lab 视图渲染（固定态 chips） ---

function paintLab(): void {
  if (!lab) return;
  themeToggle.hidden = true;
  titleEl.textContent = lab.title;
  specEl.textContent = `${formatPath({
    surface: lab.surface || "tui-lab",
    id: lab.id,
    state: labStateId,
  })} · 设计对照 · 运行时真值在产品代码`;
  paintLabChips();
  paintLabStageBar();
  paintLabStage();
  paintModuleDetailOnly(lab);
}

function paintModuleDetailOnly(mod: ModulePreview): void {
  detailEl.replaceChildren();
  paintModuleDetail(mod);
}

function paintLabChips(): void {
  chipsEl.replaceChildren();
  if (!lab) return;
  for (const id of Object.keys(lab.states)) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.textContent = id;
    btn.setAttribute("aria-pressed", String(id === labStateId));
    btn.addEventListener("click", () => {
      lastAction = `selected state chip: ${labStateId} → ${id}`;
      labStateId = id;
      frame = 0;
      setPlaying(false);
      paint("push");
    });
    chipsEl.appendChild(btn);
  }
}

function paintLabStageBar(): void {
  stageBarEl.replaceChildren();
  const st = lab?.states[labStateId];
  const hint = document.createElement("p");
  hint.className = "stage-hint";
  hint.textContent = "上方按钮切换固定态；spinner 动画用播放/暂停；格子文本可框选复制";
  stageBarEl.appendChild(hint);
  if (!st || !hasSpin(st)) return;
  const n = spinFrames(st).length;
  const wrap = document.createElement("div");
  wrap.className = "frame-controls";
  const label = document.createElement("span");
  label.id = "frame-label";
  label.textContent = `frame ${frame + 1}/${n}`;
  const prev = document.createElement("button");
  prev.type = "button";
  prev.textContent = "上一帧";
  prev.addEventListener("click", (ev) => {
    ev.stopPropagation();
    lastAction = "clicked prev frame";
    stepFrame(-1);
  });
  const play = document.createElement("button");
  play.type = "button";
  play.textContent = playing ? "暂停" : "播放";
  play.setAttribute("aria-pressed", String(playing));
  play.addEventListener("click", (ev) => {
    ev.stopPropagation();
    const next = !playing;
    lastAction = next ? "clicked play" : "clicked pause";
    setPlaying(next);
    paint("replace");
  });
  const next = document.createElement("button");
  next.type = "button";
  next.textContent = "下一帧";
  next.addEventListener("click", (ev) => {
    ev.stopPropagation();
    lastAction = "clicked next frame";
    stepFrame(1);
  });
  wrap.append(prev, play, next, label);
  stageBarEl.appendChild(wrap);
}

function paintLabStage(): void {
  stageEl.replaceChildren();
  const state = lab?.states[labStateId];
  if (!state) {
    stageEl.textContent = "无固定态";
    return;
  }
  const term = document.createElement("div");
  term.className = "term-frame";
  term.setAttribute("aria-label", "设计稿预览");
  term.appendChild(renderGrid(applyFrame(state, frame)));
  stageEl.appendChild(term);
}

// --- 右栏（共用：区域文档 / lab 模块文档） ---

function section(title: string, body: HTMLElement): HTMLElement {
  const wrap = document.createElement("section");
  const h = document.createElement("h3");
  h.textContent = title;
  wrap.append(h, body);
  return wrap;
}

function paintAlignment(al: { fixed?: string; item?: string; "todo-bar"?: string }): HTMLElement {
  const dl = document.createElement("dl");
  dl.className = "align";
  const zoneLabels = { fixed: "位置", item: "与对话条目", "todo-bar": "与待办栏" };
  for (const key of ["fixed", "item", "todo-bar"] as const) {
    const val = al[key];
    if (!val) continue;
    const dt = document.createElement("dt");
    dt.textContent = zoneLabels[key];
    const dd = document.createElement("dd");
    dd.textContent = val;
    dl.append(dt, dd);
  }
  if (!dl.childElementCount) {
    const p = document.createElement("p");
    p.className = "empty";
    p.textContent = "无位置与边界备注";
    return p;
  }
  return dl;
}

function paintTodos(todos: TodoItem[]): HTMLElement {
  const ul = document.createElement("ul");
  ul.className = "todos";
  if (!todos.length) {
    const li = document.createElement("li");
    li.className = "empty";
    li.textContent = "无待办";
    ul.appendChild(li);
    return ul;
  }
  for (const todo of todos) {
    const li = document.createElement("li");
    const box = document.createElement("input");
    box.type = "checkbox";
    box.checked = todo.done;
    box.disabled = true;
    const span = document.createElement("span");
    span.textContent = todo.text;
    if (todo.done) span.className = "done";
    li.append(box, span);
    ul.appendChild(li);
  }
  return ul;
}

function paintModuleDetail(mod: ModulePreview): void {
  const summary = document.createElement("p");
  summary.className = "summary";
  summary.textContent = mod.summary;

  detailEl.append(
    section("摘要", summary),
    section("位置与边界", paintAlignment(mod.alignment)),
    section("待办", paintTodos(mod.todos)),
  );

  const keys = mod.keys ?? [];
  if (keys.length) {
    const ul = document.createElement("ul");
    ul.className = "keys-list";
    for (const k of keys) {
      const li = document.createElement("li");
      const kbd = document.createElement("kbd");
      kbd.textContent = k.chord;
      const when = document.createElement("span");
      when.textContent = k.when;
      li.append(kbd, when);
      ul.appendChild(li);
    }
    detailEl.appendChild(section("组件快捷键", ul));
  }

  if (mod.notes) {
    const pre = document.createElement("pre");
    pre.className = "notes";
    pre.textContent = mod.notes;
    detailEl.appendChild(section("备注", pre));
  }

  if (mod.intent.trim()) {
    detailEl.appendChild(section("预期行为（intent）", renderMarkdown(mod.intent.trim())));
  }
}

// --- 总控 ---

function paint(mode: "replace" | "push" = "replace"): void {
  if (isLabMode()) {
    paintLab();
  } else {
    document.body.dataset.view = "shell";
    paintShell();
  }
  commit(mode);
  paintHandoff();
  if (isLabMode() && playing) startPlay();
  else stopPlay();
}

function paintHandoff(): void {
  const route = routeNow();
  handoffMdEl.replaceChildren();
  if (!route) return;
  const mod = isLabMode() ? lab : shellDetailModule();
  handoffMdEl.appendChild(
    renderMarkdown(handoffMarkdown({ route, mod, origin: location.origin, lastAction })),
  );
}

async function copyHandoff(): Promise<void> {
  const route = routeNow();
  if (!route) return;
  const mod = isLabMode() ? lab : shellDetailModule();
  const text = handoffCopy({ route, mod, origin: location.origin, lastAction });
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    const ta = document.createElement("textarea");
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand("copy");
    ta.remove();
  }
  const prev = copyBtn.textContent;
  copyBtn.textContent = "已复制";
  window.setTimeout(() => {
    copyBtn.textContent = prev;
  }, 1200);
}

copyBtn.addEventListener("click", () => {
  void copyHandoff();
});

themeToggle.addEventListener("click", () => {
  const next: ShellTheme = termTheme === "dark" ? "light" : "dark";
  lastAction = `terminal theme → ${next}`;
  termTheme = next;
  paint("replace");
});

handoffToggle.addEventListener("click", () => {
  const willShow = handoffMdEl.hidden;
  handoffMdEl.hidden = !willShow;
  handoffToggle.textContent = willShow ? "收起 ▴" : "展开 ▾";
  handoffToggle.setAttribute("aria-expanded", String(willShow));
  lastAction = willShow ? "expanded handoff" : "collapsed handoff";
});

window.addEventListener("popstate", () => {
  applyRoute(parseLocation());
  lastAction = "browser back/forward";
  paint("replace");
});

applyRoute(parseLocation());
paint("replace");
