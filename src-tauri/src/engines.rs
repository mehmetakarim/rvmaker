//! Seslendirme motorları.
//!
//! Her motorun kendi uç noktası, karakter sınırı ve ses listesi var. Ortak
//! sözleşme: metni al, **MP3 baytları** döndür. Parçalama, hız ve sessizlik
//! `tts` modülünde motordan bağımsız uygulanıyor.

use std::process::Command;

use serde::{Deserialize, Serialize};

const KEYCHAIN_SERVICE: &str = "com.rvmaker.desktop";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    #[serde(rename = "googletranslate")]
    Google,
    #[serde(rename = "elevenlabs")]
    ElevenLabs,
    #[serde(rename = "openai")]
    OpenAi,
    #[serde(rename = "gemini")]
    Gemini,
    #[serde(rename = "system")]
    System,
}

impl Engine {
    pub fn id(&self) -> &'static str {
        match self {
            Engine::Google => "googletranslate",
            Engine::ElevenLabs => "elevenlabs",
            Engine::OpenAi => "openai",
            Engine::Gemini => "gemini",
            Engine::System => "system",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Engine::Google => "Google Translate",
            Engine::ElevenLabs => "ElevenLabs",
            Engine::OpenAi => "OpenAI",
            Engine::Gemini => "Gemini",
            Engine::System => "Sistem sesi",
        }
    }

    /// İstek başına güvenli karakter sınırı. Google'ınki ölçülerek bulundu
    /// (200 → 200 OK, 210 → 400); diğerleri belgelenmiş sınırların altında.
    pub fn max_chars(&self) -> usize {
        match self {
            Engine::Google => 190,
            Engine::ElevenLabs => 2500,
            Engine::OpenAi => 3500,
            Engine::Gemini => 3000,
            Engine::System => 5000,
        }
    }

    pub fn requires_key(&self) -> bool {
        matches!(self, Engine::ElevenLabs | Engine::OpenAi | Engine::Gemini)
    }
}

// ---------------------------------------------------------------- anahtarlar

fn key_entry(engine: Engine) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, &format!("{}-api-key", engine.id()))
        .map_err(|e| format!("Anahtar zincirine erişilemedi: {e}"))
}

/// Hata yanıtının gövdesindeki açıklamayı çıkarır.
///
/// Servisler sebebi gövdede söylüyor; yalnızca durum kodunu göstermek
/// kullanıcıyı karanlıkta bırakıyordu (ölçülen örnek: ElevenLabs 400 yanıtı
/// "anahtar yerine anahtar kimliği kullanılmış" diyordu, biz yutuyorduk).
pub async fn error_detail(response: reqwest::Response) -> String {
    let raw = response.text().await.unwrap_or_default();
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return raw.chars().take(200).collect();
    };

    // Yaygın biçimler: {detail:{message}}, {error:{message}}, {message}
    for yol in [
        payload.pointer("/detail/message"),
        payload.pointer("/error/message"),
        payload.pointer("/message"),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(metin) = yol.as_str() {
            if !metin.is_empty() {
                return metin.chars().take(200).collect();
            }
        }
    }

    raw.chars().take(200).collect()
}

/// ElevenLabs hatasını okunur bir mesaja çevirir.
pub fn elevenlabs_error(status: u16, detail: &str) -> String {
    // En sık karşılaşılan tuzak: panoya anahtarın kimliği kopyalanıyor.
    // Gerçek anahtar `sk_` ile başlıyor ve yalnızca oluşturulurken gösteriliyor.
    if detail.contains("api_key_id_used_as_api_key")
        || detail.contains("API key ID used as API key")
    {
        return "ElevenLabs anahtarı yerine anahtarın kimliği (ID) yapıştırılmış. \
                Gerçek anahtar `sk_` ile başlıyor ve yalnızca oluşturulduğu ya da \
                yenilendiği anda bir kez gösteriliyor."
            .to_string();
    }

    // Kısıtlı anahtarlarda 401 dönüyor ama sebep "anahtar yanlış" değil,
    // "bu izin verilmemiş". Hangi izin olduğunu söylemezsek kullanıcı
    // aboneliğinde sorun var sanıyor (ölçüldü).
    if detail.contains("missing_permissions") || detail.contains("missing the permission") {
        let izin = detail
            .split("missing the permission ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .unwrap_or("");
        return if izin.is_empty() {
            "ElevenLabs anahtarında gereken izinler yok. Anahtarı düzenleyip \
             \"Full access\" seç ya da en az `text_to_speech` ve `voices_read` \
             izinlerini aç."
                .to_string()
        } else {
            format!(
                "ElevenLabs anahtarında `{izin}` izni yok. Anahtarı düzenleyip \
                 \"Full access\" seç ya da en az `text_to_speech` ve `voices_read` \
                 izinlerini aç."
            )
        };
    }

    match status {
        401 => "ElevenLabs anahtarı reddedildi.".to_string(),
        422 => "ElevenLabs metni kabul etmedi (ses kimliği yanlış olabilir).".to_string(),
        429 => "ElevenLabs kotası doldu veya hız sınırına takıldı.".to_string(),
        code if detail.is_empty() => format!("ElevenLabs {code} yanıtı verdi."),
        code => format!("ElevenLabs {code} yanıtı verdi: {detail}"),
    }
}

/// ElevenLabs anahtarı yerine anahtar kimliği yapıştırılmış mı?
///
/// Kimlikler salt onaltılık ve 32/64 karakter; gerçek anahtarlar `sk_` ile
/// başlıyor. Bunu kaydederken yakalamak, kullanıcıyı "bağlantıyı test et"
/// adımındaki anlamsız 400 hatasından kurtarıyor.
pub fn looks_like_key_id(value: &str) -> bool {
    let v = value.trim();
    !v.starts_with("sk_")
        && matches!(v.len(), 32 | 64)
        && v.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn store_key(engine: Engine, value: &str) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return clear_key(engine);
    }

    if engine == Engine::ElevenLabs && looks_like_key_id(trimmed) {
        return Err(
            "Bu, ElevenLabs anahtarının kimliği (ID) gibi görünüyor. Gerçek anahtar \
             `sk_` ile başlıyor ve yalnızca oluşturulduğu ya da yenilendiği anda \
             bir kez gösteriliyor."
                .to_string(),
        );
    }

    crate::keychain::write(
        KEYCHAIN_SERVICE,
        &format!("{}-api-key", engine.id()),
        trimmed,
    )
}

pub fn clear_key(engine: Engine) -> Result<(), String> {
    let entry = key_entry(engine)?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Anahtar silinemedi: {e}")),
    }
}

pub fn read_key(engine: Engine) -> Option<String> {
    let entry = key_entry(engine).ok()?;
    let value = entry.get_password().ok()?;
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

pub fn key_present(engine: Engine) -> bool {
    read_key(engine).is_some()
}

// ------------------------------------------------------------------- sesler

#[derive(Serialize, Clone, Debug)]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    pub detail: String,
    pub engine: String,
}

/// OpenAI'nin sabit ses listesi — API'de listeleme uç noktası yok.
const OPENAI_VOICES: [(&str, &str); 6] = [
    ("alloy", "Nötr · dengeli"),
    ("echo", "Erkek · sakin"),
    ("fable", "Erkek · anlatıcı"),
    ("onyx", "Erkek · derin"),
    ("nova", "Kadın · canlı"),
    ("shimmer", "Kadın · yumuşak"),
];

/// Gemini'nin hazır ses listesi.
const GEMINI_VOICES: [(&str, &str); 6] = [
    ("Kore", "Kadın · net"),
    ("Aoede", "Kadın · yumuşak"),
    ("Leda", "Kadın · genç"),
    ("Puck", "Erkek · canlı"),
    ("Charon", "Erkek · derin"),
    ("Fenrir", "Erkek · sıcak"),
];

fn system_voices(lang: &str) -> Vec<VoiceInfo> {
    // `say` macOS'a özel. Windows'ta bu motorun sesi hiç yok; boş liste
    // dönünce ayarlar ekranı "kullanılabilir ses bulunamadı" diyor.
    if !cfg!(target_os = "macos") {
        return Vec::new();
    }

    let Ok(output) = Command::new("say").arg("-v").arg("?").output() else {
        return Vec::new();
    };

    let text = String::from_utf8_lossy(&output.stdout);
    let prefix = format!("{}_", lang.split('-').next().unwrap_or("tr"));

    text.lines()
        .filter_map(|line| {
            // Biçim: "Yelda               tr_TR    # Merhaba, benim adım Yelda."
            let (name_part, rest) = line.split_once("  ")?;
            let name = name_part.trim();
            let locale = rest.split_whitespace().next()?;
            if !locale.to_lowercase().starts_with(&prefix) {
                return None;
            }
            Some(VoiceInfo {
                id: name.to_string(),
                name: name.to_string(),
                detail: format!("Sistem · {locale}"),
                engine: "system".to_string(),
            })
        })
        .collect()
}

async fn elevenlabs_voices(client: &reqwest::Client, key: &str) -> Result<Vec<VoiceInfo>, String> {
    let response = crate::tts::send_with_retry(
        client
            .get("https://api.elevenlabs.io/v1/voices")
            .header("xi-api-key", key),
        "ElevenLabs'e ulaşılamadı",
    )
    .await?;

    let status = response.status();
    if !status.is_success() {
        let detail = error_detail(response).await;
        return Err(elevenlabs_error(status.as_u16(), &detail));
    }

    let payload: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("ElevenLabs yanıtı okunamadı: {e}"))?;

    Ok(payload
        .get("voices")
        .and_then(|v| v.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|v| {
                    let id = v.get("voice_id")?.as_str()?.to_string();
                    let name = v.get("name")?.as_str()?.to_string();
                    let labels = v.get("labels");
                    let gender = labels
                        .and_then(|l| l.get("gender"))
                        .and_then(|g| g.as_str())
                        .map(|g| match g {
                            "female" => "Kadın",
                            "male" => "Erkek",
                            other => other,
                        })
                        .unwrap_or("—");
                    let desc = labels
                        .and_then(|l| l.get("description"))
                        .and_then(|d| d.as_str())
                        .unwrap_or("");
                    Some(VoiceInfo {
                        id,
                        name,
                        detail: if desc.is_empty() {
                            gender.to_string()
                        } else {
                            format!("{gender} · {desc}")
                        },
                        engine: "elevenlabs".to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default())
}

/// Bir motorun kullanılabilir seslerini döndürür.
pub async fn list_voices(
    client: &reqwest::Client,
    engine: Engine,
    lang: &str,
) -> Result<Vec<VoiceInfo>, String> {
    match engine {
        Engine::Google => Ok(vec![VoiceInfo {
            id: "tr-google".to_string(),
            name: "Türkçe".to_string(),
            detail: "Google · tek ses".to_string(),
            engine: "googletranslate".to_string(),
        }]),
        Engine::System => Ok(system_voices(lang)),
        Engine::OpenAi => Ok(OPENAI_VOICES
            .iter()
            .map(|(id, detail)| VoiceInfo {
                id: id.to_string(),
                name: id.to_string(),
                detail: detail.to_string(),
                engine: "openai".to_string(),
            })
            .collect()),
        Engine::Gemini => Ok(GEMINI_VOICES
            .iter()
            .map(|(id, detail)| VoiceInfo {
                id: id.to_string(),
                name: id.to_string(),
                detail: detail.to_string(),
                engine: "gemini".to_string(),
            })
            .collect()),
        Engine::ElevenLabs => {
            let key = read_key(Engine::ElevenLabs)
                .ok_or_else(|| "ElevenLabs anahtarı tanımlı değil.".to_string())?;
            elevenlabs_voices(client, &key).await
        }
    }
}

// -------------------------------------------------------------- seslendirme

/// Ham PCM veya AIFF veriyi MP3'e çevirir (Gemini ve sistem sesi için).
fn transcode_to_mp3(input: &[u8], input_args: &[&str]) -> Result<Vec<u8>, String> {
    let dir = std::env::temp_dir().join("rvmaker").join("cevrim");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Geçici klasör oluşturulamadı: {e}"))?;

    let raw = dir.join(format!("giris-{}", std::process::id()));
    let out = dir.join(format!("cikis-{}.mp3", std::process::id()));
    std::fs::write(&raw, input).map_err(|e| format!("Geçici ses yazılamadı: {e}"))?;

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-y");
    for arg in input_args {
        cmd.arg(arg);
    }
    cmd.arg("-i").arg(&raw).args(["-c:a", "libmp3lame", "-q:a", "4"]).arg(&out);

    let status = cmd
        .output()
        .map_err(|e| format!("ffmpeg çalıştırılamadı: {e}"))?;

    let _ = std::fs::remove_file(&raw);

    if !status.status.success() {
        let err = String::from_utf8_lossy(&status.stderr);
        let tail = err.lines().rev().take(2).collect::<Vec<_>>().join(" · ");
        return Err(format!("Ses dönüştürülemedi: {tail}"));
    }

    let bytes = std::fs::read(&out).map_err(|e| format!("Dönüştürülen ses okunamadı: {e}"))?;
    let _ = std::fs::remove_file(&out);
    Ok(bytes)
}

async fn elevenlabs_speak(
    client: &reqwest::Client,
    text: &str,
    voice: &str,
    key: &str,
) -> Result<Vec<u8>, String> {
    let voice_id = if voice.trim().is_empty() { "21m00Tcm4TlvDq8ikWAM" } else { voice };
    let response = crate::tts::send_with_retry(
        client
            .post(format!(
                "https://api.elevenlabs.io/v1/text-to-speech/{voice_id}"
            ))
            .header("xi-api-key", key)
            .header("Accept", "audio/mpeg")
            .json(&serde_json::json!({
                "text": text,
                "model_id": "eleven_multilingual_v2",
            })),
        "ElevenLabs'e ulaşılamadı",
    )
    .await?;

    let status = response.status();
    if !status.is_success() {
        let detail = error_detail(response).await;
        return Err(elevenlabs_error(status.as_u16(), &detail));
    }

    Ok(response
        .bytes()
        .await
        .map_err(|e| format!("Ses verisi okunamadı: {e}"))?
        .to_vec())
}

async fn openai_speak(
    client: &reqwest::Client,
    text: &str,
    voice: &str,
    key: &str,
) -> Result<Vec<u8>, String> {
    let voice = if voice.trim().is_empty() { "alloy" } else { voice };
    let response = crate::tts::send_with_retry(
        client
            .post("https://api.openai.com/v1/audio/speech")
            .bearer_auth(key)
            .json(&serde_json::json!({
                "model": "gpt-4o-mini-tts",
                "input": text,
                "voice": voice,
                "response_format": "mp3",
            })),
        "OpenAI'ye ulaşılamadı",
    )
    .await?;

    let status = response.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 => "OpenAI anahtarı reddedildi.".to_string(),
            429 => "OpenAI hız sınırına takıldı veya kotan doldu.".to_string(),
            code => format!("OpenAI {code} yanıtı verdi."),
        });
    }

    Ok(response
        .bytes()
        .await
        .map_err(|e| format!("Ses verisi okunamadı: {e}"))?
        .to_vec())
}

/// Gemini TTS model zinciri.
///
/// Ücretsiz katmanda hız sınırı **model başına** uygulanıyor; biri dolduğunda
/// diğerine geçmek toplam kotayı fiilen çoğaltıyor. Sıra ücretsiz kotası en
/// geniş olandan dara doğru.
pub const GEMINI_MODELS: [&str; 3] = [
    "gemini-2.5-flash-preview-tts",
    "gemini-3.1-flash-tts-preview",
    "gemini-2.5-pro-preview-tts",
];

/// Son başarılı modelin sırası — sonraki parça buradan başlar, böylece
/// her seferinde dolmuş modeli yeniden denemiyoruz.
static GEMINI_CURSOR: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Bu oturumda var olmadığı anlaşılan modeller — bir daha denenmez.
fn gemini_dead_models() -> &'static std::sync::Mutex<Vec<usize>> {
    static DEAD: std::sync::OnceLock<std::sync::Mutex<Vec<usize>>> = std::sync::OnceLock::new();
    DEAD.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

fn mark_dead(index: usize) {
    if let Ok(mut dead) = gemini_dead_models().lock() {
        if !dead.contains(&index) {
            dead.push(index);
        }
    }
}

fn is_dead(index: usize) -> bool {
    gemini_dead_models()
        .lock()
        .map(|d| d.contains(&index))
        .unwrap_or(false)
}

/// Tek bir modele istek atar. `Ok(None)` = bu model kullanılamadı, sıradakini dene.
async fn gemini_try_model(
    client: &reqwest::Client,
    model: &str,
    text: &str,
    voice: &str,
    key: &str,
) -> Result<Option<Vec<u8>>, String> {
    let response = crate::tts::send_with_retry(
        client
            .post(format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={key}"
            ))
            .json(&serde_json::json!({
                "contents": [{ "parts": [{ "text": text }] }],
                "generationConfig": {
                    "responseModalities": ["AUDIO"],
                    "speechConfig": {
                        "voiceConfig": {
                            "prebuiltVoiceConfig": { "voiceName": voice }
                        }
                    }
                }
            })),
        "Gemini'ye ulaşılamadı",
    )
    .await?;

    let status = response.status();

    if status.is_success() {
        let payload: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Gemini yanıtı okunamadı: {e}"))?;

        let base64_audio = payload
            .pointer("/candidates/0/content/parts/0/inlineData/data")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("{model} yanıtında ses verisi yok."))?;

        let pcm = base64_decode(base64_audio)?;
        // Gemini 24 kHz, 16-bit, tek kanal ham PCM döndürüyor.
        return transcode_to_mp3(&pcm, &["-f", "s16le", "-ar", "24000", "-ac", "1"]).map(Some);
    }

    match status.as_u16() {
        // Kota veya geçici yoğunluk: sıradaki modeli dene.
        429 | 503 => Ok(None),
        // Model yok ya da bu anahtara kapalı: bir daha hiç deneme.
        404 => Err(format!("__olu__{model}")),
        // Anahtar sorunu: model değiştirmek çözmez, hemen bildir.
        403 => Err("Gemini anahtarı reddedildi.".to_string()),
        400 => {
            // 400 hem "model yok" hem "istek hatalı" olabilir; gövdeye bakıyoruz.
            let body = response.text().await.unwrap_or_default();
            if body.contains("not found") || body.contains("not supported") {
                Err(format!("__olu__{model}"))
            } else {
                Err(format!("Gemini isteği reddetti: {}", body.chars().take(160).collect::<String>()))
            }
        }
        code => Ok(if (500..600).contains(&code) { None } else { None }),
    }
}

/// Ücretsiz katman sınırları dakikalık; zincirin tamamı tükendiğinde bir süre
/// bekleyip tekrar denemek çoğu durumda yetiyor. Motor değiştirmek yerine
/// beklemeyi tercih ediyoruz, çünkü video ortasında ses değişmesi kötü olur.
const GEMINI_QUOTA_WAIT_SEC: u64 = 20;

async fn gemini_speak(
    client: &reqwest::Client,
    text: &str,
    voice: &str,
    key: &str,
) -> Result<Vec<u8>, String> {
    match gemini_try_chain(client, text, voice, key).await {
        Ok(bytes) => Ok(bytes),
        Err(first) if first.contains("kota") => {
            // Dakikalık sınır dolmuş olabilir; bir kez bekleyip tekrar dene.
            // Kullanıcı ekranda duraklama görmesin diye sebebini bildiriyoruz.
            crate::events::emit(
                "tts-quota-wait",
                format!(
                    "Gemini ücretsiz katman sınırına takıldı — {GEMINI_QUOTA_WAIT_SEC} sn bekleyip yeniden denenecek."
                ),
            );
            tokio::time::sleep(std::time::Duration::from_secs(GEMINI_QUOTA_WAIT_SEC)).await;
            gemini_try_chain(client, text, voice, key)
                .await
                .map_err(|second| format!("{second} ({GEMINI_QUOTA_WAIT_SEC} sn beklendikten sonra da alınamadı)"))
        }
        Err(other) => Err(other),
    }
}

async fn gemini_try_chain(
    client: &reqwest::Client,
    text: &str,
    voice: &str,
    key: &str,
) -> Result<Vec<u8>, String> {
    let voice = if voice.trim().is_empty() { "Kore" } else { voice };
    let start = GEMINI_CURSOR.load(std::sync::atomic::Ordering::Relaxed);
    let mut last_reason = String::new();
    let mut tried = 0;

    for step in 0..GEMINI_MODELS.len() {
        let index = (start + step) % GEMINI_MODELS.len();
        if is_dead(index) {
            continue;
        }
        tried += 1;
        let model = GEMINI_MODELS[index];

        match gemini_try_model(client, model, text, voice, key).await {
            Ok(Some(bytes)) => {
                // Bu model çalışıyor; sonraki parça da buradan başlasın.
                GEMINI_CURSOR.store(index, std::sync::atomic::Ordering::Relaxed);
                return Ok(bytes);
            }
            Ok(None) => {
                last_reason = format!("{model}: kota dolu veya geçici olarak meşgul");
            }
            Err(e) if e.starts_with("__olu__") => {
                mark_dead(index);
                last_reason = format!("{}: bu anahtarla kullanılamıyor", e.trim_start_matches("__olu__"));
            }
            // Anahtar hatası gibi model değiştirmenin çözmeyeceği durumlar
            Err(e) => return Err(e),
        }
    }

    if tried == 0 {
        return Err("Hiçbir Gemini TTS modeli bu anahtarla kullanılamıyor.".to_string());
    }

    Err(format!(
        "Gemini'nin {} modelinin hepsi denendi, hiçbiri yanıt vermedi. Son sebep — {last_reason}",
        GEMINI_MODELS.len()
    ))
}

/// Küçük bir base64 çözücü — tek kullanım için ek bağımlılık istemedik.
fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut lookup = [255u8; 256];
    for (i, c) in TABLE.iter().enumerate() {
        lookup[*c as usize] = i as u8;
    }

    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buffer = 0u32;
    let mut bits = 0u32;

    for byte in input.bytes() {
        if byte == b'=' || byte == b'\n' || byte == b'\r' {
            continue;
        }
        let value = lookup[byte as usize];
        if value == 255 {
            return Err("Ses verisi çözülemedi (geçersiz base64).".to_string());
        }
        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
    }

    Ok(out)
}

fn system_speak(text: &str, voice: &str) -> Result<Vec<u8>, String> {
    if !cfg!(target_os = "macos") {
        return Err(
            "Sistem sesi yalnızca macOS'ta çalışıyor (`say` komutu). \
             Windows'ta Google, Gemini, ElevenLabs ya da OpenAI motorlarından \
             birini seç."
                .to_string(),
        );
    }

    let dir = std::env::temp_dir().join("rvmaker").join("sistem");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Geçici klasör oluşturulamadı: {e}"))?;
    let aiff = dir.join(format!("ses-{}.aiff", std::process::id()));

    let mut cmd = Command::new("say");
    if !voice.trim().is_empty() {
        cmd.args(["-v", voice]);
    }
    cmd.arg("-o").arg(&aiff).arg(text);

    let output = cmd
        .output()
        .map_err(|e| format!("`say` çalıştırılamadı: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Sistem sesi üretilemedi: {}", err.trim()));
    }

    let bytes = std::fs::read(&aiff).map_err(|e| format!("Ses dosyası okunamadı: {e}"))?;
    let _ = std::fs::remove_file(&aiff);
    transcode_to_mp3(&bytes, &[])
}

/// Bir metin parçasını seslendirip MP3 baytları döndürür.
pub async fn speak(
    client: &reqwest::Client,
    engine: Engine,
    text: &str,
    voice: &str,
    lang: &str,
) -> Result<Vec<u8>, String> {
    match engine {
        Engine::Google => crate::tts::google_speak(client, text, lang).await,
        Engine::System => {
            let voice = voice.to_string();
            let text = text.to_string();
            tauri::async_runtime::spawn_blocking(move || system_speak(&text, &voice))
                .await
                .map_err(|e| format!("Seslendirme görevi başlatılamadı: {e}"))?
        }
        Engine::ElevenLabs => {
            let key = read_key(engine)
                .ok_or_else(|| "ElevenLabs anahtarı tanımlı değil.".to_string())?;
            elevenlabs_speak(client, text, voice, &key).await
        }
        Engine::OpenAi => {
            let key =
                read_key(engine).ok_or_else(|| "OpenAI anahtarı tanımlı değil.".to_string())?;
            openai_speak(client, text, voice, &key).await
        }
        Engine::Gemini => {
            let key =
                read_key(engine).ok_or_else(|| "Gemini anahtarı tanımlı değil.".to_string())?;
            gemini_speak(client, text, voice, &key).await
        }
    }
}

/// Motorun gerçekten çalıştığını kısa bir istekle sınar.
pub async fn test_engine(client: &reqwest::Client, engine: Engine) -> Result<String, String> {
    if engine.requires_key() && !key_present(engine) {
        return Err(format!("{} anahtarı tanımlı değil.", engine.label()));
    }

    let voices = list_voices(client, engine, "tr").await?;
    if voices.is_empty() {
        return Err(format!("{} için kullanılabilir ses bulunamadı.", engine.label()));
    }

    Ok(format!("{} ses kullanılabilir", voices.len()))
}

#[cfg(test)]
mod anahtar_testleri {
    use super::{elevenlabs_error, looks_like_key_id};

    #[test]
    fn anahtar_kimligini_tanir() {
        // Ölçülen gerçek durum: 64 karakterlik onaltılık kimlik.
        assert!(looks_like_key_id(&"0123456789abcdef".repeat(4)));
        assert!(looks_like_key_id("0123456789abcdef0123456789abcdef"));
    }

    /// Anahtar zincirindeki gerçek anahtarla ElevenLabs'i yoklar ve kullanıcının
    /// göreceği mesajı basar:
    /// `cargo test canli_elevenlabs -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn canli_elevenlabs() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = crate::tts::client().unwrap();
        let sonuc = rt.block_on(super::list_voices(&client, super::Engine::ElevenLabs, "tr"));
        match sonuc {
            Ok(sesler) => println!("{} ses geldi", sesler.len()),
            Err(e) => println!("kullanıcının göreceği mesaj:\n  {e}"),
        }
    }

    #[test]
    fn gercek_anahtari_reddetmez() {
        assert!(!looks_like_key_id("sk_0123456789abcdef0123456789abcdef"));
        // Onaltılık olmayan ya da farklı uzunlukta bir şeye karışmıyoruz.
        assert!(!looks_like_key_id("bambaska-bir-deger"));
        assert!(!looks_like_key_id("0123456789abcdef"));
    }

    #[test]
    fn kimlik_hatasini_acik_anlatir() {
        let gövde = r#"{"detail":{"code":"invalid_api_key","status":"api_key_id_used_as_api_key"}}"#;
        let mesaj = elevenlabs_error(400, gövde);
        assert!(mesaj.contains("kimliği"), "mesaj sorunu adıyla söylemeli: {mesaj}");
        assert!(mesaj.contains("sk_"), "doğru anahtarın nasıl göründüğünü söylemeli");
    }

    /// Ölçülen gerçek yanıt: kısıtlı anahtar 401 döndürüyor ama sorun
    /// aboneliğin değil, izinlerin.
    #[test]
    fn eksik_izni_adiyla_soyler() {
        let gövde = "The API key you used is missing the permission text_to_speech to execute this operation.";
        let mesaj = elevenlabs_error(401, gövde);
        assert!(mesaj.contains("text_to_speech"), "eksik izni adıyla söylemeli: {mesaj}");
        assert!(!mesaj.contains("reddedildi"), "yanıltıcı 'reddedildi' mesajı çıkmamalı");
    }

    #[test]
    fn izin_adi_okunamazsa_da_yol_gosterir() {
        let mesaj = elevenlabs_error(401, r#"{"detail":{"status":"missing_permissions"}}"#);
        assert!(mesaj.contains("izin"), "yine de izin sorununu söylemeli: {mesaj}");
    }

    #[test]
    fn bilinmeyen_hatada_gövdeyi_tasir() {
        let mesaj = elevenlabs_error(418, "çaydanlık");
        assert!(mesaj.contains("418"));
        assert!(mesaj.contains("çaydanlık"), "servisin açıklaması kaybolmamalı");
    }

    #[test]
    fn gövde_yoksa_yalniz_kodu_soyler() {
        assert_eq!(elevenlabs_error(500, ""), "ElevenLabs 500 yanıtı verdi.");
    }
}

#[cfg(test)]
mod tests {
    use super::{base64_decode, Engine};

    #[test]
    fn base64_cozer() {
        assert_eq!(base64_decode("TWVyaGFiYQ==").unwrap(), b"Merhaba");
        assert_eq!(base64_decode("YQ==").unwrap(), b"a");
        assert_eq!(base64_decode("YWI=").unwrap(), b"ab");
    }

    #[test]
    fn base64_gecersiz_girdiyi_reddeder() {
        assert!(base64_decode("!!!").is_err());
    }

    /// Arka arkaya seslendirme — zincirin gerçek yük altındaki davranışı.
    /// `cargo test --lib -- --ignored gemini_ardisik --nocapture`
    #[tokio::test]
    #[ignore]
    async fn gemini_ardisik() {
        let client = crate::tts::client().unwrap();
        let metinler = [
            "Çoğu insan pirinci suyu berraklaşana kadar yıkamak gerektiğini sanıyor.",
            "Yarasaların kör olduğu yaygın bir yanılgı, gayet iyi görüyorlar.",
            "Japon balıklarının üç saniyelik hafızası olduğu doğru değil.",
            "Çin Seddi çıplak gözle uzaydan görünmüyor.",
            "Parmak çıtlatmak eklem iltihabına yol açmıyor.",
            "Beynimizin yalnızca yüzde onunu kullandığımız da doğru değil.",
        ];

        let mut basarili = 0;
        let mut toplam_bayt = 0usize;

        for (i, metin) in metinler.iter().enumerate() {
            let started = std::time::Instant::now();
            match super::speak(&client, Engine::Gemini, metin, "Kore", "tr").await {
                Ok(bytes) => {
                    let model = super::GEMINI_MODELS
                        [super::GEMINI_CURSOR.load(std::sync::atomic::Ordering::Relaxed)];
                    println!(
                        "{}/{} tamam · {} bayt · {} ms · {model}",
                        i + 1,
                        metinler.len(),
                        bytes.len(),
                        started.elapsed().as_millis()
                    );
                    basarili += 1;
                    toplam_bayt += bytes.len();
                }
                Err(e) => println!("{}/{} HATA: {e}", i + 1, metinler.len()),
            }
        }

        println!("--- {basarili}/{} parça üretildi, toplam {toplam_bayt} bayt", metinler.len());
        assert!(basarili > 0, "hiçbir parça üretilemedi");
    }

    /// Ağ + anahtar zincirindeki Gemini anahtarı gerektirir:
    /// `cargo test --lib -- --ignored gemini_modelleri --nocapture`
    #[tokio::test]
    #[ignore]
    async fn gemini_modelleri() {
        let key = super::read_key(Engine::Gemini).expect("Gemini anahtarı yok");
        let client = crate::tts::client().unwrap();

        for model in super::GEMINI_MODELS {
            let started = std::time::Instant::now();
            let result =
                super::gemini_try_model(&client, model, "Merhaba, bu bir denemedir.", "Kore", &key)
                    .await;
            let ms = started.elapsed().as_millis();

            match result {
                Ok(Some(bytes)) => println!("{model:38} ÇALIŞIYOR  {} bayt  {ms} ms", bytes.len()),
                Ok(None) => println!("{model:38} kota/meşgul  {ms} ms"),
                Err(e) if e.starts_with("__olu__") => {
                    println!("{model:38} YOK (bu anahtarla kullanılamıyor)  {ms} ms")
                }
                Err(e) => println!("{model:38} HATA: {e}  {ms} ms"),
            }
        }
    }

    #[test]
    fn gemini_zinciri_bos_degil() {
        assert!(super::GEMINI_MODELS.len() >= 2, "zincirin anlamı için en az iki model gerekir");
        // Zincirdeki adlar benzersiz olmalı, yoksa aynı kotayı iki kez deneriz
        let mut sorted = super::GEMINI_MODELS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), super::GEMINI_MODELS.len(), "zincirde tekrar eden model var");
    }

    #[test]
    fn anahtar_gerektiren_motorlar_dogru() {
        assert!(Engine::ElevenLabs.requires_key());
        assert!(Engine::OpenAi.requires_key());
        assert!(Engine::Gemini.requires_key());
        assert!(!Engine::Google.requires_key());
        assert!(!Engine::System.requires_key());
    }

    #[test]
    fn karakter_sinirlari_makul() {
        // Google'ın sınırı ölçülerek bulundu; diğerleri belgelenmiş sınırın altında
        assert_eq!(Engine::Google.max_chars(), 190);
        assert!(Engine::ElevenLabs.max_chars() > Engine::Google.max_chars());
    }
}
