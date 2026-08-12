import { ref } from "vue";
import type { ToastItem } from "../components/ToastStack.vue";

/** Per-view toast state: push a message, it auto-dismisses after
 * `timeoutMs`. Shared by every view via the ToastStack component. */
export function useToasts(timeoutMs = 4000) {
  const toasts = ref<ToastItem[]>([]);
  let seq = 0;

  function toast(kind: ToastItem["kind"], text: string) {
    const id = ++seq;
    toasts.value.push({ id, kind, text });
    window.setTimeout(() => {
      toasts.value = toasts.value.filter((item) => item.id !== id);
    }, timeoutMs);
  }

  return { toasts, toast };
}
