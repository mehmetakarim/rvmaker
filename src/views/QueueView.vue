<script setup lang="ts">
import { computed } from "vue";
import {
  Play,
  Pause,
  GripVertical,
  X,
  Check,
  FolderOpen,
  RotateCw,
} from "@lucide/vue";
import { ref } from "vue";
import { useRouter } from "vue-router";
import { useJobsStore } from "@/stores/jobs";
import { isTauri, revealInFinder } from "@/lib/api";
import RvButton from "@/components/ui/RvButton.vue";
import RvBadge from "@/components/ui/RvBadge.vue";
import RvTooltip from "@/components/ui/RvTooltip.vue";

const jobs = useJobsStore();
const router = useRouter();

// --- Sürükleyerek sıralama ---
const surukleneninIndeksi = ref<number | null>(null);
const uzerindekiIndeks = ref<number | null>(null);

function surukleBasla(index: number) {
  surukleneninIndeksi.value = index;
}

function uzerinde(index: number, event: DragEvent) {
  event.preventDefault();
  uzerindekiIndeks.value = index;
}

function birak(hedef: number) {
  const kaynak = surukleneninIndeksi.value;
  surukleneninIndeksi.value = null;
  uzerindekiIndeks.value = null;
  if (kaynak === null || kaynak === hedef) return;
  jobs.moveJob(jobs.jobs[kaynak].id, hedef - kaynak);
}

async function reveal(path?: string) {
  if (!path || !isTauri()) return;
  try {
    await revealInFinder(path);
  } catch {
    /* dosya taşınmış olabilir */
  }
}

const summary = computed(
  () => `${jobs.waitingCount} bekliyor · ${jobs.runningCount} çalışıyor · toplam ~9 dk`,
);
</script>

<template>
  <main class="main">
    <div class="toolbar">
      <RvButton
        variant="primary"
        :disabled="jobs.waitingCount === 0 || jobs.runningCount > 0"
        :title="
          jobs.runningCount > 0
            ? 'Bir iş zaten çalışıyor'
            : jobs.waitingCount === 0
              ? 'Bekleyen iş yok'
              : 'Bekleyen işleri başlat'
        "
        @click="jobs.startQueue()"
      >
        <Play :size="14" />
        Tümünü başlat
      </RvButton>
      <RvButton
        variant="secondary"
        :disabled="jobs.runningCount === 0"
        @click="jobs.cancel()"
      >
        <Pause :size="14" />
        Durdur
      </RvButton>

      <div class="divider"></div>

      <RvTooltip text="Render sırasında ffmpeg zaten tüm çekirdekleri kullanıyor; ikinci bir iş hızlandırmaz, yalnızca ikisini de yavaşlatır.">
        <span class="label serial">Sırayla üretilir</span>
      </RvTooltip>

      <span class="summary">{{ summary }}</span>
    </div>

    <div class="rows rv-scroll">
      <div v-if="jobs.jobs.length === 0" class="empty-queue">
        Kuyruk boş. "Yeni Video" ekranından bir üretim başlattığında işler burada görünür.
      </div>

      <div
        v-for="(job, index) in jobs.jobs"
        :key="job.id"
        class="row"
        :class="[
          job.status,
          {
            dragging: surukleneninIndeksi === index,
            dropTarget: uzerindekiIndeks === index && surukleneninIndeksi !== index,
          },
        ]"
        data-component="QueueRow"
        draggable="true"
        @dragstart="surukleBasla(index)"
        @dragover="uzerinde(index, $event)"
        @drop="birak(index)"
        @dragend="surukleneninIndeksi = null; uzerindekiIndeks = null"
      >
        <GripVertical :size="14" class="grip" />
        <span class="index rv-mono">{{ index + 1 }}</span>

        <div class="body">
          <div class="head">
            <span class="title" :class="{ strong: job.status === 'running' }">{{ job.title }}</span>
            <span v-if="job.status === 'running'" class="preset">{{ job.presetSummary }}</span>
          </div>

          <template v-if="job.status === 'running'">
            <div class="bar">
              <div class="fill" :style="{ width: `${job.progress * 100}%` }"></div>
            </div>
            <span class="detail">{{ job.statusDetail }}</span>
          </template>

          <span v-else class="detail">{{ job.statusDetail ?? job.presetSummary }}</span>
        </div>

        <RvBadge v-if="job.status === 'running'" tone="accent">Çalışıyor</RvBadge>
        <RvBadge v-else-if="job.status === 'waiting'" tone="neutral">Bekliyor</RvBadge>
        <RvBadge v-else-if="job.status === 'failed'" tone="danger">Hata</RvBadge>
        <RvBadge v-else tone="success">
          <Check :size="12" />
          Bitti
        </RvBadge>

        <RvButton
          v-if="job.status === 'failed'"
          variant="secondary"
          size="sm"
          @click="router.push('/render'); jobs.retry()"
        >
          <RotateCw :size="13" />
          Tekrar dene
        </RvButton>
        <RvTooltip
          v-else-if="job.status === 'done'"
          :text="job.outputPath ? 'Klasörde göster' : 'Çıktı bulunamadı'"
        >
          <button
            type="button"
            class="icon-btn"
            :disabled="!job.outputPath"
            @click="reveal(job.outputPath)"
          >
            <FolderOpen :size="16" />
          </button>
        </RvTooltip>
        <RvTooltip v-else text="Kuyruktan çıkar">
          <button type="button" class="icon-btn" @click="jobs.removeJob(job.id)">
            <X :size="16" />
          </button>
        </RvTooltip>
      </div>
    </div>
  </main>
</template>

<style scoped>
.serial {
  border-bottom: 1px dotted var(--rv-border-strong);
  cursor: help;
}

.main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.toolbar {
  flex: none;
  display: flex;
  align-items: center;
  gap: 10px;
  height: 52px;
  padding: 0 20px;
  border-bottom: 1px solid var(--rv-border);
}

.divider {
  width: 1px;
  height: 20px;
  background: var(--rv-border);
  margin: 0 var(--rv-space-1);
}

.label {
  font-size: 12px;
  color: var(--rv-text-muted);
}

.summary {
  margin-left: auto;
  font-size: 12px;
  color: var(--rv-text-faint);
}

.rows {
  flex: 1;
  padding: var(--rv-space-4) 20px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.row {
  display: flex;
  align-items: center;
  gap: var(--rv-space-3);
  padding: 12px 14px;
  border-radius: var(--rv-radius-md);
  background: var(--rv-bg-surface);
  border: 1px solid var(--rv-border);
}

.row.running {
  border-color: color-mix(in srgb, var(--rv-accent) 32%, transparent);
}

.row.failed {
  border-color: color-mix(in srgb, var(--rv-danger) 28%, transparent);
}

.row.done {
  opacity: 0.85;
}

.row.dragging {
  opacity: 0.4;
}

.row.dropTarget {
  border-color: var(--rv-accent);
  box-shadow: inset 0 2px 0 var(--rv-accent);
}

.empty-queue {
  padding: 40px 20px;
  text-align: center;
  font-size: 12px;
  color: var(--rv-text-faint);
  line-height: 1.6;
}

.grip {
  color: var(--rv-text-faint);
  flex: none;
  cursor: grab;
}

.index {
  width: 18px;
  font-size: 12px;
  color: var(--rv-text-faint);
  flex: none;
}

.body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.row.running .body {
  gap: 6px;
}

.head {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.title.strong {
  font-weight: 500;
}

.preset {
  font-size: 11px;
  color: var(--rv-text-faint);
  flex: none;
}

.bar {
  height: 4px;
  border-radius: var(--rv-radius-pill);
  background: var(--rv-bg-inset);
  overflow: hidden;
}

.fill {
  height: 100%;
  background: var(--rv-accent);
}

.detail {
  font-size: 11px;
  color: var(--rv-text-faint);
}

.row.running .detail {
  color: var(--rv-text-muted);
}

.icon-btn {
  background: none;
  border: none;
  color: var(--rv-text-faint);
  cursor: pointer;
  padding: 0;
  display: grid;
  place-items: center;
}

.icon-btn:hover:not(:disabled) {
  color: var(--rv-text);
}

.icon-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}
</style>
