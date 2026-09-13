<script setup lang="ts">
import { Check } from "@lucide/vue";

defineProps<{ modelValue: boolean; indeterminate?: boolean; disabled?: boolean }>();
defineEmits<{ "update:modelValue": [value: boolean] }>();
</script>

<template>
  <button
    type="button"
    role="checkbox"
    class="cb"
    :class="{ on: modelValue || indeterminate }"
    :aria-checked="indeterminate ? 'mixed' : modelValue"
    :disabled="disabled"
    @click="$emit('update:modelValue', !modelValue)"
  >
    <span v-if="indeterminate" class="dash"></span>
    <Check v-else-if="modelValue" :size="11" />
  </button>
</template>

<style scoped>
.cb {
  width: 15px;
  height: 15px;
  flex: none;
  border-radius: 4px;
  border: 1px solid var(--rv-border-strong);
  background: transparent;
  color: #fff;
  display: grid;
  place-items: center;
  cursor: pointer;
  padding: 0;
}

.cb.on {
  background: var(--rv-accent);
  border-color: transparent;
}

.cb:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.dash {
  width: 8px;
  height: 1.5px;
  background: #fff;
}
</style>
