<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    modelValue: number;
    min?: number;
    max?: number;
    step?: number;
    disabled?: boolean;
  }>(),
  { min: 0, max: 100, step: 1, disabled: false },
);

defineEmits<{ "update:modelValue": [value: number] }>();

const percent = computed(() => {
  const span = props.max - props.min || 1;
  return Math.min(100, Math.max(0, ((props.modelValue - props.min) / span) * 100));
});
</script>

<template>
  <div class="slider" :class="{ disabled }">
    <div class="track">
      <div class="fill" :style="{ width: `${percent}%` }"></div>
      <div class="thumb" :style="{ left: `${percent}%` }"></div>
    </div>
    <input
      type="range"
      :value="modelValue"
      :min="min"
      :max="max"
      :step="step"
      :disabled="disabled"
      @input="$emit('update:modelValue', Number(($event.target as HTMLInputElement).value))"
    />
  </div>
</template>

<style scoped>
.slider {
  position: relative;
  flex: 1;
  height: 14px;
  display: flex;
  align-items: center;
}

.slider.disabled {
  opacity: 0.45;
}

.track {
  position: absolute;
  inset: 5px 0;
  height: 4px;
  border-radius: var(--rv-radius-pill);
  background: var(--rv-bg-inset);
}

.fill {
  height: 100%;
  background: var(--rv-accent);
  border-radius: var(--rv-radius-pill);
}

.thumb {
  position: absolute;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 14px;
  height: 14px;
  border-radius: var(--rv-radius-pill);
  background: var(--rv-text);
  border: 1px solid var(--rv-border-strong);
  pointer-events: none;
}

input[type="range"] {
  position: absolute;
  inset: 0;
  width: 100%;
  margin: 0;
  opacity: 0;
  cursor: pointer;
}

input[type="range"]:focus-visible + .track,
.slider:focus-within .thumb {
  box-shadow: var(--rv-focus);
}
</style>
