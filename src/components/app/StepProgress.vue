<script setup lang="ts">
import { Check, ChevronRight } from "@lucide/vue";

defineProps<{ current: number }>();

const steps = ["Kaynak", "İçerik", "Ses", "Görünüm"];
</script>

<template>
  <div class="strip" data-component="StepProgress">
    <template v-for="(label, i) in steps" :key="label">
      <ChevronRight v-if="i > 0" :size="13" class="sep" />
      <span class="step" :class="{ done: i < current, now: i === current }">
        <Check v-if="i < current" :size="13" class="check" />
        {{ label }}
      </span>
    </template>
    <div class="action"><slot /></div>
  </div>
</template>

<style scoped>
.strip {
  flex: none;
  display: flex;
  align-items: center;
  gap: var(--rv-space-2);
  height: 44px;
  padding: 0 20px;
  border-bottom: 1px solid var(--rv-border);
  font-size: 12px;
  color: var(--rv-text-faint);
}

.step {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.step.done {
  color: var(--rv-text-muted);
}

.step.now {
  color: var(--rv-text);
  font-weight: 500;
}

.check {
  color: var(--rv-success);
}

.sep {
  flex: none;
}

.action {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: var(--rv-space-2);
}
</style>
