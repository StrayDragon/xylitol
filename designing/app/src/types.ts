export type Span = {
  text: string;
  token?: string;
  bg?: string;
  rev?: boolean;
  spin?: boolean;
};

export type SpinDoc = {
  frames?: string[];
  ms?: number;
};

export type StateDoc = {
  id: string;
  cols?: number;
  model?: string;
  must_contain?: string[];
  must_not_contain?: string[];
  spin?: SpinDoc;
  lines: Span[][];
};

export type TodoItem = {
  done: boolean;
  text: string;
};

export type KeyHint = {
  chord: string;
  when: string;
};

export type Alignment = {
  chrome?: string;
  item?: string;
  "todo-bar"?: string;
};

export type DraftDoc = {
  id: string;
  title: string;
  surface: string;
  summary: string;
  alignment: Alignment;
  todos: TodoItem[];
  keys?: KeyHint[];
  notes: string;
};

export type ModulePreview = DraftDoc & {
  intent: string;
  states: Record<string, StateDoc>;
};
