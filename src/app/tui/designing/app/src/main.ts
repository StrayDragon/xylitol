import "../../generated/tokens.css";
import "./shell.css";
import { loadModules } from "./catalog";
import { renderGrid } from "./cell-grid";
import type { ModulePreview } from "./types";

const modules = loadModules();

modules.sort((a, b) => a.id.localeCompare(b.id));

const nav = document.getElementById("nav") as HTMLElement;
const titleEl = document.getElementById("module-title") as HTMLElement;
const specEl = document.getElementById("module-spec") as HTMLElement;
const chipsEl = document.getElementById("state-chips") as HTMLElement;
const stageEl = document.getElementById("stage") as HTMLElement;

let current: ModulePreview | null = modules[0] ?? null;
let stateId = current ? Object.keys(current.states)[0] : "";

function paintNav(): void {
  nav.replaceChildren();
  for (const mod of modules) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.textContent = mod.title;
    btn.setAttribute("aria-current", String(mod === current));
    btn.addEventListener("click", () => {
      current = mod;
      stateId = Object.keys(mod.states)[0] ?? "";
      paint();
    });
    nav.appendChild(btn);
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
      stateId = id;
      paint();
    });
    chipsEl.appendChild(btn);
  }
}

function paintStage(): void {
  stageEl.replaceChildren();
  const state = current?.states[stateId];
  if (!state) {
    stageEl.textContent = "无模块";
    return;
  }
  const frame = document.createElement("div");
  frame.className = "term-frame";
  frame.appendChild(renderGrid(state));
  stageEl.appendChild(frame);
}

function paint(): void {
  titleEl.textContent = current?.title ?? "designing";
  specEl.textContent = current
    ? `modules/${current.id}/intent.md · 固定态对照，非产品真值`
    : "";
  paintNav();
  paintChips();
  paintStage();
}

paint();
