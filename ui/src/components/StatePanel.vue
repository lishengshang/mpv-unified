<script setup lang="ts">
import { t } from "../i18n";

/** Unified loading / error / empty placeholder used by every view. */
defineProps<{
  kind: "loading" | "error" | "empty";
  message?: string;
  detail?: string;
  showRetry?: boolean;
}>();

defineEmits<{ retry: [] }>();
</script>

<template>
  <div class="state-panel" :class="kind" role="status" aria-live="polite">
    <span v-if="kind === 'loading'" class="spinner" aria-hidden="true" />
    <p v-if="message" class="message">{{ message }}</p>
    <p v-if="detail" class="detail">{{ detail }}</p>
    <button v-if="showRetry" class="retry" type="button" @click="$emit('retry')">
      {{ t("common.retry") }}
    </button>
  </div>
</template>

<style scoped>
.state-panel {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  padding: 40px 32px;
  border: 1px dashed var(--border);
  border-radius: var(--radius-card);
  text-align: center;
  color: var(--text-muted);
}

.state-panel.error {
  border-color: rgba(229, 115, 115, 0.45);
}

.message {
  margin: 0;
  font-weight: 600;
}

.state-panel.error .message {
  color: var(--danger);
}

.detail {
  margin: 0;
  max-width: 560px;
  font-size: 12px;
  line-height: 1.6;
  word-break: break-all;
}

.retry {
  margin-top: 4px;
  padding: 8px 18px;
  border: none;
  border-radius: var(--radius-control);
  background-color: var(--accent);
  color: #10131a;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: filter 0.15s;
}

.retry:hover {
  filter: brightness(1.1);
}

.spinner {
  width: 26px;
  height: 26px;
  border: 3px solid rgba(124, 156, 255, 0.25);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
