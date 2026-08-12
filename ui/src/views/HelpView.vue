<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { onMounted, ref } from "vue";
import { t, tList } from "../i18n";
import StatePanel from "../components/StatePanel.vue";

const tutorials = ref<string[]>([]);
const tutorialsLoading = ref(true);
const uoscInstalled = ref<boolean | null>(null);

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
        <p class="about">{{ t("help.aboutVersion", { version: "0.1.0" }) }}</p>
        <p class="about muted">{{ t("help.aboutLicense") }}</p>
      </article>
    </div>
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
</style>
