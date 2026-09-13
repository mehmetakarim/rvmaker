<script setup lang="ts">
import { ChevronDown } from "@lucide/vue";

defineProps<{
  modelValue: string;
  options: { value: string; label: string }[];
  disabled?: boolean;
}>();
defineEmits<{ "update:modelValue": [value: string] }>();
</script>

<template>
  <div class="select" :class="{ disabled }">
    <select
      :value="modelValue"
      :disabled="disabled"
      @change="$emit('update:modelValue', ($event.target as HTMLSelectElement).value)"
    >
      <option v-for="opt in options" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
    </select>
    <ChevronDown :size="14" class="chevron" />
  </div>
</template>

<style scoped>
.select {
  position: relative;
  display: flex;
  align-items: center;
  height: 32px;
  padding: 0 10px;
  border-radius: var(--rv-radius-sm);
  background: var(--rv-bg-inset);
  border: 1px solid var(--rv-border);
}

.select:focus-within {
  border-color: var(--rv-accent);
  box-shadow: var(--rv-focus);
}

.select.disabled {
  opacity: 0.5;
}

select {
  appearance: none;
  flex: 1;
  min-width: 0;
  background: none;
  border: none;
  outline: none;
  color: var(--rv-text);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}

.chevron {
  color: var(--rv-text-faint);
  pointer-events: none;
  flex: none;
}
</style>
