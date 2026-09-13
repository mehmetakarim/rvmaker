<script setup lang="ts">
defineProps<{
  modelValue: string;
  tabs: { value: string; label: string }[];
}>();
defineEmits<{ "update:modelValue": [value: string] }>();
</script>

<template>
  <div class="tabs" role="tablist">
    <button
      v-for="tab in tabs"
      :key="tab.value"
      type="button"
      role="tab"
      :aria-selected="modelValue === tab.value"
      :class="{ on: modelValue === tab.value }"
      @click="$emit('update:modelValue', tab.value)"
    >
      {{ tab.label }}
    </button>
    <slot name="end" />
  </div>
</template>

<style scoped>
.tabs {
  display: flex;
  align-items: center;
  gap: 6px;
  border-bottom: 1px solid var(--rv-border);
}

button {
  padding: 7px 10px;
  font-size: 12px;
  background: none;
  border: none;
  color: var(--rv-text-muted);
  font-family: inherit;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  margin-bottom: -1px;
}

button:hover:not(.on) {
  color: var(--rv-text);
}

button.on {
  color: var(--rv-text);
  font-weight: 500;
  border-bottom-color: var(--rv-accent);
}
</style>
