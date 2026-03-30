import { defineStore } from "pinia";
import { computed, ref } from "vue";

import {
  translate,
  type DesktopLocale,
  type MessageKey
} from "../copy";

export type ThemeMode = "system" | "light" | "dark";

export interface PreferenceState {
  locale: DesktopLocale;
  themeMode: ThemeMode;
}

const PREFERENCES_STORAGE_KEY = "octopus.desktop.preferences";

function currentStorage(): Storage | null {
  if (typeof window === "undefined") {
    return null;
  }

  return window.localStorage;
}

function detectSystemLocale(): DesktopLocale {
  if (typeof navigator !== "undefined" && navigator.language.toLowerCase().startsWith("zh")) {
    return "zh-CN";
  }

  return "en-US";
}

function detectSystemTheme(): "light" | "dark" {
  if (
    typeof window !== "undefined" &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-color-scheme: dark)").matches
  ) {
    return "dark";
  }

  return "light";
}

function normalizeLocale(value: unknown): DesktopLocale {
  return value === "zh-CN" ? "zh-CN" : "en-US";
}

function normalizeThemeMode(value: unknown): ThemeMode {
  if (value === "light" || value === "dark") {
    return value;
  }

  return "system";
}

function loadPreferenceState(): PreferenceState {
  const storage = currentStorage();
  if (!storage) {
    return {
      locale: detectSystemLocale(),
      themeMode: "system"
    };
  }

  const raw = storage.getItem(PREFERENCES_STORAGE_KEY);
  if (!raw) {
    return {
      locale: detectSystemLocale(),
      themeMode: "system"
    };
  }

  try {
    const parsed = JSON.parse(raw) as Partial<PreferenceState>;
    return {
      locale: normalizeLocale(parsed.locale),
      themeMode: normalizeThemeMode(parsed.themeMode)
    };
  } catch {
    storage.removeItem(PREFERENCES_STORAGE_KEY);
    return {
      locale: detectSystemLocale(),
      themeMode: "system"
    };
  }
}

function persistPreferenceState(state: PreferenceState): void {
  currentStorage()?.setItem(PREFERENCES_STORAGE_KEY, JSON.stringify(state));
}

export const usePreferencesStore = defineStore("preferences", () => {
  const initialized = ref(false);
  const locale = ref<DesktopLocale>("en-US");
  const themeMode = ref<ThemeMode>("system");
  const resolvedTheme = ref<"light" | "dark">("light");

  function applyTheme(): void {
    resolvedTheme.value =
      themeMode.value === "system" ? detectSystemTheme() : themeMode.value;

    if (typeof document === "undefined") {
      return;
    }

    document.documentElement.setAttribute("lang", locale.value);
    document.documentElement.setAttribute("data-theme", resolvedTheme.value);
    document.documentElement.setAttribute("data-theme-mode", themeMode.value);
  }

  function initialize(): void {
    const state = loadPreferenceState();
    locale.value = state.locale;
    themeMode.value = state.themeMode;
    applyTheme();

    if (
      !initialized.value &&
      typeof window !== "undefined" &&
      typeof window.matchMedia === "function"
    ) {
      const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
      const listener = () => {
        if (themeMode.value === "system") {
          applyTheme();
        }
      };
      mediaQuery.addEventListener?.("change", listener);
    }

    initialized.value = true;
  }

  function setLocale(nextLocale: DesktopLocale): void {
    locale.value = normalizeLocale(nextLocale);
    persistPreferenceState({
      locale: locale.value,
      themeMode: themeMode.value
    });
    applyTheme();
  }

  function setThemeMode(nextThemeMode: ThemeMode): void {
    themeMode.value = normalizeThemeMode(nextThemeMode);
    persistPreferenceState({
      locale: locale.value,
      themeMode: themeMode.value
    });
    applyTheme();
  }

  function t(
    key: MessageKey,
    params: Record<string, string | number> = {}
  ): string {
    return translate(locale.value, key, params);
  }

  const localeLabel = computed(() =>
    locale.value === "zh-CN" ? t("common.chinese") : t("common.english")
  );
  const themeLabel = computed(() => {
    switch (themeMode.value) {
      case "light":
        return t("common.light");
      case "dark":
        return t("common.dark");
      default:
        return t("common.followSystem");
    }
  });

  return {
    locale,
    themeMode,
    resolvedTheme,
    localeLabel,
    themeLabel,
    initialize,
    setLocale,
    setThemeMode,
    t
  };
});
