import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type { Clip } from "@/lib/api";
import type { RenderResult } from "@/lib/api";
import {
  cancelJob as cancelJobCommand,
  isTauri,
  notifySystem,
  onQuotaWait,
  onTtsFallback,
  onTtsProgress,
  resetCancel,
  renderVideo,
  revealInFinder,
  saveCard,
  saveJobDraft,
  saveJobInfo,
  synthesizeSpeech,
} from "@/lib/api";
import { useLibraryStore } from "@/stores/library";
import { useToastStore } from "@/stores/toasts";
import { renderCard } from "@/lib/cardRenderer";
import type { Job, LogLevel, LogLine, Stage, StageId } from "@/lib/types";
import { useDraftStore } from "@/stores/draft";
import { useSettingsStore } from "@/stores/settings";

/** Aşama iskeleti. Henüz bağlanmamış olanlar `blocked` olarak başlar —
 *  sahte ilerleme göstermektense ne olmadığını açıkça söylüyoruz. */
function freshStages(): Stage[] {
  return [
    { id: "fetch", label: "Gönderi çekiliyor", state: "pending" },
    { id: "translate", label: "Çeviri", state: "pending" },
    { id: "tts", label: "Seslendirme", state: "pending" },
    { id: "cards", label: "Kartlar oluşturuluyor", state: "pending" },
    { id: "background", label: "Arka plan hazırlanıyor", state: "pending" },
    { id: "render", label: "Video render ediliyor", state: "pending" },
  ];
}

export const useJobsStore = defineStore("jobs", () => {
  const jobs = ref<Job[]>([]);
  const paused = ref(false);

  const pendingCount = computed(
    () => jobs.value.filter((j) => j.status === "waiting" || j.status === "running").length,
  );
  const waitingCount = computed(() => jobs.value.filter((j) => j.status === "waiting").length);
  const runningCount = computed(() => jobs.value.filter((j) => j.status === "running").length);
  const activeJob = computed(() => jobs.value.find((j) => j.status === "running") ?? null);

  const stages = ref<Stage[]>(freshStages());
  const log = ref<LogLine[]>([]);
  const clips = ref<Clip[]>([]);
  const cardPaths = ref<string[]>([]);
  const video = ref<RenderResult | null>(null);
  const renderState = ref<"idle" | "running" | "done" | "failed" | "cancelled">("idle");
  const renderError = ref({ stageId: "tts" as StageId, title: "", message: "", at: "", technical: [] as string[] });
  const startedAt = ref<number | null>(null);
  const cancelling = ref(false);

  const displayStages = computed(() => stages.value);
  const doneCount = computed(() => stages.value.filter((s) => s.state === "done").length);
  const logLineCount = computed(() => log.value.length);

  /** Bağlanmış aşamaların ağırlıkları; bağlanmamışlar ilerlemeye katılmaz. */
  const overallProgress = computed(() => {
    const weights: Partial<Record<StageId, number>> = {
      fetch: 0.05,
      translate: 0.1,
      tts: 0.3,
      cards: 0.15,
      background: 0.05,
      render: 0.35,
    };
    let total = 0;
    for (const stage of stages.value) {
      const w = weights[stage.id];
      if (!w) continue;
      if (stage.state === "done") total += w;
      else if (stage.state === "active") total += w * (stage.progress ?? 0);
    }
    return total;
  });

  function now() {
    return new Date().toLocaleTimeString("tr-TR", { hour12: false });
  }

  function addLog(level: LogLevel, text: string) {
    log.value.push({ time: now(), level, text });
  }

  function setStage(id: StageId, patch: Partial<Stage>) {
    const stage = stages.value.find((s) => s.id === id);
    if (stage) Object.assign(stage, patch);
  }

  function formatDuration(totalSeconds: number) {
    const s = Math.round(totalSeconds);
    return `${String(Math.floor(s / 60)).padStart(2, "0")}:${String(s % 60).padStart(2, "0")}`;
  }

  function fail(stageId: StageId, title: string, error: unknown) {
    const message = String(error);
    setStage(stageId, { state: "failed", detail: "durdu" });
    renderState.value = "failed";
    renderError.value = {
      stageId,
      title,
      message,
      at: now(),
      technical: [message],
    };
    addLog("error", message);
    useToastStore().push(
      { tone: "error", text: `${title}: ${message}`.slice(0, 160) },
      { sound: true },
    );
    const job = jobs.value.find((j) => j.status === "running");
    if (job) {
      job.status = "failed";
      job.statusDetail = message.slice(0, 80);
    }
  }

  /** Kullanıcı iptal etti mi? Aşamalar arasında kontrol ediliyor. */
  function isCancelled() {
    return renderState.value === "cancelled";
  }

  /** Çalışan üretimi durdurur — Rust tarafındaki ffmpeg de sonlandırılır. */
  async function cancel() {
    if (renderState.value !== "running") return;
    cancelling.value = true;
    addLog("warn", "İptal isteniyor…");
    try {
      if (isTauri()) await cancelJobCommand();
    } finally {
      renderState.value = "cancelled";
      cancelling.value = false;
      stages.value.forEach((s) => {
        if (s.state === "active") {
          s.state = "blocked";
          s.detail = "iptal edildi";
        }
      });
      const job = jobs.value.find((j) => j.status === "running");
      if (job) {
        job.status = "failed";
        job.statusDetail = "İptal edildi";
      }
      addLog("warn", "Üretim iptal edildi.");
      useToastStore().push({ tone: "warning", text: "Üretim iptal edildi." });
    }
  }

  /** Üretimi başlatır. Çekme ve çeviri taslakta zaten yapıldığı için
   *  gerçek sayılarıyla tamamlanmış işaretlenir; seslendirme burada koşar. */
  /** Yarıda kalan işin klasörü — "Yeniden dene" aynı yerde sürdürsün diye. */
  const lastJobId = ref("");

  async function runJob(resumeJobId = "") {
    const draft = useDraftStore();
    const settings = useSettingsStore();

    const selected = draft.selectedComments;
    if (selected.length === 0) {
      addLog("error", "Seçili yorum yok.");
      return;
    }

    stages.value = freshStages();
    log.value = [];
    clips.value = [];
    renderState.value = "running";
    startedAt.value = Date.now();
    cardPaths.value = [];
    video.value = null;
    if (isTauri()) await resetCancel();

    // Bu ayarlarla üretim yapıldı; bir dahaki videoda hazır gelsin.
    draft.rememberLastSettings();

    // Aynı klasörde sürdürmek, üretilmiş ses parçalarını korumanın tek yolu;
    // yeni klasör açmak her seferinde sıfırdan seslendirmek demekti.
    const jobId = resumeJobId || `job-${Date.now()}`;
    lastJobId.value = jobId;
    jobs.value = jobs.value.filter((j) => j.id !== jobId);
    jobs.value.unshift({
      id: jobId,
      title: draft.post.titleTr || draft.post.title,
      subreddit: draft.post.subreddit,
      source: draft.source,
      presetSummary: `Türkçe · Google · ${draft.backgroundVideos.find((b) => b.id === draft.look.backgroundVideoId)?.label ?? "arka plan"}`,
      status: "running",
      progress: 0,
      statusDetail: "Seslendirme",
      outputPath: "",
    });

    // Taslağı en baştan diske yazıyoruz: uygulama kapansa bile bu iş
    // kitaplıktan sürdürülebilsin. Ses parçaları zaten aynı klasörde.
    if (isTauri()) {
      await saveJobDraft(
        `${settings.video.outputDir}/${jobId}`,
        JSON.stringify(draft.jobDraftSnapshot()),
      ).catch(() => {
        /* taslak yazılamazsa üretim yine de sürsün */
      });
    }

    // 1 — Çekme (kaynak adımında yapıldı)
    setStage("fetch", { state: "done", detail: `${draft.post.fetchedCount} yorum` });
    addLog("success", `gönderi hazır · ${draft.sourceHandle} · ${draft.post.id} · ${draft.post.fetchedCount} yorum getirildi`);

    // 2 — Çeviri (içerik adımında yapıldı)
    setStage("translate", { state: "done", detail: `${selected.length} yorum` });
    addLog("success", `çeviri hazır · ${selected.length} yorum · ${draft.totalChars} karakter`);

    if (!isTauri()) {
      setStage("tts", { state: "blocked", detail: "tarayıcıda arka uç yok" });
      addLog("warn", "Tarayıcıda çalışıyorsun; seslendirme yalnızca uygulamada koşar.");
      renderState.value = "idle";
      return;
    }

    // 3 — Seslendirme (gerçek)
    const items = [
      { id: "baslik", text: draft.post.titleTr || draft.post.title },
      ...selected.map((c) => ({ id: c.id, text: c.translated })),
    ];

    setStage("tts", { state: "active", progress: 0, detail: `0 / ${items.length} parça` });
    addLog(
      "info",
      `seslendirme başladı · ${draft.engineId} · ${draft.selectedVoice?.name ?? "varsayılan"} · ${items.length} parça · hız ${draft.speed.toFixed(2)}× · sessizlik ${draft.silenceMs} ms`,
    );

    // Kota nedeniyle motor değişirse kullanıcı bunu günlükte ve bildirimde görsün.
    // Kota beklemesi sessiz bir duraklama gibi görünmesin.
    const unlistenQuota = await onQuotaWait((message) => {
      addLog("warn", message);
      useToastStore().push({ tone: "warning", text: message });
    });

    const unlistenFallback = await onTtsFallback((message) => {
      addLog("warn", message);
      useToastStore().push({ tone: "warning", text: message });
    });

    const unlisten = await onTtsProgress((p) => {
      setStage("tts", {
        progress: p.done / p.total,
        detail: `${p.done} / ${p.total} parça`,
      });
      const job = jobs.value.find((j) => j.id === jobId);
      if (job) job.progress = overallProgress.value;
      addLog("info", `ses parçası hazır · ${p.id}`);
    });

    try {
      const outDir = `${settings.video.outputDir}/${jobId}`;
      const result = await synthesizeSpeech(
        items,
        draft.engineId,
        draft.voiceId,
        settings.translation.targetLang,
        outDir,
        draft.speechShape,
        settings.defaults.fallbackOnQuota,
        // Sıra işaretleri yalnızca X içeriğinde çıkıyor; kartta kalsın,
        // seslendirmeye girmesin.
        draft.source === "x",
      );
      clips.value = result;

      const reused = result.filter((c) => c.reused).length;
      if (reused > 0) {
        addLog(
          "success",
          `${reused} ses parçası diskten alındı · ${result.length - reused} parça yeniden üretildi`,
        );
      }

      const totalSeconds = result.reduce((sum, c) => sum + c.duration_sec, 0);
      setStage("tts", {
        state: "done",
        progress: 1,
        detail: `${result.length} parça · ${formatDuration(totalSeconds)}`,
      });
      addLog("success", `seslendirme tamamlandı · ${result.length} dosya · ${formatDuration(totalSeconds)}`);
      addLog("info", `dosyalar: ${outDir}`);

    } catch (error) {
      unlisten();
      unlistenFallback();
      unlistenQuota();
      if (isCancelled() || String(error).includes("İptal edildi")) return;
      fail("tts", "Seslendirme tamamlanamadı", error);
      return;
    }
    unlisten();
    unlistenFallback();
    unlistenQuota();
    if (isCancelled()) return;

    // 4 — Kart görselleri (gerçek)
    const outDir = `${settings.video.outputDir}/${jobId}`;
    const cards = [
      {
        id: "baslik",
        author: draft.sourceHandle,
        upvotes: "",
        text: draft.post.titleTr || draft.post.title,
      },
      ...selected.map((c) => ({
        id: c.id,
        author: c.author,
        upvotes: `${c.upvotes.toLocaleString("tr-TR")} ${draft.scoreWord}`,
        text: c.translated,
      })),
    ];

    setStage("cards", { state: "active", progress: 0, detail: `0 / ${cards.length} kart` });
    addLog("info", `kart görselleri üretiliyor · 1080×1920 · ${cards.length} kart`);

    try {
      for (const [index, card] of cards.entries()) {
        if (isCancelled()) return;
        const blob = await renderCard({
          author: card.author,
          upvotes: card.upvotes,
          text: card.text,
          theme: draft.look.cardTheme,
          fontSize: draft.look.fontSize,
          cardWidth: draft.look.cardWidth,
        });
        const path = await saveCard(outDir, card.id, blob);
        cardPaths.value.push(path);

        setStage("cards", {
          progress: (index + 1) / cards.length,
          detail: `${index + 1} / ${cards.length} kart`,
        });
        const job = jobs.value.find((j) => j.id === jobId);
        if (job) job.progress = overallProgress.value;
      }

      setStage("cards", {
        state: "done",
        progress: 1,
        detail: `${cards.length} kart`,
      });
      addLog("success", `kartlar hazır · ${cards.length} PNG · ${outDir}/kartlar`);
    } catch (error) {
      if (isCancelled()) return;
      fail("cards", "Kart görselleri üretilemedi", error);
      return;
    }
    if (isCancelled()) return;

    // 5 — Arka plan seçimi
    const backgroundPath = draft.selectedBackgroundPath;
    setStage("background", {
      state: "done",
      detail: backgroundPath
        ? (draft.backgroundVideos.find((b) => b.id === draft.look.backgroundVideoId)?.label ??
          "seçildi")
        : "düz zemin",
    });
    addLog(
      backgroundPath ? "success" : "warn",
      backgroundPath
        ? `arka plan: ${backgroundPath}`
        : "arka plan videosu yok — düz zemin kullanılacak",
    );

    // 6 — Video render (gerçek)
    setStage("render", { state: "active", progress: 0, detail: "ffmpeg çalışıyor" });
    addLog(
      "info",
      `render başladı · ${settings.video.resolution} · ${settings.video.fps} fps · ${settings.video.codec}${settings.video.hardwareAccel ? " (donanım)" : ""} · ${settings.video.bitrateMbps} Mbps`,
    );

    try {
      const segments = cards.map((_, index) => ({
        card_path: cardPaths.value[index],
        audio_path: clips.value[index]?.path,
        duration_sec: clips.value[index]?.duration_sec ?? 0,
      }));

      const musicPath = draft.selectedMusicPath;
      const result = await renderVideo({
        segments,
        background_path: backgroundPath,
        music_path: musicPath,
        music_volume: draft.look.audioVolume,
        fps: Number(settings.video.fps),
        out_path: `${outDir}/${settings.outputFileName(draft.post.subreddit, draft.post.titleTr || draft.post.title)}`,
        resolution: settings.video.resolution,
        codec: settings.video.codec,
        bitrate_mbps: settings.video.bitrateMbps,
        hardware_accel: settings.video.hardwareAccel,
        outro_sec: settings.video.outroSec,
      });

      video.value = result;

      // Kitaplığın okuyacağı özet — başlık ve kaynak diskten geri okunabilsin.
      await saveJobInfo(outDir, {
        title: draft.post.titleTr || draft.post.title,
        subreddit: draft.post.subreddit,
        source: draft.source,
        sourceUrl: draft.post.url,
        voice: `${draft.engineId} · ${draft.selectedVoice?.name ?? "varsayılan"}`,
        background:
          draft.backgroundVideos.find((b) => b.id === draft.look.backgroundVideoId)?.label ??
          "düz zemin",
      }).catch(() => {
        /* özet yazılamazsa video yine de geçerli */
      });
      setStage("render", {
        state: "done",
        progress: 1,
        detail: `${formatDuration(result.duration_sec)} · ${(result.size_bytes / 1_048_576).toFixed(1)} MB`,
      });
      addLog(
        "success",
        `video hazır · ${formatDuration(result.duration_sec)} · ${(result.size_bytes / 1_048_576).toFixed(1)} MB · ${result.path}`,
      );
    } catch (error) {
      if (isCancelled() || String(error).includes("İptal edildi")) return;
      fail("render", "Video birleştirilemedi", error);
      return;
    }

    const job = jobs.value.find((j) => j.id === jobId);
    if (job) {
      job.status = "done";
      job.progress = 1;
      job.statusDetail = video.value
        ? `${formatDuration(video.value.duration_sec)} · ${(video.value.size_bytes / 1_048_576).toFixed(1)} MB`
        : `${clips.value.length} ses · ${cardPaths.value.length} kart`;
      job.outputPath = video.value?.path ?? "";
    }
    renderState.value = "done";

    // Kitaplık diskten okuduğu için yeni işi görmesi adına tazeleniyor.
    useLibraryStore().refresh();

    const sure = video.value ? formatDuration(video.value.duration_sec) : "";
    const yol = video.value?.path;
    useToastStore().push(
      {
        tone: "success",
        text: `Video hazır · ${sure}`,
        actionLabel: yol ? "Klasörde göster" : undefined,
        action: yol ? () => revealJob(yol) : undefined,
      },
      { sound: settings.general.notifyOnFinish },
    );

    if (settings.general.notifyOnFinish) {
      notifySystem("RVMaker", `Video hazır · ${sure}`);
    }
  }

  /** Bekleyen işleri sırayla koşturur.
   *
   *  Bilerek tek iş: ffmpeg zaten tüm çekirdekleri kullanıyor, üstelik iptal
   *  mekanizması (`render.rs` içindeki `RUNNING`) tek bir pid tutuyor ve
   *  render durumu/günlüğü tek iş için yazılmış. Paralel koşmak hızlandırmaz,
   *  buna karşılık "İptal"i sessizce bozardı. */
  async function startQueue() {
    if (runningCount.value > 0) return;
    paused.value = false;

    while (!paused.value) {
      const next = jobs.value.find((j) => j.status === "waiting");
      if (!next) break;
      next.status = "running";
      await runJob();
      if (renderState.value === "cancelled") break;
    }
  }

  /** Hata veya iptalden sonra hattı baştan koşturur. */
  /** Hata veya iptalden sonra hattı sürdürür.
   *
   *  Aynı iş klasörüyle koşuyor: aynı ayarla üretilmiş ses parçaları diskten
   *  alınıyor, yalnızca eksikler yeniden seslendiriliyor. */
  async function retry() {
    await runJob(lastJobId.value);
  }

  /** Kuyruk ve render ekranındaki "klasörde göster" eylemleri. */
  async function revealJob(path: string) {
    if (!path || !isTauri()) return;
    try {
      await revealInFinder(path);
    } catch {
      /* dosya taşınmış veya silinmiş olabilir */
    }
  }

  const totalAudioSeconds = computed(() =>
    clips.value.reduce((sum, c) => sum + c.duration_sec, 0),
  );

  function removeJob(id: string) {
    jobs.value = jobs.value.filter((j) => j.id !== id);
  }

  function moveJob(id: string, delta: number) {
    const i = jobs.value.findIndex((j) => j.id === id);
    const target = i + delta;
    if (i < 0 || target < 0 || target >= jobs.value.length) return;
    const [item] = jobs.value.splice(i, 1);
    jobs.value.splice(target, 0, item);
  }

  return {
    jobs,
    paused,
    pendingCount,
    waitingCount,
    runningCount,
    activeJob,
    stages,
    displayStages,
    doneCount,
    renderState,
    renderError,
    log,
    logLineCount,
    clips,
    cardPaths,
    video,
    totalAudioSeconds,
    overallProgress,
    formatDuration,
    runJob,
    startQueue,
    retry,
    lastJobId,
    cancel,
    cancelling,
    revealJob,
    removeJob,
    moveJob,
  };
});
