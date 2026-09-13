<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import {
  Link,
  ArrowRight,
  Clipboard,
  Compass,
  Flame,
  Plus,
  Check,
  CornerDownLeft,
  CircleAlert,
  X,
} from "@lucide/vue";
import { useDraftStore } from "@/stores/draft";
import RvButton from "@/components/ui/RvButton.vue";
import RvTextField from "@/components/ui/RvTextField.vue";
import RvSegmented from "@/components/ui/RvSegmented.vue";

const draft = useDraftStore();
const router = useRouter();

const yeniAyarAdi = ref("");
const ayarKaydediliyor = ref(false);

onMounted(async () => {
  draft.loadPresets();
  draft.restoreLastSettings();
  await panodanAl();
});

/** Pencere odağa geldiğinde panoda Reddit bağlantısı varsa alanı doldurur.
 *  Tarayıcıdan kopyalayıp uygulamaya geçince alan hazır oluyor. */
async function panodanAl() {
  if (draft.url.trim()) return;
  try {
    const text = await navigator.clipboard.readText();
    if (
      /reddit\.com\/r\/[^/]+\/comments\/|redd\.it\//i.test(text) ||
      /(?:twitter|x)\.com\/[^/]+\/status(?:es)?\/\d+/i.test(text)
    ) {
      draft.url = text.trim();
    }
  } catch {
    /* pano izni yoksa sessizce geç — kullanıcı elle yapıştırabilir */
  }
}

function ayarKaydet() {
  draft.savePreset(yeniAyarAdi.value);
  yeniAyarAdi.value = "";
  ayarKaydediliyor.value = false;
}

async function submit() {
  draft.urlTouched = true;
  if (!draft.urlValid) return;

  const ok = await draft.loadSource(draft.url);
  if (ok) router.push("/yeni/icerik");
}

/** Son kullanılanlar kısaltılmış yol tutuyor; tam adresi kaynağa göre kuruyoruz. */
function useRecent(item: { url: string; source: "reddit" | "x" }) {
  draft.url =
    item.source === "x"
      ? `https://x.com/${item.url.replace(/^@/, "")}`.replace(/\/(\d+)$/, "/status/$1")
      : `https://old.reddit.com/${item.url}/`;
  draft.urlTouched = false;
}
</script>

<template>
  <div class="rv-scroll">
    <div class="column">
      <header>
        <h1>Yeni video</h1>
        <p>Bir Reddit gönderi ya da X tweet bağlantısı yapıştır, gerisini hazır ayar halletsin.</p>
      </header>

      <div class="url-row" data-component="UrlField">
        <RvTextField
          v-model="draft.url"
          size="lg"
          placeholder="reddit.com/r/AskReddit/comments/… ya da x.com/…/status/…"
          :invalid="!!draft.urlError"
          :error="draft.urlError"
          class="url-field"
          @keydown.enter="submit"
        >
          <template #leading><Link :size="18" /></template>
        </RvTextField>
        <RvButton variant="primary" size="lg" :loading="draft.isLoading" @click="submit">
          {{ draft.isLoading ? "Getiriliyor…" : "Devam et" }}
          <ArrowRight v-if="!draft.isLoading" :size="16" />
        </RvButton>
      </div>

      <!-- X bağlantısı yapıştırıldığında ne çekileceğini sormak gerekiyor;
           Reddit'te böyle bir ayrım yok. -->
      <div v-if="draft.urlKind === 'x'" class="x-mode">
        <span class="x-mode-label">Videoda ne olsun</span>
        <RvSegmented
          v-model="draft.xMode"
          :options="[
            { value: 'replies', label: 'Yanıtlar' },
            { value: 'thread', label: 'Thread' },
          ]"
        />
        <span class="x-mode-hint">
          {{
            draft.xMode === "thread"
              ? "Yazarın kendi devam tweet'leri, atıldığı sırayla"
              : "Tweet'e gelen yanıtlar"
          }}
        </span>
      </div>

      <div v-if="draft.loadError" class="fetch-error">
        <CircleAlert :size="14" class="fetch-error-icon" />
        <span>{{ draft.loadError }}</span>
      </div>

      <div class="hints">
        <button type="button" class="hint clickable" @click="panodanAl">
          <Clipboard :size="14" />
          Panodan al
          <kbd class="rv-mono">⌘V</kbd>
        </button>
        <button type="button" class="hint clickable" @click="router.push('/yeni/subreddit')">
          <Compass :size="14" />
          <span class="link-like">Bir subreddit'ten seç</span>
        </button>
        <button type="button" class="hint clickable" @click="router.push('/yeni/viral')">
          <Flame :size="14" />
          <span class="link-like">X'te viral olanlar</span>
        </button>
      </div>

      <section>
        <div class="section-title">Hazır ayarlar</div>
        <div class="presets">
          <div
            v-for="preset in draft.presets"
            :key="preset.id"
            class="preset"
            :class="{ on: draft.selectedPresetId === preset.id }"
            data-component="PresetCard"
            @click="draft.applyPreset(preset.id)"
          >
            <div class="preset-head">
              <span class="name">{{ preset.name }}</span>
              <span v-if="draft.selectedPresetId === preset.id" class="tick">
                <Check :size="11" />
              </span>
              <button
                v-else
                type="button"
                class="preset-remove"
                title="Hazır ayarı sil"
                @click.stop="draft.deletePreset(preset.id)"
              >
                <X :size="12" />
              </button>
            </div>
            <div class="summary">{{ preset.summary }}</div>
            <div class="chips">
              <span class="chip">{{ preset.commentCount }} yorum</span>
            </div>
          </div>

          <div v-if="ayarKaydediliyor" class="preset saving">
            <RvTextField
              v-model="yeniAyarAdi"
              placeholder="Hazır ayar adı"
              @keydown.enter="ayarKaydet"
            />
            <div class="preset-actions">
              <RvButton
                variant="primary"
                size="sm"
                :disabled="!yeniAyarAdi.trim()"
                @click="ayarKaydet"
              >
                Kaydet
              </RvButton>
              <RvButton variant="ghost" size="sm" @click="ayarKaydediliyor = false">
                Vazgeç
              </RvButton>
            </div>
          </div>

          <button
            v-else
            type="button"
            class="preset new"
            title="Şu anki ses ve görünüm ayarlarını adlandırarak kaydet"
            @click="ayarKaydediliyor = true"
          >
            <Plus :size="14" />
            Hazır ayar oluştur
          </button>
        </div>

        <span v-if="draft.presets.length === 0 && !ayarKaydediliyor" class="preset-note">
          Ses ve görünüm ayarlarını bir kez kurup buraya kaydedebilirsin. Son kullandığın
          ayarlar zaten otomatik hatırlanıyor.
        </span>
      </section>

      <section class="recent">
        <div class="section-title">Son kullanılanlar</div>
        <button
          v-for="item in draft.recent"
          :key="item.url"
          type="button"
          class="recent-row"
          @click="useRecent(item)"
        >
          <span class="rv-mono path">{{ item.url }}</span>
          <span class="when">{{ item.when }}</span>
          <CornerDownLeft :size="14" class="enter" />
        </button>
      </section>
    </div>
  </div>
</template>

<style scoped>
.x-mode {
  display: flex;
  align-items: center;
  gap: var(--rv-space-3);
}

.x-mode-label {
  font-size: 12px;
  color: var(--rv-text-muted);
}

.x-mode-hint {
  font-size: 11px;
  color: var(--rv-text-faint);
}

.column {
  padding: 40px 40px 0;
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-5);
  max-width: 820px;
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
}

.url-row {
  display: flex;
  gap: var(--rv-space-2);
  align-items: flex-start;
}

.url-field {
  flex: 1;
}

.fetch-error {
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

.fetch-error-icon {
  color: var(--rv-danger);
  flex: none;
  margin-top: 1px;
}

.hints {
  display: flex;
  align-items: center;
  gap: var(--rv-space-4);
  font-size: 12px;
  color: var(--rv-text-faint);
}

.hint {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

kbd {
  padding: 1px 5px;
  border: 1px solid var(--rv-border);
  border-radius: 4px;
}

section {
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-2);
}

.section-title {
  font-size: 12px;
  font-weight: 500;
  color: var(--rv-text-faint);
  letter-spacing: 0.02em;
  text-transform: uppercase;
}

.presets {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--rv-space-3);
}

.preset {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 14px;
  border-radius: var(--rv-radius-md);
  background: var(--rv-bg-surface);
  border: 1px solid var(--rv-border);
  color: var(--rv-text);
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.preset:hover {
  border-color: var(--rv-border-strong);
}

.preset.on {
  border-color: var(--rv-accent);
}

.preset.new {
  align-items: center;
  justify-content: center;
  flex-direction: row;
  gap: var(--rv-space-2);
  background: transparent;
  border: 1px dashed var(--rv-border-strong);
  color: var(--rv-text-faint);
  font-size: 12px;
}

.preset-remove {
  background: none;
  border: none;
  color: var(--rv-text-faint);
  cursor: pointer;
  padding: 0;
  display: grid;
  place-items: center;
  opacity: 0;
}

.preset:hover .preset-remove {
  opacity: 1;
}

.preset-remove:hover {
  color: var(--rv-danger);
}

.preset.saving {
  gap: var(--rv-space-2);
  border-color: var(--rv-accent);
}

.preset-actions {
  display: flex;
  gap: 6px;
}

.preset-note {
  font-size: 12px;
  color: var(--rv-text-faint);
  line-height: 1.5;
}

.clickable {
  background: none;
  border: none;
  color: inherit;
  font: inherit;
  cursor: pointer;
  padding: 0;
}

.clickable:hover {
  color: var(--rv-text);
}

.link-like {
  color: var(--rv-accent-quiet);
}

.clickable:hover .link-like {
  color: var(--rv-accent-hover);
}

.preset-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.name {
  font-weight: 500;
}

.tick {
  width: 16px;
  height: 16px;
  display: grid;
  place-items: center;
  border-radius: var(--rv-radius-pill);
  background: var(--rv-accent);
  color: #fff;
}

.summary {
  font-size: 12px;
  color: var(--rv-text-muted);
  line-height: 1.5;
}

.chips {
  display: flex;
  gap: 6px;
}

.chip {
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--rv-bg-inset);
  font-size: 11px;
  color: var(--rv-text-faint);
}

.recent {
  gap: 2px;
  padding-bottom: 40px;
}

.recent .section-title {
  margin-bottom: 6px;
}

.recent-row {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 34px;
  padding: 0 10px;
  border-radius: var(--rv-radius-sm);
  background: transparent;
  border: 1px solid transparent;
  color: inherit;
  font: inherit;
  cursor: pointer;
}

.recent-row:hover {
  background: var(--rv-bg-surface);
  border-color: var(--rv-border);
}

.path {
  font-size: 12px;
  color: var(--rv-text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.when {
  margin-left: auto;
  font-size: 11px;
  color: var(--rv-text-faint);
}

.enter {
  color: var(--rv-text-faint);
  opacity: 0;
}

.recent-row:hover .enter {
  opacity: 1;
}
</style>
