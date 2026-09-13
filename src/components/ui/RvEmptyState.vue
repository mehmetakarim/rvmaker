<script setup lang="ts">
defineProps<{ title: string; description?: string }>();
</script>

<template>
  <div class="empty" data-component="EmptyState">
    <div class="icon"><slot name="icon" /></div>
    <div class="title">{{ title }}</div>
    <div v-if="description" class="desc">{{ description }}</div>
    <div class="actions"><slot name="actions" /></div>
  </div>
</template>

<style scoped>
.empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--rv-space-3);
  text-align: center;
  padding: var(--rv-space-6);
}

.icon {
  width: 56px;
  height: 56px;
  display: grid;
  place-items: center;
  border-radius: var(--rv-radius-md);
  background: var(--rv-bg-surface);
  border: 1px solid var(--rv-border);
  color: var(--rv-text-faint);
  margin-bottom: var(--rv-space-1);
}

.title {
  font-size: 15px;
  font-weight: 600;
  letter-spacing: -0.01em;
}

.desc {
  font-size: 13px;
  color: var(--rv-text-muted);
  max-width: 380px;
  line-height: 1.5;
  text-wrap: pretty;
}

.actions {
  display: flex;
  gap: var(--rv-space-2);
  margin-top: var(--rv-space-1);
}
</style>
