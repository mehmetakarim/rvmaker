mod engines;
mod events;
mod keychain;
mod reddit;
mod render;
mod translate;
mod toolpath;
mod tts;
mod twitter;

use std::process::Command;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

#[derive(Serialize, Clone)]
pub struct EnvCheck {
    id: String,
    label: String,
    detail: String,
    /// "ready" | "missing" | "warning"
    state: String,
    /// Eksikse video üretilemez mi, yoksa yalnızca kalite mi düşer?
    required: bool,
    /// Kullanıcının ne yapması gerektiği
    fix_hint: String,
    /// Kopyalanabilir kurulum komutu (varsa)
    fix_command: String,
}

#[derive(Deserialize)]
pub struct EnvParams {
    backgrounds_dir: String,
    output_dir: String,
    /// Arayüz biliyor: çerez veya istemci kimliği tanımlı mı
    reddit_configured: bool,
}

fn which(binary: &str) -> Option<String> {
    let out = Command::new(if cfg!(target_os = "windows") { "where" } else { "which" })
        .arg(binary)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}

fn first_line(command: &str, arg: &str) -> String {
    Command::new(command)
        .arg(arg)
        .output()
        .ok()
        .map(|out| {
            let text = if out.stdout.is_empty() {
                String::from_utf8_lossy(&out.stderr).to_string()
            } else {
                String::from_utf8_lossy(&out.stdout).to_string()
            };
            text.lines().next().unwrap_or("").trim().to_string()
        })
        .unwrap_or_default()
}

/// Uygulamanın gerçekten ihtiyaç duyduğu şeyleri denetler.
///
/// Not: RVMaker Python kullanmıyor (saf Rust + Vue), o yüzden Python denetimi yok.
#[tauri::command]
async fn check_environment(params: EnvParams) -> Vec<EnvCheck> {
    let mut checks = Vec::new();

    // 1 — ffmpeg ve ffprobe: seslendirme birleştirme, kapak görseli ve render bunlara bağlı
    match (which("ffmpeg"), which("ffprobe")) {
        (Some(ffmpeg), Some(_)) => {
            let version = first_line(&ffmpeg, "-version");
            let short = version
                .split_whitespace()
                .nth(2)
                .unwrap_or("")
                .to_string();
            checks.push(EnvCheck {
                id: "ffmpeg".into(),
                label: "ffmpeg".into(),
                detail: format!("{short} · {ffmpeg}"),
                state: "ready".into(),
                required: true,
                fix_hint: String::new(),
                fix_command: String::new(),
            });
        }
        _ => checks.push(EnvCheck {
            id: "ffmpeg".into(),
            label: "ffmpeg".into(),
            detail: "Bulunamadı — seslendirme ve video üretilemez".into(),
            state: "missing".into(),
            required: true,
            fix_hint: if cfg!(target_os = "windows") {
                "winget ile kurulabilir.".into()
            } else {
                "Homebrew ile kurulabilir.".into()
            },
            fix_command: if cfg!(target_os = "windows") {
                "winget install Gyan.FFmpeg".into()
            } else {
                "brew install ffmpeg".into()
            },
        }),
    }

    // 2 — Reddit erişimi
    checks.push(if params.reddit_configured {
        EnvCheck {
            id: "reddit".into(),
            label: "Reddit erişimi".into(),
            detail: "Oturum çerezi veya istemci kimliği tanımlı".into(),
            state: "ready".into(),
            required: true,
            fix_hint: String::new(),
            fix_command: String::new(),
        }
    } else {
        EnvCheck {
            id: "reddit".into(),
            label: "Reddit erişimi".into(),
            detail: "Kimlik tanımlı değil — gönderi çekilemez".into(),
            state: "missing".into(),
            required: true,
            fix_hint: "Ayarlar → Genel → Reddit erişimi".into(),
            fix_command: String::new(),
        }
    });

    // 3 — Çeviri servisi (gerçek istek)
    let translate_ok = match translate::client() {
        Ok(client) => translate::translate_text(&client, "test", "en", "tr", true)
            .await
            .is_ok(),
        Err(_) => false,
    };
    checks.push(EnvCheck {
        id: "translate".into(),
        label: "Çeviri servisi".into(),
        detail: if translate_ok {
            "Bağlantı doğrulandı".into()
        } else {
            "Ulaşılamıyor — yorumlar özgün diliyle kalır".into()
        },
        state: if translate_ok { "ready".into() } else { "warning".into() },
        required: false,
        fix_hint: if translate_ok {
            String::new()
        } else {
            "İnternet bağlantısını kontrol et; hız sınırı da olabilir.".into()
        },
        fix_command: String::new(),
    });

    // 4 — Arka plan kitaplığı
    let background_count = scan_media(&params.backgrounds_dir, &["mp4", "mov", "mkv", "webm"]).len();
    checks.push(EnvCheck {
        id: "backgrounds".into(),
        label: "Arka plan kitaplığı".into(),
        detail: if background_count > 0 {
            format!("{background_count} video hazır")
        } else {
            "Boş — videolar düz zemin üzerine üretilir".into()
        },
        state: if background_count > 0 { "ready".into() } else { "warning".into() },
        required: false,
        fix_hint: if background_count > 0 {
            String::new()
        } else {
            "Ayarlar → Video → Arka plan klasörüne bir video koy.".into()
        },
        fix_command: String::new(),
    });

    // 5 — Çıktı klasörü yazılabilir mi
    let out_dir = expand_home(&params.output_dir);
    let writable = std::fs::create_dir_all(&out_dir).is_ok()
        && {
            let probe = out_dir.join(".rvmaker-yazma-testi");
            let ok = std::fs::write(&probe, b"x").is_ok();
            let _ = std::fs::remove_file(&probe);
            ok
        };
    checks.push(EnvCheck {
        id: "output".into(),
        label: "Çıktı klasörü".into(),
        detail: if writable {
            out_dir.to_string_lossy().to_string()
        } else {
            format!("Yazılamıyor: {}", out_dir.display())
        },
        state: if writable { "ready".into() } else { "missing".into() },
        required: true,
        fix_hint: if writable {
            String::new()
        } else {
            "Ayarlar → Video → Çıktı klasörünü değiştir.".into()
        },
        fix_command: String::new(),
    });

    checks
}

#[derive(Serialize, Clone)]
struct InstallProgress {
    line: String,
    done: bool,
    ok: bool,
}

/// ffmpeg'i Homebrew ile kurar; çıktıyı `install-progress` olayıyla akıtır.
#[tauri::command]
async fn install_ffmpeg(app: AppHandle) -> Result<(), String> {
    if cfg!(target_os = "windows") {
        return Err(
            "Otomatik kurulum yalnızca macOS'ta var. Windows'ta bir kez şunu çalıştır: \
             winget install Gyan.FFmpeg"
                .to_string(),
        );
    }

    let brew = which("brew").ok_or_else(|| {
        "Homebrew bulunamadı. brew.sh adresinden kurup tekrar dene.".to_string()
    })?;

    tauri::async_runtime::spawn_blocking(move || {
        let output = Command::new(&brew).args(["install", "ffmpeg"]).output();

        match output {
            Ok(out) => {
                let text = format!(
                    "{}{}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
                for line in text.lines().rev().take(6).collect::<Vec<_>>().into_iter().rev() {
                    let _ = app.emit(
                        "install-progress",
                        InstallProgress {
                            line: line.to_string(),
                            done: false,
                            ok: true,
                        },
                    );
                }
                let _ = app.emit(
                    "install-progress",
                    InstallProgress {
                        line: if out.status.success() {
                            "ffmpeg kuruldu.".to_string()
                        } else {
                            "Kurulum başarısız.".to_string()
                        },
                        done: true,
                        ok: out.status.success(),
                    },
                );
                if out.status.success() {
                    Ok(())
                } else {
                    Err("Homebrew kurulumu başarısız oldu.".to_string())
                }
            }
            Err(e) => Err(format!("brew çalıştırılamadı: {e}")),
        }
    })
    .await
    .map_err(|e| format!("Kurulum görevi başlatılamadı: {e}"))?
}

/// Reddit gönderisini ve yorumlarını getirir.
#[tauri::command]
async fn fetch_reddit_post(
    client_id: String,
    url: String,
    limit: usize,
) -> Result<reddit::Post, String> {
    reddit::fetch_post(&client_id, &url, limit).await
}

// --- X (Twitter) ---

#[tauri::command]
fn set_x_credentials(raw: String) -> Result<(), String> {
    twitter::store_credentials(&raw)
}

#[tauri::command]
fn clear_x_credentials() -> Result<(), String> {
    twitter::clear_credentials()
}

#[tauri::command]
fn x_credentials_present() -> bool {
    twitter::credentials_present()
}

/// Çerezlerin geçerliliğini sınar; bağlanılan hesabı döndürür.
#[tauri::command]
async fn verify_x_credentials() -> Result<String, String> {
    twitter::verify_credentials().await
}

/// Tweet içeriğini getirir — kimlik gerekmez.
#[tauri::command]
async fn fetch_tweet(url: String) -> Result<twitter::Tweet, String> {
    twitter::fetch_tweet(&url).await
}

/// Tweet'in yanıtlarını getirir — video için "yorumlar" bunlar.
#[tauri::command]
async fn fetch_tweet_replies(url: String, limit: usize) -> Result<Vec<twitter::Reply>, String> {
    twitter::fetch_replies(&url, limit).await
}

/// Çeviri ayarları — Ayarlar → Çeviri sekmesinden geliyor.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TranslateOptions {
    /// Metin zaten hedef dildeyse çeviriye hiç gönderme.
    skip_if_same_language: bool,
    /// Satır satır `kaynak=hedef` terim listesi.
    glossary: String,
    timeout_sec: u64,
}

/// Ölçütlere uyan viral tweet'leri getirir — "Viral" ekranının listesi.
#[tauri::command]
async fn search_viral_tweets(query: twitter::ViralQuery) -> Result<Vec<twitter::Tweet>, String> {
    twitter::search_viral(query).await
}

/// Bir tweet'i ve yanıtlarını birlikte getirir — Reddit'teki gönderi+yorum karşılığı.
#[tauri::command]
async fn fetch_tweet_thread(url: String, limit: usize) -> Result<twitter::TweetThread, String> {
    twitter::fetch_thread(&url, limit).await
}

/// Bir kullanıcının kendi thread'ini getirir — kök tweet + yazarın devamı.
#[tauri::command]
async fn fetch_author_thread(url: String, limit: usize) -> Result<twitter::TweetThread, String> {
    twitter::fetch_author_thread(&url, limit).await
}

/// X entegrasyonunun dayandığı `bird` aracı kurulu mu?
#[tauri::command]
fn bird_installed() -> bool {
    twitter::bird_path().is_some()
}

/// Bir subreddit'in gönderi listesini getirir.
#[tauri::command]
async fn fetch_subreddit(
    client_id: String,
    subreddit: String,
    sort: String,
    limit: usize,
) -> Result<Vec<reddit::PostSummary>, String> {
    reddit::fetch_subreddit(&client_id, &subreddit, &sort, limit).await
}

/// Oturum çerezini sistem anahtar zincirine yazar.
/// Değer bir daha arayüze dönmez; yalnızca varlığı sorgulanabilir.
#[tauri::command]
fn set_reddit_cookie(cookie: String) -> Result<(), String> {
    if cookie.trim().is_empty() {
        return reddit::clear_cookie();
    }
    reddit::store_cookie(&cookie)
}

#[tauri::command]
fn clear_reddit_cookie() -> Result<(), String> {
    reddit::clear_cookie()
}

#[tauri::command]
fn reddit_cookie_present() -> bool {
    reddit::cookie_present()
}

#[derive(Serialize, Clone)]
struct TranslateProgress {
    done: usize,
    total: usize,
}

/// Verilen metinleri sırayla çevirir ve ilerlemeyi `translate-progress`
/// olayıyla arayüze bildirir. Tek tek başarısızlıklar tüm işi düşürmez;
/// çevrilemeyen metin özgün hâliyle döner.
#[tauri::command]
async fn translate_texts(
    app: AppHandle,
    texts: Vec<String>,
    source: String,
    target: String,
    options: TranslateOptions,
) -> Result<Vec<String>, String> {
    let client = translate::client_with_timeout(options.timeout_sec)?;
    let total = texts.len();
    let mut out = Vec::with_capacity(total);
    let mut first_error: Option<String> = None;

    for (index, text) in texts.iter().enumerate() {
        match translate::translate_text(
            &client,
            text,
            &source,
            &target,
            options.skip_if_same_language,
        )
        .await
        {
            Ok(translated) => out.push(translate::apply_glossary(&translated, &options.glossary)),
            Err(err) => {
                if first_error.is_none() {
                    first_error = Some(err);
                }
                out.push(text.clone());
            }
        }

        let _ = app.emit(
            "translate-progress",
            TranslateProgress {
                done: index + 1,
                total,
            },
        );
    }

    // Hiçbiri çevrilemediyse bunu hata olarak bildir; kısmi başarıda sessiz kal.
    if let Some(err) = first_error {
        if out.iter().zip(texts.iter()).all(|(a, b)| a == b) {
            return Err(err);
        }
    }

    Ok(out)
}

/// `~` ile başlayan yolları kullanıcının ev dizinine göre çözer.
fn expand_home(path: &str) -> std::path::PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = toolpath::home_dir() {
            return std::path::PathBuf::from(home).join(rest);
        }
    }
    std::path::PathBuf::from(path)
}

#[derive(serde::Deserialize)]
pub struct SpeechItem {
    id: String,
    text: String,
}

#[tauri::command]
fn set_engine_key(engine: engines::Engine, key: String) -> Result<(), String> {
    engines::store_key(engine, &key)
}

#[tauri::command]
fn clear_engine_key(engine: engines::Engine) -> Result<(), String> {
    engines::clear_key(engine)
}

#[tauri::command]
fn engine_key_present(engine: engines::Engine) -> bool {
    engines::key_present(engine)
}

#[tauri::command]
async fn list_engine_voices(
    engine: engines::Engine,
    lang: String,
) -> Result<Vec<engines::VoiceInfo>, String> {
    let client = tts::client()?;
    engines::list_voices(&client, engine, &lang).await
}

#[tauri::command]
async fn test_engine(engine: engines::Engine) -> Result<String, String> {
    let client = tts::client()?;
    engines::test_engine(&client, engine).await
}

#[derive(serde::Deserialize)]
pub struct SpeechShapeArg {
    speed: f64,
    silence_ms: u32,
}

impl From<SpeechShapeArg> for tts::SpeechShape {
    fn from(a: SpeechShapeArg) -> Self {
        Self {
            speed: a.speed,
            silence_ms: a.silence_ms,
        }
    }
}

#[derive(Serialize, Clone)]
struct TtsProgress {
    done: usize,
    total: usize,
    id: String,
}

/// Seçilen metinleri sırayla seslendirir ve MP3 dosyalarına yazar.
/// İlerleme `tts-progress` olayıyla bildirilir.
#[tauri::command]
async fn synthesize_speech(
    app: AppHandle,
    items: Vec<SpeechItem>,
    engine: engines::Engine,
    voice: String,
    lang: String,
    out_dir: String,
    shape: SpeechShapeArg,
    fallback_on_quota: bool,
    // X zincirlerinde `🧵1/6` gibi sıra işaretleri okunmasın diye.
    strip_thread_markers: bool,
) -> Result<Vec<tts::Clip>, String> {
    let shape: tts::SpeechShape = shape.into();
    let mut engine = engine;
    let mut voice = voice;
    let client = tts::client()?;
    let base = expand_home(&out_dir);
    std::fs::create_dir_all(&base)
        .map_err(|e| format!("Çıktı klasörü oluşturulamadı ({}): {e}", base.display()))?;
    let total = items.len();
    let mut clips = Vec::with_capacity(total);

    // Yarım kalmış bir işi sürdürürken, aynı ayarla üretilmiş parçaları
    // yeniden seslendirmiyoruz: hem kotayı hem zamanı boşa harcıyordu.
    let mut manifest = tts::load_manifest(&base);

    for (index, item) in items.iter().enumerate() {
        if cancelled() {
            return Err("İptal edildi.".to_string());
        }
        let path = base.join(format!("{}.mp3", item.id));

        // Kart metni olduğu gibi kalıyor; yalnızca okunan metin temizleniyor.
        let spoken = if strip_thread_markers {
            tts::strip_thread_marker(&item.text)
        } else {
            item.text.clone()
        };

        let signature =
            tts::clip_signature(engine.id(), &voice, shape.speed, shape.silence_ms, &spoken);

        // Diskte hazır ve aynı ayarla üretilmişse dokunma.
        if manifest.get(&item.id) == Some(&signature) {
            if let Ok(d) = tts::mp3_duration(&path) {
                clips.push(tts::Clip {
                    id: item.id.clone(),
                    path: path.to_string_lossy().to_string(),
                    duration_sec: d,
                    reused: true,
                });
                let _ = app.emit(
                    "tts-progress",
                    TtsProgress {
                        done: index + 1,
                        total,
                        id: item.id.clone(),
                    },
                );
                continue;
            }
        }

        let duration = match tts::synthesize_to_file(
            &client, engine, &spoken, &voice, &lang, &path, shape,
        )
        .await
        {
            Ok(d) => d,
            Err(e) => {
                // Kota hatasında motoru komple değiştiriyoruz — ama yalnızca
                // henüz hiç parça üretilmemişse. Video ortasında ses değiştirmek
                // dinleyicide kopukluk yaratır; o durumda hatayı bildirmek daha dürüst.
                let quota_issue = e.contains("kota") || e.contains("hız sınırı");
                let can_fallback = fallback_on_quota
                    && quota_issue
                    && index == 0
                    && engine != engines::Engine::Google;

                if can_fallback {
                    let _ = app.emit(
                        "tts-fallback",
                        format!(
                            "{} kotası dolu — Google Translate sesine geçildi.",
                            engine.label()
                        ),
                    );
                    engine = engines::Engine::Google;
                    voice = String::new();
                    tts::synthesize_to_file(&client, engine, &spoken, &voice, &lang, &path, shape)
                        .await?
                } else {
                    return Err(e);
                }
            }
        };

        // Kaydı hemen yazıyoruz: iş burada kesilse bile bu parça korunsun.
        manifest.insert(item.id.clone(), signature);
        tts::save_manifest(&base, &manifest);

        clips.push(tts::Clip {
            id: item.id.clone(),
            path: path.to_string_lossy().to_string(),
            duration_sec: duration,
            reused: false,
        });

        let _ = app.emit(
            "tts-progress",
            TtsProgress {
                done: index + 1,
                total,
                id: item.id.clone(),
            },
        );
    }

    Ok(clips)
}

/// Arayüzde canvas ile üretilen kart görselini diske yazar.
/// Görsel baytları webview'den geldiği için önizlemeyle bire bir aynıdır.
#[tauri::command]
fn save_card(out_dir: String, id: String, data: Vec<u8>) -> Result<String, String> {
    if data.is_empty() {
        return Err("Kart görseli boş geldi.".to_string());
    }

    let dir = expand_home(&out_dir).join("kartlar");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Kart klasörü oluşturulamadı: {e}"))?;

    let path = dir.join(format!("{id}.png"));
    std::fs::write(&path, &data).map_err(|e| format!("Kart görseli yazılamadı: {e}"))?;

    Ok(path.to_string_lossy().to_string())
}

#[derive(Serialize)]
pub struct BackgroundEntry {
    id: String,
    label: String,
    path: String,
    width: u32,
    height: u32,
    duration_sec: f64,
}

/// Verilen klasördeki medya dosyalarını listeler.
/// Bulunmayan klasör hata değil; yalnızca boş liste döner.
fn scan_media(dir: &str, extensions: &[&str]) -> Vec<BackgroundEntry> {
    let base = expand_home(dir);
    let Ok(entries) = std::fs::read_dir(&base) else {
        return Vec::new();
    };

    let ffprobe = toolpath::find_tool("ffprobe").unwrap_or_default();

    let mut list = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let matches_kind = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| extensions.contains(&e.to_lowercase().as_str()))
            .unwrap_or(false);
        if !matches_kind {
            continue;
        }

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("arka plan")
            .to_string();

        let (mut width, mut height, mut duration) = (0u32, 0u32, 0f64);
        if !ffprobe.is_empty() {
            if let Ok(out) = Command::new(&ffprobe)
                .args([
                    "-v", "error",
                    "-select_streams", "v:0",
                    "-show_entries", "stream=width,height",
                    "-show_entries", "format=duration",
                    "-of", "csv=p=0",
                ])
                .arg(&path)
                .output()
            {
                let text = String::from_utf8_lossy(&out.stdout);
                let fields: Vec<&str> = text.split(['\n', ',']).map(|f| f.trim()).collect();
                for field in &fields {
                    if width == 0 {
                        if let Ok(v) = field.parse::<u32>() {
                            width = v;
                            continue;
                        }
                    }
                    if height == 0 {
                        if let Ok(v) = field.parse::<u32>() {
                            height = v;
                            continue;
                        }
                    }
                    if duration == 0.0 {
                        if let Ok(v) = field.parse::<f64>() {
                            duration = v;
                        }
                    }
                }
            }
        }

        // Dosya adını okunur bir etikete çevir: "local-simple-gradient" → "Simple gradient"
        let label = stem
            .trim_start_matches("local-")
            .replace(['-', '_'], " ");
        let label = label
            .char_indices()
            .map(|(i, c)| if i == 0 { c.to_uppercase().to_string() } else { c.to_string() })
            .collect::<String>();

        list.push(BackgroundEntry {
            id: stem,
            label,
            path: path.to_string_lossy().to_string(),
            width,
            height,
            duration_sec: duration,
        });
    }

    list.sort_by(|a, b| a.label.cmp(&b.label));
    list
}

#[tauri::command]
fn list_backgrounds(dir: String) -> Vec<BackgroundEntry> {
    scan_media(&dir, &["mp4", "mov", "mkv", "webm"])
}

/// Videonun ilk karesinden küçük bir kapak görseli üretir.
/// Zaten üretilmişse yeniden üretmez; kapaklar önbellek klasöründe durur.
#[tauri::command]
async fn background_thumbnail(video_path: String) -> Result<String, String> {
    let source = expand_home(&video_path);
    if !source.exists() {
        return Err("Video bulunamadı.".to_string());
    }

    let cache_dir = std::env::temp_dir().join("rvmaker").join("kapaklar");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Kapak klasörü oluşturulamadı: {e}"))?;

    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("kapak")
        .to_string();
    let out = cache_dir.join(format!("{stem}.jpg"));

    if out.exists() {
        return Ok(out.to_string_lossy().to_string());
    }

    let ffmpeg = which("ffmpeg").ok_or_else(|| "ffmpeg bulunamadı.".to_string())?;

    let result = tauri::async_runtime::spawn_blocking(move || {
        Command::new(&ffmpeg)
            .args(["-y", "-ss", "1", "-i"])
            .arg(&source)
            .args([
                "-frames:v", "1",
                // Kart oranıyla aynı görünsün diye 9:16'ya kırpıyoruz.
                "-vf", "scale=270:480:force_original_aspect_ratio=increase,crop=270:480",
                "-q:v", "4",
            ])
            .arg(&out)
            .output()
            .map(|o| (o.status.success(), out))
            .map_err(|e| format!("ffmpeg çalıştırılamadı: {e}"))
    })
    .await
    .map_err(|e| format!("Kapak görevi başlatılamadı: {e}"))??;

    if !result.0 || !result.1.exists() {
        return Err("Kapak görseli üretilemedi.".to_string());
    }

    Ok(result.1.to_string_lossy().to_string())
}

#[tauri::command]
fn list_audio(dir: String) -> Vec<BackgroundEntry> {
    scan_media(&dir, &["mp3", "wav", "m4a", "aac", "ogg"])
}

/// Kartları, sesi ve arka planı tek bir MP4'te birleştirir.
#[tauri::command]
async fn render_video(options: render::RenderOptions) -> Result<render::RenderResult, String> {
    // Yolların hepsi `~` içerebilir; ffmpeg bunu çözmez, biz çözüyoruz.
    let mut options = options;
    options.out_path = expand_home(&options.out_path).to_string_lossy().to_string();
    options.background_path = options
        .background_path
        .map(|p| expand_home(&p).to_string_lossy().to_string());
    options.music_path = options
        .music_path
        .map(|p| expand_home(&p).to_string_lossy().to_string());

    if cancelled() {
        return Err("İptal edildi.".to_string());
    }

    // ffmpeg bloke edici çalışıyor; arayüzü kilitlememek için ayrı iş parçacığında.
    tauri::async_runtime::spawn_blocking(move || render::render(&options))
        .await
        .map_err(|e| format!("Render görevi başlatılamadı: {e}"))?
}

/// İş klasörüne, yarım kalırsa sürdürebilmek için taslağın kendisini yazar.
///
/// `bilgi.json` yalnızca iş bittiğinde yazılıyor; oturum kapandıktan sonra
/// yarım bir işi sürdürebilmek için girdileri en baştan saklamak gerekiyor.
#[tauri::command]
fn save_job_draft(out_dir: String, data: String) -> Result<(), String> {
    let dir = expand_home(&out_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Klasör oluşturulamadı: {e}"))?;
    std::fs::write(dir.join("taslak.json"), data)
        .map_err(|e| format!("İş taslağı yazılamadı: {e}"))
}

/// İş klasörüne, kitaplığın okuyacağı özet bilgiyi yazar.
#[tauri::command]
fn save_job_info(out_dir: String, data: String) -> Result<(), String> {
    let dir = expand_home(&out_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Klasör oluşturulamadı: {e}"))?;
    std::fs::write(dir.join("bilgi.json"), data)
        .map_err(|e| format!("İş bilgisi yazılamadı: {e}"))
}

#[derive(Serialize)]
pub struct JobEntry {
    id: String,
    dir: String,
    video_path: String,
    has_video: bool,
    size_bytes: u64,
    duration_sec: f64,
    created_ms: u64,
    card_count: usize,
    /// `bilgi.json` içeriği; yoksa boş.
    info: String,
    /// `taslak.json` içeriği; yoksa boş. Yarım işi sürdürmek için gerekli.
    draft: String,
    /// Üretilmiş ses parçası sayısı — yarım işin ne kadar ilerlediğini gösterir.
    clip_count: usize,
}

/// Çıktı klasöründeki üretilmiş işleri tarar — kitaplığın kaynağı.
#[tauri::command]
fn list_jobs(dir: String) -> Vec<JobEntry> {
    let base = expand_home(&dir);
    let Ok(entries) = std::fs::read_dir(&base) else {
        return Vec::new();
    };

    let ffprobe = toolpath::find_tool("ffprobe").unwrap_or_default();

    let mut jobs = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let id = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if !id.starts_with("job-") {
            continue;
        }

        // Çıktı adı artık şablondan geliyor; sabit "video.mp4" aramak yanlıştı.
        // Klasördeki ilk mp4'ü buluyoruz.
        let video = std::fs::read_dir(&path)
            .ok()
            .and_then(|entries| {
                entries
                    .flatten()
                    .map(|e| e.path())
                    .find(|p| {
                        p.extension()
                            .and_then(|e| e.to_str())
                            .map(|e| e.eq_ignore_ascii_case("mp4"))
                            .unwrap_or(false)
                    })
            })
            .unwrap_or_else(|| path.join("video.mp4"));
        let has_video = video.exists();
        let size_bytes = std::fs::metadata(&video).map(|m| m.len()).unwrap_or(0);

        let mut duration = 0.0;
        if has_video && !ffprobe.is_empty() {
            if let Ok(out) = Command::new(&ffprobe)
                .args(["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0"])
                .arg(&video)
                .output()
            {
                duration = String::from_utf8_lossy(&out.stdout).trim().parse().unwrap_or(0.0);
            }
        }

        let card_count = std::fs::read_dir(path.join("kartlar"))
            .map(|d| d.flatten().count())
            .unwrap_or(0);

        // Klasör adındaki zaman damgası: "job-1788384138545"
        let created_ms = id
            .trim_start_matches("job-")
            .parse::<u64>()
            .unwrap_or(0);

        let info = std::fs::read_to_string(path.join("bilgi.json")).unwrap_or_default();
        let draft = std::fs::read_to_string(path.join("taslak.json")).unwrap_or_default();

        let clip_count = std::fs::read_dir(&path)
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|e| e.path().extension().is_some_and(|x| x == "mp3"))
                    .count()
            })
            .unwrap_or(0);

        jobs.push(JobEntry {
            id,
            dir: path.to_string_lossy().to_string(),
            video_path: video.to_string_lossy().to_string(),
            has_video,
            size_bytes,
            duration_sec: duration,
            created_ms,
            card_count,
            info,
            draft,
            clip_count,
        });
    }

    jobs.sort_by(|a, b| b.created_ms.cmp(&a.created_ms));
    jobs
}

/// Üretim iptali. Uzun süren adımlar (seslendirme, render) bu bayrağa bakar.
static CANCELLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn cancelled() -> bool {
    CANCELLED.load(std::sync::atomic::Ordering::Relaxed)
}

#[tauri::command]
fn cancel_job() {
    CANCELLED.store(true, std::sync::atomic::Ordering::Relaxed);
    render::kill_running();
}

#[tauri::command]
fn reset_cancel() {
    CANCELLED.store(false, std::sync::atomic::Ordering::Relaxed);
}

fn dir_size(path: &std::path::Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| {
            let p = e.path();
            if p.is_dir() {
                dir_size(&p)
            } else {
                std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0)
            }
        })
        .sum()
}

fn work_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("rvmaker")
}

#[derive(Serialize)]
pub struct MaintenanceInfo {
    temp_bytes: u64,
    thumbnail_bytes: u64,
    output_bytes: u64,
    output_jobs: usize,
    temp_dir: String,
}

#[tauri::command]
fn maintenance_info(output_dir: String) -> MaintenanceInfo {
    let work = work_dir();
    let thumbs = work.join("kapaklar");
    let out = expand_home(&output_dir);

    let jobs = std::fs::read_dir(&out)
        .map(|d| {
            d.flatten()
                .filter(|e| {
                    e.path().is_dir()
                        && e.file_name().to_string_lossy().starts_with("job-")
                })
                .count()
        })
        .unwrap_or(0);

    MaintenanceInfo {
        temp_bytes: dir_size(&work).saturating_sub(dir_size(&thumbs)),
        thumbnail_bytes: dir_size(&thumbs),
        output_bytes: dir_size(&out),
        output_jobs: jobs,
        temp_dir: work.to_string_lossy().to_string(),
    }
}

/// Ara dosyaları siler (kapak önbelleği hariç — onun kendi düğmesi var).
#[tauri::command]
fn clear_temp_files() -> Result<u64, String> {
    let work = work_dir();
    let before = dir_size(&work);
    let Ok(entries) = std::fs::read_dir(&work) else {
        return Ok(0);
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.file_name().and_then(|n| n.to_str()) == Some("kapaklar") {
            continue;
        }
        let _ = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
    }

    Ok(before.saturating_sub(dir_size(&work)))
}

#[tauri::command]
fn clear_thumbnail_cache() -> Result<u64, String> {
    let thumbs = work_dir().join("kapaklar");
    let before = dir_size(&thumbs);
    let _ = std::fs::remove_dir_all(&thumbs);
    Ok(before)
}

/// Tek bir işi (video, sesler, kartlar) kalıcı olarak siler.
#[tauri::command]
fn delete_job(dir: String) -> Result<(), String> {
    let path = expand_home(&dir);

    // Güvenlik: yalnızca "job-" ile başlayan klasörler silinebilir.
    let is_job = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with("job-"))
        .unwrap_or(false);
    if !is_job {
        return Err("Yalnızca iş klasörleri silinebilir.".to_string());
    }

    std::fs::remove_dir_all(&path).map_err(|e| format!("Klasör silinemedi: {e}"))
}

/// Çıktı klasöründeki bütün işleri siler.
#[tauri::command]
fn delete_all_outputs(output_dir: String) -> Result<usize, String> {
    let out = expand_home(&output_dir);
    let Ok(entries) = std::fs::read_dir(&out) else {
        return Ok(0);
    };

    let mut removed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with("job-")).unwrap_or(false) {
            if std::fs::remove_dir_all(&path).is_ok() {
                removed += 1;
            }
        }
    }

    Ok(removed)
}

/// Çıktı klasörü gibi ayarlarda kullanıcıya klasör seçtirir.
/// Yol `~` ile kısaltılabilir olsun diye ev dizinine göre kısaltılır.
#[tauri::command]
fn shorten_home(path: String) -> String {
    if let Some(home) = toolpath::home_dir() {
        let home = home.to_string_lossy().to_string();
        if let Some(rest) = path.strip_prefix(&home) {
            return format!("~{rest}");
        }
    }
    path
}

/// Ses ayarları ekranındaki önizleme düğmesi için tek seferlik seslendirme.
#[tauri::command]
async fn preview_speech(
    engine: engines::Engine,
    voice: String,
    text: String,
    lang: String,
    shape: SpeechShapeArg,
) -> Result<tts::Clip, String> {
    let client = tts::client()?;
    let path = std::env::temp_dir().join("rvmaker").join("onizleme.mp3");
    let duration =
        tts::synthesize_to_file(&client, engine, &text, &voice, &lang, &path, shape.into())
            .await?;

    Ok(tts::Clip {
        id: "onizleme".to_string(),
        path: path.to_string_lossy().to_string(),
        duration_sec: duration,
        reused: false,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Finder'dan açılan uygulama kabuğun PATH'ini almıyor; ffmpeg ve node
    // gibi araçlar bulunabilsin diye her şeyden önce genişletiyoruz.
    toolpath::augment();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            check_environment,
            install_ffmpeg,
            fetch_reddit_post,
            fetch_subreddit,
            set_x_credentials,
            clear_x_credentials,
            x_credentials_present,
            verify_x_credentials,
            fetch_tweet,
            fetch_tweet_replies,
            search_viral_tweets,
            fetch_tweet_thread,
            fetch_author_thread,
            bird_installed,
            set_reddit_cookie,
            clear_reddit_cookie,
            reddit_cookie_present,
            translate_texts,
            synthesize_speech,
            preview_speech,
            save_card,
            list_backgrounds,
            background_thumbnail,
            list_audio,
            render_video,
            shorten_home,
            save_job_info,
            save_job_draft,
            list_jobs,
            cancel_job,
            reset_cancel,
            maintenance_info,
            clear_temp_files,
            clear_thumbnail_cache,
            delete_job,
            delete_all_outputs,
            set_engine_key,
            clear_engine_key,
            engine_key_present,
            list_engine_voices,
            test_engine
        ])
        .run(tauri::generate_context!())
        .expect("Tauri uygulaması başlatılamadı");
}

#[cfg(test)]
mod kitaplik_testleri {
    /// Yarım kalmış bir iş klasörü kurup `list_jobs`'un taslağı ve üretilmiş
    /// parça sayısını doğru döndürdüğünü sınar — kitaplıktaki "Devam et"
    /// satırı tamamen bu iki alana dayanıyor.
    #[test]
    fn yarim_isi_taslagiyla_birlikte_dondurur() {
        let kok = std::env::temp_dir().join(format!("rvmaker-kitaplik-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&kok);

        let yarim = kok.join("job-1700000000000");
        std::fs::create_dir_all(&yarim).unwrap();
        std::fs::write(yarim.join("taslak.json"), r#"{"version":1}"#).unwrap();
        std::fs::write(yarim.join("baslik.mp3"), b"x").unwrap();
        std::fs::write(yarim.join("yorum.mp3"), b"x").unwrap();

        // Taslağı olmayan eski yetim: sürdürülemez, listede taslağı boş gelmeli.
        let yetim = kok.join("job-1600000000000");
        std::fs::create_dir_all(&yetim).unwrap();
        std::fs::write(yetim.join("baslik.mp3"), b"x").unwrap();

        let isler = super::list_jobs(kok.to_string_lossy().to_string());
        assert_eq!(isler.len(), 2, "iki klasör de taranmalı");

        let y = isler
            .iter()
            .find(|j| j.id == "job-1700000000000")
            .expect("yarım iş listede olmalı");
        assert_eq!(y.draft, r#"{"version":1}"#, "taslak geri gelmeli");
        assert_eq!(y.clip_count, 2, "iki ses parçası sayılmalı");
        assert!(!y.has_video);

        let e = isler
            .iter()
            .find(|j| j.id == "job-1600000000000")
            .expect("yetim de listede olmalı");
        assert!(e.draft.is_empty(), "taslağı yok, sürdürülemez olarak işaretlenmeli");
        assert_eq!(e.clip_count, 1);

        let _ = std::fs::remove_dir_all(&kok);
    }
}
