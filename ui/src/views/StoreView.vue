<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onMounted, ref } from "vue";

type Status = "Available" | "Installed" | "Updatable" | "Pending";

interface StoreEntry {
  name: string;
  description: string;
  installed_version: string | null;
  available_version: string | null;
  pending: boolean;
  status: Status;
  file_count: number;
  source: string;
  repo: string | null;
}

interface ActionResult {
  ok: boolean;
  message: string;
  warnings: string[];
}

type Filter = "all" | "installed" | "updatable" | "pending";

interface Toast {
  id: number;
  kind: "ok" | "err";
  text: string;
}

const packages = ref<StoreEntry[]>([]);
const loading = ref(true);
const error = ref("");
const query = ref("");
const filter = ref<Filter>("all");
const busy = ref("");
const toasts = ref<Toast[]>([]);
const regenHint = ref("");
let toastSeq = 0;

const filters: { key: Filter; label: string }[] = [
  { key: "all", label: "全部" },
  { key: "installed", label: "已安装" },
  { key: "updatable", label: "可更新" },
  { key: "pending", label: "pending" },
];

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  return packages.value.filter((pkg) => {
    if (q) {
      const haystack = `${pkg.name} ${pkg.description} ${pkg.repo ?? ""}`.toLowerCase();
      if (!haystack.includes(q)) return false;
    }
    switch (filter.value) {
      case "installed":
        return pkg.status === "Installed" || pkg.status === "Updatable";
      case "updatable":
        return pkg.status === "Updatable";
      case "pending":
        return pkg.status === "Pending";
      case "all":
        return true;
    }
  });
});

function toast(kind: Toast["kind"], text: string) {
  const id = ++toastSeq;
  toasts.value.push({ id, kind, text });
  setTimeout(() => {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  }, 4200);
}

function badgeText(pkg: StoreEntry): string {
  switch (pkg.status) {
    case "Available":
      return "未安装";
    case "Installed":
      return `已装 v${pkg.installed_version}`;
    case "Updatable":
      return "可更新";
    case "Pending":
      return "pending git";
  }
}

function versionLine(pkg: StoreEntry): string {
  if (pkg.status === "Updatable") {
    const next = pkg.available_version ?? "git";
    return `v${pkg.installed_version} → ${next}`;
  }
  if (pkg.installed_version) return `v${pkg.installed_version}`;
  if (pkg.available_version) return `待装 ${pkg.available_version}`;
  return "—";
}

async function load() {
  loading.value = true;
  error.value = "";
  try {
    packages.value = await invoke<StoreEntry[]>("list_packages");
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

async function refreshIndex() {
  busy.value = "index";
  try {
    const path = await invoke<string>("update_index");
    toast("ok", `索引已更新并校验通过:${path}`);
    await load();
  } catch (cause) {
    toast("err", `索引更新失败:${cause}`);
  } finally {
    busy.value = "";
  }
}

async function install(name: string) {
  busy.value = name;
  try {
    const result = await invoke<ActionResult>("install_package", { name });
    toast("ok", result.message);
    for (const warning of result.warnings) toast("err", warning);
    regenHint.value = `已安装「${name}」,请到「方案」页点击「应用并生成」重新生成配置使其生效。`;
    await load();
  } catch (cause) {
    toast("err", `安装失败:${cause}`);
  } finally {
    busy.value = "";
  }
}

async function uninstall(name: string) {
  busy.value = name;
  try {
    const result = await invoke<ActionResult>("uninstall_package", { name });
    toast("ok", result.message);
    await load();
  } catch (cause) {
    toast("err", `卸载失败:${cause}`);
  } finally {
    busy.value = "";
  }
}

async function update(name: string) {
  busy.value = name;
  try {
    const result = await invoke<ActionResult>("update_package", { name });
    toast("ok", result.message);
    for (const warning of result.warnings) toast("err", warning);
    if (result.message.includes("已更新")) {
      regenHint.value = `已更新「${name}」,请到「方案」页重新生成配置使其生效。`;
    }
    await load();
  } catch (cause) {
    toast("err", `更新失败:${cause}`);
  } finally {
    busy.value = "";
  }
}

function busyOn(pkg: StoreEntry): boolean {
  return busy.value !== "" && busy.value !== pkg.name;
}

onMounted(load);
</script>

<template>
  <section class="store">
    <header class="page-head">
      <div>
        <h2>包商店</h2>
        <p class="sub">
          浏览、搜索并安装社区脚本与着色器包。安装与卸载直接调用本机包管理器,
          冲突与依赖会在安装前校验,不会写入现役 mpv 配置目录。
        </p>
      </div>
      <div class="head-actions">
        <input
          v-model="query"
          class="search"
          type="search"
          placeholder="搜索包名称 / 描述…"
          aria-label="搜索包"
        />
        <button
          class="apply"
          :disabled="busy !== ''"
          @click="refreshIndex"
        >
          {{ busy === "index" ? "刷新中…" : "刷新索引" }}
        </button>
      </div>
    </header>

    <div class="toolbar">
      <div class="filters" role="tablist" aria-label="状态筛选">
        <button
          v-for="item in filters"
          :key="item.key"
          class="chip"
          :class="{ active: filter === item.key }"
          role="tab"
          :aria-selected="filter === item.key"
          @click="filter = item.key"
        >
          {{ item.label }}
        </button>
      </div>
      <span class="count">{{ filtered.length }} / {{ packages.length }} 个包</span>
    </div>

    <p v-if="regenHint" class="hint">{{ regenHint }}</p>

    <div v-if="loading" class="state-panel">加载包清单中…</div>

    <div v-else-if="error" class="state-panel error">
      <p>包清单加载失败</p>
      <p class="detail">{{ error }}</p>
      <button class="apply" @click="load">重试</button>
    </div>

    <div v-else-if="filtered.length === 0" class="state-panel">
      没有符合条件的包<span v-if="packages.length === 0">
        。点击「刷新索引」从 GitHub Releases 拉取最新包索引。</span
      >
    </div>

    <ul v-else class="list">
      <li v-for="pkg in filtered" :key="pkg.name" class="row">
        <div class="body">
          <div class="name-line">
            <h3>{{ pkg.name }}</h3>
            <span class="badge" :class="pkg.status">{{ badgeText(pkg) }}</span>
          </div>
          <p class="desc">{{ pkg.description || "（无描述）" }}</p>
          <p class="meta">
            {{ versionLine(pkg) }}
            <template v-if="pkg.file_count"> · {{ pkg.file_count }} 个文件</template>
            <template v-if="pkg.repo"> · 来源:{{ pkg.repo }}</template>
          </p>
        </div>
        <div class="actions">
          <template v-if="pkg.status === 'Installed' || pkg.status === 'Updatable'">
            <button
              class="ghost"
              :disabled="busyOn(pkg)"
              @click="update(pkg.name)"
            >
              {{ busy === pkg.name ? "处理中…" : "更新" }}
            </button>
            <button
              class="ghost danger"
              :disabled="busyOn(pkg)"
              @click="uninstall(pkg.name)"
            >
              {{ busy === pkg.name ? "处理中…" : "卸载" }}
            </button>
          </template>
          <button
            v-else
            class="apply"
            :disabled="busyOn(pkg)"
            @click="install(pkg.name)"
          >
            {{ busy === pkg.name ? "处理中…" : pkg.status === "Pending" ? "安装(克隆)" : "安装" }}
          </button>
        </div>
      </li>
    </ul>

    <div class="toasts" aria-live="polite">
      <div v-for="t in toasts" :key="t.id" class="toast" :class="t.kind">
        {{ t.text }}
      </div>
    </div>
  </section>
</template>

<style scoped>
.page-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 24px;
  margin-bottom: 16px;
}

h2 {
  margin: 0 0 6px;
  font-size: 22px;
}

.sub {
  margin: 0;
  max-width: 560px;
  color: var(--text-muted);
  font-size: 13px;
  line-height: 1.6;
}

.head-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}

.search {
  width: 240px;
  padding: 10px 14px;
  border: 1px solid var(--border);
  border-radius: var(--radius-control);
  background-color: var(--surface);
  color: var(--text);
  font-size: 13px;
  outline: none;
  transition: border-color 0.15s;
}

.search:focus {
  border-color: var(--accent);
}

.search::placeholder {
  color: var(--text-muted);
}

.apply {
  padding: 10px 20px;
  border: none;
  border-radius: var(--radius-control);
  background-color: var(--accent);
  color: #10131a;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: filter 0.15s, transform 0.15s;
}

.apply:hover:not(:disabled) {
  filter: brightness(1.1);
}

.apply:active:not(:disabled) {
  transform: translateY(1px);
}

.apply:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 16px;
}

.filters {
  display: flex;
  gap: 8px;
}

.chip {
  padding: 6px 14px;
  border: 1px solid var(--border);
  border-radius: 999px;
  background-color: transparent;
  color: var(--text-muted);
  font-size: 12px;
  cursor: pointer;
  transition: border-color 0.15s, color 0.15s, background-color 0.15s;
}

.chip:hover {
  border-color: #4a5060;
  color: var(--text);
}

.chip.active {
  border-color: var(--accent);
  background-color: var(--accent-soft);
  color: var(--accent);
}

.count {
  color: var(--text-muted);
  font-size: 12px;
}

.hint {
  margin: 0 0 16px;
  padding: 10px 14px;
  border: 1px solid rgba(76, 195, 138, 0.4);
  border-radius: var(--radius-control);
  background-color: rgba(76, 195, 138, 0.1);
  color: var(--ok);
  font-size: 13px;
  line-height: 1.5;
}

.list {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 16px 18px;
  border: 1px solid var(--border);
  border-radius: var(--radius-card);
  background-color: var(--surface);
  transition: border-color 0.15s, background-color 0.15s;
}

.row:hover {
  border-color: #4a5060;
  background-color: var(--surface-hover);
}

.body {
  min-width: 0;
}

.name-line {
  display: flex;
  align-items: center;
  gap: 10px;
}

.name-line h3 {
  margin: 0;
  font-size: 15px;
}

.desc {
  margin: 4px 0 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-muted);
  font-size: 13px;
}

.meta {
  margin: 6px 0 0;
  color: var(--text-muted);
  font-size: 11px;
  font-family: ui-monospace, "SFMono-Regular", Consolas, monospace;
  opacity: 0.85;
}

.badge {
  padding: 2px 9px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 600;
  white-space: nowrap;
}

.badge.Available {
  background-color: rgba(168, 173, 184, 0.14);
  color: var(--text-muted);
}

.badge.Installed {
  background-color: rgba(76, 195, 138, 0.16);
  color: var(--ok);
}

.badge.Updatable {
  background-color: rgba(224, 180, 92, 0.16);
  color: var(--warn);
}

.badge.Pending {
  background-color: var(--accent-soft);
  color: var(--accent);
}

.actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.ghost {
  padding: 8px 16px;
  border: 1px solid var(--border);
  border-radius: var(--radius-control);
  background-color: transparent;
  color: var(--text);
  font-size: 13px;
  cursor: pointer;
  transition: border-color 0.15s, color 0.15s, background-color 0.15s;
}

.ghost:hover:not(:disabled) {
  border-color: var(--accent);
  color: var(--accent);
}

.ghost.danger:hover:not(:disabled) {
  border-color: var(--danger);
  color: var(--danger);
}

.ghost:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.state-panel {
  padding: 32px;
  border: 1px dashed var(--border);
  border-radius: var(--radius-card);
  text-align: center;
  color: var(--text-muted);
}

.state-panel.error p:first-child {
  margin: 0 0 8px;
  color: var(--danger);
  font-weight: 600;
}

.state-panel.error .detail {
  margin: 0 0 16px;
  font-size: 13px;
}

.toasts {
  position: fixed;
  right: 24px;
  bottom: 24px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  z-index: 100;
}

.toast {
  max-width: 460px;
  padding: 11px 16px;
  border-radius: var(--radius-control);
  background-color: var(--surface-hover);
  border: 1px solid var(--border);
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.35);
  font-size: 13px;
  line-height: 1.5;
  animation: toast-in 0.2s ease;
}

.toast.ok {
  border-color: rgba(76, 195, 138, 0.5);
  color: var(--ok);
}

.toast.err {
  border-color: rgba(229, 115, 115, 0.5);
  color: var(--danger);
}

@keyframes toast-in {
  from {
    transform: translateY(8px);
    opacity: 0;
  }
  to {
    transform: translateY(0);
    opacity: 1;
  }
}
</style>
