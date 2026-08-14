import "../../generated/tokens.css";
import "./shell.css";
import { applyFrame, hasSpin, spinFrames, spinMs } from "./animate";
import { loadModules } from "./catalog";
import { renderGrid } from "./cell-grid";
import { handoffCopy, handoffMarkdown } from "./handoff";
import { renderMarkdown } from "./markdown";
import { cycleState } from "./interact";
import {
  formatPath,
  formatUrl,
  parseLocation,
  writeLocation,
  type Route,
  type Scheme,
} from "./route";
import type { Alignment, ModulePreview, TodoItem } from "./types";

const modules = loadModules();
const nav = document.getElementById("nav") as HTMLElement;
const titleEl = document.getElementById("module-title") as HTMLElement;
const specEl = document.getElementById("module-spec") as HTMLElement;
const chipsEl = document.getElementById("state-chips") as HTMLElement;
const stageBarEl = document.getElementById("stage-bar") as HTMLElement;
const stageEl = document.getElementById("stage") as HTMLElement;
const detailEl = document.getElementById("detail") as HTMLElement;
const endpointEl = document.getElementById("endpoint") as HTMLElement;
const copyBtn = document.getElementById("copy-handoff") as HTMLButtonElement;
const handoffMdEl = document.getElementById("handoff-md") as HTMLElement;
const schemeBtn = document.getElementById("scheme") as HTMLButtonElement;

let current: ModulePreview | null = modules[0] ?? null;
let stateId = current ? Object.keys(current.states)[0] : "";
let frame = 0;
let playing = false;
let scheme: Scheme = "light";
let playTimer = 0;
let lastAction = "opened";

function currentState() {
  return current?.states[stateId];
}

function routeNow(): Route | null {
  if (!current || !stateId) return null;
  return {
    surface: current.surface || "tui",
    id: current.id,
    state: stateId,
    frame: playing ? 0 : frame,
    play: playing,
    scheme,
  };
}

function applyRoute(partial: Partial<Route>, fallback: ModulePreview | null): void {
  const surface = partial.surface;
  const hit =
    modules.find((m) => m.id === partial.id && (!surface || m.surface === surface)) ??
    modules.find((m) => m.surface === (surface || "tui")) ??
    fallback;
  if (!hit) return;
  current = hit;
  const keys = Object.keys(hit.states);
  stateId = partial.state && keys.includes(partial.state) ? partial.state : (keys[0] ?? "");
  const st = current.states[stateId];
  const n = st ? spinFrames(st).length : 0;
  frame = n ? (partial.frame ?? 0) % n : 0;
  playing = Boolean(partial.play) && n > 1;
  if (partial.scheme) scheme = partial.scheme;
}

function syncSchemeUi(): void {
  document.body.dataset.scheme = scheme;
  schemeBtn.textContent = scheme === "light" ? "dark" : "light";
  schemeBtn.setAttribute("aria-pressed", String(scheme === "dark"));
}

function commit(mode: "replace" | "push"): void {
  const route = routeNow();
  if (!route) return;
  writeLocation(route, mode);
  endpointEl.textContent = formatUrl(route);
}

function startPlay(): void {
  stopPlay();
  const st = currentState();
  if (!st || !hasSpin(st) || spinFrames(st).length < 2) {
    playing = false;
    return;
  }
  playing = true;
  playTimer = window.setInterval(() => {
    const live = currentState();
    const n = live ? spinFrames(live).length : 0;
    if (!n) return;
    frame = (frame + 1) % n;
    paintStage();
    const label = document.getElementById("frame-label");
    if (label) {
      label.textContent = `frame ${frame + 1}/${n}`;
    }
  }, spinMs(st));
}

function stopPlay(): void {
  if (playTimer) {
    window.clearInterval(playTimer);
    playTimer = 0;
  }
}

function setPlaying(next: boolean): void {
  const st = currentState();
  if (!st || !hasSpin(st)) {
    playing = false;
    stopPlay();
    return;
  }
  playing = next;
  if (playing) startPlay();
  else stopPlay();
}

function paintNav(): void {
  nav.replaceChildren();
  const groups = new Map<string, ModulePreview[]>();
  for (const mod of modules) {
    const surface = mod.surface || "tui";
    const list = groups.get(surface) ?? [];
    list.push(mod);
    groups.set(surface, list);
  }
  for (const [surface, list] of groups) {
    const h = document.createElement("h3");
    h.textContent = `/${surface}/`;
    nav.appendChild(h);
    for (const mod of list) {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.textContent = mod.title;
      btn.title = `/${mod.surface}/${mod.id}`;
      btn.setAttribute("aria-current", String(mod === current));
      btn.addEventListener("click", () => {
        current = mod;
        stateId = Object.keys(mod.states)[0] ?? "";
        frame = 0;
        lastAction = `opened module ${mod.surface}/${mod.id} state ${stateId}`;
        setPlaying(false);
        paint("push");
      });
      nav.appendChild(btn);
    }
  }
}

function paintChips(): void {
  chipsEl.replaceChildren();
  if (!current) return;
  for (const id of Object.keys(current.states)) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.textContent = id;
    btn.setAttribute("aria-pressed", String(id === stateId));
    btn.addEventListener("click", () => {
      lastAction = `selected state chip: ${stateId} → ${id}`;
      stateId = id;
      frame = 0;
      setPlaying(false);
      paint("push");
    });
    chipsEl.appendChild(btn);
  }
}

function paintStageBar(): void {
  stageBarEl.replaceChildren();
  const st = currentState();
  const hint = document.createElement("p");
  hint.className = "stage-hint";
  hint.textContent = "点击预览切固定态 · 动画只用播放/暂停按钮";
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

function paintStage(): void {
  stageEl.replaceChildren();
  const state = currentState();
  if (!state) {
    stageEl.textContent = "无固定态";
    return;
  }
  const term = document.createElement("div");
  term.className = "term-frame";
  term.setAttribute("aria-label", "设计稿预览，点击切固定态");
  const stateKeys = current ? Object.keys(current.states) : [];
  if (stateKeys.length > 1) term.dataset.clickable = "true";
  term.appendChild(renderGrid(applyFrame(state, frame)));
  term.addEventListener("click", () => {
    const sel = window.getSelection();
    if (sel && String(sel).length) return;
    if (!current || Object.keys(current.states).length < 2) return;
    const next = cycleState(current, stateId, 1);
    if (next === stateId) return;
    lastAction = `clicked preview: ${current.id}/${stateId} → ${current.id}/${next}`;
    stateId = next;
    frame = 0;
    setPlaying(false);
    paint("push");
  });
  stageEl.appendChild(term);
}

function stepFrame(dir: 1 | -1): void {
  const st = currentState();
  const n = st ? spinFrames(st).length : 0;
  if (!n) return;
  setPlaying(false);
  frame = (frame + dir + n) % n;
  paint("replace");
}

function section(title: string, body: HTMLElement): HTMLElement {
  const wrap = document.createElement("section");
  const h = document.createElement("h3");
  h.textContent = title;
  wrap.append(h, body);
  return wrap;
}

function paintAlignment(al: Alignment): HTMLElement {
  const dl = document.createElement("dl");
  dl.className = "align";
  for (const key of ["chrome", "item", "todo-bar"] as const) {
    const val = al[key];
    if (!val) continue;
    const dt = document.createElement("dt");
    dt.textContent = key;
    const dd = document.createElement("dd");
    dd.textContent = val;
    dl.append(dt, dd);
  }
  if (!dl.childElementCount) {
    const p = document.createElement("p");
    p.className = "empty";
    p.textContent = "无对齐备注";
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

function paintDetail(): void {
  detailEl.replaceChildren();
  if (!current) return;
  const summary = document.createElement("p");
  summary.className = "summary";
  summary.textContent = current.summary;

  detailEl.append(
    section("摘要", summary),
    section("对齐", paintAlignment(current.alignment)),
    section("待办", paintTodos(current.todos)),
  );

  const keys = current.keys ?? [];
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
    detailEl.appendChild(section("组件键", ul));
  }

  if (current.notes) {
    const pre = document.createElement("pre");
    pre.className = "notes";
    pre.textContent = current.notes;
    detailEl.appendChild(section("备注", pre));
  }

  if (current.intent.trim()) {
    detailEl.appendChild(section("intent", renderMarkdown(current.intent.trim())));
  }
}

function paint(mode: "replace" | "push" = "replace"): void {
  titleEl.textContent = current?.title ?? "designing";
  specEl.textContent = current
    ? `${formatPath({ surface: current.surface || "tui", id: current.id, state: stateId })} · 代码 SSOT · 本稿对照`
    : "";
  syncSchemeUi();
  commit(mode);
  paintNav();
  paintChips();
  paintStageBar();
  paintStage();
  paintDetail();
  paintHandoff();
  if (playing) startPlay();
  else stopPlay();
}

function paintHandoff(): void {
  const route = routeNow();
  handoffMdEl.replaceChildren();
  if (!route || !current) return;
  handoffMdEl.appendChild(
    renderMarkdown(handoffMarkdown({ route, mod: current, origin: location.origin, lastAction })),
  );
}

async function copyHandoff(): Promise<void> {
  const route = routeNow();
  if (!route || !current) return;
  const text = handoffCopy({ route, mod: current, origin: location.origin, lastAction });
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

schemeBtn.addEventListener("click", () => {
  const next = scheme === "light" ? "dark" : "light";
  lastAction = `clicked scheme: ${scheme} → ${next}`;
  scheme = next;
  paint("replace");
});

copyBtn.addEventListener("click", () => {
  void copyHandoff();
});

window.addEventListener("popstate", () => {
  applyRoute(parseLocation(), current);
  lastAction = "browser back/forward";
  paint("replace");
});

applyRoute(parseLocation(), current);
{
  const opened = routeNow();
  lastAction = opened ? `opened ${formatUrl(opened)}` : "opened";
}
paint("replace");
