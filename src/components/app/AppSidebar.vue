<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { Plus, ListOrdered, Library, Settings } from "@lucide/vue";
import { useJobsStore } from "@/stores/jobs";
import { useSettingsStore } from "@/stores/settings";
import { appVersion } from "@/lib/api";

const jobs = useJobsStore();
const settings = useSettingsStore();

const version = ref("");

onMounted(async () => {
  version.value = await appVersion();
});

/** Derleme damgası: gün.ay saat:dakika — hangi paketin çalıştığını ayırt eder. */
const buildTime = __BUILD_TIME__;

const buildStamp = computed(() => {
  const d = new Date(__BUILD_TIME__);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${pad(d.getDate())}.${pad(d.getMonth() + 1)} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
});

const items = computed(() => [
  { to: "/yeni", label: "Yeni Video", icon: Plus, badge: 0 },
  { to: "/kuyruk", label: "Kuyruk", icon: ListOrdered, badge: jobs.pendingCount },
  { to: "/kitaplik", label: "Kitaplık", icon: Library, badge: 0 },
  { to: "/ayarlar", label: "Ayarlar", icon: Settings, badge: 0 },
]);
</script>

<template>
  <aside class="sidebar">
    <nav>
      <RouterLink
        v-for="item in items"
        :key="item.to"
        :to="item.to"
        class="item"
        active-class="active"
      >
        <component :is="item.icon" :size="16" />
        {{ item.label }}
        <span v-if="item.badge > 0" class="badge rv-tabular">{{ item.badge }}</span>
      </RouterLink>
    </nav>
    <div class="status">
      <div class="status-line">
        <span class="dot" :class="{ bad: !settings.environmentReady }"></span>
        {{ settings.environmentSummary }}
      </div>
      <div class="version rv-mono" :title="`Derleme: ${buildTime}`">
        v{{ version }} · {{ buildStamp }}
      </div>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: var(--rv-sidebar-w);
  flex: none;
  display: flex;
  flex-direction: column;
  padding: var(--rv-space-2);
  border-right: 1px solid var(--rv-border);
  background: var(--rv-bg-surface);
}

nav {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.item {
  display: flex;
  align-items: center;
  gap: var(--rv-space-2);
  height: 30px;
  padding: 0 var(--rv-space-2);
  border-radius: var(--rv-radius-sm);
  color: var(--rv-text-muted);
  font-size: 13px;
}

.item:hover {
  background: var(--rv-bg-elevated);
  color: var(--rv-text);
}

.item.active {
  background: var(--rv-accent-soft);
  color: var(--rv-accent-quiet);
  font-weight: 500;
}

.badge {
  margin-left: auto;
  font-size: 11px;
  color: var(--rv-text-faint);
}

.status {
  margin-top: auto;
  display: flex;
  flex-direction: column;
  gap: 3px;
  padding: 10px var(--rv-space-2) var(--rv-space-1);
  border-top: 1px solid var(--rv-border);
  font-size: 11px;
  color: var(--rv-text-faint);
}

.status-line {
  display: flex;
  align-items: center;
  gap: 6px;
}

.version {
  font-size: 10px;
  color: var(--rv-text-faint);
  opacity: 0.75;
}

.dot {
  width: 6px;
  height: 6px;
  border-radius: var(--rv-radius-pill);
  background: var(--rv-success);
  flex: none;
}

.dot.bad {
  background: var(--rv-warning);
}
</style>
