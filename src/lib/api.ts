import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Tauri kabuğu içinde mi çalışıyoruz? Tarayıcıda `npm run dev` ile
 *  geliştirirken sahte verilerle devam edebilmek için gerekiyor. */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export interface RawComment {
  id: string;
  author: string;
  upvotes: number;
  body: string;
}

export interface RawPost {
  id: string;
  subreddit: string;
  title: string;
  selftext: string;
  url: string;
  upvotes: number;
  comment_count: number;
  is_nsfw: boolean;
  created_utc: number;
  /** Gönderi kaldırılmış/silinmişse açıklama; sağlamsa null. */
  removal_note: string | null;
  comments: RawComment[];
}

// --- X (Twitter) ---

export interface Tweet {
  id: string;
  author: string;
  author_name: string;
  text: string;
  likes: number;
  retweets: number;
  replies: number;
  created_utc: number;
  url: string;
}

export function setXCredentials(raw: string): Promise<void> {
  return invoke<void>("set_x_credentials", { raw });
}

export function clearXCredentials(): Promise<void> {
  return invoke<void>("clear_x_credentials");
}

export function xCredentialsPresent(): Promise<boolean> {
  return invoke<boolean>("x_credentials_present");
}

/** Çerezlerin geçerliliğini sınar; bağlanılan hesabı döndürür. */
export function verifyXCredentials(): Promise<string> {
  return invoke<string>("verify_x_credentials");
}

export interface Reply {
  id: string;
  author: string;
  text: string;
  likes: number;
}

/** Tweet'in yanıtlarını getirir — video için "yorumlar" bunlar. */
export function fetchTweetReplies(url: string, limit: number): Promise<Reply[]> {
  return invoke<Reply[]>("fetch_tweet_replies", { url, limit });
}

/** X entegrasyonunun dayandığı `bird` aracı kurulu mu? */
export function birdInstalled(): Promise<boolean> {
  return invoke<boolean>("bird_installed");
}

/** Tweet içeriğini getirir — kimlik gerekmez. */
export function fetchTweet(url: string): Promise<Tweet> {
  return invoke<Tweet>("fetch_tweet", { url });
}

export interface TweetThread {
  tweet: Tweet;
  replies: Reply[];
}

/** Tweet'i ve yanıtlarını birlikte getirir — gönderi + yorum karşılığı. */
export function fetchTweetThread(url: string, limit: number): Promise<TweetThread> {
  return invoke<TweetThread>("fetch_tweet_thread", { url, limit });
}

/** Bir kullanıcının kendi thread'ini getirir — kök tweet + yazarın devamı. */
export function fetchAuthorThread(url: string, limit: number): Promise<TweetThread> {
  return invoke<TweetThread>("fetch_author_thread", { url, limit });
}

/** "Viral" ekranının arama ölçütleri — X'in arama sözdizimine çevriliyor. */
export interface ViralQuery {
  keyword: string;
  lang: string;
  min_faves: number;
  min_replies: number;
  hours: number;
  only_text: boolean;
  limit: number;
}

/** Ölçütlere uyan viral tweet'leri getirir. */
export function searchViralTweets(query: ViralQuery): Promise<Tweet[]> {
  return invoke<Tweet[]>("search_viral_tweets", { query });
}

export interface PostSummary {
  id: string;
  subreddit: string;
  title: string;
  url: string;
  upvotes: number;
  comment_count: number;
  created_utc: number;
  is_nsfw: boolean;
  is_video: boolean;
  is_self: boolean;
}

/** Bir subreddit'in gönderilerini listeler. */
export function fetchSubreddit(
  clientId: string,
  subreddit: string,
  sort: string,
  limit: number,
): Promise<PostSummary[]> {
  return invoke<PostSummary[]>("fetch_subreddit", { clientId, subreddit, sort, limit });
}

export interface EnvCheck {
  id: string;
  label: string;
  detail: string;
  state: "ready" | "missing" | "warning";
  required: boolean;
  fix_hint: string;
  fix_command: string;
}

export interface EnvParams {
  backgrounds_dir: string;
  output_dir: string;
  reddit_configured: boolean;
}

export function checkEnvironment(params: EnvParams): Promise<EnvCheck[]> {
  return invoke<EnvCheck[]>("check_environment", { params });
}

export function installFfmpeg(): Promise<void> {
  return invoke<void>("install_ffmpeg");
}

export interface InstallProgress {
  line: string;
  done: boolean;
  ok: boolean;
}

export function onInstallProgress(
  handler: (p: InstallProgress) => void,
): Promise<UnlistenFn> {
  return listen<InstallProgress>("install-progress", (e) => handler(e.payload));
}

export function fetchRedditPost(
  clientId: string,
  url: string,
  limit: number,
): Promise<RawPost> {
  return invoke<RawPost>("fetch_reddit_post", { clientId, url, limit });
}

/** Oturum çerezini anahtar zincirine yazar. Boş değer kaydı siler. */
export function setRedditCookie(cookie: string): Promise<void> {
  return invoke<void>("set_reddit_cookie", { cookie });
}

export function clearRedditCookie(): Promise<void> {
  return invoke<void>("clear_reddit_cookie");
}

/** Yalnızca varlığını söyler; çerezin kendisi arayüze hiç dönmez. */
export function redditCookiePresent(): Promise<boolean> {
  return invoke<boolean>("reddit_cookie_present");
}

/** Ayarlar → Çeviri sekmesinden gelen seçenekler. */
export interface TranslateOptions {
  skipIfSameLanguage: boolean;
  glossary: string;
  timeoutSec: number;
}

export function translateTexts(
  texts: string[],
  source: string,
  target: string,
  options: TranslateOptions,
): Promise<string[]> {
  return invoke<string[]>("translate_texts", { texts, source, target, options });
}

export interface TranslateProgress {
  done: number;
  total: number;
}

export function onTranslateProgress(
  handler: (progress: TranslateProgress) => void,
): Promise<UnlistenFn> {
  return listen<TranslateProgress>("translate-progress", (event) => handler(event.payload));
}

export interface Clip {
  id: string;
  path: string;
  duration_sec: number;
  /** Diskte hazır bulunup yeniden üretilmeden kullanıldı mı? */
  reused: boolean;
}

export interface SpeechItem {
  id: string;
  text: string;
}

export type EngineId = "googletranslate" | "elevenlabs" | "openai" | "gemini" | "system";

export interface VoiceInfo {
  id: string;
  name: string;
  detail: string;
  engine: string;
}

/** Motor anahtarını sistem anahtar zincirine yazar. Boş değer kaydı siler. */
export function setEngineKey(engine: EngineId, key: string): Promise<void> {
  return invoke<void>("set_engine_key", { engine, key });
}

export function clearEngineKey(engine: EngineId): Promise<void> {
  return invoke<void>("clear_engine_key", { engine });
}

/** Yalnızca varlığını söyler; anahtarın kendisi arayüze dönmez. */
export function engineKeyPresent(engine: EngineId): Promise<boolean> {
  return invoke<boolean>("engine_key_present", { engine });
}

export function listEngineVoices(engine: EngineId, lang: string): Promise<VoiceInfo[]> {
  return invoke<VoiceInfo[]>("list_engine_voices", { engine, lang });
}

/** Motoru kısa bir istekle sınar; başarılıysa özet döner. */
export function testEngine(engine: EngineId): Promise<string> {
  return invoke<string>("test_engine", { engine });
}

export interface SpeechShape {
  speed: number;
  silence_ms: number;
}

/** Seçili yorumları seslendirir; ilerleme `tts-progress` olayıyla gelir. */
export function onQuotaWait(handler: (message: string) => void): Promise<UnlistenFn> {
  return listen<string>("tts-quota-wait", (e) => handler(e.payload));
}

export function onTtsFallback(handler: (message: string) => void): Promise<UnlistenFn> {
  return listen<string>("tts-fallback", (e) => handler(e.payload));
}

export function synthesizeSpeech(
  items: SpeechItem[],
  engine: EngineId,
  voice: string,
  lang: string,
  outDir: string,
  shape: SpeechShape,
  fallbackOnQuota: boolean,
  /** X zincirlerinde `🧵1/6` gibi sıra işaretleri okunmasın diye. */
  stripThreadMarkers: boolean,
): Promise<Clip[]> {
  return invoke<Clip[]>("synthesize_speech", {
    items,
    engine,
    voice,
    lang,
    outDir,
    shape,
    fallbackOnQuota,
    stripThreadMarkers,
  });
}

/** Ses ayarları ekranındaki tek seferlik önizleme. */
export function previewSpeech(
  engine: EngineId,
  voice: string,
  text: string,
  lang: string,
  shape: SpeechShape,
): Promise<Clip> {
  return invoke<Clip>("preview_speech", { engine, voice, text, lang, shape });
}

/** Canvas'ta üretilen kart görselini diske yazar, dosya yolunu döndürür. */
export async function saveCard(outDir: string, id: string, blob: Blob): Promise<string> {
  const bytes = Array.from(new Uint8Array(await blob.arrayBuffer()));
  return invoke<string>("save_card", { outDir, id, data: bytes });
}

export interface BackgroundEntry {
  id: string;
  label: string;
  path: string;
  width: number;
  height: number;
  duration_sec: number;
}

/** Klasördeki gerçek arka plan videolarını listeler. */
export function listBackgrounds(dir: string): Promise<BackgroundEntry[]> {
  return invoke<BackgroundEntry[]>("list_backgrounds", { dir });
}

/** Videonun ilk karesinden kapak görseli üretir; yolu döndürür. */
export function backgroundThumbnail(videoPath: string): Promise<string> {
  return invoke<string>("background_thumbnail", { videoPath });
}

/** Klasördeki gerçek müzik dosyalarını listeler. */
export function listAudio(dir: string): Promise<BackgroundEntry[]> {
  return invoke<BackgroundEntry[]>("list_audio", { dir });
}

export interface RenderSegment {
  card_path: string;
  audio_path: string;
  duration_sec: number;
}

export interface RenderOptions {
  segments: RenderSegment[];
  background_path: string | null;
  music_path: string | null;
  music_volume: number;
  fps: number;
  out_path: string;
  resolution: string;
  codec: string;
  bitrate_mbps: number;
  hardware_accel: boolean;
  outro_sec: number;
}

export interface RenderResult {
  path: string;
  duration_sec: number;
  size_bytes: number;
}

export function renderVideo(options: RenderOptions): Promise<RenderResult> {
  return invoke<RenderResult>("render_video", { options });
}

/** Pencere arkadayken görünsün diye sistem bildirimi gönderir. */
export async function notifySystem(title: string, body: string): Promise<void> {
  try {
    const {
      isPermissionGranted,
      requestPermission,
      sendNotification,
    } = await import("@tauri-apps/plugin-notification");

    let granted = await isPermissionGranted();
    if (!granted) {
      granted = (await requestPermission()) === "granted";
    }
    if (granted) sendNotification({ title, body });
  } catch {
    /* bildirim izni yoksa toast zaten görünüyor */
  }
}

/** Çalışan üretimi durdurur; ffmpeg süreci de sonlandırılır. */
export function cancelJob(): Promise<void> {
  return invoke<void>("cancel_job");
}

export function resetCancel(): Promise<void> {
  return invoke<void>("reset_cancel");
}

export interface MaintenanceInfo {
  temp_bytes: number;
  thumbnail_bytes: number;
  output_bytes: number;
  output_jobs: number;
  temp_dir: string;
}

export function maintenanceInfo(outputDir: string): Promise<MaintenanceInfo> {
  return invoke<MaintenanceInfo>("maintenance_info", { outputDir });
}

export function clearTempFiles(): Promise<number> {
  return invoke<number>("clear_temp_files");
}

export function clearThumbnailCache(): Promise<number> {
  return invoke<number>("clear_thumbnail_cache");
}

/** Tek bir işi kalıcı olarak siler. */
export function deleteJob(dir: string): Promise<void> {
  return invoke<void>("delete_job", { dir });
}

export function deleteAllOutputs(outputDir: string): Promise<number> {
  return invoke<number>("delete_all_outputs", { outputDir });
}

export interface JobEntry {
  id: string;
  dir: string;
  video_path: string;
  has_video: boolean;
  size_bytes: number;
  duration_sec: number;
  created_ms: number;
  card_count: number;
  info: string;
  /** `taslak.json` içeriği; yoksa boş. Yarım işi sürdürmek için gerekli. */
  draft: string;
  /** Üretilmiş ses parçası sayısı. */
  clip_count: number;
}

/** Yarım kalırsa sürdürebilmek için taslağı iş klasörüne yazar. */
export function saveJobDraft(outDir: string, data: string): Promise<void> {
  return invoke<void>("save_job_draft", { outDir, data });
}

/** Çıktı klasöründeki üretilmiş işleri okur — kitaplığın kaynağı. */
export function listJobs(dir: string): Promise<JobEntry[]> {
  return invoke<JobEntry[]>("list_jobs", { dir });
}

export function saveJobInfo(outDir: string, data: unknown): Promise<void> {
  return invoke<void>("save_job_info", { outDir, data: JSON.stringify(data) });
}

/** `~` ile kısaltılmış yol döndürür (ayarlarda göstermek için). */
export function shortenHome(path: string): Promise<string> {
  return invoke<string>("shorten_home", { path });
}

/** Dosyayı Finder'da seçili olarak gösterir. */
export async function revealInFinder(path: string): Promise<void> {
  const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
  await revealItemInDir(path);
}

/** Dosyayı sistemin varsayılan uygulamasında açar. */
export async function openPath(path: string): Promise<void> {
  const { openPath: open } = await import("@tauri-apps/plugin-opener");
  await open(path);
}

/** Klasör seçtirir; iptal edilirse null döner. */
export async function pickDirectory(): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({ directory: true, multiple: false });
  return typeof picked === "string" ? picked : null;
}

export interface TtsProgress {
  done: number;
  total: number;
  id: string;
}

export function onTtsProgress(
  handler: (progress: TtsProgress) => void,
): Promise<UnlistenFn> {
  return listen<TtsProgress>("tts-progress", (event) => handler(event.payload));
}

/** Yerel dosya yolunu webview'in oynatabileceği bir adrese çevirir. */
export function fileUrl(path: string): string {
  return convertFileSrc(path);
}

/** Uygulama sürümü ve derleme damgası — kenar çubuğunda gösterilir. */
export async function appVersion(): Promise<string> {
  if (!isTauri()) return "geliştirme";
  try {
    const { getVersion } = await import("@tauri-apps/api/app");
    return await getVersion();
  } catch {
    return "?";
  }
}

/** Reddit'in `created_utc` değerini "4 sa önce" gibi bir ifadeye çevirir. */
export function relativeTime(unixSeconds: number): string {
  if (!unixSeconds) return "";
  const diff = Date.now() / 1000 - unixSeconds;
  const minutes = Math.round(diff / 60);
  if (minutes < 60) return `${Math.max(1, minutes)} dk önce`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} sa önce`;
  const days = Math.round(hours / 24);
  if (days < 7) return `${days} gün önce`;
  return `${Math.round(days / 7)} hafta önce`;
}
