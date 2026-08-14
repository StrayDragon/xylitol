export type Span = {
  text: string;
  token?: string;
};

export type StateDoc = {
  id: string;
  cols?: number;
  model?: string;
  must_contain?: string[];
  must_not_contain?: string[];
  lines: Span[][];
};

export type ModulePreview = {
  id: string;
  title: string;
  states: Record<string, StateDoc>;
};
