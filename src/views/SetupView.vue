<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import {
  Check,
  TriangleAlert,
  CircleAlert,
  RefreshCw,
  Copy,
  ArrowRight,
} from "@lucide/vue";
import { useSettingsStore } from "@/stores/settings";
import RvButton from "@/components/ui/RvButton.vue";

const settings = useSettingsStore();
const router = useRouter();

const copied = ref("");

onMounted(() => {
  settings.runEnvironmentCheck();
});

/** Denetim yapılmadan "hazır" diyemeyiz; boş liste bilgi değil, bilgisizlik. */
const checked = computed(() => settings.setupItems.length > 0);
const ready = computed(() => checked.value && settings.blockingIssues.length === 0);

async function copyCommand(command: string) {
  try {
    await navigator.clipboard.writeText(command);
    copied.value = command;
    setTimeout(() => (copied.value = ""), 2000);
  } catch {
    /* pano erişimi yoksa sessizce geç */
  }
}

function proceed() {
  router.push("/yeni/kaynak");
}
</script>

<template>
  <div class="page rv-scroll">
    <div class="column">
      <header>
        <h1>RVMaker'ı hazırlayalım</h1>
        <p>
          Video üretmek için gereken bileşenler denetleniyor. Zorunlu olanlar eksikse üretim
          yapılamaz; uyarılar yalnızca kaliteyi etkiler.
        </p>
      </header>

      <div class="checklist" data-component="SetupChecklist">
        <div v-if="settings.setupItems.length === 0" class="row empty">
          <span class="mark working"><RefreshCw :size="14" /></span>
          <div class="info">
            <span class="label">
              {{ settings.setupLoading ? "Denetleniyor…" : "Henüz denetlenmedi" }}
            </span>
            <span class="detail">Bu birkaç saniye sürebilir; çeviri servisi ağ üzerinden sınanıyor.</span>
          </div>
        </div>

        <div
          v-for="item in settings.setupItems"
          :key="item.id"
          class="row"
          :class="item.state"
        >
          <span class="mark" :class="item.state">
            <Check v-if="item.state === 'ready'" :size="14" />
            <CircleAlert v-else-if="item.state === 'missing'" :size="14" />
            <TriangleAlert v-else :size="14" />
          </span>

          <div class="info">
            <div class="label-line">
              <span class="label">{{ item.label }}</span>
              <span v-if="!item.required" class="optional">isteğe bağlı</span>
            </div>
            <span class="detail">{{ item.detail }}</span>

            <div v-if="item.fixHint" class="fix">
              <span>{{ item.fixHint }}</span>
              <button
                v-if="item.fixCommand"
                type="button"
                class="command rv-mono"
                :title="copied === item.fixCommand ? 'Kopyalandı' : 'Komutu kopyala'"
                @click="copyCommand(item.fixCommand)"
              >
                {{ item.fixCommand }}
                <Copy :size="12" />
              </button>
            </div>
          </div>

          <span v-if="item.state === 'ready'" class="state ready">Hazır</span>
          <RvButton
            v-else-if="item.id === 'ffmpeg'"
            variant="primary"
            size="sm"
            :loading="settings.installing"
            @click="settings.runInstall(item.id)"
          >
            Kur
          </RvButton>
          <span v-else-if="item.state === 'warning'" class="state warning">Uyarı</span>
          <span v-else class="state missing">Eksik</span>
        </div>
      </div>

      <div v-if="settings.installLog.length > 0" class="install-log rv-mono">
        <div v-for="(line, i) in settings.installLog" :key="i">{{ line }}</div>
      </div>

      <div v-if="settings.setupError" class="error">{{ settings.setupError }}</div>

      <footer>
        <RvButton
          variant="secondary"
          :loading="settings.setupLoading"
          @click="settings.runEnvironmentCheck()"
        >
          <RefreshCw v-if="!settings.setupLoading" :size="14" />
          Yeniden denetle
        </RvButton>

        <span class="note">
          <template v-if="!checked">Ortam henüz denetlenmedi.</template>
          <template v-else-if="ready">Her şey hazır, başlayabilirsin.</template>
          <template v-else>
            {{ settings.blockingIssues.length }} zorunlu bileşen eksik.
          </template>
        </span>

        <RvButton :variant="ready ? 'primary' : 'ghost'" class="skip" @click="proceed">
          {{ ready ? "Başla" : "Yine de devam et" }}
          <ArrowRight v-if="ready" :size="15" />
        </RvButton>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.page {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.column {
  width: 620px;
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-5);
  padding: var(--rv-space-6) 0;
}

header {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

h1 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
  letter-spacing: -0.01em;
}

header p {
  margin: 0;
  font-size: 13px;
  color: var(--rv-text-muted);
  line-height: 1.5;
  text-wrap: pretty;
}

.checklist {
  display: flex;
  flex-direction: column;
  background: var(--rv-bg-surface);
  border: 1px solid var(--rv-border);
  border-radius: var(--rv-radius-md);
  overflow: hidden;
}

.row {
  display: flex;
  align-items: flex-start;
  gap: var(--rv-space-3);
  padding: 14px var(--rv-space-4);
  border-bottom: 1px solid var(--rv-border);
}

.row:last-child {
  border-bottom: none;
}

.row.missing {
  background: color-mix(in srgb, var(--rv-danger) 5%, transparent);
}

.row.warning {
  background: color-mix(in srgb, var(--rv-warning) 4%, transparent);
}

.mark {
  width: 24px;
  height: 24px;
  flex: none;
  display: grid;
  place-items: center;
  border-radius: var(--rv-radius-sm);
  margin-top: 1px;
}

.mark.ready {
  background: var(--rv-success-soft);
  color: var(--rv-success);
}

.mark.working {
  background: var(--rv-accent-soft);
  color: var(--rv-accent-quiet);
  animation: rv-spin 1.4s linear infinite;
}

.mark.warning {
  background: var(--rv-warning-soft);
  color: var(--rv-warning);
}

.mark.missing {
  background: var(--rv-danger-soft);
  color: var(--rv-danger);
}

.info {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex: 1;
  min-width: 0;
}

.label-line {
  display: flex;
  align-items: baseline;
  gap: var(--rv-space-2);
}

.label {
  font-weight: 500;
}

.optional {
  font-size: 11px;
  color: var(--rv-text-faint);
}

.detail {
  font-size: 12px;
  color: var(--rv-text-faint);
  overflow: hidden;
  text-overflow: ellipsis;
}

.fix {
  display: flex;
  align-items: center;
  gap: var(--rv-space-2);
  flex-wrap: wrap;
  font-size: 12px;
  color: var(--rv-text-muted);
  margin-top: 2px;
}

.command {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 3px 8px;
  border-radius: 4px;
  background: var(--rv-bg-inset);
  border: 1px solid var(--rv-border);
  color: var(--rv-text);
  font-size: 11px;
  cursor: pointer;
}

.command:hover {
  border-color: var(--rv-border-strong);
}

.state {
  margin-left: auto;
  font-size: 12px;
  color: var(--rv-text-muted);
  flex: none;
}

.state.ready {
  color: var(--rv-success);
}

.state.warning {
  color: var(--rv-warning);
}

.state.missing {
  color: var(--rv-danger);
}

.install-log {
  max-height: 140px;
  overflow: auto;
  padding: 10px var(--rv-space-3);
  border-radius: var(--rv-radius-sm);
  background: var(--rv-bg-inset);
  border: 1px solid var(--rv-border);
  font-size: 11.5px;
  line-height: 1.7;
  color: var(--rv-text-muted);
}

.error {
  padding: 10px var(--rv-space-3);
  border-radius: var(--rv-radius-sm);
  background: var(--rv-danger-soft);
  border: 1px solid color-mix(in srgb, var(--rv-danger) 30%, transparent);
  font-size: 12px;
  color: var(--rv-text-muted);
}

footer {
  display: flex;
  align-items: center;
  gap: var(--rv-space-3);
}

.note {
  font-size: 12px;
  color: var(--rv-text-faint);
}

.skip {
  margin-left: auto;
}
</style>
