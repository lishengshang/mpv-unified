<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { RouterLink } from "vue-router";
import { computed, onMounted, ref } from "vue";

interface TutorialInfo {
  slug: string;
  title: string;
}

interface Toast {
  id: number;
  kind: "ok" | "err";
  text: string;
}

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

type Tab = "editor" | "tutorials" | "about";

const activeTab = ref<Tab>("editor");
const toasts = ref<Toast[]>([]);
let toastSeq = 0;

/* ---------- user.conf editor ---------- */
const confContent = ref("");
const confExists = ref<boolean | null>(null);
const confDirty = ref(false);
const confBusy = ref(false);
const confError = ref("");
const exampleLoaded = ref(false);

/* ---------- tutorials ---------- */
const tutorials = ref<TutorialInfo[]>([]);
const tutorialsError = ref("");
const loadingTutorials = ref(true);
const openSlug = ref("");
const openBody = ref("");
const openTitle = ref("");
const bodyLoading = ref(false);

const onboardingVisible = computed(
  () => confExists.value === false && activeTab.value === "editor",
);

/* ---------- about / update ---------- */
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
    toast("ok", "配置已重新生成,方案与选项表单已生效");
  } catch (cause) {
    toast("err", String(cause));
  } finally {
    regenerating.value = false;
  }
}

function toast(kind: Toast["kind"], text: string) {
  const id = ++toastSeq;
  toasts.value.push({ id, kind, text });
  setTimeout(() => {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  }, 4200);
}

async function loadEditor() {
  try {
    const stored = await invoke<string | null>("get_user_conf");
    confExists.value = stored !== null;
    confContent.value = stored ?? "";
    confDirty.value = false;
  } catch (cause) {
    confError.value = String(cause);
  }
}

async function saveConf() {
  confBusy.value = true;
  try {
    await invoke("save_user_conf", { content: confContent.value });
    confExists.value = true;
    confDirty.value = false;
    exampleLoaded.value = false;
    toast("ok", "已保存 user/user.conf(round-trip 校验通过)");
  } catch (cause) {
    toast("err", String(cause));
  } finally {
    confBusy.value = false;
  }
}

async function loadExample() {
  try {
    confContent.value = await invoke<string>("load_example_conf");
    confDirty.value = true;
    exampleLoaded.value = true;
  } catch (cause) {
    toast("err", String(cause));
  }
}

async function loadTutorials() {
  loadingTutorials.value = true;
  tutorialsError.value = "";
  try {
    tutorials.value = await invoke<TutorialInfo[]>("list_tutorials");
  } catch (cause) {
    tutorialsError.value = String(cause);
  } finally {
    loadingTutorials.value = false;
  }
}

async function openTutorial(tutorial: TutorialInfo) {
  if (openSlug.value === tutorial.slug) {
    openSlug.value = "";
    return;
  }
  openSlug.value = tutorial.slug;
  openTitle.value = tutorial.title;
  bodyLoading.value = true;
  try {
    openBody.value = await invoke<string>("get_tutorial", { slug: tutorial.slug });
  } catch (cause) {
    openBody.value = "";
    toast("err", String(cause));
  } finally {
    bodyLoading.value = false;
  }
}

function switchTab(tab: Tab) {
  activeTab.value = tab;
  if (tab === "tutorials" && tutorials.value.length === 0) {
    loadTutorials();
  }
}

onMounted(() => {
  loadEditor();
  loadTutorials();
});
</script>

<template>
  <section class="help">
    <header class="page-head">
      <div>
        <h2>帮助</h2>
        <p class="sub">
          高级配置(user.conf 文本编辑,只影响个人层)与长尾选项教程。
          源文件(config/base.conf 等)不可在此编辑,修改源文件请走 git 流程。
        </p>
      </div>
    </header>

    <nav class="tabs" role="tablist">
      <button
        role="tab"
        :aria-selected="activeTab === 'editor'"
        :class="{ active: activeTab === 'editor' }"
        @click="switchTab('editor')"
      >
        高级配置(user.conf)
      </button>
      <button
        role="tab"
        :aria-selected="activeTab === 'tutorials'"
        :class="{ active: activeTab === 'tutorials' }"
        @click="switchTab('tutorials')"
      >
        教程({{ tutorials.length }})
      </button>
      <button
        role="tab"
        :aria-selected="activeTab === 'about'"
        :class="{ active: activeTab === 'about' }"
        @click="switchTab('about')"
      >
        关于与更新
      </button>
    </nav>

    <!-- ============ Tab 1: user.conf editor ============ -->
    <div v-if="activeTab === 'editor'" class="editor-tab">
      <div v-if="onboardingVisible" class="onboarding">
        <strong>首次运行:user/user.conf 尚未创建</strong>
        <p>
          个人配置层还不存在。可以先前往<RouterLink to="/profiles"
            >方案页</RouterLink
          >启用推荐方案,或在下方直接编辑并保存(保存时会自动创建
          user.conf,并校验语法 round-trip 无损)。
        </p>
      </div>

      <div v-if="confError" class="state-panel error">
        <p>读取 user.conf 失败</p>
        <p class="detail">{{ confError }}</p>
        <button class="apply" @click="loadEditor">重试</button>
      </div>

      <template v-else>
        <div class="editor-toolbar">
          <span class="path-hint">user/user.conf {{ exampleLoaded ? "(来自示例模板)" : "" }}</span>
          <div class="toolbar-actions">
            <button class="ghost" :disabled="confBusy" @click="loadExample">打开示例</button>
            <button class="apply" :disabled="confBusy || !confDirty" @click="saveConf">
              {{ confBusy ? "校验保存中…" : "保存" }}
            </button>
          </div>
        </div>
        <textarea
          v-model="confContent"
          class="conf-editor"
          :spellcheck="false"
          placeholder="# 在这里写 mpv 选项,例如:&#10;sub-font-size=44&#10;hwdec=no"
        ></textarea>
        <p v-if="!confDirty" class="dirty-hint">已保存(与磁盘一致)。保存时用 mpv 解析器校验,非法内容报错含行号且不会写盘。</p>
        <p v-else class="dirty-hint">有未保存的修改。</p>
      </template>
    </div>

    <!-- ============ Tab 2: tutorials ============ -->
    <div v-else class="tutorials-tab">
      <div v-if="loadingTutorials" class="state-panel">加载教程列表…</div>

      <div v-else-if="tutorialsError" class="state-panel error">
        <p>教程加载失败</p>
        <p class="detail">{{ tutorialsError }}</p>
      </div>

      <div v-else-if="tutorials.length === 0" class="state-panel">
        未找到教程文档(docs/tutorials 目录缺失)。仓库安装不完整,请重新获取完整仓库。
      </div>

      <template v-else>
        <ul class="tutorial-list">
          <li v-for="tutorial in tutorials" :key="tutorial.slug">
            <button
              class="tutorial-item"
              :class="{ open: openSlug === tutorial.slug }"
              @click="openTutorial(tutorial)"
            >
              <span class="tutorial-title">{{ tutorial.title }}</span>
              <span class="tutorial-slug">{{ tutorial.slug }}</span>
            </button>
            <div v-if="openSlug === tutorial.slug" class="tutorial-body-wrap">
              <p v-if="bodyLoading" class="state-panel">加载中…</p>
              <pre v-else class="tutorial-body">{{ openBody }}</pre>
            </div>
          </li>
        </ul>
        <p class="tutorial-count">共 {{ tutorials.length }} 篇教程,长尾选项持续扩充;更多选项见
          <a href="https://mpv.io/manual/master/" target="_blank" rel="noreferrer">mpv 手册 ↗</a>。
        </p>
      </template>
    </div>

    <!-- ============ Tab 3: about / update ============ -->
    <div v-else class="about-tab">
      <div class="about-card">
        <h3>mpv-config</h3>
        <p class="about-desc">
          跨平台 mpv 配置生成与包管理工具。升级只替换 app 层
          (config/ scripts/ shaders/ 等),user/ 个人层(含 API 密钥)永不覆盖。
        </p>
        <p class="version-row">
          当前版本:<code>{{ updateInfo?.current || "—" }}</code>
          <button class="ghost" :disabled="checking || upgrading" @click="checkForUpdate">
            {{ checking ? "检查中…" : "检查更新" }}
          </button>
        </p>
      </div>

      <div v-if="updateInfo?.error" class="state-panel error">
        <p>检查更新失败</p>
        <p class="detail">{{ updateInfo.error }}</p>
      </div>

      <div v-else-if="updateInfo && updateInfo.latest === null" class="state-panel">
        索引未提供版本信息,无法检查更新。
      </div>

      <div v-else-if="updateInfo && !updateInfo.has_update" class="state-panel">
        已是最新版本({{ updateInfo.latest }})。
      </div>

      <div v-else-if="updateInfo" class="update-card">
        <p class="update-title">
          发现新版本:{{ updateInfo.current }} → <strong>{{ updateInfo.latest }}</strong>
        </p>
        <p class="update-warning">
          升级会替换 app 层文件并自动备份到缓存;user/ 层(个人配置与密钥)不受影响。
        </p>
        <div class="update-actions">
          <a
            v-if="updateInfo.changelog_url"
            class="ghost link"
            :href="updateInfo.changelog_url"
            target="_blank"
            rel="noreferrer"
          >
            查看更新日志 ↗
          </a>
          <button v-if="!confirmingUpgrade" class="apply" :disabled="upgrading" @click="confirmingUpgrade = true">
            {{ upgrading ? "升级中…" : "执行升级" }}
          </button>
          <template v-else>
            <span class="confirm-note">确认执行升级?</span>
            <button class="apply" :disabled="upgrading" @click="runUpgrade">确认</button>
            <button class="ghost" :disabled="upgrading" @click="confirmingUpgrade = false">取消</button>
          </template>
        </div>
      </div>

      <div v-if="upgradeResult" class="upgrade-result" :class="{ bad: upgradeResult.applied_version === null }">
        <p>{{ upgradeResult.message }}</p>
        <p v-if="upgradeResult.applied_version !== null" class="result-hint">
          升级后请重新生成配置,让新版本的方案与选项生效。
          <button class="apply" :disabled="regenerating" @click="regenerateAfterUpgrade">
            {{ regenerating ? "重新生成中…" : "重新生成配置" }}
          </button>
        </p>
        <p v-else-if="upgradeResult.rolled_back" class="result-hint">
          app 层已从备份还原,当前版本未变化,可安全重试。
        </p>
      </div>
    </div>

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
  max-width: 720px;
  color: var(--text-muted);
  font-size: 13px;
  line-height: 1.6;
}

.tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 18px;
  padding: 4px;
  border: 1px solid var(--border);
  border-radius: var(--radius-control);
  background-color: var(--surface);
  width: fit-content;
}

.tabs button {
  padding: 8px 18px;
  border: none;
  border-radius: 6px;
  background-color: transparent;
  color: var(--text-muted);
  font-size: 13px;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.tabs button:hover {
  color: var(--text);
}

.tabs button.active {
  background-color: var(--accent-soft);
  color: var(--accent);
  font-weight: 600;
}

/* --- editor --- */
.onboarding {
  margin-bottom: 16px;
  padding: 16px 18px;
  border: 1px solid rgba(224, 180, 92, 0.45);
  border-radius: var(--radius-card);
  background-color: rgba(224, 180, 92, 0.08);
  font-size: 13px;
  line-height: 1.7;
}

.onboarding strong {
  color: var(--warn);
}

.onboarding p {
  margin: 6px 0 0;
  color: var(--text-muted);
}

.onboarding a {
  color: var(--accent);
}

.editor-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 10px;
}

.path-hint {
  color: var(--text-muted);
  font-size: 12px;
  font-family: ui-monospace, "SFMono-Regular", Consolas, monospace;
}

.toolbar-actions {
  display: flex;
  gap: 10px;
}

.conf-editor {
  width: 100%;
  min-height: 460px;
  padding: 14px 16px;
  border: 1px solid var(--border);
  border-radius: var(--radius-card);
  background-color: #16181d;
  color: var(--text);
  font-family: ui-monospace, "SFMono-Regular", Consolas, monospace;
  font-size: 13px;
  line-height: 1.6;
  resize: vertical;
  tab-size: 4;
}

.conf-editor:focus {
  outline: none;
  border-color: var(--accent);
}

/* --- buttons --- */
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

.apply:active:not(:disabled) {
  transform: translateY(1px);
}

.apply:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.ghost {
  padding: 9px 16px;
  border: 1px solid var(--border);
  border-radius: var(--radius-control);
  background-color: transparent;
  color: var(--text-muted);
  font-size: 13px;
  cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
}

.ghost:hover:not(:disabled) {
  color: var(--accent);
  border-color: var(--accent);
}

.ghost:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.dirty-hint {
  margin: 12px 0 0;
  padding: 10px 14px;
  border-radius: var(--radius-control);
  background-color: var(--accent-soft);
  color: var(--text-muted);
  font-size: 12px;
}

/* --- tutorials --- */
.tutorial-list {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.tutorial-item {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 16px;
  width: 100%;
  padding: 12px 16px;
  border: 1px solid var(--border);
  border-radius: var(--radius-card);
  background-color: var(--surface);
  color: var(--text);
  font-size: 14px;
  text-align: left;
  cursor: pointer;
  transition: border-color 0.15s, background-color 0.15s;
}

.tutorial-item:hover {
  border-color: #4a5060;
  background-color: var(--surface-hover);
}

.tutorial-item.open {
  border-color: var(--accent);
}

.tutorial-title {
  font-weight: 500;
}

.tutorial-slug {
  color: var(--text-muted);
  font-family: ui-monospace, "SFMono-Regular", Consolas, monospace;
  font-size: 11px;
  flex-shrink: 0;
}

.tutorial-body-wrap {
  margin-top: 6px;
  padding: 4px 2px;
}

.tutorial-body {
  margin: 0;
  padding: 16px 18px;
  border: 1px solid var(--border);
  border-left: 3px solid var(--accent);
  border-radius: var(--radius-card);
  background-color: #16181d;
  color: #cfd3dc;
  font-family: system-ui, "PingFang SC", "Microsoft YaHei", sans-serif;
  font-size: 13px;
  line-height: 1.8;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 560px;
  overflow-y: auto;
}

.tutorial-count {
  margin: 14px 0 0;
  color: var(--text-muted);
  font-size: 12px;
}

.tutorial-count a {
  color: var(--accent);
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

/* --- about / update --- */
.about-card {
  padding: 20px 22px;
  border: 1px solid var(--border);
  border-radius: var(--radius-card);
  background-color: var(--surface);
}

.about-card h3 {
  margin: 0 0 6px;
  font-size: 18px;
}

.about-desc {
  margin: 0 0 16px;
  color: var(--text-muted);
  font-size: 13px;
  line-height: 1.7;
}

.version-row {
  display: flex;
  align-items: center;
  gap: 14px;
  margin: 0;
  font-size: 13px;
}

.version-row code {
  padding: 3px 8px;
  border-radius: 6px;
  background-color: #16181d;
  border: 1px solid var(--border);
  font-family: ui-monospace, "SFMono-Regular", Consolas, monospace;
}

.update-card {
  margin-top: 14px;
  padding: 18px 22px;
  border: 1px solid rgba(224, 180, 92, 0.45);
  border-radius: var(--radius-card);
  background-color: rgba(224, 180, 92, 0.08);
}

.update-title {
  margin: 0 0 8px;
  font-size: 14px;
}

.update-title strong {
  color: var(--warn);
}

.update-warning {
  margin: 0 0 14px;
  color: var(--text-muted);
  font-size: 12px;
  line-height: 1.6;
}

.update-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.update-actions .link {
  display: inline-block;
  text-decoration: none;
}

.confirm-note {
  color: var(--warn);
  font-size: 13px;
  font-weight: 600;
}

.upgrade-result {
  margin-top: 14px;
  padding: 16px 18px;
  border: 1px solid rgba(76, 195, 138, 0.5);
  border-radius: var(--radius-card);
  background-color: rgba(76, 195, 138, 0.07);
  font-size: 13px;
  line-height: 1.7;
}

.upgrade-result p {
  margin: 0;
}

.upgrade-result.bad {
  border-color: rgba(229, 115, 115, 0.5);
  background-color: rgba(229, 115, 115, 0.07);
}

.result-hint {
  margin-top: 10px;
  display: flex;
  align-items: center;
  gap: 12px;
  color: var(--text-muted);
}

/* --- toasts --- */
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
  max-width: 420px;
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
