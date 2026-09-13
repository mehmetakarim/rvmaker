<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: "primary" | "secondary" | "ghost" | "danger" | "dangerSolid";
    size?: "sm" | "md" | "lg";
    loading?: boolean;
    disabled?: boolean;
    type?: "button" | "submit";
  }>(),
  { variant: "secondary", size: "md", loading: false, disabled: false, type: "button" },
);
</script>

<template>
  <button
    :type="type"
    class="btn"
    :class="[`v-${variant}`, `s-${size}`, { loading }]"
    :disabled="disabled || loading"
  >
    <span v-if="loading" class="spinner" aria-hidden="true"></span>
    <slot />
  </button>
</template>

<style scoped>
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  border-radius: var(--rv-radius-sm);
  border: 1px solid transparent;
  font-weight: 500;
  line-height: 1;
  cursor: pointer;
  white-space: nowrap;
  transition: background 0.12s ease, border-color 0.12s ease;
}

.btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.s-sm {
  height: 28px;
  padding: 0 12px;
  font-size: 12px;
}

.s-md {
  height: 30px;
  padding: 0 14px;
  font-size: 13px;
}

.s-lg {
  height: 48px;
  padding: 0 20px;
  font-size: 14px;
  font-weight: 600;
  border-radius: var(--rv-radius-md);
}

.v-primary {
  background: var(--rv-accent);
  color: var(--rv-accent-contrast);
}

.v-primary:hover:not(:disabled) {
  background: var(--rv-accent-hover);
}

.v-primary:active:not(:disabled) {
  background: var(--rv-accent-press);
}

.v-secondary {
  background: var(--rv-bg-elevated);
  color: var(--rv-text);
  border-color: var(--rv-border-strong);
}

.v-secondary:hover:not(:disabled) {
  background: var(--rv-elevated-hover);
}

.v-secondary:active:not(:disabled) {
  background: var(--rv-elevated-press);
}

.v-ghost {
  background: transparent;
  color: var(--rv-text-muted);
  padding-inline: 12px;
}

.v-ghost:hover:not(:disabled) {
  background: var(--rv-bg-elevated);
  color: var(--rv-text);
}

.v-danger {
  background: transparent;
  color: var(--rv-danger);
  border-color: color-mix(in srgb, var(--rv-danger) 40%, transparent);
}

.v-danger:hover:not(:disabled) {
  background: var(--rv-danger-soft);
}

.v-dangerSolid {
  background: var(--rv-danger);
  color: #fff;
}

.v-dangerSolid:hover:not(:disabled) {
  background: color-mix(in srgb, var(--rv-danger) 85%, black);
}

.spinner {
  width: 12px;
  height: 12px;
  flex: none;
  border-radius: var(--rv-radius-pill);
  border: 2px solid color-mix(in srgb, currentColor 35%, transparent);
  border-top-color: currentColor;
  animation: rv-spin 0.7s linear infinite;
}
</style>
