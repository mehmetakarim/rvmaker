<script setup lang="ts">
withDefaults(
  defineProps<{ tone?: "neutral" | "accent" | "success" | "danger" | "warning" }>(),
  { tone: "neutral" },
);
</script>

<template>
  <span class="badge" :class="`t-${tone}`"><slot /></span>
</template>

<style scoped>
.badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 9px;
  border-radius: var(--rv-radius-pill);
  font-size: 11px;
  white-space: nowrap;
}

.t-neutral {
  background: var(--rv-bg-elevated);
  border: 1px solid var(--rv-border);
  color: var(--rv-text-muted);
}

.t-accent {
  background: var(--rv-accent-soft);
  color: var(--rv-accent-quiet);
  font-weight: 500;
}

.t-success {
  background: var(--rv-success-soft);
  color: var(--rv-success);
  font-weight: 500;
}

.t-danger {
  background: var(--rv-danger-soft);
  color: var(--rv-danger);
  font-weight: 500;
}

.t-warning {
  background: var(--rv-warning-soft);
  color: var(--rv-warning);
  font-weight: 500;
}
</style>
