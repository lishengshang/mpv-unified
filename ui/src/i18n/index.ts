import { ref } from "vue";
import { enUS } from "./en-US";
import { zhCN, type MessageSchema } from "./zh-CN";

export type Locale = "zh-CN" | "en-US";

export const LOCALE_STORAGE_KEY = "mpv-config:locale";

/** A leaf value: a single string, or a readonly string array (e.g. the
 * quick-start step list). */
type Leaf = string | readonly string[];

/** Template-literal type walking a nested object to leaves:
 * `"nav.profiles" | "store.filters.all" | "help.quickStart" | ...` —
 * misspelled keys are compile errors in t(). */
type MessagePath = {
  [K in keyof MessageSchema]: MessageSchema[K] extends Leaf
    ? `${K & string}`
    : {
        [P in keyof MessageSchema[K]]: MessageSchema[K][P] extends Leaf
          ? `${K & string}.${P & string}`
          : `${K & string}.${P & string}.${keyof MessageSchema[K][P] & string}`;
      }[keyof MessageSchema[K]];
}[keyof MessageSchema];

const messages: Record<Locale, MessageSchema> = { "zh-CN": zhCN, "en-US": enUS };

function initialLocale(): Locale {
  const stored = localStorage.getItem(LOCALE_STORAGE_KEY);
  if (stored === "zh-CN" || stored === "en-US") return stored;
  return navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en-US";
}

const locale = ref<Locale>(initialLocale());

/** Set the UI language, persist it and mirror it on <html lang>. */
export function setLocale(next: Locale) {
  locale.value = next;
  localStorage.setItem(LOCALE_STORAGE_KEY, next);
  document.documentElement.lang = next;
}

/** Toggle between the two supported languages (keeps the other state). */
export function toggleLocale() {
  setLocale(locale.value === "zh-CN" ? "en-US" : "zh-CN");
}

function resolve(path: string): string | readonly string[] {
  let value: unknown = messages[locale.value];
  for (const segment of path.split(".")) {
    if (typeof value !== "object" || value === null) return path;
    value = (value as Record<string, unknown>)[segment];
  }
  if (typeof value === "string") return value;
  if (Array.isArray(value)) return value;
  return path;
}

/** Look up a message path, substituting `{name}` placeholders. Reads
 * `locale` during render, so changing the locale re-renders dependents.
 * String leaves only — use [`tList`] for array leaves. */
export function t<Path extends MessagePath>(
  path: Path,
  params?: Record<string, string | number>,
): string {
  const value = resolve(path);
  if (typeof value !== "string") return path;
  if (!params) return value;
  return value.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in params ? String(params[name]) : match,
  );
}

/** Look up an array leaf (e.g. `help.quickStart`) for `v-for` iteration;
 * `[]` when the path does not resolve to an array. */
export function tList<Path extends MessagePath>(path: Path): readonly string[] {
  const value = resolve(path);
  return Array.isArray(value) ? value : [];
}

/** Reactive access to the current locale (for conditional rendering). */
export function useI18n() {
  return { locale, t, setLocale, toggleLocale };
}
