// 轻量 i18n（无第三方依赖）：
// - 首次访问按浏览器语言（navigator.language）选择中文/英文；
// - 用户手动切换后写入 localStorage（gyid.locale），优先于浏览器语言；
// - locale 是全局信号，t() / dict() 在 Solid 跟踪域内自动随语言切换更新。
import { createEffect, createSignal } from "solid-js";
import { zh, type Dict } from "./zh";
import { en } from "./en";

export type Locale = "zh" | "en";

const STORAGE_KEY = "gyid.locale";

/** 语言检测：手动选择 > 浏览器语言 > 中文兜底。 */
export function detectLocale(): Locale {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === "zh" || saved === "en") return saved;
  } catch {
    /* localStorage 不可用时忽略 */
  }
  const langs = navigator.languages?.length ? navigator.languages : [navigator.language];
  return langs.some((l) => l?.toLowerCase().startsWith("zh")) ? "zh" : "en";
}

const [locale, setLocaleSignal] = createSignal<Locale>(detectLocale());

/** 当前语言（信号读取函数）。 */
export { locale };

/** 切换语言并记住用户选择。 */
export function setLocale(l: Locale): void {
  setLocaleSignal(l);
  try {
    localStorage.setItem(STORAGE_KEY, l);
  } catch {
    /* 忽略持久化失败 */
  }
}

export function toggleLocale(): void {
  setLocale(locale() === "zh" ? "en" : "zh");
}

const dictionaries: Record<Locale, Dict> = { zh, en };

/** 取整棵词典（响应式；用于数组/对象型文案）。 */
export function dict(): Dict {
  return dictionaries[locale()];
}

// 仅对纯对象/字符串节点生成点路径，数组不通过 t() 寻址（请用 dict()）。
type Path<T> = T extends object
  ? {
      [K in keyof T & string]: T[K] extends string
        ? K
        : T[K] extends readonly unknown[]
          ? never
          : T[K] extends object
            ? `${K}.${Path<T[K]>}`
            : never;
    }[keyof T & string]
  : never;

export type TKey = Path<Dict>;

/** 翻译标量文案；{name} 形式的占位符用 params 填充。 */
export function t(
  key: TKey,
  params?: Record<string, string | number>,
): string {
  const parts = key.split(".");
  let cur: unknown = dictionaries[locale()];
  for (const p of parts) {
    cur = (cur as Record<string, unknown>)[p];
  }
  let s = cur as string;
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      s = s.replaceAll(`{${k}}`, String(v));
    }
  }
  return s;
}

/** 跟随当前语言的 BCP 47 标签（用于 toLocaleString 等）。 */
export function localeTag(): string {
  return locale() === "zh" ? "zh-CN" : "en-US";
}

export function fmtDateTime(ms: number): string {
  return new Date(ms).toLocaleString(localeTag());
}

export function fmtTime(sec: number): string {
  return new Date(sec * 1000).toLocaleTimeString(localeTag());
}

// 同步 <html lang> 与文档标题。
if (typeof document !== "undefined") {
  createEffect(() => {
    document.documentElement.lang = locale() === "zh" ? "zh-CN" : "en";
    document.title = t("meta.title");
  });
}
