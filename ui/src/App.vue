<script setup lang="ts">
import { RouterLink, RouterView } from "vue-router";

const navItems = [
  { to: "/profiles", label: "方案" },
  { to: "/config", label: "配置" },
  { to: "/store", label: "包商店" },
  { to: "/help", label: "帮助" },
] as const;
</script>

<template>
  <div class="shell">
    <aside class="sidebar">
      <h1 class="brand">mpv-config</h1>
      <nav>
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
    <main class="content">
      <RouterView />
    </main>
  </div>
</template>

<style>
:root {
  color-scheme: dark;
  font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  color: #e6e6e6;
  background-color: #1b1d22;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
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

.content {
  flex: 1;
  padding: 32px 40px;
  overflow-y: auto;
}
</style>
