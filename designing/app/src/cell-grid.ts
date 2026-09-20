import type { StateDoc } from "./types";

export function flattenLines(state: StateDoc): string {
  return state.lines
    .map((row) => row.map((span) => span.text).join(""))
    .join("\n");
}

export function renderGrid(state: StateDoc, cols = state.cols ?? 80): HTMLElement {
  const grid = document.createElement("div");
  grid.className = "cell-grid";
  grid.style.setProperty("--cols", String(cols));
  for (const row of state.lines) {
    const line = document.createElement("div");
    line.className = "cell-row";
    if (row.some((s) => s.rev)) line.dataset.rev = "true";
    for (const span of row) {
      const el = document.createElement("span");
      el.className = "cell";
      if (span.token) el.dataset.token = span.token;
      if (span.bg) el.dataset.bg = span.bg;
      if (span.rev) el.dataset.rev = "true";
      el.textContent = span.text;
      line.appendChild(el);
    }
    grid.appendChild(line);
  }
  return grid;
}
