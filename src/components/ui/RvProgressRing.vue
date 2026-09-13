<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(defineProps<{ value: number; size?: number; stroke?: number }>(), {
  size: 36,
  stroke: 3,
});

const radius = computed(() => (props.size - props.stroke) / 2);
const circumference = computed(() => 2 * Math.PI * radius.value);
const offset = computed(() => circumference.value * (1 - Math.min(1, Math.max(0, props.value))));
</script>

<template>
  <svg class="ring" :width="size" :height="size" :viewBox="`0 0 ${size} ${size}`">
    <circle
      :cx="size / 2"
      :cy="size / 2"
      :r="radius"
      fill="none"
      :stroke-width="stroke"
      class="track"
    />
    <circle
      :cx="size / 2"
      :cy="size / 2"
      :r="radius"
      fill="none"
      :stroke-width="stroke"
      stroke-linecap="round"
      class="value"
      :stroke-dasharray="circumference"
      :stroke-dashoffset="offset"
    />
  </svg>
</template>

<style scoped>
.ring {
  transform: rotate(-90deg);
  flex: none;
}

.track {
  stroke: var(--rv-bg-inset);
}

.value {
  stroke: var(--rv-accent);
  transition: stroke-dashoffset 0.3s ease;
}
</style>
