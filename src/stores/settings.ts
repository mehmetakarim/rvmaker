import { defineStore } from "pinia";
import { computed, ref, watch } from "vue";
import type { SetupItem, TtsEngine } from "@/lib/types";
import { useToastStore } from "@/stores/toasts";
import type { EngineId } from "@/lib/api";
import {
  checkEnvironment,
  clearEngineKey,
  clearTempFiles,
  clearXCredentials,
  clearThumbnailCache,
  deleteAllOutputs,
  maintenanceInfo,
  clearRedditCookie,
  engineKeyPresent,
  setEngineKey,
  testEngine,
  installFfmpeg,
  isTauri,
  onInstallProgress,
  redditCookiePresent,
  setRedditCookie,
  birdInstalled,
  setXCredentials,
  verifyXCredentials,
  xCredentialsPresent,
} from "@/lib/api";

type Theme = "dark" | "light";

export const useSettingsStore = defineStore("settings", () => {
  const theme = ref<Theme>((localStorage.getItem("rv-theme") as Theme) ?? "dark");

  watch(
    theme,
    (t) => {
      document.documentElement.dataset.theme = t;
      try {
        localStorage.setItem("rv-theme", t);
      } catch {
        /* depolama kapalı olabilir */
      }
    },
    { immediate: true },
  );

  function toggleTheme() {
    theme.value = theme.value === "dark" ? "light" : "dark";
  }

  const setupItems = ref<SetupItem[]>([]);
  const setupLoading = ref(false);
  const setupError = ref("");
  const installLog = ref<string[]>([]);
  const installing = ref(false);

  /** Ortamı gerçekten denetler. Tarayıcıda arka uç yok, liste boş kalır. */
  async function runEnvironmentCheck() {
    if (!isTauri()) return;
    setupLoading.value = true;
    setupError.value = "";
    try {
      // Çerez durumu bilinmeden Reddit erişimi doğru değerlendirilemez.
      await refreshCookieStatus();
      const checks = await checkEnvironment({
        backgrounds_dir: backgroundsDir.value,
        output_dir: video.value.outputDir,
        reddit_configured: Boolean(reddit.value.clientId.trim()) || redditCookieSaved.value,
      });

      setupItems.value = checks.map((c) => ({
        id: c.id,
        label: c.label,
        detail: c.detail,
        state: c.state,
        required: c.required,
        fixHint: c.fix_hint,
        fixCommand: c.fix_command,
      }));
    } catch (error) {
      setupError.value = String(error);
    } finally {
      setupLoading.value = false;
    }
  }

  /** ffmpeg'i Homebrew ile kurar; çıktı canlı akar. */
  async function runInstall(id: string) {
    if (!isTauri() || id !== "ffmpeg") return;
    installing.value = true;
    installLog.value = ["Homebrew ile kurulum başlatıldı…"];

    const unlisten = await onInstallProgress((p) => {
      installLog.value.push(p.line);
      if (p.done) installing.value = false;
    });

    try {
      await installFfmpeg();
      await runEnvironmentCheck();
    } catch (error) {
      installLog.value.push(String(error));
      setupError.value = String(error);
    } finally {
      installing.value = false;
      unlisten();
    }
  }

  /** Zorunlu bir bileşen eksikse üretim yapılamaz. */
  const blockingIssues = computed(() =>
    setupItems.value.filter((i) => i.required && i.state === "missing"),
  );

  /** Kenar çubuğundaki gösterge: zorunlu bileşenlerin hepsi hazır mı. */
  const environmentReady = computed(
    () => setupItems.value.length > 0 && blockingIssues.value.length === 0,
  );

  const environmentSummary = computed(() => {
    if (setupItems.value.length === 0) return "ortam denetlenmedi";
    if (blockingIssues.value.length > 0) {
      return `${blockingIssues.value.map((i) => i.label).join(", ")} eksik`;
    }
    const warnings = setupItems.value.filter((i) => i.state === "warning").length;
    return warnings > 0 ? `hazır · ${warnings} uyarı` : "her şey hazır";
  });

  const engines = ref<TtsEngine[]>([
    { id: "googletranslate", label: "Google Translate", requiresKey: false, keyPresent: true },
    { id: "elevenlabs", label: "ElevenLabs", requiresKey: true, keyPresent: false },
    { id: "openai", label: "OpenAI", requiresKey: true, keyPresent: false },
    { id: "gemini", label: "Gemini", requiresKey: true, keyPresent: false },
    { id: "system", label: "Sistem sesi", requiresKey: false, keyPresent: true },
  ]);

  /** Reddit artık kimliksiz erişimi kapattığı için kendi uygulama kimliğimiz gerekiyor.
   *  Bu bir gizli anahtar değil, yalnızca istemci kimliği. */
  const reddit = ref({
    clientId: localStorage.getItem("rv-reddit-client-id") ?? "",
  });

  watch(
    () => reddit.value.clientId,
    (value) => {
      try {
        localStorage.setItem("rv-reddit-client-id", value);
      } catch {
        /* depolama kapalı olabilir */
      }
    },
  );

  /** Çerez anahtar zincirinde durur; burada yalnızca varlığını biliriz.
   *  `cookieDraft` kullanıcının yazdığı geçici değer — kaydedildikten sonra temizlenir. */
  const redditCookieSaved = ref(false);
  const redditCookieDraft = ref("");
  const redditCookieBusy = ref(false);
  const redditCookieError = ref("");

  async function refreshCookieStatus() {
    if (!isTauri()) return;
    try {
      redditCookieSaved.value = await redditCookiePresent();
    } catch {
      redditCookieSaved.value = false;
    }
  }

  async function saveRedditCookie() {
    if (!isTauri() || !redditCookieDraft.value.trim()) return;
    redditCookieBusy.value = true;
    redditCookieError.value = "";
    try {
      await setRedditCookie(redditCookieDraft.value);
      redditCookieDraft.value = "";
      await refreshCookieStatus();
      useToastStore().push({ tone: "success", text: "Reddit çerezi kaydedildi" });
    } catch (error) {
      redditCookieError.value = String(error);
    } finally {
      redditCookieBusy.value = false;
    }
  }

  async function removeRedditCookie() {
    if (!isTauri()) return;
    redditCookieBusy.value = true;
    redditCookieError.value = "";
    try {
      await clearRedditCookie();
      await refreshCookieStatus();
    } catch (error) {
      redditCookieError.value = String(error);
    } finally {
      redditCookieBusy.value = false;
    }
  }

  /** Motor anahtarları anahtar zincirinde durur; burada yalnızca varlığı bilinir.
   *  `draft` kullanıcının yazdığı geçici değer, kaydedilince temizlenir. */
  interface EngineKeyState {
    id: EngineId;
    label: string;
    saved: boolean;
    draft: string;
    busy: boolean;
    status: "unknown" | "ok" | "error";
    statusText: string;
  }

  // --- X (Twitter) erişimi ---
  const xCookieDraft = ref("");
  const xCookieSaved = ref(false);
  const xCookieBusy = ref(false);
  const xStatus = ref<"unknown" | "ok" | "error">("unknown");
  const xStatusText = ref("");
  /** X entegrasyonu `bird` aracına dayanıyor; kurulu değilse söylemek gerek. */
  const xToolInstalled = ref(true);

  async function refreshXStatus() {
    if (!isTauri()) return;
    try {
      xCookieSaved.value = await xCredentialsPresent();
    } catch {
      xCookieSaved.value = false;
    }
    // Aracın yokluğu çerezin kaydını geçersiz kılmamalı — ayrı ayrı sor.
    try {
      xToolInstalled.value = await birdInstalled();
    } catch {
      xToolInstalled.value = true;
    }
  }

  async function saveXCredentials() {
    if (!isTauri() || !xCookieDraft.value.trim()) return;
    xCookieBusy.value = true;
    xStatusText.value = "";
    try {
      await setXCredentials(xCookieDraft.value);
      xCookieDraft.value = "";
      xStatus.value = "unknown";
      xStatusText.value = "Kaydedildi, henüz test edilmedi.";
      await refreshXStatus();
    } catch (error) {
      xStatus.value = "error";
      xStatusText.value = String(error);
    } finally {
      xCookieBusy.value = false;
    }
  }

  async function removeXCredentials() {
    if (!isTauri()) return;
    xCookieBusy.value = true;
    try {
      await clearXCredentials();
      xStatus.value = "unknown";
      xStatusText.value = "";
      await refreshXStatus();
    } finally {
      xCookieBusy.value = false;
    }
  }

  /** Gerçek bir istekle sınar — hangi hesapla bağlanıldığını söyler. */
  async function checkXCredentials() {
    if (!isTauri()) return;
    xCookieBusy.value = true;
    try {
      xStatusText.value = await verifyXCredentials();
      xStatus.value = "ok";
    } catch (error) {
      xStatus.value = "error";
      xStatusText.value = String(error);
    } finally {
      xCookieBusy.value = false;
    }
  }

  const apiKeys = ref<EngineKeyState[]>([
    { id: "elevenlabs", label: "ElevenLabs", saved: false, draft: "", busy: false, status: "unknown", statusText: "" },
    { id: "openai", label: "OpenAI", saved: false, draft: "", busy: false, status: "unknown", statusText: "" },
    { id: "gemini", label: "Gemini", saved: false, draft: "", busy: false, status: "unknown", statusText: "" },
  ]);

  async function refreshEngineKeys() {
    if (!isTauri()) return;
    for (const key of apiKeys.value) {
      try {
        key.saved = await engineKeyPresent(key.id);
      } catch {
        key.saved = false;
      }
    }
    // Motor listesindeki "anahtar var mı" göstergeleri de güncellensin.
    engines.value = engines.value.map((e) => ({
      ...e,
      keyPresent: e.requiresKey
        ? (apiKeys.value.find((k) => k.id === e.id)?.saved ?? false)
        : true,
    }));
  }

  async function saveEngineKey(id: EngineId) {
    const key = apiKeys.value.find((k) => k.id === id);
    if (!key || !isTauri() || !key.draft.trim()) return;
    key.busy = true;
    try {
      await setEngineKey(id, key.draft);
      key.draft = "";
      key.status = "unknown";
      key.statusText = "Kaydedildi, henüz test edilmedi.";
      useToastStore().push({ tone: "success", text: `${key.label} anahtarı kaydedildi` });
      await refreshEngineKeys();
    } catch (error) {
      key.status = "error";
      key.statusText = String(error);
    } finally {
      key.busy = false;
    }
  }

  async function removeEngineKey(id: EngineId) {
    const key = apiKeys.value.find((k) => k.id === id);
    if (!key || !isTauri()) return;
    key.busy = true;
    try {
      await clearEngineKey(id);
      key.status = "unknown";
      key.statusText = "";
      await refreshEngineKeys();
    } finally {
      key.busy = false;
    }
  }

  /** Motoru gerçek bir istekle sınar. */
  async function checkEngine(id: EngineId) {
    const key = apiKeys.value.find((k) => k.id === id);
    if (!isTauri()) return;
    if (key) key.busy = true;
    try {
      const summary = await testEngine(id);
      if (key) {
        key.status = "ok";
        key.statusText = `Bağlantı doğrulandı · ${summary}`;
        useToastStore().push({ tone: "success", text: `${key.label} bağlantısı doğrulandı` });
      }
    } catch (error) {
      if (key) {
        key.status = "error";
        key.statusText = String(error);
      }
    } finally {
      if (key) key.busy = false;
    }
  }

  const defaults = ref({
    engineId: "googletranslate",
    voiceId: "tr-google",
    fallbackOnQuota: true,
  });

  const translation = ref({
    provider: "google",
    targetLang: "tr",
    /** Zaten hedef dildeki metni tekrar çevirip bozmamak için. */
    skipIfSameLanguage: true,
    glossary: "AI=yapay zeka\nOP=gönderi sahibi",
  });

  /** Pencere API'si hatası — ayar ekranında görünür. */
  const windowError = ref("");

  const general = ref({
    startOnLaunch: (localStorage.getItem("rv-start-on-launch") ?? "new") as
      | "new"
      | "library"
      | "setup",
    notifyOnFinish: localStorage.getItem("rv-notify") !== "0",
    keepWindowOnTop: localStorage.getItem("rv-on-top") === "1",
  });

  watch(
    () => general.value.startOnLaunch,
    (v) => localStorage.setItem("rv-start-on-launch", v),
  );
  watch(
    () => general.value.notifyOnFinish,
    (v) => localStorage.setItem("rv-notify", v ? "1" : "0"),
  );

  /** Pencereyi diğer uygulamaların üstünde tutma tercihi anında uygulanır. */
  watch(
    () => general.value.keepWindowOnTop,
    async (v) => {
      localStorage.setItem("rv-on-top", v ? "1" : "0");
      if (!isTauri()) return;
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        await getCurrentWindow().setAlwaysOnTop(v);
        windowError.value = "";
      } catch (error) {
        // Sessizce yutmak, çalışmayan bir ayarı çalışıyor gibi gösteriyordu.
        windowError.value = `Pencere ayarı uygulanamadı: ${error}`;
        general.value.keepWindowOnTop = !v;
      }
    },
    { immediate: true },
  );

  /** Açılışta gidilecek rota. */
  const startRoute = computed(() => {
    if (general.value.startOnLaunch === "library") return "/kitaplik";
    if (general.value.startOnLaunch === "setup") return "/kurulum";
    return "/yeni/kaynak";
  });

  const advanced = ref({
    /** Ağ isteklerinin zaman aşımı — yavaş bağlantılarda artırılabilir. */
    timeoutSec: 30,
  });

  /** Dosya adı şablonunu gerçek değerlerle doldurur ve dosya sistemi için güvenli hale getirir. */
  function outputFileName(subreddit: string, title: string) {
    const slug = title
      .toLocaleLowerCase("tr")
      .replace(/[çğıöşü]/g, (c) => ({ ç: "c", ğ: "g", ı: "i", ö: "o", ş: "s", ü: "u" })[c] ?? c)
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 48);

    const date = new Date().toISOString().slice(0, 10);
    const name = video.value.nameTemplate
      .replace("{tarih}", date)
      .replace("{subreddit}", subreddit.toLowerCase())
      .replace("{baslik-kisa}", slug.split("-").slice(0, 5).join("-"))
      .replace("{baslik}", slug)
      // Şablonda kalan geçersiz karakterleri ayıkla
      .replace(/[/\\:*?"<>|]/g, "-");

    return `${name || "video"}.mp4`;
  }

  // --- Bakım ---
  const maintenanceBusy = ref(false);
  const maintenanceMessage = ref("");
  const tempDir = ref("");
  const maintenanceBytes = ref({ temp: 0, thumbnails: 0, output: 0, jobs: 0 });

  function formatBytes(bytes: number) {
    if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(1)} GB`;
    if (bytes >= 1_048_576) return `${Math.round(bytes / 1_048_576)} MB`;
    return `${Math.round(bytes / 1024)} KB`;
  }

  const maintenanceSummary = computed(() => {
    const m = maintenanceBytes.value;
    if (m.temp + m.thumbnails + m.output === 0) return "hesaplanmadı";
    return `Geçici ${formatBytes(m.temp)} · kapaklar ${formatBytes(m.thumbnails)} · çıktılar ${formatBytes(m.output)} (${m.jobs} iş)`;
  });

  async function refreshMaintenance() {
    if (!isTauri()) return;
    try {
      const info = await maintenanceInfo(video.value.outputDir);
      maintenanceBytes.value = {
        temp: info.temp_bytes,
        thumbnails: info.thumbnail_bytes,
        output: info.output_bytes,
        jobs: info.output_jobs,
      };
      tempDir.value = info.temp_dir;
    } catch {
      /* klasör yoksa sıfır kalır */
    }
  }

  async function clearTempFilesAction() {
    if (!isTauri()) return;
    maintenanceBusy.value = true;
    try {
      const freed = await clearTempFiles();
      maintenanceMessage.value = `Geçici dosyalar temizlendi · ${formatBytes(freed)} kazanıldı`;
      useToastStore().push({ tone: "success", text: maintenanceMessage.value });
      await refreshMaintenance();
    } catch (error) {
      maintenanceMessage.value = String(error);
    } finally {
      maintenanceBusy.value = false;
    }
  }

  async function clearThumbnailCacheAction() {
    if (!isTauri()) return;
    maintenanceBusy.value = true;
    try {
      const freed = await clearThumbnailCache();
      maintenanceMessage.value = `Kapak önbelleği boşaltıldı · ${formatBytes(freed)}`;
      useToastStore().push({ tone: "success", text: maintenanceMessage.value });
      await refreshMaintenance();
    } catch (error) {
      maintenanceMessage.value = String(error);
    } finally {
      maintenanceBusy.value = false;
    }
  }

  /** Geri alınamaz: çıktı klasöründeki bütün işleri siler. */
  async function deleteAllOutputsAction() {
    if (!isTauri()) return;
    const onay = window.confirm(
      "Çıktı klasöründeki bütün videolar, sesler ve kartlar kalıcı olarak silinecek. Devam edilsin mi?",
    );
    if (!onay) return;

    maintenanceBusy.value = true;
    try {
      const removed = await deleteAllOutputs(video.value.outputDir);
      maintenanceMessage.value = `${removed} iş silindi`;
      useToastStore().push({ tone: "warning", text: maintenanceMessage.value });
      await refreshMaintenance();
    } catch (error) {
      maintenanceMessage.value = String(error);
    } finally {
      maintenanceBusy.value = false;
    }
  }

  /** Arka plan videolarının bulunduğu klasör. Varsayılan olarak ana projenin
   *  indirdiği kitaplığı kullanıyoruz; kullanıcı değiştirebilir. */
  const backgroundsDir = ref(
    localStorage.getItem("rv-backgrounds-dir") ??
      "/Volumes/Mac Harici Disk/VibeProject/RedditVideoMakerBot/assets/backgrounds/video",
  );

  watch(backgroundsDir, (value) => {
    try {
      localStorage.setItem("rv-backgrounds-dir", value);
    } catch {
      /* depolama kapalı olabilir */
    }
  });

  const musicDir = ref(
    localStorage.getItem("rv-music-dir") ??
      "/Volumes/Mac Harici Disk/VibeProject/RedditVideoMakerBot/assets/backgrounds/audio",
  );

  watch(musicDir, (value) => {
    try {
      localStorage.setItem("rv-music-dir", value);
    } catch {
      /* depolama kapalı olabilir */
    }
  });

  const video = ref({
    resolution: "1080x1920",
    fps: "30",
    codec: "h264",
    bitrateMbps: 8,
    hardwareAccel: true,
    outputDir: "~/Movies/RVMaker",
    nameTemplate: "{tarih}_{subreddit}_{baslik-kisa}",
    /** Son karttan sonra arka planın tek başına göründüğü kuyruk süresi. */
    outroSec: 5,
  });

  const cacheSizeMb = ref(1840);

  return {
    theme,
    toggleTheme,
    setupItems,
    setupLoading,
    setupError,
    installLog,
    installing,
    runEnvironmentCheck,
    runInstall,
    blockingIssues,
    environmentReady,
    environmentSummary,
    engines,
    xCookieDraft,
    xCookieSaved,
    xCookieBusy,
    xStatus,
    xStatusText,
    xToolInstalled,
    refreshXStatus,
    saveXCredentials,
    removeXCredentials,
    checkXCredentials,
    refreshEngineKeys,
    saveEngineKey,
    removeEngineKey,
    checkEngine,
    reddit,
    redditCookieSaved,
    redditCookieDraft,
    redditCookieBusy,
    redditCookieError,
    refreshCookieStatus,
    saveRedditCookie,
    removeRedditCookie,
    apiKeys,
    defaults,
    translation,
    general,
    windowError,
    startRoute,
    advanced,
    video,
    outputFileName,
    maintenanceBusy,
    maintenanceMessage,
    maintenanceSummary,
    tempDir,
    refreshMaintenance,
    clearTempFiles: clearTempFilesAction,
    clearThumbnailCache: clearThumbnailCacheAction,
    deleteAllOutputs: deleteAllOutputsAction,
    backgroundsDir,
    musicDir,
    cacheSizeMb,
  };
});
