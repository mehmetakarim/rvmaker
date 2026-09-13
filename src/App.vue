<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRoute } from "vue-router";
import { useSettingsStore } from "@/stores/settings";
import TitleBar from "@/components/app/TitleBar.vue";
import AppSidebar from "@/components/app/AppSidebar.vue";
import RvToast from "@/components/ui/RvToast.vue";

const route = useRoute();
const settings = useSettingsStore();

// Kenar çubuğundaki durum göstergesi gerçek denetime dayanıyor.
onMounted(() => {
  settings.runEnvironmentCheck();
});

// Kurulum ekranı kenar çubuğu olmadan, tek sütun açılır.
const bare = computed(() => route.meta.chrome === "bare");
const title = computed(() => (route.meta.title as string) ?? "RVMaker");
</script>

<template>
  <div class="shell">
    <TitleBar :title="title" />
    <div class="body">
      <AppSidebar v-if="!bare" />
      <RouterView />
    </div>

    <RvToast />
  </div>
</template>

<style scoped>
.shell {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--rv-bg-base);
  color: var(--rv-text);
}

.body {
  flex: 1;
  display: flex;
  min-height: 0;
}
</style>
