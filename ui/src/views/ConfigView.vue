<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { RouterLink } from "vue-router";
import { computed, onMounted, reactive, ref } from "vue";

/** One curated option, as serialized by core::options_gui::GuiOption. */
interface GuiOption {
  key: string;
  category: string;
  type: "switch" | "number" | "select" | "string" | "path";
  min: number | null;
  max: number | null;
  choices: string[];
  default: string;
  desc_zh: string;
  desc_en: string;
  doc_url: string;
}

interface Toast {
  id: number;
  kind: "ok" | "err";
  text: string;
}

const CATEGORY_LABELS: Record<string, string> = {
  general: "通用",
  video: "视频",
  audio: "音频",
  subtitle: "字幕",
  performance: "性能",
  network: "网络",
  window: "窗口",
  other: "其他",
};

const options = ref<GuiOption[]>([]);
const loading = ref(true);
const error = ref("");
const busy = ref(false);
const activeCategory = ref("");
const toasts = ref<Toast[]>([]);
/** Complete form state: every option key → current value (all strings, mpv world). */
const form = reactive<Record<string, string>>({});
/** Snapshot of the last saved state, for dirty tracking. */
let savedSnapshot = "";
let toastSeq = 0;

const categories = computed(() => {
  const counts = new Map<string, number>();
  for (const option of options.value) {
    counts.set(option.category, (counts.get(option.category) ?? 0) + 1);
  }
  const order = Object.keys(CATEGORY_LABELS).filter((c) => counts.has(c));
  return order.map((category) => ({
    category,
    label: CATEGORY_LABELS[category],
    count: counts.get(category) ?? 0,
  }));
});

const activeOptions = computed(() =>
  options.value.filter((option) => option.category === activeCategory.value),
);

const dirty = computed(() => snapshot() !== savedSnapshot);

function toast(kind: Toast["kind"], text: string) {
  const id = ++toastSeq;
  toasts.value.push({ id, kind, text });
  setTimeout(() => {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  }, 3600);
}

function snapshot(): string {
  return JSON.stringify(
    Object.keys(form)
      .sort()
      .map((key) => [key, form[key] ?? ""]),
  );
}

/** Current form value of an option, falling back to the curated default. */
function valueOf(option: GuiOption): string {
  return form[option.key] ?? option.default;
}

/** The slider step: fractional ranges move by 0.1, integer ranges by 1. */
function sliderStep(option: GuiOption): number {
  return option.min !== null &&
    option.max !== null &&
    Number.isInteger(option.min) &&
    Number.isInteger(option.max)
    ? 1
    : 0.1;
}

function asNumber(option: GuiOption, value: string): number {
  const parsed = Number.parseFloat(value);
  if (Number.isNaN(parsed)) return Number.parseFloat(option.default);
  return parsed;
}

function onSlider(option: GuiOption, raw: string) {
  const number = Number.parseFloat(raw);
  if (!Number.isNaN(number)) {
    form[option.key] = String(number);
  }
}

async function load() {
  loading.value = true;
  error.value = "";
  try {
    const [list, stored] = await Promise.all([
      invoke<GuiOption[]>("list_options"),
      invoke<[string, string][]>("get_gui_values"),
    ]);
    options.value = list;
    for (const option of list) {
      form[option.key] = option.default;
    }
    for (const [key, value] of stored) {
      if (key in form) {
        form[key] = value;
      }
    }
    activeCategory.value = categories.value[0]?.category ?? "";
    savedSnapshot = snapshot();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

async function saveAndGenerate() {
  busy.value = true;
  try {
    const values: [string, string][] = options.value.map((option) => [
      option.key,
      valueOf(option),
    ]);
    await invoke("save_gui_values", { values });
    savedSnapshot = snapshot();
    const report = await invoke<{ files: { path: string; line_count: number }[] }>("regenerate");
    const summary = report.files
      .map((file) => `${file.path} (${file.line_count} 行)`)
      .join(", ");
    toast("ok", `已保存并重新生成:${summary}`);
  } catch (cause) {
    toast("err", String(cause));
  } finally {
    busy.value = false;
  }
}

async function resetOption(option: GuiOption) {
  form[option.key] = option.default;
  try {
    await invoke("reset_gui_value", { key: option.key });
    savedSnapshot = snapshot();
    toast("ok", `「${option.key}」已恢复默认`);
  } catch (cause) {
    toast("err", String(cause));
  }
}

onMounted(load);
</script>

<template>
  <section class="config">
    <header class="page-head">
      <div>
        <h2>配置</h2>
        <p class="sub">
          精选常用选项(共 {{ options.length }} 项),保存时只把与默认不同的设置写入
          user/gui.conf,不会覆盖手动编辑内容;gui.conf 优先级高于 user.conf。
        </p>
      </div>
      <div class="head-actions">
        <RouterLink class="raw-link" to="/help">编辑原始文件 →</RouterLink>
        <button
          class="apply"
          :disabled="busy || !dirty"
          @click="saveAndGenerate"
        >
          {{ busy ? "处理中…" : "保存并生成" }}
        </button>
      </div>
    </header>

    <div v-if="loading" class="state-panel">加载选项表中…</div>

    <div v-else-if="error" class="state-panel error">
      <p>配置加载失败</p>
      <p class="detail">{{ error }}</p>
      <button class="apply" @click="load">重试</button>
    </div>

    <div v-else class="config-layout">
      <aside class="cat-tree" aria-label="选项章节">
        <button
          v-for="item in categories"
          :key="item.category"
          class="cat-item"
          :class="{ active: item.category === activeCategory }"
          @click="activeCategory = item.category"
        >
          <span>{{ item.label }}</span>
          <span class="count">{{ item.count }}</span>
        </button>
      </aside>

      <div class="form" role="list">
        <div v-for="option in activeOptions" :key="option.key" class="row">
          <div class="row-info">
            <div class="row-head">
              <code class="key">{{ option.key }}</code>
              <a
                class="doc"
                :href="option.doc_url"
                target="_blank"
                rel="noreferrer"
                title="mpv 手册"
                >手册 ↗</a
              >
            </div>
            <p class="desc-zh">{{ option.desc_zh }}</p>
            <p class="desc-en">{{ option.desc_en }}</p>
            <p class="default">
              默认:{{ option.default }}
              <button
                class="reset"
                :disabled="form[option.key] === option.default"
                @click="resetOption(option)"
              >
                恢复默认
              </button>
            </p>
          </div>

          <div class="row-control">
            <!-- switch → 开关 -->
            <label v-if="option.type === 'switch'" class="toggle">
              <input
                type="checkbox"
                :checked="form[option.key] === 'yes'"
                @change="
                  form[option.key] = ($event.target as HTMLInputElement).checked
                    ? 'yes'
                    : 'no'
                "
              />
              <span class="toggle-track" aria-hidden="true" />
              <span class="toggle-state">{{
                form[option.key] === "yes" ? "开" : "关"
              }}</span>
            </label>

            <!-- number → 滑块 + 数字输入 -->
            <div v-else-if="option.type === 'number'" class="number-control">
              <input
                type="range"
                :min="option.min ?? 0"
                :max="option.max ?? 100"
                :step="sliderStep(option)"
                :value="asNumber(option, valueOf(option))"
                @input="onSlider(option, ($event.target as HTMLInputElement).value)"
              />
              <input
                v-model="form[option.key]"
                class="number-input"
                type="number"
                :min="option.min ?? 0"
                :max="option.max ?? 100"
                :step="sliderStep(option)"
              />
            </div>

            <!-- select → 下拉 -->
            <select v-else-if="option.type === 'select'" v-model="form[option.key]">
              <option v-for="choice in option.choices" :key="choice" :value="choice">
                {{ choice }}
              </option>
            </select>

            <!-- string / path → 文本输入 -->
            <input
              v-else
              v-model="form[option.key]"
              class="text-input"
              type="text"
              :spellcheck="false"
            />
          </div>
        </div>
        <p v-if="activeOptions.length === 0" class="state-panel">该章节暂无选项。</p>
      </div>
    </div>

    <p v-if="!loading && !error && dirty" class="dirty-hint">
      有未保存的修改,保存后写入 user/gui.conf。
    </p>

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
  margin-bottom: 20px;
}

h2 {
  margin: 0 0 6px;
  font-size: 22px;
}

.sub {
  margin: 0;
  max-width: 640px;
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

.raw-link {
  color: var(--text-muted);
  font-size: 13px;
  text-decoration: none;
}

.raw-link:hover {
  color: var(--accent);
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

.config-layout {
  display: grid;
  grid-template-columns: 168px 1fr;
  gap: 20px;
  align-items: start;
}

.cat-tree {
  position: sticky;
  top: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-card);
  background-color: var(--surface);
}

.cat-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 9px 12px;
  border: none;
  border-radius: var(--radius-control);
  background-color: transparent;
  color: var(--text-muted);
  font-size: 13px;
  cursor: pointer;
  transition: background-color 0.15s, color 0.15s;
}

.cat-item:hover {
  background-color: var(--surface-hover);
  color: var(--text);
}

.cat-item.active {
  background-color: var(--accent-soft);
  color: var(--accent);
  font-weight: 600;
}

.cat-item .count {
  font-size: 11px;
  opacity: 0.7;
}

.form {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-width: 0;
}

.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  padding: 16px 18px;
  border: 1px solid var(--border);
  border-radius: var(--radius-card);
  background-color: var(--surface);
  transition: border-color 0.15s;
}

.row:hover {
  border-color: #4a5060;
}

.row-info {
  min-width: 0;
}

.row-head {
  display: flex;
  align-items: center;
  gap: 10px;
}

.key {
  padding: 2px 8px;
  border-radius: 6px;
  background-color: #2c3038;
  font-family: ui-monospace, "SFMono-Regular", Consolas, monospace;
  font-size: 12px;
  color: var(--accent);
}

.doc {
  font-size: 11px;
  color: var(--text-muted);
  text-decoration: none;
}

.doc:hover {
  color: var(--accent);
}

.desc-zh {
  margin: 8px 0 2px;
  font-size: 13px;
}

.desc-en {
  margin: 0;
  color: var(--text-muted);
  font-size: 12px;
}

.default {
  margin: 8px 0 0;
  color: var(--text-muted);
  font-size: 12px;
}

.reset {
  margin-left: 8px;
  padding: 2px 8px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background-color: transparent;
  color: var(--text-muted);
  font-size: 11px;
  cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
}

.reset:hover:not(:disabled) {
  color: var(--warn);
  border-color: var(--warn);
}

.reset:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.row-control {
  flex-shrink: 0;
  min-width: 220px;
  max-width: 320px;
}

/* switch */
.toggle {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  cursor: pointer;
}

.toggle input {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-track {
  width: 40px;
  height: 22px;
  border-radius: 999px;
  background-color: #3a3f4a;
  position: relative;
  transition: background-color 0.15s;
}

.toggle-track::after {
  content: "";
  position: absolute;
  top: 3px;
  left: 3px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background-color: #a8adb8;
  transition: transform 0.15s, background-color 0.15s;
}

.toggle input:checked + .toggle-track {
  background-color: var(--accent);
}

.toggle input:checked + .toggle-track::after {
  transform: translateX(18px);
  background-color: #10131a;
}

.toggle input:focus-visible + .toggle-track {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.toggle-state {
  font-size: 12px;
  color: var(--text-muted);
}

/* number: slider + input */
.number-control {
  display: flex;
  align-items: center;
  gap: 12px;
}

.number-control input[type="range"] {
  flex: 1;
  accent-color: var(--accent);
}

.number-input {
  width: 76px;
  padding: 7px 9px;
  border: 1px solid var(--border);
  border-radius: var(--radius-control);
  background-color: #1d2026;
  color: var(--text);
  font-size: 13px;
}

/* select / text */
select,
.text-input {
  width: 100%;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-control);
  background-color: #1d2026;
  color: var(--text);
  font-size: 13px;
}

select {
  cursor: pointer;
}

select:focus,
.text-input:focus,
.number-input:focus {
  outline: none;
  border-color: var(--accent);
}

.text-input {
  font-family: ui-monospace, "SFMono-Regular", Consolas, monospace;
  font-size: 12px;
}

.dirty-hint {
  margin: 16px 0 0;
  padding: 10px 14px;
  border: 1px solid var(--border);
  border-radius: var(--radius-control);
  background-color: var(--accent-soft);
  color: var(--text-muted);
  font-size: 13px;
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
