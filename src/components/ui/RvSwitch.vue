<script setup lang="ts">
defineProps<{ modelValue: boolean; disabled?: boolean }>();
defineEmits<{ "update:modelValue": [value: boolean] }>();
</script>

<template>
  <button
    type="button"
    role="switch"
    class="sw"
    :class="{ on: modelValue }"
    :aria-checked="modelValue"
    :disabled="disabled"
    @click="$emit('update:modelValue', !modelValue)"
  >
    <span class="knob"></span>
  </button>
</template>

<style scoped>
.sw {
  width: 34px;
  height: 20px;
  flex: none;
  border-radius: var(--rv-radius-pill);
  background: var(--rv-bg-inset);
  border: 1px solid var(--rv-border-strong);
  display: flex;
  align-items: center;
  padding: 2px;
  cursor: pointer;
  transition: background 0.15s ease;
}

.sw.on {
  background: var(--rv-accent);
  border-color: transparent;
}

.sw:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.knob {
  width: 16px;
  height: 16px;
  border-radius: var(--rv-radius-pill);
  background: var(--rv-text-faint);
  transition: margin 0.15s ease, background 0.15s ease;
}

.sw.on .knob {
  margin-left: auto;
  background: #fff;
}
</style>
