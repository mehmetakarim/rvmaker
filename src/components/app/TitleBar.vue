<script setup lang="ts">
import { computed } from "vue";
import { Layers } from "@lucide/vue";
import { useJobsStore } from "@/stores/jobs";

defineProps<{ title: string }>();

const jobs = useJobsStore();
const pending = computed(() => jobs.pendingCount);
</script>

<template>
  <header class="titlebar" data-tauri-drag-region>
    <!-- macOS trafik ışıkları webview'in üstüne biniyor; bu boşluk onlar için -->
    <div class="traffic-lights" aria-hidden="true" data-tauri-drag-region></div>
    <span class="title" data-tauri-drag-region>{{ title }}</span>
    <span v-if="pending > 0" class="hint" data-tauri-drag-region>
      <Layers :size="14" />
      Kuyrukta {{ pending }} iş
    </span>
  </header>
</template>

<style scoped>
.titlebar {
  height: var(--rv-titlebar-h);
  flex: none;
  display: flex;
  align-items: center;
  gap: var(--rv-space-2);
  padding: 0 var(--rv-space-3);
  border-bottom: 1px solid var(--rv-border);
  background: var(--rv-bg-surface);
}

.traffic-lights {
  width: var(--rv-traffic-lights-w);
  flex: none;
}

.title {
  font-size: 12px;
  font-weight: 500;
  color: var(--rv-text-muted);
}

.hint {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--rv-text-faint);
}
</style>
