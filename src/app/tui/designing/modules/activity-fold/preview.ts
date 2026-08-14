import type { ModulePreview } from "../../app/src/types";

/** 模块入口：状态 YAML 由 app 组装为 cell/span，本文件只登记身份。 */
export const preview: Pick<ModulePreview, "id" | "title"> = {
  id: "activity-fold",
  title: "Activity fold",
};
