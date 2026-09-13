<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { useRouter } from "vue-router";
import { Mic, Languages, Sparkles, Gem, Monitor, Search, KeyRound } from "@lucide/vue";
import { useDraftStore } from "@/stores/draft";
import { useSettingsStore } from "@/stores/settings";
import type { TtsEngineId } from "@/lib/types";
import StepProgress from "@/components/app/StepProgress.vue";
import VoicePreviewRow from "@/components/app/VoicePreviewRow.vue";
import RvButton from "@/components/ui/RvButton.vue";
import RvSegmented from "@/components/ui/RvSegmented.vue";
import RvSlider from "@/components/ui/RvSlider.vue";
import RvTextField from "@/components/ui/RvTextField.vue";

const draft = useDraftStore();
const settings = useSettingsStore();
const router = useRouter();

const icons: Record<TtsEngineId, typeof Mic> = {
  googletranslate: Languages,
  elevenlabs: Mic,
  openai: Sparkles,
  gemini: Gem,
  system: Monitor,
};

const activeEngine = computed(() => settings.engines.find((e) => e.id === draft.engineId));

onMounted(() => draft.loadVoices(settings.translation.targetLang));

// Motor değiştiğinde o motorun gerçek sesleri çekiliyor.
watch(
  () => draft.engineId,
  () => draft.loadVoices(settings.translation.targetLang),
);

const missingKeyEngine = computed(() =>
  settings.engines.find((e) => e.requiresKey && !e.keyPresent),
);

/** Motorun ses çeşitliliği hakkında tek satırlık gerçek bilgi. */
const engineNote = computed(() => {
  if (draft.engineVoices.length === 0) return "";
  return draft.engineVoices.length === 1
    ? "Bu motor dil başına tek ses veriyor"
    : `${draft.engineVoices.length} ses`;
});

function toggleVoice(id: string) {
  draft.togglePreview(id, settings.translation.targetLang);
}
</script>

<template>
  <StepProgress :current="2">
    <RvButton variant="primary" @click="router.push('/yeni/gorunum')">Görünüme geç</RvButton>
  </StepProgress>

  <div class="split">
    <!-- Motor listesi -->
    <aside class="engines rv-scroll">
      <div class="section-label">Motor</div>
      <button
        v-for="engine in settings.engines"
        :key="engine.id"
        type="button"
        class="engine"
        :class="{ on: draft.engineId === engine.id }"
        @click="draft.engineId = engine.id"
      >
        <component :is="icons[engine.id]" :size="15" class="engine-icon" />
        {{ engine.label }}
        <span v-if="!engine.requiresKey && engine.id === 'googletranslate'" class="free">ücretsiz</span>
        <span
          v-else-if="engine.requiresKey"
          class="dot"
          :class="engine.keyPresent ? 'ok' : 'warn'"
        ></span>
      </button>

      <div v-if="missingKeyEngine" class="key-warning">
        <div class="key-text">
          <KeyRound :size="14" class="key-icon" />
          {{ missingKeyEngine.label }} için API anahtarı tanımlı değil.
        </div>
        <RouterLink to="/ayarlar/tts">Ayarlar → Seslendirme</RouterLink>
      </div>
    </aside>

    <!-- Ses listesi -->
    <div class="voices rv-scroll">
      <div class="voices-head">
        <div class="voices-title">{{ activeEngine?.label }} sesleri</div>
        <RvTextField v-model="draft.voiceSearch" placeholder="Ses ara" class="search">
          <template #leading><Search :size="13" /></template>
        </RvTextField>
        <RvSegmented
          v-if="draft.hasGenderVariants"
          v-model="draft.voiceFilter"
          class="filter"
          :options="[
            { value: 'all', label: 'Tümü' },
            { value: 'female', label: 'Kadın' },
            { value: 'male', label: 'Erkek' },
          ]"
        />
        <span v-else class="engine-note">{{ engineNote }}</span>
      </div>

      <div class="voice-list">
        <VoicePreviewRow
          v-for="voice in draft.filteredVoices"
          :key="voice.id"
          :voice="voice"
          :selected="draft.voiceId === voice.id"
          :playing="draft.playingVoiceId === voice.id"
          :played="0.42"
          elapsed="0:03"
          duration="0:08"
          @select="draft.voiceId = voice.id"
          @toggle="toggleVoice(voice.id)"
        />
        <div v-if="draft.voicesLoading" class="no-voice">Sesler yükleniyor…</div>
        <div v-else-if="draft.voicesError" class="no-voice error">
          {{ draft.voicesError }}
        </div>
        <div v-else-if="draft.filteredVoices.length === 0" class="no-voice">
          <template v-if="draft.engineVoices.length === 0">
            {{ activeEngine?.label }} için ses bulunamadı.
          </template>
          <template v-else>Bu filtreye uyan ses yok.</template>
        </div>
      </div>

      <div v-if="draft.engineId === 'gemini'" class="engine-info">
        <Info :size="14" class="info-icon" />
        <span>
          Gemini'nin ücretsiz katmanında hız sınırı model başına uygulanıyor. Uygulama üç
          model arasında geçiş yapıyor; hepsi dolarsa <strong>20 saniye bekleyip</strong>
          yeniden deniyor, bu yüzden uzun videolarda seslendirme duraklayabilir. Duraklama
          olursa günlükte ve bildirimde sebebini görürsün.
        </span>
      </div>

      <div v-if="draft.previewError" class="preview-error">
        <KeyRound :size="14" class="key-icon" />
        Önizleme çalınamadı: {{ draft.previewError }}
      </div>

      <div class="sliders">
        <div class="slider-block">
          <div class="slider-head">
            <span>Konuşma hızı</span>
            <span class="rv-tabular">{{ draft.speed.toFixed(2).replace(".", ",") }}×</span>
          </div>
          <RvSlider v-model="draft.speed" :min="0.75" :max="1.75" :step="0.05" />
          <div class="slider-scale"><span>0,75×</span><span>1,75×</span></div>
        </div>

        <div class="slider-block">
          <div class="slider-head">
            <span>Cümle arası sessizlik</span>
            <span class="rv-tabular">{{ draft.silenceMs }} ms</span>
          </div>
          <RvSlider v-model="draft.silenceMs" :min="0" :max="1000" :step="10" />
          <div class="slider-scale"><span>0 ms</span><span>1000 ms</span></div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.split {
  flex: 1;
  display: flex;
  min-height: 0;
}

.engines {
  width: 236px;
  flex: none;
  border-right: 1px solid var(--rv-border);
  padding: var(--rv-space-3);
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.section-label {
  font-size: 11px;
  font-weight: 500;
  color: var(--rv-text-faint);
  letter-spacing: 0.02em;
  text-transform: uppercase;
  padding: 4px 8px 8px;
}

.engine {
  display: flex;
  align-items: center;
  gap: var(--rv-space-2);
  height: 32px;
  padding: 0 var(--rv-space-2);
  border-radius: var(--rv-radius-sm);
  background: transparent;
  border: none;
  color: var(--rv-text-muted);
  font: inherit;
  font-size: 13px;
  cursor: pointer;
  text-align: left;
}

.engine:hover:not(.on) {
  background: var(--rv-bg-elevated);
  color: var(--rv-text);
}

.engine.on {
  background: var(--rv-accent-soft);
  color: var(--rv-text);
  font-weight: 500;
}

.engine.on .engine-icon {
  color: var(--rv-accent-quiet);
}

.engine-icon {
  flex: none;
}

.free {
  margin-left: auto;
  font-size: 11px;
  color: var(--rv-text-faint);
}

.dot {
  margin-left: auto;
  width: 6px;
  height: 6px;
  border-radius: var(--rv-radius-pill);
}

.dot.ok {
  background: var(--rv-success);
}

.dot.warn {
  background: var(--rv-warning);
}

.key-warning {
  margin-top: var(--rv-space-3);
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-2);
  padding: 10px;
  border-radius: var(--rv-radius-sm);
  background: var(--rv-warning-soft);
  border: 1px solid color-mix(in srgb, var(--rv-warning) 28%, transparent);
}

.key-text {
  display: flex;
  gap: var(--rv-space-2);
  font-size: 12px;
  color: var(--rv-text-muted);
  line-height: 1.5;
}

.key-icon {
  color: var(--rv-warning);
  flex: none;
  margin-top: 2px;
}

.key-warning a {
  font-size: 12px;
  font-weight: 500;
}

.voices {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  padding: 20px;
  gap: 20px;
}

.voices-head {
  display: flex;
  align-items: center;
  gap: 10px;
}

.voices-title {
  font-size: 15px;
  font-weight: 600;
}

.search {
  width: 160px;
}

.search :deep(.wrap) {
  height: 28px;
}

.filter {
  margin-left: auto;
}

.voice-list {
  /* Panel kayar, liste kendi boyunu korur. flex-shrink açıkken 36 sesin
     30'u `overflow: hidden` altında kırpılıyor ve hiç kaydırılamıyordu. */
  flex: none;
  display: flex;
  flex-direction: column;
  border-radius: var(--rv-radius-md);
  border: 1px solid var(--rv-border);
  background: var(--rv-bg-surface);
  overflow: hidden;
}

.engine-info {
  display: flex;
  align-items: flex-start;
  gap: var(--rv-space-2);
  padding: 10px var(--rv-space-3);
  border-radius: var(--rv-radius-sm);
  background: var(--rv-bg-surface);
  border: 1px solid var(--rv-border);
  font-size: 12px;
  color: var(--rv-text-muted);
  line-height: 1.55;
}

.info-icon {
  color: var(--rv-accent-quiet);
  flex: none;
  margin-top: 1px;
}

.preview-error {
  display: flex;
  align-items: flex-start;
  gap: var(--rv-space-2);
  padding: 10px var(--rv-space-3);
  border-radius: var(--rv-radius-sm);
  background: var(--rv-danger-soft);
  border: 1px solid color-mix(in srgb, var(--rv-danger) 30%, transparent);
  font-size: 12px;
  color: var(--rv-text-muted);
  line-height: 1.5;
}

.engine-note {
  margin-left: auto;
  font-size: 12px;
  color: var(--rv-text-faint);
}

.no-voice {
  padding: 20px;
  text-align: center;
  font-size: 12px;
  color: var(--rv-text-faint);
  line-height: 1.5;
}

.no-voice.error {
  color: var(--rv-danger);
}

.sliders {
  display: flex;
  gap: var(--rv-space-6);
}

.slider-block {
  flex: 1;
  max-width: 320px;
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-2);
}

.slider-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12px;
  color: var(--rv-text-muted);
}

.slider-scale {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: var(--rv-text-faint);
}
</style>
