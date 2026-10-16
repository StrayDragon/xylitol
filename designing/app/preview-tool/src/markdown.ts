import { marked } from "marked";

marked.use({
  gfm: true,
  breaks: false,
  renderer: {
    html() {
      return "";
    },
  },
});

export function renderMarkdown(src: string): HTMLElement {
  const wrap = document.createElement("div");
  wrap.className = "md-body";
  wrap.innerHTML = marked.parse(src, { async: false }) as string;
  return wrap;
}
