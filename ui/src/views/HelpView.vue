<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { onMounted, ref } from "vue";
import { t, tList } from "../i18n";
import StatePanel from "../components/StatePanel.vue";
import ToastStack from "../components/ToastStack.vue";
import { useToasts } from "../composables/useToasts";

interface UpdateInfo {
  current: string;
  latest: string | null;
  has_update: boolean;
  changelog_url: string | null;
  error: string | null;
}

interface UpgradeResult {
  applied_version: string | null;
  backup_dir: string | null;
  files_replaced: number;
  rolled_back: boolean;
  message: string;
}

const tutorials = ref<string[]>([]);
const tutorialsLoading = ref(true);
const uoscInstalled = ref<boolean | null>(null);

const { toasts, toast } = useToasts(4200);

const updateInfo = ref<UpdateInfo | null>(null);
const checking = ref(false);
const confirmingUpgrade = ref(false);
const upgrading = ref(false);
const upgradeResult = ref<UpgradeResult | null>(null);
const regenerating = ref(false);

async function checkForUpdate() {
  checking.value = true;
  upgradeResult.value = null;
  confirmingUpgrade.value = false;
  try {
    updateInfo.value = await invoke<UpdateInfo>("check_update");
  } catch (cause) {
    updateInfo.value = {
      current: "",
      latest: null,
      has_update: false,
      changelog_url: null,
      error: String(cause),
    };
  } finally {
    checking.value = false;
  }
}

async function runUpgrade() {
  confirmingUpgrade.value = false;
  upgrading.value = true;
  upgradeResult.value = null;
  try {
    upgradeResult.value = await invoke<UpgradeResult>("perform_upgrade");
    if (upgradeResult.value.applied_version !== null) {
      updateInfo.value = {
        current: upgradeResult.value.applied_version,
        latest: null,
        has_update: false,
        changelog_url: updateInfo.value?.changelog_url ?? null,
        error: null,
      };
    }
  } catch (cause) {
    upgradeResult.value = {
      applied_version: null,
      backup_dir: null,
      files_replaced: 0,
      rolled_back: false,
      message: String(cause),
    };
  } finally {
    upgrading.value = false;
  }
}

async function regenerateAfterUpgrade() {
  regenerating.value = true;
  try {
    await invoke("regenerate");
    toast("ok", t("help.updateRegenerated"));
  } catch (cause) {
    toast("err", String(cause));
  } finally {
    regenerating.value = false;
  }
}

onMounted(async () => {
  try {
    tutorials.value = await invoke<string[]>("list_tutorials");
  } catch {
    tutorials.value = [];
  } finally {
    tutorialsLoading.value = false;
  }
  try {
    uoscInstalled.value = await invoke<boolean>("uosc_status");
  } catch {
    uoscInstalled.value = null;
  }
});
</script>

<template>
  <section class="help">
    <header class="page-head">
      <div>
        <h2>{{ t("help.title") }}</h2>
        <p class="sub">{{ t("help.subtitle") }}</p>
      </div>
    </header>

    <div class="grid">
      <article class="panel">
        <h3>{{ t("help.quickStartTitle") }}</h3>
        <ol class="steps">
          <li v-for="(step, index) in tList('help.quickStart')" :key="index">{{ step }}</li>
        </ol>
      </article>

      <article class="panel">
        <h3>{{ t("help.uoscTitle") }}</h3>
        <p v-if="uoscInstalled === true" class="status ok">● {{ t("help.uoscDetected") }}</p>
        <p v-else-if="uoscInstalled === false" class="status warn">● {{ t("help.uoscMissing") }}</p>
        <p v-else class="status">{{ t("common.loading") }}</p>
      </article>

      <article class="panel wide">
        <h3>{{ t("help.tutorialsTitle") }}</h3>
        <StatePanel
          v-if="tutorialsLoading"
          kind="loading"
          :message="t('help.tutorialsLoading')"
        />
        <StatePanel
          v-else-if="tutorials.length === 0"
          kind="empty"
          :message="t('help.tutorialsEmpty')"
        />
        <ul v-else class="tutorial-list">
          <li v-for="name in tutorials" :key="name">{{ name }}</li>
        </ul>
      </article>

      <article class="panel">
        <h3>{{ t("help.aboutTitle") }}</h3>
        <p class="about">
          {{ t("help.aboutVersion", { version: updateInfo?.current || "—" }) }}
        </p>
        <p class="about muted">{{ t("help.aboutLicense") }}</p>
        <div class="update-actions">
          <button class="ghost" :disabled="checking || upgrading" @click="checkForUpdate">
            {{ checking ? t("help.updateChecking") : t("help.updateCheck") }}
          </button>
          <a
            v-if="updateInfo?.changelog_url"
            class="ghost link"
            :href="updateInfo.changelog_url"
            target="_blank"
            rel="noreferrer"
          >
            {{ t("help.updateChangelog") }}
          </a>
        </div>

        <StatePanel
          v-if="updateInfo?.error"
          kind="error"
          :message="t('help.updateErrorTitle')"
          :detail="updateInfo.error"
        />

        <StatePanel
          v-else-if="updateInfo && updateInfo.latest === null"
          kind="empty"
          :message="t('help.updateNoInfo')"
        />

        <div v-else-if="updateInfo && !updateInfo.has_update" class="update-line ok">
          {{ t("help.updateUpToDate", { version: updateInfo.latest ?? "" }) }}
        </div>

        <div v-else-if="updateInfo" class="update-card">
          <p class="update-title">
            {{ t("help.updateNewVersion", { current: updateInfo.current, latest: updateInfo.latest ?? "" }) }}
          </p>
          <p class="update-warning">{{ t("help.updateWarning") }}</p>
          <div class="update-actions">
            <button v-if="!confirmingUpgrade" class="apply" :disabled="upgrading" @click="confirmingUpgrade = true">
              {{ upgrading ? t("help.updateRunning") : t("help.updateRun") }}
            </button>
            <template v-else>
              <span class="confirm-note">{{ t("help.updateConfirm") }}</span>
              <button class="apply" :disabled="upgrading" @click="runUpgrade">
                {{ t("help.updateConfirmYes") }}
              </button>
              <button class="ghost" :disabled="upgrading" @click="confirmingUpgrade = false">
                {{ t("help.updateCancel") }}
              </button>
            </template>
          </div>
        </div>

        <div v-if="upgradeResult" class="update-line" :class="upgradeResult.applied_version !== null ? 'ok' : 'bad'">
          <p>{{ upgradeResult.message }}</p>
          <p v-if="upgradeResult.applied_version !== null" class="regen-row">
            {{ t("help.updateRegenHint") }}
            <button class="apply" :disabled="regenerating" @click="regenerateAfterUpgrade">
              {{ regenerating ? t("help.updateRegenerating") : t("help.updateRegenerate") }}
            </button>
          </p>
          <p v-else-if="upgradeResult.rolled_back">{{ t("help.updateRolledBack") }}</p>
        </div>
      </article>
    </div>

    <ToastStack :toasts="toasts" />
  </section>
</template>

<style scoped>
.page-head {
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

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 16px;
}

.panel {
  padding: 18px;
  border: 1px solid var(--border);
  border-radius: var(--radius-card);
  background-color: var(--surface);
}

.panel.wide {
  grid-column: 1 / -1;
}

.panel h3 {
  margin: 0 0 12px;
  font-size: 15px;
}

.steps {
  margin: 0;
  padding-left: 20px;
  color: var(--text-muted);
  font-size: 13px;
  line-height: 1.8;
}

.status {
  margin: 0;
  font-size: 13px;
  line-height: 1.7;
}

.status.ok {
  color: var(--ok);
}

.status.warn {
  color: var(--warn);
}

.about {
  margin: 0 0 8px;
  font-size: 13px;
  line-height: 1.7;
}

.about.muted {
  color: var(--text-muted);
  font-size: 12px;
}

.tutorial-list {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.tutorial-list li {
  padding: 6px 12px;
  border: 1px solid var(--border);
  border-radius: 999px;
  color: var(--text-muted);
  font-size: 12px;
  font-family: ui-monospace, "SFMono-Regular", Consolas, monospace;
}

/* --- update check / upgrade --- */
.update-actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 12px;
}

.update-actions .link {
  display: inline-block;
  text-decoration: none;
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

.ghost:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.apply {
  padding: 9px 20px;
  border: none;
  border-radius: var(--radius-control);
  background-color: var(--accent);
  color: #10131a;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: filter 0.15s, transform 0.15s;
}

.apply:hover:not(:disabled) {
  filter: brightness(1.1);
}

.apply:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.confirm-note {
  color: var(--warn);
  font-size: 13px;
  font-weight: 600;
}

.update-card {
  margin-top: 14px;
  padding: 14px 16px;
  border: 1px solid rgba(224, 180, 92, 0.45);
  border-radius: var(--radius-card);
  background-color: rgba(224, 180, 92, 0.08);
}

.update-title {
  margin: 0 0 8px;
  font-size: 14px;
  font-weight: 600;
}

.update-warning {
  margin: 0;
  color: var(--text-muted);
  font-size: 12px;
  line-height: 1.6;
}

.update-line {
  margin-top: 14px;
  padding: 12px 14px;
  border: 1px solid var(--border);
  border-radius: var(--radius-control);
  background-color: var(--surface);
  font-size: 13px;
  line-height: 1.7;
}

.update-line p {
  margin: 0;
}

.update-line.ok {
  border-color: rgba(76, 195, 138, 0.5);
  color: var(--ok);
}

.update-line.bad {
  border-color: rgba(229, 115, 115, 0.5);
  color: var(--danger);
}

.regen-row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 12px;
  margin-top: 10px !important;
  color: var(--text-muted);
}
</style>
