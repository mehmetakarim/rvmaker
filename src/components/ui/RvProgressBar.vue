<script setup lang="ts">
withDefaults(defineProps<{ value: number; height?: number }>(), { height: 4 });
</script>

<template>
  <div
    class="bar"
    :style="{ height: `${height}px` }"
    role="progressbar"
    :aria-valuenow="Math.round(value * 100)"
    aria-valuemin="0"
    aria-valuemax="100"
  >
    <div class="fill" :style="{ width: `${Math.min(100, Math.max(0, value * 100))}%` }"></div>
  </div>
</template>

<style scoped>
.bar {
  flex: 1;
  border-radius: var(--rv-radius-pill);
  background: var(--rv-bg-inset);
  overflow: hidden;
}

.fill {
  height: 100%;
  background: var(--rv-accent);
  border-radius: var(--rv-radius-pill);
  transition: width 0.25s ease;
}
</style>
