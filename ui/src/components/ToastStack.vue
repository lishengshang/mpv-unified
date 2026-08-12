<script setup lang="ts">
export interface ToastItem {
  id: number;
  kind: "ok" | "err";
  text: string;
}

defineProps<{ toasts: ToastItem[] }>();
</script>

<template>
  <div class="toasts" aria-live="polite">
    <div v-for="toast in toasts" :key="toast.id" class="toast" :class="toast.kind">
      {{ toast.text }}
    </div>
  </div>
</template>

<style scoped>
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
