import type { Route } from "./route";
import type { ModulePreview } from "./types";

export type HandoffInput = {
  route: Route;
  mod: ModulePreview;
};

export function handoffPaths(input: HandoffInput): string[] {
  const { route, mod } = input;
  return [
    `designing/${mod.surface}/modules/${mod.id}/intent.md`,
    `designing/${mod.surface}/modules/${mod.id}/draft.yaml`,
    `designing/${mod.surface}/modules/${mod.id}/states/${route.state}.yaml`,
    "designing/AGENTS.md",
    "designing/generated/AGENT-INDEX.md",
    "src/app/tui/DESIGN.md",
    "src/app/tui/",
  ];
}

export function handoffCopy(input: HandoffInput): string {
  return handoffPaths(input).join("\n");
}

export function handoffMarkdown(input: HandoffInput): string {
  return handoffPaths(input).map((p) => `- \`${p}\``).join("\n");
}
