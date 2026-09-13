<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: string;
    placeholder?: string;
    invalid?: boolean;
    error?: string;
    disabled?: boolean;
    size?: "md" | "lg";
    mono?: boolean;
    type?: "text" | "password";
  }>(),
  { invalid: false, disabled: false, size: "md", mono: false, type: "text" },
);

defineEmits<{ "update:modelValue": [value: string] }>();
</script>

<template>
  <div class="field">
    <div class="wrap" :class="[`s-${size}`, { invalid, disabled }]">
      <slot name="leading" />
      <input
        :value="modelValue"
        :type="type"
        :placeholder="placeholder"
        :disabled="disabled"
        :class="{ mono }"
        @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
      />
      <slot name="trailing" />
    </div>
    <span v-if="invalid && error" class="error">{{ error }}</span>
  </div>
</template>

<style scoped>
.field {
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-1);
  min-width: 0;
}

.wrap {
  display: flex;
  align-items: center;
  gap: 10px;
  border-radius: var(--rv-radius-sm);
  background: var(--rv-bg-inset);
  border: 1px solid var(--rv-border);
  color: var(--rv-text-faint);
}

.s-md {
  height: 32px;
  padding: 0 10px;
}

.s-lg {
  height: 48px;
  padding: 0 14px;
  border-radius: var(--rv-radius-md);
}

.wrap:focus-within {
  border-color: var(--rv-accent);
  box-shadow: var(--rv-focus);
}

.wrap.invalid {
  border-color: var(--rv-danger);
}

.wrap.disabled {
  opacity: 0.5;
}

input {
  flex: 1;
  min-width: 0;
  background: none;
  border: none;
  outline: none;
  color: var(--rv-text);
  font: inherit;
  font-size: 13px;
}

.s-lg input {
  font-size: 15px;
}

input.mono {
  font-family: var(--rv-font-mono);
  font-size: 12px;
}

input::placeholder {
  color: var(--rv-text-faint);
}

.error {
  font-size: 11px;
  color: var(--rv-danger);
}
</style>
