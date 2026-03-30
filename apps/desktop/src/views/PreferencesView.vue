<script setup lang="ts">
import { computed } from "vue";

import { usePreferencesStore, type ThemeMode } from "../stores/preferences";

const preferences = usePreferencesStore();
preferences.initialize();

const themeLabel = computed(() => {
  switch (preferences.resolvedTheme) {
    case "dark":
      return preferences.t("common.dark");
    default:
      return preferences.t("common.light");
  }
});

function setLocale(value: string): void {
  preferences.setLocale(value === "zh-CN" ? "zh-CN" : "en-US");
}

function setThemeMode(value: string): void {
  const nextTheme =
    value === "light" || value === "dark" ? (value as ThemeMode) : "system";
  preferences.setThemeMode(nextTheme);
}
</script>

<template>
  <section class="page-grid">
    <article class="panel panel-hero">
      <p class="eyebrow">{{ preferences.t("nav.preferences") }}</p>
      <h1 class="page-title">{{ preferences.t("preferences.title") }}</h1>
      <p class="page-subtitle">{{ preferences.t("preferences.subtitle") }}</p>
    </article>

    <article class="panel">
      <label class="field-stack">
        <span>{{ preferences.t("preferences.locale") }}</span>
        <select
          data-testid="preferences-locale"
          :value="preferences.locale"
          @change="setLocale(($event.target as HTMLSelectElement).value)"
        >
          <option value="en-US">{{ preferences.t("common.english") }}</option>
          <option value="zh-CN">{{ preferences.t("common.chinese") }}</option>
        </select>
      </label>

      <p class="muted-copy">{{ preferences.t("preferences.systemLocale") }}</p>
    </article>

    <article class="panel">
      <label class="field-stack">
        <span>{{ preferences.t("preferences.theme") }}</span>
        <select
          data-testid="preferences-theme"
          :value="preferences.themeMode"
          @change="setThemeMode(($event.target as HTMLSelectElement).value)"
        >
          <option value="system">{{ preferences.t("common.followSystem") }}</option>
          <option value="light">{{ preferences.t("common.light") }}</option>
          <option value="dark">{{ preferences.t("common.dark") }}</option>
        </select>
      </label>

      <p class="muted-copy">
        {{ preferences.t("preferences.currentTheme", { theme: themeLabel }) }}
      </p>
    </article>
  </section>
</template>
