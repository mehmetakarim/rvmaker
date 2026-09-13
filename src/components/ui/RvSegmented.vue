<script setup lang="ts">
defineProps<{
  modelValue: string;
  options: { value: string; label: string }[];
}>();
defineEmits<{ "update:modelValue": [value: string] }>();
</script>

<template>
  <div class="seg" role="tablist">
    <button
      v-for="opt in options"
      :key="opt.value"
      type="button"
      role="tab"
      :aria-selected="modelValue === opt.value"
      :class="{ on: modelValue === opt.value }"
      @click="$emit('update:modelValue', opt.value)"
    >
      {{ opt.label }}
    </button>
  </div>
</template>

<style scoped>
.seg {
  display: inline-flex;
  padding: 2px;
  gap: 2px;
  border-radius: var(--rv-radius-sm);
  background: var(--rv-bg-inset);
  border: 1px solid var(--rv-border);
  font-size: 12px;
}

button {
  padding: 4px 12px;
  border-radius: 4px;
  border: none;
  background: transparent;
  color: var(--rv-text-muted);
  font: inherit;
  cursor: pointer;
}

button:hover:not(.on) {
  color: var(--rv-text);
}

button.on {
  background: var(--rv-bg-elevated);
  color: var(--rv-text);
  font-weight: 500;
}
</style>
