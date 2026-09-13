<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import {
  Search,
  Heart,
  MessageSquare,
  Repeat2,
  ArrowLeft,
  CircleAlert,
  Flame,
} from "@lucide/vue";
import { useDraftStore } from "@/stores/draft";
import { relativeTime } from "@/lib/api";
import RvButton from "@/components/ui/RvButton.vue";
import RvTextField from "@/components/ui/RvTextField.vue";
import RvSegmented from "@/components/ui/RvSegmented.vue";
import RvSlider from "@/components/ui/RvSlider.vue";
import RvSwitch from "@/components/ui/RvSwitch.vue";
import RvEmptyState from "@/components/ui/RvEmptyState.vue";

const draft = useDraftStore();
const router = useRouter();

onMounted(() => {
  if (draft.viralTweets.length === 0) draft.loadViral();
});

/** Tweet'i seçip içerik adımına geçer — yanıtlar yorum olarak geliyor. */
async function selectTweet(url: string) {
  draft.url = url;
  draft.urlTouched = false;
  const ok = await draft.loadTweet(url);
  if (ok) router.push("/yeni/icerik");
}

function formatCount(n: number) {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1).replace(".", ",")} M`;
  if (n >= 1000) return `${(n / 1000).toFixed(1).replace(".", ",")} B`;
  return String(n);
}

/** Kartta okunması güç olan bağlantı kalıntılarını gizler. */
function preview(text: string) {
  return text.replace(/https?:\/\/\S+/g, "").trim();
}

const hiddenCount = computed(
  () => draft.viralTweets.length - draft.viralUsable.length,
);
</script>

<template>
  <div class="split">
    <!-- Sol: arama ölçütleri -->
    <aside class="filters rv-scroll">
      <button type="button" class="back" @click="router.push('/yeni/kaynak')">
        <ArrowLeft :size="14" />
        Bağlantı yapıştırmaya dön
      </button>

      <div class="control">
        <label>Videoda ne olsun</label>
        <RvSegmented
          v-model="draft.xMode"
          :options="[
            { value: 'replies', label: 'Yanıtlar' },
            { value: 'thread', label: 'Thread' },
          ]"
        />
        <span class="hint">
          {{
            draft.xMode === "thread"
              ? "Yazarın kendi devam tweet'leri, atıldığı sırayla."
              : "Tweet'e gelen yanıtlar — Reddit yorumlarının karşılığı."
          }}
        </span>
      </div>

      <div class="control">
        <label>Anahtar kelime</label>
        <RvTextField
          v-model="draft.viralKeyword"
          placeholder="boş bırakırsan gündemin tamamı"
          @keydown.enter="draft.loadViral()"
        />
      </div>

      <div class="control">
        <label>Dil</label>
        <RvSegmented
          v-model="draft.viralLang"
          :options="[
            { value: 'tr', label: 'Türkçe' },
            { value: 'en', label: 'İngilizce' },
            { value: '', label: 'Hepsi' },
          ]"
        />
      </div>

      <div class="control">
        <label>Zaman aralığı</label>
        <RvSegmented
          v-model="draft.viralHours"
          :options="[
            { value: '24', label: '24 saat' },
            { value: '168', label: '7 gün' },
            { value: '720', label: '30 gün' },
          ]"
        />
      </div>

      <div class="control">
        <label>En az beğeni</label>
        <div class="slider-row">
          <RvSlider
            v-model="draft.viralMinFaves"
            :min="500"
            :max="20000"
            :step="500"
          />
          <span class="rv-tabular value wide">{{ formatCount(draft.viralMinFaves) }}</span>
        </div>
      </div>

      <div class="control">
        <label>Kaç tweet</label>
        <div class="slider-row">
          <RvSlider v-model="draft.viralLimit" :min="10" :max="50" :step="5" />
          <span class="rv-tabular value">{{ draft.viralLimit }}</span>
        </div>
      </div>

      <div class="control inline">
        <RvSwitch v-model="draft.viralOnlyText" />
        <span class="switch-label">
          Yalnızca metin tweet'leri
          <span class="muted">(bağlantı ve medya olmadan)</span>
        </span>
      </div>

      <RvButton variant="primary" :loading="draft.viralLoading" @click="draft.loadViral()">
        <Search v-if="!draft.viralLoading" :size="14" />
        Getir
      </RvButton>

      <p class="note">
        Yanıtı az olan tweet'ler listelenmiyor: videoda yalnızca başlık kartı kalırdı.
      </p>
    </aside>

    <!-- Sağ: tweet listesi -->
    <div class="list-pane">
      <div class="summary">
        <span class="est">
          <Flame :size="14" class="flame" />
          X'te viral
        </span>
        <span class="sub-note">
          {{ draft.viralUsable.length }} tweet
          <template v-if="hiddenCount > 0"> · {{ hiddenCount }} tanesi yanıtsız</template>
          <template v-if="draft.isLoading"> · tweet çekiliyor…</template>
        </span>
      </div>

      <div v-if="draft.viralError" class="error-strip">
        <CircleAlert :size="14" class="error-icon" />
        {{ draft.viralError }}
      </div>

      <div v-if="draft.viralUsable.length > 0" class="rows rv-scroll">
        <button
          v-for="tweet in draft.viralUsable"
          :key="tweet.id"
          type="button"
          class="tweet"
          :disabled="draft.isLoading"
          @click="selectTweet(tweet.url)"
        >
          <div class="tweet-body">
            <div class="tweet-author">
              <span class="handle">{{ tweet.author }}</span>
              <span v-if="tweet.author_name" class="name">{{ tweet.author_name }}</span>
              <span class="when">{{ relativeTime(tweet.created_utc) }}</span>
            </div>
            <div class="tweet-text">{{ preview(tweet.text) }}</div>
            <div class="tweet-meta">
              <span class="metric">
                <Heart :size="12" />
                {{ formatCount(tweet.likes) }}
              </span>
              <span class="metric">
                <MessageSquare :size="12" />
                {{ formatCount(tweet.replies) }}
              </span>
              <span class="metric">
                <Repeat2 :size="12" />
                {{ formatCount(tweet.retweets) }}
              </span>
            </div>
          </div>
          <span class="pick">Seç</span>
        </button>
      </div>

      <RvEmptyState
        v-else-if="!draft.viralLoading && !draft.viralError"
        title="Ölçütlere uyan tweet yok"
        description="Beğeni eşiğini düşür, zaman aralığını genişlet ya da anahtar kelimeyi kaldır. Yalnızca metin süzgeci de listeyi epey daraltıyor."
      >
        <template #icon><Flame :size="26" /></template>
      </RvEmptyState>

      <div v-else-if="draft.viralLoading" class="pane-loading">Tweet'ler getiriliyor…</div>
    </div>
  </div>
</template>

<style scoped>
.split {
  flex: 1;
  display: flex;
  min-height: 0;
}

/* Ölçütler */
.filters {
  width: 300px;
  flex: none;
  border-right: 1px solid var(--rv-border);
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-4);
}

/* Sütun taşarsa panel kayar; ölçütler kendi boylarını korur.
   Aksi hâlde flex-shrink düğmeyi ve kaydırıcıları eziyor. */
.filters > * {
  flex: none;
}

.back {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  align-self: flex-start;
  background: none;
  border: none;
  padding: 0;
  color: var(--rv-text-faint);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

.back:hover {
  color: var(--rv-text);
}

.control {
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-2);
}

.control.inline {
  flex-direction: row;
  align-items: center;
  gap: 10px;
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

.value {
  font-size: 12px;
  color: var(--rv-text-muted);
  width: 28px;
  text-align: right;
}

.value.wide {
  width: 46px;
}

.switch-label {
  font-size: 12px;
  color: var(--rv-text-muted);
  line-height: 1.4;
}

.muted {
  color: var(--rv-text-faint);
}

.hint {
  font-size: 11px;
  line-height: 1.45;
  color: var(--rv-text-faint);
}

.note {
  margin: 0;
  font-size: 11px;
  line-height: 1.5;
  color: var(--rv-text-faint);
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

.flame {
  color: var(--rv-accent);
}

.sub-note {
  font-size: 12px;
  color: var(--rv-text-faint);
}

.error-strip {
  flex: none;
  display: flex;
  align-items: flex-start;
  gap: var(--rv-space-2);
  margin: var(--rv-space-3) 20px 0;
  padding: 10px var(--rv-space-3);
  border-radius: var(--rv-radius-sm);
  background: var(--rv-danger-soft);
  border: 1px solid color-mix(in srgb, var(--rv-danger) 30%, transparent);
  font-size: 12px;
  color: var(--rv-text-muted);
  line-height: 1.5;
}

.error-icon {
  color: var(--rv-danger);
  flex: none;
  margin-top: 1px;
}

.rows {
  flex: 1;
  padding: var(--rv-space-3) 20px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.tweet {
  display: flex;
  align-items: center;
  gap: var(--rv-space-3);
  padding: var(--rv-space-3);
  border-radius: var(--rv-radius-md);
  background: var(--rv-bg-surface);
  border: 1px solid var(--rv-border);
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.tweet:hover:not(:disabled) {
  border-color: var(--rv-accent);
}

.tweet:disabled {
  opacity: 0.5;
  cursor: wait;
}

.tweet-body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.tweet-author {
  display: flex;
  align-items: baseline;
  gap: 8px;
  font-size: 11px;
  min-width: 0;
}

.handle {
  color: var(--rv-accent-quiet);
  font-family: var(--rv-font-mono);
}

.name {
  color: var(--rv-text-faint);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.when {
  color: var(--rv-text-faint);
  margin-left: auto;
  flex: none;
}

.tweet-text {
  line-height: 1.45;
  text-wrap: pretty;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.tweet-meta {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 11px;
  color: var(--rv-text-faint);
}

.metric {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.pick {
  flex: none;
  font-size: 12px;
  color: var(--rv-text-faint);
  padding: 4px 10px;
  border-radius: var(--rv-radius-sm);
  border: 1px solid var(--rv-border);
}

.tweet:hover:not(:disabled) .pick {
  color: var(--rv-accent-quiet);
  border-color: var(--rv-accent);
}

.pane-loading {
  flex: 1;
  display: grid;
  place-items: center;
  font-size: 12px;
  color: var(--rv-text-faint);
}
</style>
