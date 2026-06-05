import type { AppDictionary, AppLanguage, LanguageOption } from "./types";
import { zhCN } from "./zh-CN";
import { enUS } from "./en-US";

export const LANGUAGE_OPTIONS: LanguageOption[] = [
  { value: "zh-CN", label: "简体中文", shortLabel: "中" },
  { value: "en-US", label: "English", shortLabel: "EN" },
];

export const dictionaries: Record<AppLanguage, AppDictionary> = {
  "zh-CN": zhCN,
  "en-US": enUS,
};

export const normalizeLanguage = (value?: string): AppLanguage => (value === "en-US" || value === "en" ? "en-US" : "zh-CN");
