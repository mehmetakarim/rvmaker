import { defineStore } from "pinia";
import { computed, ref } from "vue";
import {
  backgroundThumbnail,
  deleteJob,
  fileUrl,
  isTauri,
  listJobs,
  revealInFinder,
} from "@/lib/api";
import { sourceLabel } from "@/lib/types";
import type { JobDraft, LibraryItem, SourceKind } from "@/lib/types";

/** Kitaplıkta "kaldığı yerden sürdür" satırı. */
export interface UnfinishedJob {
  id: string;
  dir: string;
  title: string;
  handle: string;
  createdAt: string;
  doneClips: number;
  totalClips: number;
  /** Ham taslak JSON'u — sürdürürken doğrudan geri yükleniyor. */
  raw: string;
}
import { useSettingsStore } from "@/stores/settings";
import { useToastStore } from "@/stores/toasts";

/** İş klasörüne yazılan özet bilgi. */
interface JobInfo {
  title?: string;
  subreddit?: string;
  source?: SourceKind;
  sourceUrl?: string;
  voice?: string;
  background?: string;
}

export const useLibraryStore = defineStore("library", () => {
  const items = ref<LibraryItem[]>([]);
  /** Video kimliğinden kapak görselinin webview adresine. */
  const thumbs = ref<Record<string, string>>({});
  const loading = ref(false);
  const loadError = ref("");

  const query = ref("");
  const subredditFilter = ref("all");
  /** Videosu olmayan ama taslağı duran işler — kaldığı yerden sürdürülebilir. */
  const unfinished = ref<UnfinishedJob[]>([]);
  const rangeFilter = ref("all");
  const selectedId = ref<string | null>(null);

  function relativeDay(ms: number) {
    if (!ms) return "";
    const days = Math.floor((Date.now() - ms) / 86_400_000);
    if (days <= 0) return "bugün";
    if (days === 1) return "dün";
    if (days < 7) return `${days} gün`;
    return `${Math.floor(days / 7)} hafta`;
  }

  /** Kitaplık diskten okunur; bellekte tutulan sahte liste yok. */
  async function refresh() {
    if (!isTauri()) return;
    loading.value = true;
    loadError.value = "";
    try {
      const settings = useSettingsStore();
      const jobs = await listJobs(settings.video.outputDir);

      // Yarım kalmış ama sürdürülebilir işler: videosu yok, taslağı var.
      // Taslağı olmayan eski yetimler burada görünmüyor — onlar için
      // sürdürecek bir girdi yok, yalnızca ses dosyaları var.
      unfinished.value = jobs
        .filter((j) => !j.has_video && j.draft)
        .map((j) => {
          let d: JobDraft | null = null;
          try {
            d = JSON.parse(j.draft) as JobDraft;
          } catch {
            /* taslak bozuksa sürdürülemez */
          }
          if (!d || d.version !== 1) return null;

          return {
            id: j.id,
            dir: j.dir,
            title: d.post.titleTr || d.post.title,
            handle: sourceLabel(d.source ?? "reddit", d.post.subreddit),
            createdAt: relativeDay(j.created_ms),
            // Başlık kartı da bir parça; kullanıcıya kart sayısını gösteriyoruz.
            doneClips: j.clip_count,
            totalClips: d.comments.filter((c) => c.selected).length + 1,
            raw: j.draft,
          } satisfies UnfinishedJob;
        })
        .filter((x): x is UnfinishedJob => x !== null);

      items.value = jobs
        .filter((j) => j.has_video)
        .map((j) => {
          let info: JobInfo = {};
          try {
            info = j.info ? (JSON.parse(j.info) as JobInfo) : {};
          } catch {
            /* bilgi dosyası bozuksa yoksay */
          }

          return {
            id: j.id,
            dir: j.dir,
            createdMs: j.created_ms,
            title: info.title || j.id,
            subreddit: info.subreddit || "—",
            // Eski kayıtlarda alan yok; onların hepsi Reddit'ten geliyordu.
            source: info.source ?? "reddit",
            durationSec: j.duration_sec,
            createdAt: relativeDay(j.created_ms),
            sizeMb: Math.round(j.size_bytes / 1_048_576),
            sourceUrl: info.sourceUrl || "",
            voice: info.voice || "Google · Türkçe",
            background: info.background || "—",
            path: j.video_path,
          } satisfies LibraryItem;
        });

      // Kapaklar arka planda üretiliyor; biri düşerse diğerleri etkilenmiyor.
      for (const item of items.value) {
        if (thumbs.value[item.id]) continue;
        backgroundThumbnail(item.path)
          .then((thumb) => {
            thumbs.value = { ...thumbs.value, [item.id]: fileUrl(thumb) };
          })
          .catch(() => {
            /* kapak üretilemezse desenli zemin kalır */
          });
      }

      if (selectedId.value && !items.value.some((i) => i.id === selectedId.value)) {
        selectedId.value = null;
      }
      if (!selectedId.value) selectedId.value = items.value[0]?.id ?? null;
    } catch (error) {
      loadError.value = String(error);
    } finally {
      loading.value = false;
    }
  }

  const subredditOptions = computed(() => [
    { value: "all", label: "Tüm kaynaklar" },
    ...[...new Set(items.value.map((i) => i.subreddit))]
      .filter((s) => s && s !== "—")
      .map((s) => ({ value: s, label: `r/${s}` })),
  ]);

  const filtered = computed(() => {
    const q = query.value.trim().toLowerCase();
    const days = rangeFilter.value === "all" ? null : Number(rangeFilter.value);
    const cutoff = days ? Date.now() - days * 86_400_000 : null;

    return items.value.filter((item) => {
      const matchesQuery =
        !q ||
        item.title.toLowerCase().includes(q) ||
        item.subreddit.toLowerCase().includes(q);
      const matchesSub =
        subredditFilter.value === "all" || item.subreddit === subredditFilter.value;
      const matchesRange = !cutoff || item.createdMs >= cutoff;
      return matchesQuery && matchesSub && matchesRange;
    });
  });

  const selected = computed(() => items.value.find((i) => i.id === selectedId.value) ?? null);

  const totalSizeGb = computed(() => items.value.reduce((sum, i) => sum + i.sizeMb, 0) / 1024);

  function formatDuration(sec: number) {
    const s = Math.round(sec);
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
  }

  async function reveal(path: string) {
    if (!isTauri()) return;
    try {
      await revealInFinder(path);
    } catch (error) {
      loadError.value = String(error);
    }
  }

  /** Diskten kalıcı olarak siler — onay çağıran tarafta alınıyor. */
  async function remove(id: string) {
    const item = items.value.find((i) => i.id === id);
    if (!item) return;

    if (isTauri()) {
      try {
        await deleteJob(item.dir);
      } catch (error) {
        loadError.value = String(error);
        return;
      }
    }

    items.value = items.value.filter((i) => i.id !== id);
    if (selectedId.value === id) selectedId.value = items.value[0]?.id ?? null;
    useToastStore().push({ tone: "warning", text: `"${item.title}" silindi` });
  }

  /** Yarım kalan bir işin klasörünü siler — ses parçaları ve taslak dahil. */
  async function removeUnfinished(id: string) {
    const job = unfinished.value.find((j) => j.id === id);
    if (!job) return;
    if (isTauri()) {
      try {
        await deleteJob(job.dir);
      } catch (error) {
        loadError.value = String(error);
        return;
      }
    }
    unfinished.value = unfinished.value.filter((j) => j.id !== id);
  }

  /** Bütün yarım işleri siler; kaç tanesinin silindiğini döndürür. */
  async function removeAllUnfinished() {
    const hepsi = [...unfinished.value];
    let silinen = 0;
    for (const job of hepsi) {
      const once = unfinished.value.length;
      await removeUnfinished(job.id);
      if (unfinished.value.length < once) silinen += 1;
    }
    if (silinen > 0) {
      useToastStore().push({ tone: "warning", text: `${silinen} yarım üretim silindi` });
    }
    return silinen;
  }

  return {
    items,
    unfinished,
    removeUnfinished,
    removeAllUnfinished,
    thumbs,
    loading,
    loadError,
    query,
    subredditFilter,
    rangeFilter,
    selectedId,
    selected,
    subredditOptions,
    filtered,
    totalSizeGb,
    formatDuration,
    refresh,
    reveal,
    remove,
  };
});
