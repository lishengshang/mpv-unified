<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onMounted, ref } from "vue";
import { t } from "../i18n";
import StatePanel from "../components/StatePanel.vue";
import ToastStack from "../components/ToastStack.vue";
import { useToasts } from "../composables/useToasts";

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

const packages = ref<StoreEntry[]>([]);
const loading = ref(true);
const error = ref("");
const query = ref("");
const filter = ref<Filter>("all");
const busy = ref("");
const regenHint = ref("");
const { toasts, toast } = useToasts(4200);

const filters = computed<{ key: Filter; label: string }[]>(() => [
  { key: "all", label: t("store.filters.all") },
  { key: "installed", label: t("store.filters.installed") },
  { key: "updatable", label: t("store.filters.updatable") },
  { key: "pending", label: t("store.filters.pending") },
]);

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

function badgeText(pkg: StoreEntry): string {
  switch (pkg.status) {
    case "Available":
      return t("store.status.available");
    case "Installed":
      return t("store.status.installed", { version: pkg.installed_version ?? "" });
    case "Updatable":
      return t("store.status.updatable");
    case "Pending":
      return t("store.status.pending");
  }
}

function versionLine(pkg: StoreEntry): string {
  if (pkg.status === "Updatable") {
    return t("store.versionUpdatable", {
      installed: pkg.installed_version ?? "?",
      next: pkg.available_version ?? "git",
    });
  }
  if (pkg.installed_version) return t("store.versionInstalled", { version: pkg.installed_version });
  if (pkg.available_version) return t("store.versionPending", { version: pkg.available_version });
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
    toast("ok", t("store.indexUpdated", { path }));
    await load();
  } catch (cause) {
    toast("err", t("store.indexFailed", { error: String(cause) }));
  } finally {
    busy.value = "";
  }
}

async function install(name: string) {
  busy.value = name;
  try {
    const result = await invoke<ActionResult>("install_package", { name });
    toast("ok", t("store.installOk", { message: result.message }));
    for (const warning of result.warnings) toast("err", warning);
    regenHint.value = t("store.regenHintInstall", { name });
    await load();
  } catch (cause) {
    toast("err", t("store.installFailed", { error: String(cause) }));
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
    toast("err", t("store.uninstallFailed", { error: String(cause) }));
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
      regenHint.value = t("store.regenHintUpdate", { name });
    }
    await load();
  } catch (cause) {
    toast("err", t("store.updateFailed", { error: String(cause) }));
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
        <h2>{{ t("store.title") }}</h2>
        <p class="sub">{{ t("store.subtitle") }}</p>
      </div>
      <div class="head-actions">
        <input
          v-model="query"
          class="search"
          type="search"
          :placeholder="t('store.searchPlaceholder')"
          :aria-label="t('store.searchAria')"
        />
        <button
          class="apply"
          :disabled="busy !== ''"
          @click="refreshIndex"
        >
          {{ busy === "index" ? t("store.refreshing") : t("store.refresh") }}
        </button>
      </div>
    </header>

    <div class="toolbar">
      <div class="filters" role="tablist" :aria-label="t('store.filterAria')">
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
      <span class="count">{{ t("store.count", { shown: filtered.length, total: packages.length }) }}</span>
    </div>

    <p v-if="regenHint" class="hint">{{ regenHint }}</p>

    <StatePanel v-if="loading" kind="loading" :message="t('store.loading')" />

    <StatePanel
      v-else-if="error"
      kind="error"
      :message="t('store.loadFailed')"
      :detail="error"
      show-retry
      @retry="load"
    />

    <StatePanel
      v-else-if="filtered.length === 0"
      kind="empty"
      :message="packages.length === 0 ? t('store.emptyIndex') : t('store.empty')"
    />

    <ul v-else class="list">
      <li v-for="pkg in filtered" :key="pkg.name" class="row">
        <div class="body">
          <div class="name-line">
            <h3>{{ pkg.name }}</h3>
            <span class="badge" :class="pkg.status">{{ badgeText(pkg) }}</span>
          </div>
          <p class="desc">{{ pkg.description || t("store.noDesc") }}</p>
          <p class="meta">
            {{ versionLine(pkg) }}
            <template v-if="pkg.file_count"> · {{ t("store.fileCount", { n: pkg.file_count }) }}</template>
            <template v-if="pkg.repo"> · {{ t("store.source", { repo: pkg.repo }) }}</template>
          </p>
        </div>
        <div class="actions">
          <template v-if="pkg.status === 'Installed' || pkg.status === 'Updatable'">
            <button
              class="ghost"
              :disabled="busyOn(pkg)"
              @click="update(pkg.name)"
            >
              {{ busy === pkg.name ? t("store.busy") : t("store.update") }}
            </button>
            <button
              class="ghost danger"
              :disabled="busyOn(pkg)"
              @click="uninstall(pkg.name)"
            >
              {{ busy === pkg.name ? t("store.busy") : t("store.uninstall") }}
            </button>
          </template>
          <button
            v-else
            class="apply"
            :disabled="busyOn(pkg)"
            @click="install(pkg.name)"
          >
            {{
              busy === pkg.name
                ? t("store.busy")
                : pkg.status === "Pending"
                  ? t("store.installClone")
                  : t("store.install")
            }}
          </button>
        </div>
      </li>
    </ul>

    <ToastStack :toasts="toasts" />
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
</style>
