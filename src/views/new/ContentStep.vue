<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import {
  GripVertical,
  Clock,
  Pencil,
  Languages,
  CircleAlert,
  Plus,
  X,
  TriangleAlert,
} from "@lucide/vue";
import { useDraftStore } from "@/stores/draft";
import { useSettingsStore } from "@/stores/settings";
import StepProgress from "@/components/app/StepProgress.vue";
import RvButton from "@/components/ui/RvButton.vue";
import RvCheckbox from "@/components/ui/RvCheckbox.vue";
import RvSelect from "@/components/ui/RvSelect.vue";
import RvSlider from "@/components/ui/RvSlider.vue";
import RvProgressBar from "@/components/ui/RvProgressBar.vue";
import RvTooltip from "@/components/ui/RvTooltip.vue";

const draft = useDraftStore();
const settings = useSettingsStore();
const router = useRouter();

// Gönderi çekildikten sonra çeviri bu ekranda, satır satır görünerek yapılır.
onMounted(() => {
  if (draft.needsTranslation) {
    draft.translateAll(settings.translation.targetLang);
  }
});

// Sıralama seçimi anında uygulanır.
watch(() => draft.sortMode, () => draft.applySort());

/** Reddit her zaman "top" ile çekiliyor; bu seçenekler yerel sıralama yapar. */
/** Thread'de sıra anlam taşıyor: 1/6, 2/6… karıştırılırsa video anlamsızlaşır,
 *  o yüzden orada tek seçenek bırakıyoruz. */
const sortOptions = computed(() => {
  if (draft.source === "x" && draft.xMode === "thread") {
    return [{ value: "fetched", label: "Zincir sırası" }];
  }
  return [
    { value: "top", label: `En çok ${draft.scoreWord} alan` },
    { value: "fetched", label: "Geldiği sıra" },
    { value: "short", label: "Kısadan uzuna" },
    { value: "long", label: "Uzundan kısaya" },
  ];
});

const yeniYorum = ref("");
const yorumEkleniyor = ref(false);

function yorumEkle() {
  draft.addManualComment(yeniYorum.value);
  yeniYorum.value = "";
  yorumEkleniyor.value = false;
}

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
  draft.moveComment(draft.comments[kaynak].id, hedef - kaynak);
}

const durationLabel = computed(() => {
  const s = draft.estimatedSeconds;
  return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
});

const durationRatio = computed(() => draft.estimatedSeconds / draft.targetSeconds);

function formatCount(n: number) {
  return n >= 1000 ? `${(n / 1000).toFixed(1).replace(".", ",")} B` : String(n);
}
</script>

<template>
  <StepProgress :current="1" />

  <div class="split">
    <!-- Gönderi özeti -->
    <aside class="post rv-scroll">
      <div class="meta">
        <span class="sub">{{ draft.sourceHandle }}</span>
        <span>{{ draft.post.postedAgo }}</span>
      </div>
      <div
        v-if="draft.post.removalNote || draft.blockReason"
        class="warn-strip"
        data-component="SourceWarning"
      >
        <TriangleAlert :size="14" class="warn-icon" />
        <span>{{ draft.post.removalNote || draft.blockReason }}</span>
      </div>

      <div class="title">{{ draft.post.titleTr || draft.post.title }}</div>
      <div v-if="draft.post.titleTr" class="title-src">Özgün: “{{ draft.post.title }}”</div>

      <div class="stats">
        <div class="stat">
          <span class="rv-tabular">{{ formatCount(draft.post.upvotes) }}</span>
          <span class="stat-label">{{ draft.scoreWord }}</span>
        </div>
        <div class="stat">
          <span class="rv-tabular">{{ draft.post.commentCount.toLocaleString("tr-TR") }}</span>
          <span class="stat-label">{{ draft.replyWord }}</span>
        </div>
        <div class="stat">
          <span class="rv-tabular">{{ draft.post.fetchedCount }}</span>
          <span class="stat-label">getirildi</span>
        </div>
      </div>

      <div class="control">
        <label>Sıralama</label>
        <RvSelect v-model="draft.sortMode" :options="sortOptions" />
      </div>

      <div class="control">
        <label>Maksimum {{ draft.replyWord }}</label>
        <div class="slider-row">
          <RvSlider v-model="draft.maxComments" :min="1" :max="25" />
          <span class="rv-tabular slider-value">{{ draft.maxComments }}</span>
        </div>
      </div>
    </aside>

    <!-- Yorum listesi -->
    <div class="list-pane">
      <div class="summary" data-component="DurationSummary">
        <span class="est">
          <Clock :size="15" class="clock" />
          Tahmini süre
          <span class="rv-tabular">{{ durationLabel }}</span>
        </span>
        <span v-if="draft.isTranslating" class="sub-note translating">
          <Languages :size="13" />
          Çevriliyor · {{ draft.translateDone }} / {{ draft.translateTotal }}
        </span>
        <span v-else class="sub-note">
          {{ draft.selectedComments.length }} {{ draft.replyWord }} seçili · {{ draft.totalChars }} karakter
        </span>
        <RvProgressBar :value="durationRatio" class="summary-bar" />
        <span class="sub-note">hedef {{ draft.targetSeconds }} sn</span>
        <RvButton
          variant="primary"
          class="go"
          :disabled="draft.isTranslating"
          @click="router.push('/yeni/ses')"
        >
          Ses ayarlarına geç
        </RvButton>
      </div>

      <div v-if="draft.translateError" class="translate-error">
        <CircleAlert :size="14" class="translate-error-icon" />
        <span>
          Çeviri tamamlanamadı ({{ draft.translateError }}). Yorumlar özgün diliyle bırakıldı;
          satırları elle düzenleyebilirsin.
        </span>
      </div>

      <div class="rows rv-scroll">
        <div
          v-for="(comment, index) in draft.comments"
          :key="comment.id"
          class="row"
          :class="{
            off: !comment.selected,
            editing: draft.editingId === comment.id,
            dragging: surukleneninIndeksi === index,
            dropTarget: uzerindekiIndeks === index && surukleneninIndeksi !== index,
          }"
          data-component="CommentRow"
          draggable="true"
          @dragstart="surukleBasla(index)"
          @dragover="uzerinde(index, $event)"
          @drop="birak(index)"
          @dragend="surukleneninIndeksi = null; uzerindekiIndeks = null"
        >
          <GripVertical :size="14" class="grip" />
          <RvCheckbox
            :model-value="comment.selected"
            class="check"
            @update:model-value="draft.toggleComment(comment.id)"
          />

          <div class="body">
            <div class="byline">
              <span>{{ comment.author }}</span>
              <template v-if="!comment.manual">
                <span>·</span>
                <span>{{ formatCount(comment.upvotes) }} {{ draft.scoreWord }}</span>
              </template>
              <span v-else class="pill">elle eklendi</span>
              <span v-if="draft.editingId === comment.id" class="pill">düzenleniyor</span>
            </div>

            <div v-if="!comment.manual" class="original">{{ comment.original }}</div>

            <textarea
              v-if="draft.editingId === comment.id"
              v-model="draft.editBuffer"
              class="editor"
              rows="2"
            ></textarea>
            <div v-else-if="comment.translated" class="translated" @dblclick="draft.startEdit(comment.id)">
              {{ comment.translated }}
            </div>
            <div v-else class="pending">Çeviri bekleniyor…</div>

            <div v-if="draft.editingId === comment.id" class="edit-actions">
              <RvButton variant="primary" size="sm" @click="draft.saveEdit()">Kaydet</RvButton>
              <RvButton variant="secondary" size="sm" @click="draft.cancelEdit()">Vazgeç</RvButton>
              <RvButton variant="ghost" size="sm">Çeviriyi yenile</RvButton>
            </div>
          </div>

          <div class="side rv-tabular">
            <span>{{ comment.translated.length }} krk</span>
            <span>~{{ draft.estimateSeconds(comment.translated) }} sn</span>
            <RvTooltip
              v-if="draft.editingId !== comment.id && comment.translated"
              text="Metni düzenle"
            >
              <button type="button" class="edit" @click="draft.startEdit(comment.id)">
                <Pencil :size="13" />
              </button>
            </RvTooltip>
            <RvTooltip v-if="comment.manual" text="Yorumu kaldır">
              <button type="button" class="edit" @click="draft.removeComment(comment.id)">
                <X :size="13" />
              </button>
            </RvTooltip>
          </div>
        </div>

          <div v-if="yorumEkleniyor" class="add-box">
            <textarea
              v-model="yeniYorum"
              class="editor"
              rows="3"
              placeholder="Kendi metnini yaz — çeviriden geçmez, olduğu gibi seslendirilir."
              autofocus
            ></textarea>
            <div class="edit-actions">
              <RvButton
                variant="primary"
                size="sm"
                :disabled="!yeniYorum.trim()"
                @click="yorumEkle"
              >
                Ekle
              </RvButton>
              <RvButton variant="secondary" size="sm" @click="yorumEkleniyor = false">
                Vazgeç
              </RvButton>
            </div>
          </div>

          <button v-else type="button" class="add-row" @click="yorumEkleniyor = true">
            <Plus :size="14" />
            Kendi metnini ekle
          </button>
        </div>
      </div>
    </div>
</template>

<style scoped>
.warn-strip {
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

.warn-icon {
  color: var(--rv-danger);
  flex: none;
  margin-top: 1px;
}

.split {
  flex: 1;
  display: flex;
  min-height: 0;
}

/* Gönderi paneli */
.post {
  width: 300px;
  flex: none;
  border-right: 1px solid var(--rv-border);
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-4);
}

.meta {
  display: flex;
  align-items: center;
  gap: var(--rv-space-2);
  font-size: 12px;
  color: var(--rv-text-faint);
}

.sub {
  padding: 2px 7px;
  border-radius: var(--rv-radius-pill);
  background: var(--rv-bg-elevated);
  border: 1px solid var(--rv-border);
  color: var(--rv-text-muted);
}

.title {
  font-size: 15px;
  font-weight: 600;
  line-height: 1.45;
  text-wrap: pretty;
}

.title-src {
  font-size: 12px;
  color: var(--rv-text-muted);
  line-height: 1.6;
}

.stats {
  display: flex;
  gap: var(--rv-space-4);
  padding: var(--rv-space-3) 0;
  border-top: 1px solid var(--rv-border);
  border-bottom: 1px solid var(--rv-border);
}

.stat {
  display: flex;
  flex-direction: column;
  gap: 2px;
  font-size: 15px;
  font-weight: 600;
}

.stat-label {
  font-size: 11px;
  font-weight: 400;
  color: var(--rv-text-faint);
}

.control {
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-2);
}

.control label {
  font-size: 12px;
  color: var(--rv-text-muted);
}

.slider-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.slider-value {
  font-size: 12px;
  color: var(--rv-text-muted);
  width: 20px;
  text-align: right;
}

/* Liste */
.list-pane {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.summary {
  flex: none;
  display: flex;
  align-items: center;
  gap: 14px;
  height: 48px;
  padding: 0 20px;
  border-bottom: 1px solid var(--rv-border);
  background: var(--rv-bg-surface);
}

.est {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-weight: 500;
}

.clock {
  color: var(--rv-accent-quiet);
}

.sub-note {
  font-size: 12px;
  color: var(--rv-text-faint);
}

.summary-bar {
  flex: 1;
  max-width: 220px;
}

.go {
  margin-left: auto;
}

.rows {
  flex: 1;
  padding: var(--rv-space-3) 20px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.row {
  display: flex;
  gap: 10px;
  padding: var(--rv-space-3);
  border-radius: var(--rv-radius-md);
  background: var(--rv-bg-surface);
  border: 1px solid var(--rv-border);
}

.row.off {
  background: transparent;
  border-color: transparent;
  opacity: 0.55;
}

.row.dragging {
  opacity: 0.4;
}

.row.dropTarget {
  border-color: var(--rv-accent);
  box-shadow: inset 0 2px 0 var(--rv-accent);
}

.add-row {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--rv-space-2);
  padding: var(--rv-space-3);
  border-radius: var(--rv-radius-md);
  border: 1px dashed var(--rv-border-strong);
  background: transparent;
  color: var(--rv-text-faint);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

.add-row:hover {
  color: var(--rv-text);
  border-color: var(--rv-accent);
}

.add-box {
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-2);
  padding: var(--rv-space-3);
  border-radius: var(--rv-radius-md);
  background: var(--rv-bg-surface);
  border: 1px solid var(--rv-accent);
}

.row.editing {
  border-color: var(--rv-accent);
  box-shadow: var(--rv-focus);
}

.grip {
  color: var(--rv-text-faint);
  margin-top: 2px;
  flex: none;
  cursor: grab;
}

.check {
  margin-top: 1px;
}

.body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.byline {
  display: flex;
  align-items: center;
  gap: var(--rv-space-2);
  font-size: 11px;
  color: var(--rv-text-faint);
}

.pill {
  margin-left: 6px;
  padding: 1px 6px;
  border-radius: var(--rv-radius-pill);
  background: var(--rv-accent-soft);
  color: var(--rv-accent-quiet);
}

.original {
  font-size: 12px;
  color: var(--rv-text-faint);
  line-height: 1.5;
}

.translated {
  line-height: 1.55;
}

.pending {
  line-height: 1.55;
  color: var(--rv-text-faint);
  animation: rv-pulse 1.4s ease-in-out infinite;
}

.translating {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--rv-accent-quiet);
}

.translate-error {
  flex: none;
  display: flex;
  align-items: flex-start;
  gap: var(--rv-space-2);
  margin: var(--rv-space-3) 20px 0;
  padding: 10px var(--rv-space-3);
  border-radius: var(--rv-radius-sm);
  background: var(--rv-warning-soft);
  border: 1px solid color-mix(in srgb, var(--rv-warning) 28%, transparent);
  font-size: 12px;
  color: var(--rv-text-muted);
  line-height: 1.5;
}

.translate-error-icon {
  color: var(--rv-warning);
  flex: none;
  margin-top: 1px;
}

.editor {
  padding: 8px 10px;
  border-radius: var(--rv-radius-sm);
  background: var(--rv-bg-inset);
  border: 1px solid var(--rv-accent);
  color: var(--rv-text);
  font: inherit;
  line-height: 1.55;
  resize: vertical;
  outline: none;
}

.edit-actions {
  display: flex;
  align-items: center;
  gap: var(--rv-space-2);
}

.side {
  flex: none;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 6px;
  font-size: 11px;
  color: var(--rv-text-faint);
}

.edit {
  background: none;
  border: none;
  color: var(--rv-text-faint);
  cursor: pointer;
  padding: 0;
}

.edit:hover {
  color: var(--rv-text);
}
</style>
