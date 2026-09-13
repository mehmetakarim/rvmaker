<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import {
  CircleAlert,
  RotateCw,
  SlidersHorizontal,
  Copy,
  ChevronDown,
  ChevronUp,
  FolderOpen,
} from "@lucide/vue";
import { useJobsStore } from "@/stores/jobs";
import { useDraftStore } from "@/stores/draft";
import { useSettingsStore } from "@/stores/settings";
import StageList from "@/components/app/StageList.vue";
import LogConsole from "@/components/app/LogConsole.vue";
import RvButton from "@/components/ui/RvButton.vue";

const jobs = useJobsStore();
const draft = useDraftStore();
const settings = useSettingsStore();
const router = useRouter();

const showTechnical = ref(false);

const failed = computed(() => jobs.renderState === "failed");
const finished = computed(() => jobs.renderState === "done");
const cancelled = computed(() => jobs.renderState === "cancelled");

async function copyLog() {
  const text = jobs.log.map((l) => `${l.time}  ${l.text}`).join("\n");
  try {
    await navigator.clipboard.writeText(text);
    logCopied.value = true;
    setTimeout(() => (logCopied.value = false), 2000);
  } catch {
    /* pano erişimi yoksa sessizce geç */
  }
}

const logCopied = ref(false);
const percent = computed(() => Math.round(jobs.overallProgress * 100));

const activeStage = computed(() => jobs.stages.find((s) => s.state === "active"));
const previewComment = computed(() => draft.selectedComments[0]);

const headline = computed(() => {
  if (failed.value) return "Üretim durdu";
  if (cancelled.value) return "Üretim iptal edildi";
  if (finished.value) return "Video hazır";
  return "Video oluşturuluyor";
});

const failedStageLabel = computed(
  () => jobs.stages.find((s) => s.id === jobs.renderError.stageId)?.label ?? "",
);


</script>

<template>
  <main class="main">
    <!-- Genel ilerleme -->
    <header class="progress-head">
      <div class="progress-info">
        <div class="line">
          <span class="headline">{{ headline }}</span>
          <span v-if="failed" class="sub">
            {{ failedStageLabel }} aşamasında · {{ jobs.renderError.at }}
          </span>
          <span v-else-if="cancelled" class="sub">
            {{ jobs.clips.length }} ses · {{ jobs.cardPaths.length }} kart üretilmişti
          </span>
          <span v-else-if="finished" class="sub">
            {{ jobs.clips.length }} ses dosyası ·
            {{ jobs.formatDuration(jobs.totalAudioSeconds) }} ·
            {{ jobs.cardPaths.length }} kart
          </span>
          <span v-else class="sub">
            {{ activeStage?.label ?? "hazırlanıyor" }}
            <template v-if="activeStage?.detail"> · {{ activeStage.detail }}</template>
          </span>
          <span class="percent rv-tabular" :class="{ muted: failed }">%{{ percent }}</span>
        </div>
        <div class="bar">
          <div
            class="fill"
            :class="{ failed: failed || cancelled }"
            :style="{ width: `${percent}%` }"
          ></div>
        </div>
      </div>
      <RvButton
        v-if="finished && jobs.video"
        variant="primary"
        @click="jobs.revealJob(jobs.video.path)"
      >
        <FolderOpen :size="14" />
        Klasörde göster
      </RvButton>
      <RvButton v-else-if="cancelled" variant="primary" :loading="false" @click="jobs.retry()">
        <RotateCw :size="14" />
        Yeniden başlat
      </RvButton>
      <RvButton
        v-else-if="!failed"
        variant="secondary"
        :loading="jobs.cancelling"
        @click="jobs.cancel()"
      >
        İptal
      </RvButton>
    </header>

    <div class="split">
      <!-- Sol: çalışırken önizleme + günlük, hatada açıklama -->
      <div v-if="!failed" class="stage-pane">
        <div class="preview-area">
          <div class="phone">
            <div class="scrim"></div>
            <div class="card">
              <div class="byline">
                <span class="avatar"></span>
                {{ previewComment?.author }}
              </div>
              <div class="text">{{ previewComment?.translated }}</div>
            </div>
            <div class="stamp rv-mono">
              {{ activeStage?.detail ?? `${draft.selectedComments.length} yorum` }}
            </div>
          </div>
        </div>
        <LogConsole :lines="jobs.log" :total-lines="jobs.logLineCount" />
      </div>

      <div v-else class="error-pane rv-scroll">
        <div class="error-card">
          <div class="error-head">
            <span class="error-icon"><CircleAlert :size="15" /></span>
            <div class="error-text">
              <span class="error-title">{{ jobs.renderError.title }}</span>
              <span class="error-message">{{ jobs.renderError.message }}</span>
            </div>
          </div>

          <div class="error-actions">
            <RvButton variant="primary" @click="jobs.retry()">
              <RotateCw :size="14" />
              Tekrar dene
            </RvButton>
            <RvButton variant="secondary" @click="router.push('/yeni/ses')">
              <SlidersHorizontal :size="14" />
              Ayarları düzenle
            </RvButton>
            <RvButton variant="ghost" @click="copyLog">
              <Copy :size="14" />
              {{ logCopied ? "Kopyalandı" : "Günlüğü kopyala" }}
            </RvButton>
            <button type="button" class="tech-toggle" @click="showTechnical = !showTechnical">
              Teknik ayrıntıyı {{ showTechnical ? "gizle" : "göster" }}
              <ChevronUp v-if="showTechnical" :size="13" />
              <ChevronDown v-else :size="13" />
            </button>
          </div>

          <div v-if="showTechnical" class="technical rv-mono">
            <div v-for="(row, i) in jobs.renderError.technical" :key="i" :class="{ dim: i === 2 }">
              {{ row }}
            </div>
          </div>
        </div>
      </div>

      <!-- Sağ: aşamalar -->
      <StageList :stages="jobs.displayStages">
        <template #footer>
          <div v-if="!failed" class="meta">
            <div class="meta-row">
              <span>Çözünürlük</span>
              <span class="rv-mono value">
                {{ settings.video.resolution.replace("x", "×") }} · {{ settings.video.fps }} fps
              </span>
            </div>
            <div class="meta-row">
              <span>Ses</span>
              <span class="value">Google · {{ draft.selectedVoice?.name ?? "Türkçe" }}</span>
            </div>
            <div class="meta-row">
              <span>Çıktı klasörü</span>
              <span class="rv-mono value">{{ settings.video.outputDir }}</span>
            </div>
          </div>
        </template>
      </StageList>
    </div>
  </main>
</template>

<style scoped>
.main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.progress-head {
  flex: none;
  display: flex;
  align-items: center;
  gap: var(--rv-space-4);
  padding: 14px 20px;
  border-bottom: 1px solid var(--rv-border);
}

.progress-info {
  display: flex;
  flex-direction: column;
  gap: 6px;
  flex: 1;
}

.line {
  display: flex;
  align-items: baseline;
  gap: 10px;
}

.headline {
  font-weight: 600;
  font-size: 15px;
}

.sub {
  font-size: 12px;
  color: var(--rv-text-faint);
}

.percent {
  margin-left: auto;
  font-weight: 600;
}

.percent.muted {
  color: var(--rv-text-muted);
}

.bar {
  height: 5px;
  border-radius: var(--rv-radius-pill);
  background: var(--rv-bg-inset);
  overflow: hidden;
}

.fill {
  height: 100%;
  background: var(--rv-accent);
  border-radius: var(--rv-radius-pill);
  transition: width 0.3s ease;
}

.fill.failed {
  background: var(--rv-danger);
}

.split {
  flex: 1;
  display: flex;
  min-height: 0;
}

.stage-pane {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.preview-area {
  flex: 1;
  min-height: 0;
  display: grid;
  place-items: center;
  padding: 20px;
  background: var(--rv-bg-inset);
}

.phone {
  width: 250px;
  height: 444px;
  position: relative;
  overflow: hidden;
  border-radius: 18px;
  border: 1px solid var(--rv-border-strong);
  background: repeating-linear-gradient(135deg, #242b3a 0 8px, #1e2431 8px 16px);
}

.scrim {
  position: absolute;
  inset: 0;
  background: linear-gradient(180deg, rgba(10, 11, 15, 0.3), rgba(10, 11, 15, 0.5));
}

.card {
  position: absolute;
  left: 14px;
  right: 14px;
  top: 50%;
  transform: translateY(-50%);
  padding: 13px;
  border-radius: 12px;
  background: rgba(20, 22, 27, 0.92);
  border: 1px solid rgba(255, 255, 255, 0.08);
  display: flex;
  flex-direction: column;
  gap: 7px;
}

.byline {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 10px;
  color: #9aa1b1;
}

.avatar {
  width: 14px;
  height: 14px;
  border-radius: var(--rv-radius-pill);
  background: #3a4152;
}

.text {
  font-size: 13px;
  line-height: 1.4;
  color: #f2f4f8;
}

.stamp {
  position: absolute;
  left: 14px;
  bottom: 14px;
  font-size: 10px;
  color: rgba(255, 255, 255, 0.65);
}

/* Hata */
.error-pane {
  flex: 1;
  min-width: 0;
  padding: var(--rv-space-5);
}

.error-card {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 18px;
  border-radius: var(--rv-radius-md);
  background: var(--rv-bg-surface);
  border: 1px solid color-mix(in srgb, var(--rv-danger) 35%, transparent);
}

.error-head {
  display: flex;
  gap: var(--rv-space-3);
}

.error-icon {
  width: 26px;
  height: 26px;
  flex: none;
  display: grid;
  place-items: center;
  border-radius: var(--rv-radius-sm);
  background: var(--rv-danger-soft);
  color: var(--rv-danger);
}

.error-text {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.error-title {
  font-size: 15px;
  font-weight: 600;
}

.error-message {
  color: var(--rv-text-muted);
  line-height: 1.55;
  text-wrap: pretty;
}

.error-actions {
  display: flex;
  align-items: center;
  gap: var(--rv-space-2);
  padding-left: 38px;
}

.tech-toggle {
  margin-left: 6px;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: none;
  border: none;
  color: var(--rv-accent-quiet);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

.technical {
  margin-left: 38px;
  padding: 10px var(--rv-space-3);
  border-radius: var(--rv-radius-sm);
  background: var(--rv-bg-inset);
  border: 1px solid var(--rv-border);
  font-size: 11.5px;
  line-height: 1.75;
  color: var(--rv-text-muted);
}

.technical .dim {
  color: var(--rv-text-faint);
}

/* Aşama paneli altı */
.meta {
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-2);
  padding-top: var(--rv-space-4);
  border-top: 1px solid var(--rv-border);
  font-size: 12px;
  color: var(--rv-text-faint);
}

.meta-row {
  display: flex;
}

.value {
  margin-left: auto;
  color: var(--rv-text-muted);
}
</style>
