<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onMounted, ref } from "vue";
import { t } from "../i18n";
import StatePanel from "../components/StatePanel.vue";
import ToastStack from "../components/ToastStack.vue";
import { useToasts } from "../composables/useToasts";

interface ProfileInfo {
  id: string;
  name: string;
  desc: string;
  icon: string;
  options: string[];
  requires: string[];
  requires_met: boolean;
}

interface RegenFile {
  path: string;
  line_count: number;
}

interface RegenSummary {
  files: RegenFile[];
  warnings: string[];
}

const profiles = ref<ProfileInfo[]>([]);
const enabled = ref<Set<string>>(new Set());
const loading = ref(true);
const error = ref("");
const busy = ref(false);
const summary = ref("");
const { toasts, toast } = useToasts();

const enabledCount = computed(() => enabled.value.size);

async function load() {
  loading.value = true;
  error.value = "";
  try {
    const [list, state] = await Promise.all([
      invoke<ProfileInfo[]>("list_profiles"),
      invoke<string[]>("get_profile_state"),
    ]);
    profiles.value = list;
    enabled.value = new Set(state);
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

async function toggle(profile: ProfileInfo) {
  if (!profile.requires_met) {
    toast(
      "err",
      t("profiles.missingDep", {
        name: profile.name,
        deps: profile.requires.join("、"),
      }),
    );
    return;
  }
  const next = new Set(enabled.value);
  if (next.has(profile.id)) {
    next.delete(profile.id);
  } else {
    next.add(profile.id);
  }
  try {
    await invoke("set_profile_state", { enabledIds: [...next] });
    enabled.value = next;
  } catch (cause) {
    toast("err", String(cause));
  }
}

async function regenerate() {
  busy.value = true;
  try {
    const report = await invoke<RegenSummary>("regenerate");
    summary.value = report.files
      .map((file) => `${file.path} (${file.line_count} 行)`)
      .join(", ");
    toast("ok", t("profiles.regenOk", { summary: summary.value }));
    for (const warning of report.warnings) {
      toast("err", warning);
    }
  } catch (cause) {
    toast("err", String(cause));
  } finally {
    busy.value = false;
  }
}

function onCardKeydown(event: KeyboardEvent, profile: ProfileInfo) {
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    toggle(profile);
  }
}

onMounted(load);
</script>

<template>
  <section class="profiles">
    <header class="page-head">
      <div>
        <h2>{{ t("profiles.title") }}</h2>
        <p class="sub">{{ t("profiles.subtitle") }}</p>
      </div>
      <div class="head-actions">
        <span class="count">{{ t("profiles.enabledCount", { n: enabledCount }) }}</span>
        <button class="apply" :disabled="busy" @click="regenerate">
          {{ busy ? t("profiles.generating") : t("profiles.apply") }}
        </button>
      </div>
    </header>

    <p v-if="summary" class="summary">{{ t("profiles.lastGen", { text: summary }) }}</p>

    <StatePanel v-if="loading" kind="loading" :message="t('profiles.loading')" />

    <StatePanel
      v-else-if="error"
      kind="error"
      :message="t('profiles.loadFailed')"
      :detail="error"
      show-retry
      @retry="load"
    />

    <div v-else class="grid">
      <article
        v-for="profile in profiles"
        :key="profile.id"
        class="card"
        :class="{
          active: enabled.has(profile.id),
          blocked: !profile.requires_met,
        }"
        role="button"
        tabindex="0"
        :aria-pressed="enabled.has(profile.id)"
        @click="toggle(profile)"
        @keydown="onCardKeydown($event, profile)"
      >
        <div class="icon" :aria-hidden="true">{{ profile.icon }}</div>
        <div class="body">
          <h3>{{ profile.name }}</h3>
          <p>{{ profile.desc }}</p>
          <p v-if="profile.options.length" class="opts">
            {{ profile.options.join(" · ") }}
          </p>
        </div>
        <span v-if="!profile.requires_met" class="badge missing">
          {{ t("profiles.missingBadge") }}
        </span>
        <span v-else-if="enabled.has(profile.id)" class="badge on">
          {{ t("profiles.enabledBadge") }}
        </span>
      </article>
    </div>

    <StatePanel
      v-if="!loading && !error && profiles.length === 0"
      kind="empty"
      :message="t('profiles.empty')"
      :detail="t('profiles.emptyHint')"
    />

    <p v-if="!loading && !error && profiles.length > 0" class="uosc-note">
      {{ t("profiles.uoscMenuNote") }}
    </p>

    <ToastStack :toasts="toasts" />
  </section>
</template>

<style scoped>
.page-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 24px;
  margin-bottom: 20px;
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
  gap: 14px;
  flex-shrink: 0;
}

.count {
  color: var(--text-muted);
  font-size: 13px;
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

.summary {
  margin: 0 0 16px;
  padding: 10px 14px;
  border: 1px solid var(--border);
  border-radius: var(--radius-control);
  background-color: var(--accent-soft);
  color: var(--text-muted);
  font-size: 13px;
}

.uosc-note {
  margin: 18px 0 0;
  padding: 10px 14px;
  border: 1px solid var(--border);
  border-radius: var(--radius-control);
  background-color: rgba(76, 195, 138, 0.08);
  color: var(--text-muted);
  font-size: 12px;
  line-height: 1.6;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
  gap: 16px;
}

.card {
  position: relative;
  display: flex;
  gap: 14px;
  padding: 18px;
  border: 1px solid var(--border);
  border-radius: var(--radius-card);
  background-color: var(--surface);
  cursor: pointer;
  transition: transform 0.15s ease, border-color 0.15s ease,
    background-color 0.15s ease, box-shadow 0.15s ease;
}

.card:hover {
  transform: translateY(-2px);
  border-color: #4a5060;
  background-color: var(--surface-hover);
}

.card.active {
  border-color: var(--accent);
  background-color: var(--accent-soft);
  box-shadow: 0 0 0 1px var(--accent);
}

.card.blocked {
  cursor: not-allowed;
  opacity: 0.7;
}

.icon {
  display: grid;
  place-items: center;
  width: 44px;
  height: 44px;
  flex-shrink: 0;
  border-radius: 10px;
  background-color: #2c3038;
  font-size: 24px;
}

.body {
  min-width: 0;
}

.body h3 {
  margin: 2px 0 4px;
  font-size: 16px;
}

.body p {
  margin: 0;
  color: var(--text-muted);
  font-size: 13px;
  line-height: 1.5;
}

.opts {
  margin-top: 8px !important;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: ui-monospace, "SFMono-Regular", Consolas, monospace;
  font-size: 11px !important;
  opacity: 0.8;
}

.badge {
  position: absolute;
  top: 12px;
  right: 12px;
  padding: 3px 9px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 600;
}

.badge.on {
  background-color: rgba(76, 195, 138, 0.16);
  color: var(--ok);
}

.badge.missing {
  background-color: rgba(224, 180, 92, 0.16);
  color: var(--warn);
}
</style>
