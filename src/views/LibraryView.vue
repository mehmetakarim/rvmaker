<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { fileUrl, isTauri } from "@/lib/api";
import { sourceLabel } from "@/lib/types";
import {
  Search,
  X,
  Play,
  FolderOpen,
  RotateCw,
  Trash2,
  Film,
  Plus,
  CirclePause,
  ChevronDown,
} from "@lucide/vue";
import { useLibraryStore, type UnfinishedJob } from "@/stores/library";
import { useDraftStore } from "@/stores/draft";
import { useJobsStore } from "@/stores/jobs";
import RvButton from "@/components/ui/RvButton.vue";
import RvTextField from "@/components/ui/RvTextField.vue";
import RvSelect from "@/components/ui/RvSelect.vue";
import RvEmptyState from "@/components/ui/RvEmptyState.vue";

const library = useLibraryStore();
const draft = useDraftStore();
const jobs = useJobsStore();
const router = useRouter();

const resumeError = ref("");

const playing = ref(false);

onMounted(() => {
  library.refresh();
});

/** Seçili videonun webview'in oynatabileceği adresi. */
const videoSrc = computed(() =>
  library.selected && isTauri() ? fileUrl(library.selected.path) : "",
);

function selectItem(id: string) {
  if (library.selectedId !== id) playing.value = false;
  library.selectedId = id;
}

/** Kalıcı silme — dosyalar diskten gidiyor, onay şart. */
async function confirmDelete(id: string, title: string) {
  const onay = window.confirm(
    `"${title}" videosu, sesleri ve kartları kalıcı olarak silinecek. Devam edilsin mi?`,
  );
  if (!onay) return;
  playing.value = false;
  await library.remove(id);
}

/** Aynı kaynakla yeni bir üretim başlatmak için taslağı doldurup akışa gönderir. */
function regenerate(sourceUrl: string) {
  if (sourceUrl) draft.url = sourceUrl;
  router.push("/yeni/kaynak");
}

/** Yarım kalan işi kaldığı yerden sürdürür.
 *
 *  Aynı iş klasörüyle koşuyor: aynı ayarla üretilmiş ses parçaları diskten
 *  alınıyor, yalnızca eksikler yeniden seslendiriliyor. */
async function resume(job: UnfinishedJob) {
  if (!draft.restoreJobDraft(job.raw)) {
    resumeError.value = `"${job.title}" taslağı okunamadı; bu iş sürdürülemiyor.`;
    return;
  }
  resumeError.value = "";
  router.push("/render");
  await jobs.runJob(job.id);
}

/** Yarım üretim şeridi ekranı kaplamasın: ilk birkaçı görünür, gerisi açılır. */
const RESUME_GORUNUR = 3;
const resumeAcik = ref(false);
const gorunenYarimlar = computed(() =>
  resumeAcik.value ? library.unfinished : library.unfinished.slice(0, RESUME_GORUNUR),
);

/** Kalıcı silme — ses parçaları ve taslak diskten gidiyor, onay şart. */
async function removeUnfinished(job: UnfinishedJob) {
  const onay = window.confirm(
    `"${job.title}" yarım üretimi ve hazır ses parçaları silinecek. Devam edilsin mi?`,
  );
  if (!onay) return;
  await library.removeUnfinished(job.id);
}

async function removeAllUnfinished() {
  const adet = library.unfinished.length;
  const onay = window.confirm(
    `${adet} yarım üretimin tamamı ve hazır ses parçaları silinecek. Devam edilsin mi?`,
  );
  if (!onay) return;
  await library.removeAllUnfinished();
  resumeAcik.value = false;
}

const rangeOptions = [
  { value: "all", label: "Tüm zamanlar" },
  { value: "1", label: "Bugün" },
  { value: "7", label: "Son 7 gün" },
  { value: "30", label: "Son 30 gün" },
];
</script>

<template>
  <main class="main">
    <!-- Yarım kalanlar, tamamlanmış video olmasa da görünmeli. -->
    <section v-if="library.unfinished.length > 0" class="resume" data-component="ResumeStrip">
      <div class="resume-head">
        <CirclePause :size="14" class="resume-icon" />
        <span>Yarım kalan üretim</span>
        <span class="resume-count rv-tabular">{{ library.unfinished.length }}</span>
        <span class="resume-note">
          Kaldığı yerden sürdürülür; hazır ses parçaları yeniden üretilmez.
        </span>
        <button
          v-if="library.unfinished.length > 1"
          type="button"
          class="resume-clear"
          @click="removeAllUnfinished"
        >
          <Trash2 :size="13" />
          Tümünü sil
        </button>
      </div>

      <div v-if="resumeError" class="load-error">{{ resumeError }}</div>

      <div v-for="job in gorunenYarimlar" :key="job.id" class="resume-row">
        <button type="button" class="resume-body" @click="resume(job)">
          <span class="resume-title">{{ job.title }}</span>
          <span class="resume-meta">
            <span class="rv-mono">{{ job.handle }}</span>
            <span>{{ job.createdAt }}</span>
            <span class="rv-tabular">{{ job.doneClips }} / {{ job.totalClips }} ses hazır</span>
          </span>
        </button>
        <button type="button" class="resume-action" @click="resume(job)">Devam et</button>
        <button
          type="button"
          class="resume-delete"
          title="Yarım üretimi sil"
          aria-label="Yarım üretimi sil"
          @click="removeUnfinished(job)"
        >
          <Trash2 :size="14" />
        </button>
      </div>

      <button
        v-if="library.unfinished.length > RESUME_GORUNUR"
        type="button"
        class="resume-more"
        @click="resumeAcik = !resumeAcik"
      >
        <ChevronDown :size="14" :class="{ acik: resumeAcik }" />
        {{
          resumeAcik
            ? "Daralt"
            : `${library.unfinished.length - RESUME_GORUNUR} tane daha göster`
        }}
      </button>
    </section>

    <template v-if="library.items.length > 0">
      <div v-if="library.loadError" class="load-error">{{ library.loadError }}</div>

      <div class="toolbar">
        <RvTextField v-model="library.query" placeholder="Başlık veya kaynak ara" class="search">
          <template #leading><Search :size="14" /></template>
        </RvTextField>
        <RvSelect v-model="library.subredditFilter" :options="library.subredditOptions" class="filter" />
        <RvSelect v-model="library.rangeFilter" :options="rangeOptions" class="filter" />
        <span class="count">
          {{ library.items.length }} video · {{ library.totalSizeGb.toFixed(1).replace(".", ",") }} GB
        </span>
      </div>

      <div class="split">
        <div class="grid rv-scroll">
          <button
            v-for="item in library.filtered"
            :key="item.id"
            type="button"
            class="card"
            data-component="LibraryCard"
            @click="selectItem(item.id)"
          >
            <div
              class="thumb"
              :class="{ on: library.selectedId === item.id }"
              :style="
                library.thumbs[item.id]
                  ? {
                      backgroundImage: `url(${library.thumbs[item.id]})`,
                      backgroundSize: 'cover',
                      backgroundPosition: 'center',
                    }
                  : undefined
              "
            >
              <span class="duration rv-mono">{{ library.formatDuration(item.durationSec) }}</span>
            </div>
            <div class="card-info">
              <span class="card-title" :class="{ on: library.selectedId === item.id }">
                {{ item.title }}
              </span>
              <div class="card-meta">
                <span class="sub">{{ sourceLabel(item.source, item.subreddit) }}</span>
                <span>{{ item.createdAt }}</span>
                <span>· {{ item.sizeMb }} MB</span>
              </div>
            </div>
          </button>
        </div>

        <aside v-if="library.selected" class="detail rv-scroll">
          <div class="detail-inner">
            <div class="detail-head">
              <span class="label">Ayrıntılar</span>
              <button type="button" class="close" @click="library.selectedId = null">
                <X :size="14" />
              </button>
            </div>

            <div
              class="player"
              :style="
                !playing && library.selected && library.thumbs[library.selected.id]
                  ? {
                      backgroundImage: `url(${library.thumbs[library.selected.id]})`,
                      backgroundSize: 'cover',
                      backgroundPosition: 'center',
                    }
                  : undefined
              "
            >
              <video
                v-if="playing && videoSrc"
                :src="videoSrc"
                class="video"
                controls
                autoplay
              ></video>
              <button
                v-else
                type="button"
                class="play-badge"
                :disabled="!videoSrc"
                :title="videoSrc ? 'Oynat' : 'Video bulunamadı'"
                @click="playing = true"
              >
                <Play :size="16" />
              </button>
            </div>

            <div class="detail-title">{{ library.selected.title }}</div>

            <div class="fields">
              <div class="field">
                <span class="field-label">Kaynak</span>
                <a class="rv-mono field-link" href="#">{{ library.selected.sourceUrl }}</a>
              </div>
              <div class="field">
                <span class="field-label">Ses</span>
                <span class="field-value">{{ library.selected.voice }}</span>
              </div>
              <div class="field">
                <span class="field-label">Arka plan</span>
                <span class="field-value">{{ library.selected.background }}</span>
              </div>
              <div class="field">
                <span class="field-label">Çıktı</span>
                <span class="field-value rv-mono small">1080×1920 · 30 fps</span>
              </div>
              <div class="field">
                <span class="field-label">Boyut</span>
                <span class="field-value">
                  {{ library.selected.sizeMb }} MB ·
                  {{ library.formatDuration(library.selected.durationSec) }}
                </span>
              </div>
            </div>

            <div class="actions">
              <button type="button" class="action" @click="library.reveal(library.selected!.path)">
                <FolderOpen :size="15" class="action-icon" />
                Klasörde göster
              </button>
              <button
                type="button"
                class="action"
                @click="regenerate(library.selected!.sourceUrl)"
              >
                <RotateCw :size="15" class="action-icon" />
                Yeniden üret
              </button>
              <button
                type="button"
                class="action danger"
                @click="confirmDelete(library.selected!.id, library.selected!.title)"
              >
                <Trash2 :size="15" />
                Sil
              </button>
            </div>
          </div>
        </aside>
      </div>
    </template>

    <RvEmptyState
      v-else
      title="Kitaplık henüz boş"
      :description="
        library.loading
          ? 'Çıktı klasörü taranıyor…'
          : 'Ürettiğin videolar burada toplanır; buradan izleyebilir, yeniden üretebilir veya dışa aktarabilirsin.'
      "
    >
      <template #icon><Film :size="26" /></template>
      <template #actions>
        <RvButton variant="primary" @click="router.push('/yeni/kaynak')">
          <Plus :size="15" />
          İlk videonu oluştur
        </RvButton>
      </template>
    </RvEmptyState>
  </main>
</template>

<style scoped>
.resume {
  flex: none;
  margin: var(--rv-space-4) 20px 0;
  padding: var(--rv-space-3);
  border-radius: var(--rv-radius-md);
  border: 1px solid color-mix(in srgb, var(--rv-accent) 30%, transparent);
  background: color-mix(in srgb, var(--rv-accent) 7%, transparent);
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-2);
}

.resume-head {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  font-weight: 500;
}

.resume-icon {
  color: var(--rv-accent);
}

.resume-note {
  font-weight: 400;
  color: var(--rv-text-faint);
}

.resume-row {
  display: flex;
  align-items: center;
  gap: var(--rv-space-2);
  padding: 8px var(--rv-space-3);
  border-radius: var(--rv-radius-sm);
  border: 1px solid var(--rv-border);
  background: var(--rv-bg-surface);
}

.resume-count {
  font-size: 11px;
  padding: 0 6px;
  border-radius: var(--rv-radius-pill);
  background: color-mix(in srgb, var(--rv-accent) 18%, transparent);
  color: var(--rv-accent-quiet);
}

.resume-clear,
.resume-more,
.resume-delete {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background: none;
  border: none;
  color: var(--rv-text-faint);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

.resume-clear {
  margin-left: auto;
}

.resume-clear:hover,
.resume-delete:hover {
  color: var(--rv-danger);
}

.resume-delete {
  flex: none;
  padding: 6px;
  border-radius: var(--rv-radius-sm);
}

.resume-more {
  align-self: flex-start;
  padding: 2px 0;
}

.resume-more:hover {
  color: var(--rv-text);
}

.resume-more .acik {
  transform: rotate(180deg);
}

.resume-row:hover {
  border-color: var(--rv-accent);
}

.resume-action {
  background: none;
  font: inherit;
  cursor: pointer;
}

.resume-body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  background: none;
  border: none;
  padding: 0;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.resume-title {
  line-height: 1.4;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.resume-meta {
  display: flex;
  gap: 12px;
  font-size: 11px;
  color: var(--rv-text-faint);
}

.resume-action {
  flex: none;
  font-size: 12px;
  padding: 4px 10px;
  border-radius: var(--rv-radius-sm);
  border: 1px solid var(--rv-border);
  color: var(--rv-text-faint);
}

.resume-row:hover .resume-action {
  color: var(--rv-accent-quiet);
  border-color: var(--rv-accent);
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
  gap: var(--rv-space-2);
  height: 48px;
  padding: 0 20px;
  border-bottom: 1px solid var(--rv-border);
}

.search {
  width: 260px;
}

.search :deep(.wrap) {
  height: 30px;
}

.filter {
  height: 30px;
  background: var(--rv-bg-elevated);
  font-size: 12px;
}

.count {
  margin-left: auto;
  font-size: 12px;
  color: var(--rv-text-faint);
}

.split {
  flex: 1;
  display: flex;
  min-height: 0;
}

.grid {
  flex: 1;
  min-width: 0;
  padding: 20px;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: var(--rv-space-4);
  align-content: start;
}

.card {
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-2);
  background: none;
  border: none;
  padding: 0;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.thumb {
  aspect-ratio: 9 / 16;
  border-radius: var(--rv-radius-md);
  border: 1px solid var(--rv-border);
  background: repeating-linear-gradient(135deg, #242b3a 0 6px, #1e2431 6px 12px);
  position: relative;
  overflow: hidden;
}

.thumb.on {
  border: 2px solid var(--rv-accent);
  background: repeating-linear-gradient(135deg, #2a3140 0 6px, #232937 6px 12px);
}

.duration {
  position: absolute;
  right: 6px;
  bottom: 6px;
  padding: 2px 6px;
  border-radius: 4px;
  background: rgba(10, 11, 15, 0.75);
  font-size: 10px;
  color: #e8eaf0;
}

.card-info {
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-1);
}

.card-title {
  font-size: 12px;
  line-height: 1.4;
}

.card-title.on {
  font-weight: 500;
}

.card-meta {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--rv-text-faint);
}

.sub {
  padding: 1px 6px;
  border-radius: var(--rv-radius-pill);
  background: var(--rv-bg-elevated);
  border: 1px solid var(--rv-border);
}

/* Ayrıntı paneli */
.detail {
  width: 320px;
  flex: none;
  border-left: 1px solid var(--rv-border);
  background: var(--rv-bg-surface);
}

.detail-inner {
  padding: var(--rv-space-4);
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.detail-head {
  display: flex;
  align-items: center;
}

.label {
  font-size: 11px;
  font-weight: 500;
  color: var(--rv-text-faint);
  letter-spacing: 0.02em;
  text-transform: uppercase;
}

.close {
  margin-left: auto;
  background: none;
  border: none;
  color: var(--rv-text-faint);
  cursor: pointer;
  padding: 0;
}

.close:hover {
  color: var(--rv-text);
}

.player {
  width: auto;
  aspect-ratio: 9 / 16;
  height: 300px;
  max-width: 100%;
  border-radius: var(--rv-radius-md);
  border: 1px solid var(--rv-border-strong);
  background: repeating-linear-gradient(135deg, #2a3140 0 8px, #232937 8px 16px);
  align-self: center;
  display: grid;
  place-items: center;
  overflow: hidden;
}

.video {
  width: 100%;
  height: 100%;
  /* Kaynak zaten 9:16; kutuyu tam dolduruyor ve oran korunuyor. */
  object-fit: cover;
  background: #000;
  display: block;
}

.load-error {
  margin: var(--rv-space-3) 20px 0;
  padding: 10px var(--rv-space-3);
  border-radius: var(--rv-radius-sm);
  background: var(--rv-danger-soft);
  border: 1px solid color-mix(in srgb, var(--rv-danger) 30%, transparent);
  font-size: 12px;
  color: var(--rv-text-muted);
}

.play-badge {
  width: 38px;
  height: 38px;
  display: grid;
  place-items: center;
  border-radius: var(--rv-radius-pill);
  background: rgba(10, 11, 15, 0.6);
  border: 1px solid rgba(255, 255, 255, 0.14);
  color: #fff;
  cursor: pointer;
}

.play-badge:hover:not(:disabled) {
  background: rgba(10, 11, 15, 0.85);
}

.play-badge:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.detail-title {
  font-size: 14px;
  font-weight: 600;
  line-height: 1.4;
}

.fields {
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-2);
  font-size: 12px;
}

.field {
  display: flex;
  gap: var(--rv-space-3);
}

.field-label {
  color: var(--rv-text-faint);
  width: 88px;
  flex: none;
}

.field-value {
  color: var(--rv-text-muted);
}

.field-link {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11px;
}

.small {
  font-size: 11px;
}

.actions {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding-top: 6px;
  border-top: 1px solid var(--rv-border);
}

.action {
  display: flex;
  align-items: center;
  gap: var(--rv-space-2);
  height: 32px;
  padding: 0 10px;
  border-radius: var(--rv-radius-sm);
  background: var(--rv-bg-elevated);
  border: 1px solid var(--rv-border);
  color: var(--rv-text);
  font: inherit;
  font-weight: 500;
  cursor: pointer;
}

.action:hover {
  background: var(--rv-elevated-hover);
}

.action-icon {
  color: var(--rv-text-muted);
}

.action.danger {
  background: transparent;
  border-color: transparent;
  color: var(--rv-danger);
}

.action.danger:hover {
  background: var(--rv-danger-soft);
}
</style>
