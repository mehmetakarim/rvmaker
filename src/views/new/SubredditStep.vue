<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import {
  Search,
  MessageSquare,
  ArrowUp,
  ArrowLeft,
  CircleAlert,
  FileText,
  Video,
  TriangleAlert,
} from "@lucide/vue";
import { useDraftStore } from "@/stores/draft";
import { relativeTime } from "@/lib/api";
import RvButton from "@/components/ui/RvButton.vue";
import RvTextField from "@/components/ui/RvTextField.vue";
import RvSegmented from "@/components/ui/RvSegmented.vue";
import RvSlider from "@/components/ui/RvSlider.vue";
import RvSwitch from "@/components/ui/RvSwitch.vue";
import RvBadge from "@/components/ui/RvBadge.vue";
import RvEmptyState from "@/components/ui/RvEmptyState.vue";

const draft = useDraftStore();
const router = useRouter();

onMounted(() => {
  if (draft.browsePosts.length === 0) draft.loadSubreddit();
});

/** Gönderiyi seçip içerik adımına geçer — URL alanını doldurmaya gerek yok. */
async function selectPost(url: string) {
  draft.url = url;
  draft.urlTouched = false;
  const ok = await draft.loadPost(url);
  if (ok) router.push("/yeni/icerik");
}

function formatCount(n: number) {
  if (n >= 1000) return `${(n / 1000).toFixed(1).replace(".", ",")} B`;
  return String(n);
}

const nsfwCount = computed(() => draft.browsePosts.filter((p) => p.is_nsfw).length);
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
        <label>Subreddit</label>
        <RvTextField
          v-model="draft.browseSubreddit"
          placeholder="AskReddit"
          @keydown.enter="draft.loadSubreddit()"
        >
          <template #leading><span class="prefix">r/</span></template>
        </RvTextField>
      </div>

      <div class="control">
        <label>Sıralama</label>
        <RvSegmented
          v-model="draft.browseSort"
          :options="[
            { value: 'hot', label: 'Popüler' },
            { value: 'top', label: 'En çok oy' },
            { value: 'new', label: 'Yeni' },
          ]"
        />
      </div>

      <div class="control">
        <label>Kaç gönderi</label>
        <div class="slider-row">
          <RvSlider v-model="draft.browseLimit" :min="10" :max="100" :step="5" />
          <span class="rv-tabular value">{{ draft.browseLimit }}</span>
        </div>
      </div>

      <div class="control inline">
        <RvSwitch v-model="draft.hideNsfw" />
        <span class="switch-label">
          Yetişkin içeriği gizle
          <span v-if="nsfwCount > 0" class="muted">({{ nsfwCount }} gizli)</span>
        </span>
      </div>

      <RvButton variant="primary" :loading="draft.browseLoading" @click="draft.loadSubreddit()">
        <Search v-if="!draft.browseLoading" :size="14" />
        Getir
      </RvButton>
    </aside>

    <!-- Sağ: gönderi listesi -->
    <div class="list-pane">
      <div class="summary">
        <span class="est">r/{{ draft.browseSubreddit }}</span>
        <span class="sub-note">
          {{ draft.visiblePosts.length }} gönderi
          <template v-if="draft.isLoading"> · gönderi çekiliyor…</template>
        </span>
      </div>

      <div v-if="draft.browseError" class="error-strip">
        <CircleAlert :size="14" class="error-icon" />
        {{ draft.browseError }}
      </div>

      <div v-if="draft.visiblePosts.length > 0" class="rows rv-scroll">
        <button
          v-for="post in draft.visiblePosts"
          :key="post.id"
          type="button"
          class="post"
          :disabled="draft.isLoading"
          @click="selectPost(post.url)"
        >
          <div class="post-body">
            <div class="post-title">{{ post.title }}</div>
            <div class="post-meta">
              <span class="metric">
                <ArrowUp :size="12" />
                {{ formatCount(post.upvotes) }}
              </span>
              <span class="metric">
                <MessageSquare :size="12" />
                {{ formatCount(post.comment_count) }}
              </span>
              <span>{{ relativeTime(post.created_utc) }}</span>
              <RvBadge v-if="post.is_self" tone="neutral">
                <FileText :size="11" />
                metin
              </RvBadge>
              <RvBadge v-else-if="post.is_video" tone="neutral">
                <Video :size="11" />
                video
              </RvBadge>
              <RvBadge v-if="post.is_nsfw" tone="danger">
                <TriangleAlert :size="11" />
                18+
              </RvBadge>
            </div>
          </div>
          <span class="pick">Seç</span>
        </button>
      </div>

      <RvEmptyState
        v-else-if="!draft.browseLoading && !draft.browseError"
        title="Gönderi bulunamadı"
        description="Başka bir subreddit adı dene ya da sıralamayı değiştir. Yetişkin içerik gizliyse liste boş görünebilir."
      >
        <template #icon><Search :size="26" /></template>
      </RvEmptyState>

      <div v-else-if="draft.browseLoading" class="pane-loading">Gönderiler getiriliyor…</div>
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

.prefix {
  color: var(--rv-text-faint);
  font-family: var(--rv-font-mono);
  font-size: 12px;
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

.switch-label {
  font-size: 12px;
  color: var(--rv-text-muted);
}

.muted {
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
  font-weight: 500;
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

.post {
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

.post:hover:not(:disabled) {
  border-color: var(--rv-accent);
}

.post:disabled {
  opacity: 0.5;
  cursor: wait;
}

.post-body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.post-title {
  line-height: 1.45;
  text-wrap: pretty;
}

.post-meta {
  display: flex;
  align-items: center;
  gap: 10px;
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

.post:hover:not(:disabled) .pick {
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
