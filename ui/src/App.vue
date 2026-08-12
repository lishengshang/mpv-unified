<script setup lang="ts">
import { RouterLink, RouterView } from "vue-router";
import { computed } from "vue";
import { t, toggleLocale } from "./i18n";

const navItems = computed(() => [
  { to: "/profiles", label: t("nav.profiles") },
  { to: "/config", label: t("nav.config") },
  { to: "/store", label: t("nav.store") },
  { to: "/help", label: t("nav.help") },
]);
</script>

<template>
  <div class="shell">
    <aside class="sidebar">
      <h1 class="brand">mpv-config</h1>
      <nav aria-label="main">
        <RouterLink
          v-for="item in navItems"
          :key="item.to"
          :to="item.to"
          class="nav-link"
        >
          {{ item.label }}
        </RouterLink>
      </nav>
    </aside>
    <div class="main-col">
      <header class="topbar">
        <span class="topbar-spacer" />
        <button
          class="lang-toggle"
          type="button"
          :title="t('app.langTitle')"
          :aria-label="t('app.langTitle')"
          @click="toggleLocale"
        >
          {{ t("app.langToggle") }}
        </button>
      </header>
      <main class="content">
        <RouterView />
      </main>
    </div>
  </div>
</template>

<style>
:root {
  color-scheme: dark;
  font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  color: #e6e6e6;
  background-color: #1b1d22;
  /* design tokens (task 18): shared by every view */
  --accent: #7c9cff;
  --accent-soft: rgba(124, 156, 255, 0.14);
  --surface: #23262d;
  --surface-hover: #2b2f38;
  --border: #33373f;
  --text: #e6e6e6;
  --text-muted: #a8adb8;
  --ok: #4cc38a;
  --warn: #e0b45c;
  --danger: #e57373;
  --radius-card: 12px;
  --radius-control: 8px;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
}

/* 键盘导航(任务 23):所有可交互元素聚焦时必须有可见焦点环 */
:where(button, a, input, select, textarea, [role="button"], [role="tab"])
  :focus-visible,
:where(button, a, input, select, textarea, [role="button"], [role="tab"]):focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.shell {
  display: flex;
  min-height: 100vh;
}

.sidebar {
  width: 200px;
  flex-shrink: 0;
  padding: 20px 12px;
  background-color: #23262d;
  border-right: 1px solid #33373f;
}

.brand {
  margin: 0 8px 24px;
  font-size: 18px;
  letter-spacing: 0.5px;
}

nav {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.nav-link {
  padding: 10px 12px;
  border-radius: 8px;
  color: #a8adb8;
  text-decoration: none;
  transition: background-color 0.15s, color 0.15s;
}

.nav-link:hover {
  background-color: #2c3038;
  color: #e6e6e6;
}

.nav-link.router-link-active {
  background-color: #3d4450;
  color: #ffffff;
}

.main-col {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.topbar {
  display: flex;
  align-items: center;
  padding: 10px 24px;
  border-bottom: 1px solid #33373f;
}

.topbar-spacer {
  flex: 1;
}

.lang-toggle {
  padding: 6px 14px;
  border: 1px solid var(--border);
  border-radius: 999px;
  background-color: transparent;
  color: var(--text-muted);
  font-size: 12px;
  cursor: pointer;
  transition: border-color 0.15s, color 0.15s;
}

.lang-toggle:hover {
  border-color: var(--accent);
  color: var(--accent);
}

.content {
  flex: 1;
  padding: 24px 40px 32px;
  overflow-y: auto;
}
</style>
