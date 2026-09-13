export type SetupItemState = "ready" | "working" | "missing" | "warning";

export interface SetupItem {
  id: string;
  label: string;
  detail: string;
  state: SetupItemState;
  progress?: number;
  /** Eksikse üretim mümkün değil mi, yoksa yalnızca kalite mi düşer? */
  required: boolean;
  fixHint: string;
  fixCommand: string;
}

/** İçeriğin geldiği platform. */
export type SourceKind = "reddit" | "x";

/** Kaynağa göre okunur etiket: `r/AskReddit` ya da `@Fenerbahce`. */
export function sourceLabel(source: SourceKind, handle: string): string {
  return source === "x" ? `@${handle}` : `r/${handle}`;
}

/** X kaynağından ne alınacağı: tweet'in yanıtları mı, yazarın kendi zinciri mi. */
export type XMode = "replies" | "thread";

export interface RecentItem {
  url: string;
  when: string;
  source: SourceKind;
}

export interface RedditPost {
  id: string;
  subreddit: string;
  title: string;
  titleTr: string;
  url: string;
  upvotes: number;
  commentCount: number;
  fetchedCount: number;
  postedAgo: string;
  /** Kaynak kaldırılmış/silinmişse kullanıcıya gösterilecek not. */
  removalNote?: string;
}

export interface CommentDraft {
  id: string;
  author: string;
  upvotes: number;
  original: string;
  translated: string;
  selected: boolean;
  editing?: boolean;
  /** Kullanıcının elle eklediği yorum mu? */
  manual?: boolean;
}

export type TtsEngineId = "googletranslate" | "elevenlabs" | "openai" | "gemini" | "system";

export interface TtsEngine {
  id: TtsEngineId;
  label: string;
  requiresKey: boolean;
  keyPresent: boolean;
}

export interface Voice {
  id: string;
  name: string;
  detail: string;
  engine: TtsEngineId;
  /** 0-1 aralığında dalga formu örnekleri */
  waveform: number[];
}

export interface BackgroundVideo {
  id: string;
  label: string;
  detail: string;
  local: boolean;
}

export interface BackgroundAudio {
  id: string;
  label: string;
}

export type CardTheme = "dark" | "light" | "transparent";

export interface LookSettings {
  backgroundVideoId: string;
  backgroundAudioId: string;
  audioVolume: number;
  cardTheme: CardTheme;
  fontSize: number;
  cardWidth: number;
}

/** Yarım kalan bir işi sürdürebilmek için diske yazılan taslak.
 *
 *  `bilgi.json` yalnızca iş bittiğinde yazılıyor; bu dosya ise iş başlarken
 *  yazılıyor ki uygulama kapansa bile kaldığı yerden devam edilebilsin. */
export interface JobDraft {
  /** Biçim değişirse eski taslakları sessizce yüklemeyelim. */
  version: 1;
  source: SourceKind;
  xMode: XMode;
  post: RedditPost;
  comments: CommentDraft[];
  engineId: TtsEngineId;
  voiceId: string;
  speed: number;
  silenceMs: number;
  sortMode: string;
  maxComments: number;
  look: LookSettings;
}

export type StageId =
  | "fetch"
  | "translate"
  | "tts"
  | "cards"
  | "background"
  | "render";

/** `blocked`: aşama gerçek, ama henüz bağlanmadı — sahte ilerleme göstermiyoruz. */
export type StageState = "pending" | "active" | "done" | "failed" | "blocked";

export interface Stage {
  id: StageId;
  label: string;
  state: StageState;
  progress?: number;
  detail?: string;
}

export type LogLevel = "info" | "warn" | "error" | "success";

export interface LogLine {
  time: string;
  level: LogLevel;
  text: string;
}

export type JobStatus = "waiting" | "running" | "done" | "failed" | "paused";

export interface Job {
  id: string;
  title: string;
  subreddit: string;
  source: SourceKind;
  presetSummary: string;
  status: JobStatus;
  progress: number;
  eta?: string;
  /** Çalışan işte aşama satırı, hatada sebep, bitmişte çıktı özeti */
  statusDetail?: string;
  /** Üretilmiş videonun yolu; "klasörde göster" bunu kullanır. */
  outputPath?: string;
}

export interface LibraryItem {
  id: string;
  /** İş klasörü — silme ve klasörde gösterme bunu kullanır. */
  dir: string;
  createdMs: number;
  title: string;
  subreddit: string;
  source: SourceKind;
  durationSec: number;
  createdAt: string;
  sizeMb: number;
  sourceUrl: string;
  voice: string;
  background: string;
  path: string;
}
