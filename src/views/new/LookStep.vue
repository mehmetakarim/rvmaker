<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import {
  Check,
  FolderPlus,
  Music,
  Clapperboard,
  SkipBack,
  SkipForward,
  Play,
  Pause,
} from "@lucide/vue";
import { useDraftStore } from "@/stores/draft";
import { useJobsStore } from "@/stores/jobs";
import { useSettingsStore } from "@/stores/settings";
import { isTauri } from "@/lib/api";
import StepProgress from "@/components/app/StepProgress.vue";
import PhonePreview from "@/components/app/PhonePreview.vue";
import RvButton from "@/components/ui/RvButton.vue";
import RvSegmented from "@/components/ui/RvSegmented.vue";
import RvSelect from "@/components/ui/RvSelect.vue";
import RvSlider from "@/components/ui/RvSlider.vue";
import RvTooltip from "@/components/ui/RvTooltip.vue";

const draft = useDraftStore();
const jobs = useJobsStore();
const settings = useSettingsStore();
const router = useRouter();

onMounted(() => {
  draft.loadBackgrounds();
  draft.loadMusic();
});

/** Yerel dosya seçtirir. Tauri dışında dosya seçici yok. */
async function pickLocalBackground() {
  if (!isTauri()) return;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({
    multiple: false,
    filters: [{ name: "Video", extensions: ["mp4", "mov", "mkv", "webm"] }],
  });
  if (typeof picked === "string") draft.addLocalBackground(picked);
}

const previewIndex = ref(1);

const audioOptions = computed(() =>
  draft.backgroundAudios.map((a) => ({ value: a.id, label: a.label })),
);

const previewComment = computed(
  () => draft.selectedComments[previewIndex.value] ?? draft.selectedComments[0],
);

const volumePercent = computed({
  get: () => Math.round(draft.look.audioVolume * 100),
  set: (v: number) => {
    draft.look.audioVolume = v / 100;
  },
});

async function start() {
  router.push("/render");
  // Render ekranı açıldıktan sonra başlat ki ilerleme canlı görünsün.
  await jobs.runJob();
}

/** Kartları ses süreleriyle orantılı biçimde sırayla gezer. */
const oynatiliyor = ref(false);
let onizlemeZamanlayici: number | null = null;

function onizlemeyiDurdur() {
  oynatiliyor.value = false;
  if (onizlemeZamanlayici !== null) {
    window.clearTimeout(onizlemeZamanlayici);
    onizlemeZamanlayici = null;
  }
}

function siradakiKart() {
  const count = draft.selectedComments.length;
  if (count === 0) return onizlemeyiDurdur();

  previewIndex.value = (previewIndex.value + 1) % count;
  const metin = draft.selectedComments[previewIndex.value]?.translated ?? "";
  const saniye = Math.max(1.5, draft.estimateSeconds(metin) / draft.speed);
  onizlemeZamanlayici = window.setTimeout(siradakiKart, saniye * 1000);
}

function onizlemeyiOynat() {
  if (oynatiliyor.value) return onizlemeyiDurdur();
  oynatiliyor.value = true;
  siradakiKart();
}

onBeforeUnmount(onizlemeyiDurdur);

function step(delta: number) {
  onizlemeyiDurdur();
  const count = draft.selectedComments.length;
  // Hiç kart yokken modülo sıfıra bölünür ve previewIndex NaN olur.
  if (count === 0) {
    previewIndex.value = 0;
    return;
  }
  previewIndex.value = (previewIndex.value + delta + count) % count;
}
</script>

<template>
  <StepProgress :current="3">
    <span v-if="!draft.canProduce" class="block-reason">{{ draft.blockReason }}</span>
    <RvButton variant="primary" :disabled="!draft.canProduce" @click="start">
      <Clapperboard :size="14" />
      Oluştur
    </RvButton>
  </StepProgress>

  <div class="split">
    <div class="settings rv-scroll">
      <!-- Arka plan videosu -->
      <section>
        <div class="section-head">
          <span class="label">Arka plan videosu</span>
          <span class="note">
            {{ draft.backgroundsLoading ? "taranıyor…" : "döngü · sessiz" }}
          </span>
        </div>
        <div v-if="draft.backgroundVideos.length === 0 && !draft.backgroundsLoading" class="empty-note">
          <span>
            Arka plan kitaplığı boş — video düz zemin üzerine üretilir. Aşağıdan yerel bir dosya
            seçebilir ya da Ayarlar'dan klasörü değiştirebilirsin.
          </span>
          <span v-if="settings.backgroundsDir" class="rv-mono path-note">{{ settings.backgroundsDir }}</span>
        </div>

        <div class="tiles">
          <button
            v-for="bg in draft.backgroundVideos"
            :key="bg.id"
            type="button"
            class="tile"
            data-component="BackgroundTile"
            @click="draft.look.backgroundVideoId = bg.id"
          >
            <div
              class="thumb"
              :class="{ on: draft.look.backgroundVideoId === bg.id }"
              :style="
                draft.backgroundThumbs[bg.id]
                  ? { backgroundImage: `url(${draft.backgroundThumbs[bg.id]})`, backgroundSize: 'cover', backgroundPosition: 'center' }
                  : undefined
              "
            >
              <span v-if="draft.look.backgroundVideoId === bg.id" class="tick">
                <Check :size="11" />
              </span>
            </div>
            <span class="tile-label" :class="{ on: draft.look.backgroundVideoId === bg.id }">
              {{ bg.label }}
            </span>
            <span class="tile-detail">{{ bg.detail }}</span>
          </button>

          <button type="button" class="tile" @click="pickLocalBackground">
            <div class="thumb add"><FolderPlus :size="18" /></div>
            <span class="tile-label">Yerel dosya</span>
          </button>
        </div>
      </section>

      <!-- Müzik -->
      <section class="music-card">
        <div class="music-head">
          <Music :size="15" class="music-icon" />
          <span class="label">Arka plan müziği</span>
          <RvSelect v-model="draft.look.backgroundAudioId" :options="audioOptions" class="music-select" />
        </div>
        <div class="slider-row">
          <span class="slider-label">Ses seviyesi</span>
          <RvSlider v-model="volumePercent" :min="0" :max="100" />
          <span class="slider-value rv-tabular">%{{ volumePercent }}</span>
        </div>
      </section>

      <!-- Kart -->
      <section>
        <span class="label">Yorum kartı</span>

        <div class="row">
          <span class="row-label">Tema</span>
          <RvSegmented
            :model-value="draft.look.cardTheme"
            :options="[
              { value: 'dark', label: 'Koyu' },
              { value: 'light', label: 'Açık' },
              { value: 'transparent', label: 'Saydam' },
            ]"
            @update:model-value="draft.look.cardTheme = $event as typeof draft.look.cardTheme"
          />
        </div>

        <div class="row">
          <span class="row-label">Yazı boyutu</span>
          <RvSlider v-model="draft.look.fontSize" :min="24" :max="64" class="narrow" />
          <span class="slider-value rv-tabular">{{ draft.look.fontSize }} px</span>
        </div>

        <div class="row">
          <span class="row-label">Kart genişliği</span>
          <RvSlider v-model="draft.look.cardWidth" :min="60" :max="100" class="narrow" />
          <span class="slider-value rv-tabular">%{{ draft.look.cardWidth }}</span>
        </div>

      </section>
    </div>

    <!-- Canlı önizleme -->
    <aside class="preview" data-component="PhonePreview">
      <div class="preview-head">
        <span>Canlı önizleme</span>
        <span class="rv-mono">1080 × 1920</span>
      </div>

      <PhonePreview
        :author="previewComment?.author ?? ''"
        :upvotes="`${(previewComment?.upvotes ?? 0) / 1000} B`.replace('.', ',')"
        :text="previewComment?.translated ?? ''"
        :theme="draft.look.cardTheme"
        :font-size="draft.look.fontSize"
        :card-width="draft.look.cardWidth"
      />

      <div class="controls">
        <RvTooltip text="Önceki kart">
          <button type="button" class="ctl" aria-label="Önceki kart" @click="step(-1)">
            <SkipBack :size="14" />
          </button>
        </RvTooltip>
        <button
          type="button"
          class="ctl play"
          :aria-label="oynatiliyor ? 'Duraklat' : 'Oynat'"
          @click="onizlemeyiOynat"
        >
          <Pause v-if="oynatiliyor" :size="15" />
          <Play v-else :size="15" />
        </button>
        <RvTooltip text="Sonraki kart">
          <button type="button" class="ctl" aria-label="Sonraki kart" @click="step(1)">
            <SkipForward :size="14" />
          </button>
        </RvTooltip>
        <span class="counter">
          Kart {{ previewIndex + 1 }} / {{ draft.selectedComments.length }}
        </span>
      </div>
    </aside>
  </div>
</template>

<style scoped>
.block-reason {
  font-size: 12px;
  color: var(--rv-danger);
  margin-right: var(--rv-space-3);
}

.split {
  flex: 1;
  display: flex;
  min-height: 0;
}

.settings {
  flex: 1;
  min-width: 0;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 22px;
}

section {
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-3);
}

.section-head {
  display: flex;
  align-items: center;
}

.label {
  font-weight: 500;
}

.note {
  margin-left: auto;
  font-size: 12px;
  color: var(--rv-text-faint);
}

.tiles {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 10px;
}

.tile {
  display: flex;
  flex-direction: column;
  gap: 6px;
  background: none;
  border: none;
  padding: 0;
  color: inherit;
  font: inherit;
  cursor: pointer;
  text-align: left;
}

.thumb {
  aspect-ratio: 9 / 16;
  border-radius: var(--rv-radius-sm);
  border: 1px solid var(--rv-border);
  background: repeating-linear-gradient(135deg, #232937 0 6px, #1d2230 6px 12px);
  position: relative;
}

.thumb.on {
  border: 2px solid var(--rv-accent);
  background: repeating-linear-gradient(135deg, #2a3140 0 6px, #232937 6px 12px);
}

.thumb.add {
  border: 1px dashed var(--rv-border-strong);
  background: none;
  display: grid;
  place-items: center;
  color: var(--rv-text-faint);
}

.tick {
  position: absolute;
  right: 5px;
  top: 5px;
  width: 16px;
  height: 16px;
  display: grid;
  place-items: center;
  border-radius: var(--rv-radius-pill);
  background: var(--rv-accent);
  color: #fff;
}

.empty-note {
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-1);
  padding: 10px var(--rv-space-3);
  border-radius: var(--rv-radius-sm);
  background: var(--rv-warning-soft);
  border: 1px solid color-mix(in srgb, var(--rv-warning) 28%, transparent);
  font-size: 12px;
  color: var(--rv-text-muted);
  line-height: 1.5;
}

.path-note {
  font-size: 11px;
  color: var(--rv-text-faint);
  overflow: hidden;
  text-overflow: ellipsis;
}

.tile-detail {
  font-size: 11px;
  color: var(--rv-text-faint);
}

.tile-label {
  font-size: 12px;
  color: var(--rv-text-muted);
}

.tile-label.on {
  color: var(--rv-text);
}

.music-card {
  padding: 14px;
  border-radius: var(--rv-radius-md);
  background: var(--rv-bg-surface);
  border: 1px solid var(--rv-border);
}

.music-head {
  display: flex;
  align-items: center;
  gap: 10px;
}

.music-icon {
  color: var(--rv-text-faint);
}

.music-select {
  margin-left: auto;
  height: 28px;
  width: 160px;
}

.row {
  display: flex;
  align-items: center;
  gap: var(--rv-space-3);
}

.row-label {
  font-size: 12px;
  color: var(--rv-text-muted);
  width: 96px;
  flex: none;
}

.slider-row {
  display: flex;
  align-items: center;
  gap: var(--rv-space-3);
  margin-top: var(--rv-space-3);
}

.slider-label {
  font-size: 12px;
  color: var(--rv-text-muted);
  width: 78px;
  flex: none;
}

.slider-value {
  font-size: 12px;
  color: var(--rv-text-muted);
  min-width: 44px;
  text-align: right;
}

.narrow {
  max-width: 280px;
}

/* Önizleme paneli */
.preview {
  width: 400px;
  flex: none;
  border-left: 1px solid var(--rv-border);
  background: var(--rv-bg-surface);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  padding: 20px;
}

.preview-head {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12px;
  color: var(--rv-text-faint);
}

.controls {
  display: flex;
  align-items: center;
  gap: var(--rv-space-2);
}

.ctl {
  width: 30px;
  height: 30px;
  display: grid;
  place-items: center;
  border-radius: var(--rv-radius-sm);
  background: var(--rv-bg-elevated);
  border: 1px solid var(--rv-border);
  color: var(--rv-text-muted);
  cursor: pointer;
}

.ctl:hover {
  color: var(--rv-text);
}

.ctl.play {
  width: 34px;
  height: 34px;
  border-radius: var(--rv-radius-pill);
  background: var(--rv-accent);
  border: none;
  color: #fff;
}

.counter {
  margin-left: var(--rv-space-2);
  font-size: 11px;
  color: var(--rv-text-faint);
}
</style>
