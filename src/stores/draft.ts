import { defineStore } from "pinia";
import { computed, ref, watch } from "vue";
import {
  backgroundThumbnail,
  fetchRedditPost,
  fetchSubreddit,
  fetchAuthorThread,
  fetchTweetThread,
  fileUrl,
  isTauri,
  listAudio,
  listBackgrounds,
  listEngineVoices,
  onTranslateProgress,
  previewSpeech,
  relativeTime,
  searchViralTweets,
  translateTexts,
} from "@/lib/api";
import { useSettingsStore } from "@/stores/settings";
import type { PostSummary, Tweet } from "@/lib/api";
import { sourceLabel } from "@/lib/types";
import type {
  BackgroundAudio,
  BackgroundVideo,
  CommentDraft,
  JobDraft,
  LookSettings,
  RecentItem,
  RedditPost,
  SourceKind,
  XMode,
  TtsEngineId,
  Voice,
} from "@/lib/types";

/** Bir hazır ayarın taşıdığı her şey — video başına yeniden kurulan ayarlar. */
export interface PresetData {
  engineId: TtsEngineId;
  voiceId: string;
  speed: number;
  silenceMs: number;
  maxComments: number;
  sortMode: string;
  look: LookSettings;
}

export interface Preset {
  id: string;
  name: string;
  summary: string;
  commentCount: number;
  approxSeconds: number;
  data?: PresetData;
}

/** Hazırlanmakta olan videonun taslağı: kaynak → içerik → ses → görünüm. */
export const useDraftStore = defineStore("draft", () => {
  const url = ref("");
  const urlTouched = ref(false);

  /** İçeriğin geldiği platform. Hat aynı; yalnızca etiketler değişiyor. */
  const source = ref<SourceKind>("reddit");

  /** X'ten ne alınacağı. Yanıtlar Reddit yorumlarının karşılığı; thread ise
   *  yazarın kendi devam tweet'leri — sırası anlam taşıdığı için karıştırılamaz. */
  const xMode = ref<XMode>((localStorage.getItem("rv-x-mode") as XMode) ?? "replies");

  watch(xMode, (value) => {
    try {
      localStorage.setItem("rv-x-mode", value);
    } catch {
      /* depolama kapalı olabilir */
    }
  });

  const REDDIT_URL = /reddit\.com\/r\/[^/]+\/comments\/[a-z0-9]+/i;
  const X_URL = /(?:twitter|x)\.com\/[^/]+\/status(?:es)?\/\d+/i;

  /** Yapıştırılan bağlantı hangi platforma ait? Tanımadıysa null. */
  const urlKind = computed<SourceKind | null>(() => {
    const value = url.value.trim();
    if (!value) return null;
    if (REDDIT_URL.test(value)) return "reddit";
    if (X_URL.test(value)) return "x";
    return null;
  });

  const urlValid = computed(() => urlKind.value !== null);

  const urlError = computed(() =>
    urlTouched.value && !urlValid.value
      ? "Bu bir Reddit gönderi ya da X tweet bağlantısı değil."
      : "",
  );

  const presets = ref<Preset[]>([]);
  const selectedPresetId = ref("");

  const PRESET_KEY = "rv-presets";
  const LAST_KEY = "rv-last-settings";

  function currentPresetData(): PresetData {
    return {
      engineId: engineId.value,
      voiceId: voiceId.value,
      speed: speed.value,
      silenceMs: silenceMs.value,
      maxComments: maxComments.value,
      sortMode: sortMode.value,
      look: { ...look.value },
    };
  }

  function applyPresetData(data: Partial<PresetData>) {
    if (data.engineId) engineId.value = data.engineId;
    if (data.voiceId) voiceId.value = data.voiceId;
    if (typeof data.speed === "number") speed.value = data.speed;
    if (typeof data.silenceMs === "number") silenceMs.value = data.silenceMs;
    if (typeof data.maxComments === "number") maxComments.value = data.maxComments;
    if (data.sortMode) sortMode.value = data.sortMode;
    if (data.look) look.value = { ...look.value, ...data.look };
  }

  /** Yarım kalan işi sürdürebilmek için taslağın tamamını çıkarır. */
  function jobDraftSnapshot(): JobDraft {
    return {
      version: 1,
      source: source.value,
      xMode: xMode.value,
      post: { ...post.value },
      comments: comments.value.map((c) => ({ ...c })),
      engineId: engineId.value,
      voiceId: voiceId.value,
      speed: speed.value,
      silenceMs: silenceMs.value,
      sortMode: sortMode.value,
      maxComments: maxComments.value,
      look: { ...look.value },
    };
  }

  /** Diskten okunan taslağı geri yükler.
   *
   *  Sürüm tutmuyorsa yüklemiyoruz: eksik alanlarla devam etmek, yarım işi
   *  yanlış ayarlarla sürdürmek demek olurdu. */
  function restoreJobDraft(raw: string): boolean {
    let data: JobDraft;
    try {
      data = JSON.parse(raw) as JobDraft;
    } catch {
      return false;
    }
    if (data?.version !== 1 || !data.post || !Array.isArray(data.comments)) return false;

    source.value = data.source ?? "reddit";
    xMode.value = data.xMode ?? "replies";
    post.value = { ...data.post };
    comments.value = data.comments.map((c) => ({ ...c }));
    engineId.value = data.engineId;
    voiceId.value = data.voiceId;
    speed.value = data.speed;
    silenceMs.value = data.silenceMs;
    sortMode.value = data.sortMode;
    maxComments.value = data.maxComments;
    look.value = { ...look.value, ...data.look };
    // Tohum veriyle çalışmadığımızı söylemezsek hat çekme adımını atlar.
    isMockData.value = false;
    return true;
  }

  /** Son kullanılan ayarları saklar — asıl tekrarı kaldıran şey bu. */
  function rememberLastSettings() {
    try {
      localStorage.setItem(LAST_KEY, JSON.stringify(currentPresetData()));
    } catch {
      /* depolama kapalı olabilir */
    }
  }

  function restoreLastSettings() {
    try {
      const raw = localStorage.getItem(LAST_KEY);
      if (raw) applyPresetData(JSON.parse(raw) as Partial<PresetData>);
    } catch {
      /* bozuk kayıt varsa varsayılanlarla devam */
    }
  }

  function loadPresets() {
    try {
      const raw = localStorage.getItem(PRESET_KEY);
      presets.value = raw ? (JSON.parse(raw) as Preset[]) : [];
    } catch {
      presets.value = [];
    }
  }

  function persistPresets() {
    try {
      localStorage.setItem(PRESET_KEY, JSON.stringify(presets.value));
    } catch {
      /* depolama kapalı olabilir */
    }
  }

  /** Şu anki ayarlardan adlandırılmış bir hazır ayar oluşturur. */
  function savePreset(name: string) {
    const trimmed = name.trim();
    if (!trimmed) return;

    const data = currentPresetData();
    const preset: Preset = {
      id: `hazir-${Date.now()}`,
      name: trimmed,
      summary: presetSummary(data),
      commentCount: data.maxComments,
      approxSeconds: 0,
      data,
    };

    presets.value = [...presets.value.filter((p) => p.name !== trimmed), preset];
    selectedPresetId.value = preset.id;
    persistPresets();
  }

  function deletePreset(id: string) {
    presets.value = presets.value.filter((p) => p.id !== id);
    if (selectedPresetId.value === id) selectedPresetId.value = "";
    persistPresets();
  }

  function applyPreset(id: string) {
    const preset = presets.value.find((p) => p.id === id);
    if (!preset?.data) return;
    selectedPresetId.value = id;
    applyPresetData(preset.data);
  }

  /** Hazır ayarın kartta görünecek tek satırlık özeti. */
  function presetSummary(data: PresetData) {
    const voice = voices.value.find((v) => v.id === data.voiceId)?.name ?? data.voiceId;
    const bg =
      backgroundVideos.value.find((b) => b.id === data.look.backgroundVideoId)?.label ??
      "düz zemin";
    return `${data.engineId} · ${voice || "varsayılan"} · ${bg}`;
  }

  const recent = ref<RecentItem[]>([
    { url: "r/tifu/comments/1e8x2m/bugun-isyerinde", when: "dün", source: "reddit" },
    { url: "r/AskReddit/comments/1d1p0q/en-tuhaf-is-gorusmesi", when: "3 gün önce", source: "reddit" },
    { url: "r/relationships/comments/1c9v4k/komsum-her-gece", when: "5 gün önce", source: "reddit" },
  ]);

  const post = ref<RedditPost>({
    id: "1f2k9x",
    subreddit: "AskReddit",
    title: "What is the thing most people are wrong about that nobody corrects?",
    titleTr: "İnsanların en çok yanıldığı, ama kimsenin düzeltmediği şey nedir?",
    url: "reddit.com/r/AskReddit/comments/1f2k9x/",
    upvotes: 48200,
    commentCount: 6104,
    fetchedCount: 24,
    postedAgo: "4 sa önce",
  });

  /** Kartlarda ve günlükte görünen kaynak etiketi: `r/AskReddit` / `@Fenerbahce`. */
  const sourceHandle = computed(() => sourceLabel(source.value, post.value.subreddit));

  /** X'te "oy" ve "yorum" yok; sayaçların adı kaynağa göre değişiyor. */
  const scoreWord = computed(() => (source.value === "x" ? "beğeni" : "oy"));
  const replyWord = computed(() => {
    if (source.value !== "x") return "yorum";
    return xMode.value === "thread" ? "tweet" : "yanıt";
  });

  const sortMode = ref("top");
  const maxComments = ref(10);

  const comments = ref<CommentDraft[]>([
    {
      id: "c1",
      author: "u/sessiz_kunduz",
      upvotes: 2100,
      original:
        "Most people think you need to rinse rice until the water runs clear, but that strips the starch you actually want.",
      translated:
        "Çoğu insan pirinci suyu berraklaşana kadar yıkamak gerektiğini sanıyor, oysa bu tam da işine yarayacak nişastayı alıp götürüyor.",
      selected: true,
    },
    {
      id: "c2",
      author: "u/gece_vardiyasi",
      upvotes: 1700,
      original: "That bats are blind. They see fine — echolocation is just an extra sense on top.",
      translated:
        "Yarasaların kör olduğu. Gayet iyi görüyorlar — ekolokasyon sadece üstüne eklenen fazladan bir duyu.",
      selected: true,
    },
    {
      id: "c3",
      author: "u/mavi_pencere",
      upvotes: 980,
      original: "Goldfish have a three second memory. They can be trained to run mazes weeks later.",
      translated:
        "Japon balıklarının üç saniyelik hafızası olduğu. Haftalar sonra bile labirent çözmeyi hatırlayacak şekilde eğitilebiliyorlar.",
      selected: true,
    },
    {
      id: "c4",
      author: "u/kirmizi_defter",
      upvotes: 640,
      original: "The Great Wall is visible from space. It is not, not with the naked eye.",
      translated: "Çin Seddi'nin uzaydan görüldüğü. Görünmüyor, en azından çıplak gözle.",
      selected: false,
    },
    {
      id: "c5",
      author: "u/uzak_ada",
      upvotes: 512,
      original: "Cracking your knuckles causes arthritis. Decades of studies say otherwise.",
      translated:
        "Parmak çıtlatmanın eklem iltihabına yol açtığı. Onlarca yıllık çalışmalar tam tersini söylüyor.",
      selected: true,
    },
    {
      id: "c6",
      author: "u/derin_kuyu",
      upvotes: 430,
      original: "We only use ten percent of our brains. Imaging shows nearly all of it is active.",
      translated:
        "Beynimizin yalnızca yüzde onunu kullandığımız. Görüntüleme neredeyse tamamının etkin olduğunu gösteriyor.",
      selected: true,
    },
    {
      id: "c7",
      author: "u/sari_bisiklet",
      upvotes: 388,
      original: "Lightning never strikes the same place twice. Tall towers get hit dozens of times a year.",
      translated:
        "Yıldırımın aynı yere iki kez düşmediği. Yüksek kuleler yılda onlarca kez yıldırım alıyor.",
      selected: true,
    },
    {
      id: "c8",
      author: "u/gri_kedi",
      upvotes: 260,
      original: "Sugar makes children hyperactive. Controlled trials have never reproduced it.",
      translated:
        "Şekerin çocukları hiperaktif yaptığı. Kontrollü çalışmalar bunu bir kez bile doğrulayamadı.",
      selected: false,
    },
  ]);

  const selectedComments = computed(() => comments.value.filter((c) => c.selected));

  /** Üretime engel bir durum varsa sebebi; yoksa boş.
   *
   *  Tek kartlık video işe yaramıyor: seslendirme biter bitmez görüntü kesiliyor
   *  ve ortaya "başlık okundu, bitti" diye bir şey çıkıyor. Bunu render
   *  başladıktan sonra günlüğe bir satır düşerek değil, düğmeyi kapatarak
   *  söylemek gerekiyor. */
  const blockReason = computed(() => {
    if (selectedComments.value.length === 0) {
      if (post.value.fetchedCount > 0) {
        return `Hiç ${replyWord.value} seçili değil; en az bir tane seç.`;
      }
      return source.value === "x" && xMode.value === "thread"
        ? "Bu tweet'in devamı yok — tek başına atılmış. Yanıtlar moduna geçebilirsin."
        : `Bu kaynaktan hiç ${replyWord.value} gelmedi; videoda yalnızca başlık kartı kalırdı.`;
    }
    return "";
  });

  const canProduce = computed(() => blockReason.value === "");

  const totalChars = computed(() =>
    selectedComments.value.reduce((sum, c) => sum + c.translated.length, 0),
  );

  /** Google TTS ile ölçüldü: 251 karakterlik Türkçe metin 21,6 saniye sürdü,
   *  yani saniyede ~11,6 karakter. Başlık için 6 saniye sabit ekleniyor. */
  const CHARS_PER_SECOND = 11.6;

  function estimateSeconds(text: string) {
    return Math.round(text.length / CHARS_PER_SECOND);
  }

  const estimatedSeconds = computed(
    () => 6 + selectedComments.value.reduce((sum, c) => sum + estimateSeconds(c.translated), 0),
  );

  const targetSeconds = ref(90);

  /** Kullanıcının elle eklediği yorum. Çeviri istemez, olduğu gibi kullanılır. */
  function addManualComment(text: string, author = "u/sen") {
    const trimmed = text.trim();
    if (!trimmed) return;
    comments.value.push({
      id: `elle-${Date.now()}`,
      author,
      upvotes: 0,
      original: trimmed,
      translated: trimmed,
      selected: true,
      manual: true,
    });
  }

  function removeComment(id: string) {
    comments.value = comments.value.filter((c) => c.id !== id);
  }

  /** Sıralama seçimini uygular. Reddit'ten gelen sıra "top"; diğerleri yerel. */
  function applySort() {
    const list = [...comments.value];
    if (sortMode.value === "top") {
      list.sort((a, b) => b.upvotes - a.upvotes);
    } else if (sortMode.value === "short") {
      list.sort((a, b) => a.translated.length - b.translated.length);
    } else if (sortMode.value === "long") {
      list.sort((a, b) => b.translated.length - a.translated.length);
    }
    // "fetched" seçiliyse Reddit'ten geldiği sıra korunur; dokunmuyoruz.
    if (sortMode.value !== "fetched") comments.value = list;
  }

  function toggleComment(id: string) {
    const c = comments.value.find((x) => x.id === id);
    if (c) c.selected = !c.selected;
  }

  function moveComment(id: string, delta: number) {
    const i = comments.value.findIndex((c) => c.id === id);
    const target = i + delta;
    if (i < 0 || target < 0 || target >= comments.value.length) return;
    const [item] = comments.value.splice(i, 1);
    comments.value.splice(target, 0, item);
  }

  const editingId = ref<string | null>(null);
  const editBuffer = ref("");

  function startEdit(id: string) {
    const c = comments.value.find((x) => x.id === id);
    if (!c) return;
    editingId.value = id;
    editBuffer.value = c.translated;
  }

  function saveEdit() {
    const c = comments.value.find((x) => x.id === editingId.value);
    if (c) c.translated = editBuffer.value;
    editingId.value = null;
  }

  function cancelEdit() {
    editingId.value = null;
  }

  // --- Gönderi çekme ---
  const isLoading = ref(false);
  const loadError = ref("");
  /** Sahte tohum veriyle mi çalışıyoruz? Tarayıcı geliştirmesinde true kalır. */
  const isMockData = ref(true);

  async function loadPost(rawUrl: string) {
    loadError.value = "";

    if (!isTauri()) {
      // Tarayıcıda arka uç yok; tohum veriyle devam et.
      isMockData.value = true;
      return true;
    }

    isLoading.value = true;
    try {
      const settings = useSettingsStore();
      const data = await fetchRedditPost(
        settings.reddit.clientId,
        rawUrl,
        maxComments.value,
      );

      post.value = {
        id: data.id,
        subreddit: data.subreddit,
        title: data.title,
        titleTr: "",
        url: data.url,
        upvotes: data.upvotes,
        commentCount: data.comment_count,
        fetchedCount: data.comments.length,
        postedAgo: relativeTime(data.created_utc),
        removalNote: data.removal_note ?? undefined,
      };

      comments.value = data.comments.map((c) => ({
        id: c.id,
        author: c.author,
        upvotes: c.upvotes,
        original: c.body,
        translated: "",
        selected: true,
      }));

      source.value = "reddit";
      isMockData.value = false;
      recent.value = [
        { url: `r/${data.subreddit}/comments/${data.id}`, when: "az önce", source: "reddit" as const },
        ...recent.value.filter((r) => !r.url.includes(data.id)),
      ].slice(0, 5);

      return true;
    } catch (error) {
      loadError.value = String(error);
      return false;
    } finally {
      isLoading.value = false;
    }
  }

  /** Bir tweet'i ve yanıtlarını çekip taslağa yerleştirir.
   *  Reddit gönderisiyle aynı biçimi dolduruyoruz; hattın geri kalanı
   *  kaynağın hangisi olduğunu bilmek zorunda kalmıyor. */
  async function loadTweet(rawUrl: string) {
    loadError.value = "";

    if (!isTauri()) {
      isMockData.value = true;
      return true;
    }

    isLoading.value = true;
    try {
      const data =
        xMode.value === "thread"
          ? await fetchAuthorThread(rawUrl, maxComments.value)
          : await fetchTweetThread(rawUrl, maxComments.value);
      const handle = data.tweet.author.replace(/^@/, "");

      post.value = {
        id: data.tweet.id,
        subreddit: handle,
        title: data.tweet.text,
        titleTr: "",
        url: data.tweet.url,
        upvotes: data.tweet.likes,
        commentCount: data.tweet.replies,
        fetchedCount: data.replies.length,
        postedAgo: relativeTime(data.tweet.created_utc),
      };

      comments.value = data.replies.map((r) => ({
        id: r.id,
        author: r.author,
        upvotes: r.likes,
        original: r.text,
        translated: "",
        selected: true,
      }));

      source.value = "x";
      // Zincirin sırası anlam taşıyor; beğeniye göre dizmek videoyu bozar.
      if (xMode.value === "thread") sortMode.value = "fetched";
      isMockData.value = false;
      recent.value = [
        { url: `@${handle}/${data.tweet.id}`, when: "az önce", source: "x" as const },
        ...recent.value.filter((r) => !r.url.includes(data.tweet.id)),
      ].slice(0, 5);

      return true;
    } catch (error) {
      loadError.value = String(error);
      return false;
    } finally {
      isLoading.value = false;
    }
  }

  /** Yapıştırılan bağlantıyı tanıyıp doğru çekiciye yönlendirir. */
  async function loadSource(rawUrl: string) {
    return urlKind.value === "x" ? loadTweet(rawUrl) : loadPost(rawUrl);
  }

  // --- Subreddit tarayıcı ---
  const browseSubreddit = ref(localStorage.getItem("rv-last-subreddit") ?? "AskReddit");
  const browseSort = ref("hot");
  const browseLimit = ref(25);
  const browsePosts = ref<PostSummary[]>([]);
  const browseLoading = ref(false);
  const browseError = ref("");
  const hideNsfw = ref(true);

  const visiblePosts = computed(() =>
    hideNsfw.value ? browsePosts.value.filter((p) => !p.is_nsfw) : browsePosts.value,
  );

  async function loadSubreddit() {
    if (!isTauri()) return;
    browseLoading.value = true;
    browseError.value = "";
    try {
      const settings = useSettingsStore();
      browsePosts.value = await fetchSubreddit(
        settings.reddit.clientId,
        browseSubreddit.value,
        browseSort.value,
        browseLimit.value,
      );
      localStorage.setItem("rv-last-subreddit", browseSubreddit.value.trim());
    } catch (error) {
      browseError.value = String(error);
      browsePosts.value = [];
    } finally {
      browseLoading.value = false;
    }
  }

  // --- X viral tarayıcı ---
  const viralKeyword = ref("");
  const viralLang = ref(localStorage.getItem("rv-viral-lang") ?? "tr");
  /** Son kaç saat içinde atılmış olsun. Segmented dizeyle çalışıyor. */
  const viralHours = ref("24");
  const viralMinFaves = ref(2000);
  const viralOnlyText = ref(true);
  const viralLimit = ref(25);
  const viralTweets = ref<Tweet[]>([]);
  const viralLoading = ref(false);
  const viralError = ref("");

  /** Yanıtı olmayan tweet'ten video çıkmaz — kart sayısı bire iner. */
  const MIN_VIRAL_REPLIES = 10;

  const viralUsable = computed(() =>
    viralTweets.value.filter((t) => t.replies >= MIN_VIRAL_REPLIES),
  );

  async function loadViral() {
    if (!isTauri()) return;
    viralLoading.value = true;
    viralError.value = "";
    try {
      viralTweets.value = await searchViralTweets({
        keyword: viralKeyword.value.trim(),
        lang: viralLang.value,
        min_faves: viralMinFaves.value,
        min_replies: MIN_VIRAL_REPLIES,
        hours: Number(viralHours.value),
        only_text: viralOnlyText.value,
        limit: viralLimit.value,
      });
      localStorage.setItem("rv-viral-lang", viralLang.value);
    } catch (error) {
      viralError.value = String(error);
      viralTweets.value = [];
    } finally {
      viralLoading.value = false;
    }
  }

  // --- Çeviri ---
  const isTranslating = ref(false);
  const translateError = ref("");
  const translateDone = ref(0);
  const translateTotal = ref(0);

  const needsTranslation = computed(
    () => !post.value.titleTr || comments.value.some((c) => !c.translated),
  );

  async function translateAll(targetLang = "tr") {
    if (!isTauri() || isMockData.value || isTranslating.value) return;

    // "Çevirme (özgün dilde bırak)" seçiliyse metni olduğu gibi taşı.
    // Türkçe kaynaklarda (X gündemi gibi) çeviri zaten zarar veriyor.
    if (useSettingsStore().translation.provider === "none") {
      post.value.titleTr = post.value.title;
      comments.value.forEach((c) => {
        c.translated = c.original;
      });
      return;
    }

    isTranslating.value = true;
    translateError.value = "";
    translateDone.value = 0;

    const texts = [post.value.title, ...comments.value.map((c) => c.original)];
    translateTotal.value = texts.length;

    const unlisten = await onTranslateProgress((p) => {
      translateDone.value = p.done;
    });

    try {
      // Kaynak dili açıkça vermek çeviri kalitesini belirgin biçimde artırıyor.
      const settings = useSettingsStore();
      const result = await translateTexts(texts, "en", targetLang, {
        skipIfSameLanguage: settings.translation.skipIfSameLanguage,
        glossary: settings.translation.glossary,
        timeoutSec: settings.advanced.timeoutSec,
      });
      post.value.titleTr = result[0] ?? post.value.title;
      comments.value.forEach((c, i) => {
        c.translated = result[i + 1] ?? c.original;
      });
    } catch (error) {
      translateError.value = String(error);
      // Çeviri düşerse özgün metinle devam et; kullanıcı elle düzeltebilir.
      post.value.titleTr = post.value.titleTr || post.value.title;
      comments.value.forEach((c) => {
        if (!c.translated) c.translated = c.original;
      });
    } finally {
      unlisten();
      isTranslating.value = false;
    }
  }

  // --- Ses ---
  const engineId = ref<TtsEngineId>("googletranslate");
  const voiceId = ref("tr-google");
  const voiceFilter = ref("all");
  const voiceSearch = ref("");
  const speed = ref(1.15);
  const silenceMs = ref(320);
  const playingVoiceId = ref<string | null>("derya");

  /** Seçili motorun gerçek sesleri — motor değişince yeniden yükleniyor. */
  const voices = ref<Voice[]>([]);
  const voicesLoading = ref(false);
  const voicesError = ref("");

  async function loadVoices(targetLang = "tr") {
    if (!isTauri()) return;
    voicesLoading.value = true;
    voicesError.value = "";
    try {
      const found = await listEngineVoices(engineId.value, targetLang);
      voices.value = found.map((v) => ({
        id: v.id,
        name: v.name,
        detail: v.detail,
        engine: engineId.value,
        waveform: waveformFor(v.id),
        own: v.own,
        usable: v.usable,
      }));

      // Seçili ses bu motorda yoksa ya da kullanılamıyorsa, kullanılabilir
      // ilk sese düş — yoksa üretim ortasında 402 alınıyor.
      const secili = voices.value.find((v) => v.id === voiceId.value);
      if (!secili || secili.usable === false) {
        voiceId.value =
          voices.value.find((v) => v.usable !== false)?.id ?? voices.value[0]?.id ?? "";
      }
    } catch (error) {
      voices.value = [];
      voicesError.value = String(error);
    } finally {
      voicesLoading.value = false;
    }
  }

  /** Ses kimliğinden sabit ama çeşitli görünen bir dalga formu üretir. */
  function waveformFor(seed: string): number[] {
    let hash = 0;
    for (const ch of seed) hash = (hash * 31 + ch.charCodeAt(0)) >>> 0;
    return Array.from({ length: 14 }, (_, i) => {
      hash = (hash * 1103515245 + 12345) >>> 0;
      return 0.3 + ((hash >>> (i % 8)) % 70) / 100;
    });
  }

  const selectedVoice = computed(
    () => voices.value.find((v) => v.id === voiceId.value) ?? voices.value[0],
  );

  /** Seçili motorun sesleri. Bağlanmamış motorlar boş liste döndürür. */
  const engineVoices = computed(() =>
    voices.value.filter((v) => v.engine === engineId.value),
  );

  /** Cinsiyet süzgeci ancak motorun kadın/erkek varyantı varsa gösterilir. */
  const hasGenderVariants = computed(
    () =>
      engineVoices.value.some((v) => v.detail.includes("Kadın")) &&
      engineVoices.value.some((v) => v.detail.includes("Erkek")),
  );

  const filteredVoices = computed(() => {
    const q = voiceSearch.value.trim().toLowerCase();
    return engineVoices.value.filter((v) => {
      const matchesQuery =
        !q || v.name.toLowerCase().includes(q) || v.detail.toLowerCase().includes(q);
      const matchesFilter =
        !hasGenderVariants.value ||
        voiceFilter.value === "all" ||
        (voiceFilter.value === "female" && v.detail.includes("Kadın")) ||
        (voiceFilter.value === "male" && v.detail.includes("Erkek"));
      return matchesQuery && matchesFilter;
    });
  });

  /** Seslendirme biçimi — hız ve cümle arası sessizlik hem önizlemede
   *  hem üretimde aynı değerlerle uygulanır. */
  const speechShape = computed(() => ({
    speed: speed.value,
    silence_ms: silenceMs.value,
  }));

  // --- Ses önizleme ---
  const previewBusy = ref(false);
  const previewError = ref("");
  let previewAudio: HTMLAudioElement | null = null;

  async function togglePreview(previewVoiceId: string, targetLang = "tr") {
    if (previewAudio) {
      previewAudio.pause();
      previewAudio = null;
    }

    if (playingVoiceId.value === previewVoiceId) {
      playingVoiceId.value = null;
      return;
    }

    playingVoiceId.value = previewVoiceId;
    previewError.value = "";

    if (!isTauri()) return;

    previewBusy.value = true;
    try {
      const sample =
        selectedComments.value[0]?.translated ||
        "Merhaba, bu bir seslendirme denemesidir.";
      const clip = await previewSpeech(
        engineId.value,
        voiceId.value,
        sample.slice(0, 180),
        targetLang,
        speechShape.value,
      );
      previewAudio = new Audio(fileUrl(clip.path));
      previewAudio.onended = () => {
        playingVoiceId.value = null;
      };
      await previewAudio.play();
    } catch (error) {
      previewError.value = String(error);
      playingVoiceId.value = null;
    } finally {
      previewBusy.value = false;
    }
  }

  // --- Görünüm ---
  const look = ref<LookSettings>({
    backgroundVideoId: "",
    backgroundAudioId: "silent",
    audioVolume: 0.15,
    cardTheme: "dark",
    fontSize: 40,
    cardWidth: 86,
  });

  /** Diskte gerçekten bulunan arka planlar. Boşsa düz zemin kullanılır. */
  const backgroundVideos = ref<BackgroundVideo[]>([]);
  const backgroundPaths = ref<Record<string, string>>({});
  /** Arka plan kapak görselleri — video kimliğinden webview adresine. */
  const backgroundThumbs = ref<Record<string, string>>({});
  const backgroundsLoading = ref(false);

  async function loadBackgrounds() {
    if (!isTauri()) return;
    backgroundsLoading.value = true;
    try {
      const settings = useSettingsStore();
      const found = await listBackgrounds(settings.backgroundsDir);

      backgroundVideos.value = found.map((b) => ({
        id: b.id,
        label: b.label,
        detail:
          b.width > 0
            ? `${b.width}×${b.height} · ${Math.round(b.duration_sec / 60)} dk`
            : "yerel dosya",
        local: true,
      }));

      backgroundPaths.value = Object.fromEntries(found.map((b) => [b.id, b.path]));

      // Kapaklar arka planda üretiliyor; biri başarısız olursa diğerleri etkilenmiyor.
      for (const bg of found) {
        backgroundThumbnail(bg.path)
          .then((thumb) => {
            backgroundThumbs.value = {
              ...backgroundThumbs.value,
              [bg.id]: fileUrl(thumb),
            };
          })
          .catch(() => {
            /* kapak üretilemezse desenli zemin kalır */
          });
      }

      // Seçili arka plan artık yoksa ilkine düş.
      if (!backgroundPaths.value[look.value.backgroundVideoId]) {
        look.value.backgroundVideoId = backgroundVideos.value[0]?.id ?? "";
      }
    } finally {
      backgroundsLoading.value = false;
    }
  }

  /** Kullanıcının seçtiği yerel dosyayı listeye ekler. */
  function addLocalBackground(path: string) {
    const name = path.split("/").pop() ?? "yerel dosya";
    const id = `yerel-${name}`;
    if (!backgroundPaths.value[id]) {
      backgroundVideos.value = [
        ...backgroundVideos.value,
        { id, label: name.replace(/\.[^.]+$/, ""), detail: "yerel dosya", local: true },
      ];
      backgroundPaths.value = { ...backgroundPaths.value, [id]: path };
      backgroundThumbnail(path)
        .then((thumb) => {
          backgroundThumbs.value = { ...backgroundThumbs.value, [id]: fileUrl(thumb) };
        })
        .catch(() => {
          /* kapak üretilemezse desenli zemin kalır */
        });
    }
    look.value.backgroundVideoId = id;
  }

  const selectedBackgroundPath = computed(
    () => backgroundPaths.value[look.value.backgroundVideoId] ?? null,
  );

  /** Diskte gerçekten bulunan müzikler. Sessiz seçeneği her zaman var. */
  const backgroundAudios = ref<BackgroundAudio[]>([{ id: "silent", label: "Sessiz" }]);
  const musicPaths = ref<Record<string, string>>({});

  async function loadMusic() {
    if (!isTauri()) return;
    const settings = useSettingsStore();
    const found = await listAudio(settings.musicDir);

    backgroundAudios.value = [
      { id: "silent", label: "Sessiz" },
      ...found.map((a) => ({ id: a.id, label: a.label })),
    ];
    musicPaths.value = Object.fromEntries(found.map((a) => [a.id, a.path]));

    if (
      look.value.backgroundAudioId !== "silent" &&
      !musicPaths.value[look.value.backgroundAudioId]
    ) {
      look.value.backgroundAudioId = "silent";
    }
  }

  /** Sessiz seçiliyse veya dosya yoksa null döner; render müzik eklemez. */
  const selectedMusicPath = computed(
    () => musicPaths.value[look.value.backgroundAudioId] ?? null,
  );

  return {
    url,
    urlTouched,
    urlValid,
    urlError,
    urlKind,
    source,
    xMode,
    sourceHandle,
    scoreWord,
    replyWord,
    blockReason,
    canProduce,
    loadTweet,
    loadSource,
    jobDraftSnapshot,
    restoreJobDraft,
    viralKeyword,
    viralLang,
    viralHours,
    viralMinFaves,
    viralOnlyText,
    viralLimit,
    viralTweets,
    viralUsable,
    viralLoading,
    viralError,
    loadViral,
    presets,
    selectedPresetId,
    loadPresets,
    savePreset,
    deletePreset,
    applyPreset,
    rememberLastSettings,
    restoreLastSettings,
    recent,
    post,
    sortMode,
    maxComments,
    comments,
    selectedComments,
    totalChars,
    estimatedSeconds,
    targetSeconds,
    estimateSeconds,
    toggleComment,
    moveComment,
    addManualComment,
    removeComment,
    applySort,
    editingId,
    editBuffer,
    startEdit,
    saveEdit,
    cancelEdit,
    browseSubreddit,
    browseSort,
    browseLimit,
    browsePosts,
    browseLoading,
    browseError,
    hideNsfw,
    visiblePosts,
    loadSubreddit,
    isLoading,
    loadError,
    isMockData,
    loadPost,
    isTranslating,
    translateError,
    translateDone,
    translateTotal,
    needsTranslation,
    translateAll,
    engineId,
    voiceId,
    voiceFilter,
    voiceSearch,
    speed,
    silenceMs,
    speechShape,
    playingVoiceId,
    voices,
    voicesLoading,
    voicesError,
    loadVoices,
    selectedVoice,
    filteredVoices,
    engineVoices,
    hasGenderVariants,
    previewBusy,
    previewError,
    togglePreview,
    backgroundVideos,
    backgroundPaths,
    backgroundThumbs,
    backgroundsLoading,
    loadBackgrounds,
    addLocalBackground,
    selectedBackgroundPath,
    backgroundAudios,
    musicPaths,
    loadMusic,
    selectedMusicPath,
    look,
  };
});
