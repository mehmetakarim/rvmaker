<script setup lang="ts">
import { Check, CircleAlert, Info, TriangleAlert, X } from "@lucide/vue";
import { useToastStore } from "@/stores/toasts";

const toasts = useToastStore();

const icons = {
  success: Check,
  error: CircleAlert,
  warning: TriangleAlert,
  info: Info,
} as const;
</script>

<template>
  <div class="stack" role="status" aria-live="polite">
    <TransitionGroup name="toast">
      <div v-for="toast in toasts.items" :key="toast.id" class="toast" :class="toast.tone">
        <span class="badge">
          <component :is="icons[toast.tone]" :size="12" />
        </span>

        <span class="text">{{ toast.text }}</span>

        <button
          v-if="toast.actionLabel"
          type="button"
          class="action"
          @click="toast.action?.(); toasts.dismiss(toast.id)"
        >
          {{ toast.actionLabel }}
        </button>

        <button type="button" class="close" aria-label="Kapat" @click="toasts.dismiss(toast.id)">
          <X :size="13" />
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.stack {
  position: fixed;
  right: var(--rv-space-4);
  bottom: var(--rv-space-4);
  z-index: 50;
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-2);
  pointer-events: none;
}

.toast {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px var(--rv-space-3);
  border-radius: var(--rv-radius-sm);
  background: var(--rv-bg-elevated);
  border: 1px solid var(--rv-border);
  box-shadow: var(--rv-shadow-1);
  font-size: 12px;
  color: var(--rv-text);
  max-width: 420px;
  pointer-events: auto;
}

.badge {
  width: 18px;
  height: 18px;
  flex: none;
  display: grid;
  place-items: center;
  border-radius: var(--rv-radius-pill);
}

.success .badge {
  background: var(--rv-success-soft);
  color: var(--rv-success);
}

.error .badge {
  background: var(--rv-danger-soft);
  color: var(--rv-danger);
}

.warning .badge {
  background: var(--rv-warning-soft);
  color: var(--rv-warning);
}

.info .badge {
  background: var(--rv-accent-soft);
  color: var(--rv-accent-quiet);
}

.error {
  border-color: color-mix(in srgb, var(--rv-danger) 30%, transparent);
}

.text {
  flex: 1;
  min-width: 0;
  line-height: 1.45;
}

.action {
  flex: none;
  background: none;
  border: none;
  color: var(--rv-accent-quiet);
  font: inherit;
  font-weight: 500;
  cursor: pointer;
  padding: 0;
}

.action:hover {
  color: var(--rv-accent-hover);
}

.close {
  flex: none;
  background: none;
  border: none;
  color: var(--rv-text-faint);
  cursor: pointer;
  padding: 0;
  display: grid;
  place-items: center;
}

.close:hover {
  color: var(--rv-text);
}

.toast-enter-active,
.toast-leave-active {
  transition: opacity 0.18s ease, transform 0.18s ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(6px);
}
</style>
