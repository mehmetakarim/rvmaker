//! Seslendirme — Google Translate'in `translate_tts` uç noktası.
//!
//! API anahtarı gerektirmez. İstek başına **200 karakter** sınırı var (ölçüldü:
//! 200 → 200 OK, 210 → 400), bu yüzden metin parçalara bölünür, her parça ayrı
//! indirilir ve ffmpeg ile tek dosyada birleştirilir.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

const USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36";

/// Google'ın 200 sınırının biraz altı; kelime bütünlüğü için pay gerekiyor.
/// Diğer motorların sınırları `engines::Engine::max_chars` içinde.
#[cfg(test)]
const MAX_CHUNK_CHARS: usize = 190;

#[derive(Serialize, Clone, Debug)]
pub struct Clip {
    pub id: String,
    pub path: String,
    pub duration_sec: f64,
    /// Diskte hazır bulunup yeniden üretilmeden kullanıldı mı?
    pub reused: bool,
}

// ------------------------------------------------------- yeniden kullanım

/// FNV-1a 64 bit. Kriptografik değil; amaç yalnızca ayar değişikliğini
/// yakalamak, o yüzden bağımlılık eklemeye değmiyor.
fn fnv1a(data: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in data.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Bir ses parçasının yeniden kullanılabilirlik imzası.
///
/// Motor, ses, hız, sessizlik ya da metnin kendisi değiştiyse imza da değişir
/// ve parça yeniden üretilir. Bu kontrol olmadan yarım kalmış bir işi farklı
/// bir sesle sürdürmek videonun ortasında ses değiştirirdi.
pub fn clip_signature(engine: &str, voice: &str, speed: f64, silence_ms: u32, text: &str) -> String {
    // Hız kayan noktalı; imzada kararlı olsun diye iki basamağa sabitliyoruz.
    let malzeme = format!("{engine}|{voice}|{speed:.2}|{silence_ms}|{text}");
    format!("{:016x}", fnv1a(&malzeme))
}

/// İş klasöründeki imza defteri — hangi parçanın hangi ayarla üretildiği.
pub fn load_manifest(dir: &Path) -> std::collections::HashMap<String, String> {
    std::fs::read_to_string(dir.join("ses-imza.json"))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Defteri yazar. Her parçadan sonra çağrılıyor: iş yarıda kesilse bile
/// o ana kadar üretilenler yeniden kullanılabilir kalsın.
pub fn save_manifest(dir: &Path, manifest: &std::collections::HashMap<String, String>) {
    if let Ok(raw) = serde_json::to_string(manifest) {
        let _ = std::fs::write(dir.join("ses-imza.json"), raw);
    }
}

pub fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP istemcisi kurulamadı: {e}"))
}

// ------------------------------------------------------------ ağ dayanıklılığı

/// Geçici ağ hatalarında isteği yeniden dener.
///
/// Durum kodları (429, 5xx) için zaten tekrar deneme vardı, ama `send()`
/// hatası hattı anında kesiyordu: kısa bir Wi-Fi kesintisi ya da DNS takılması
/// yarım kalmış bir işe mal oluyordu. Yalnızca bağlantı ve zaman aşımı
/// hatalarını yeniden deniyoruz — sertifika ya da istek kurma hatasında
/// beklemenin anlamı yok.
pub async fn send_with_retry(
    request: reqwest::RequestBuilder,
    label: &str,
) -> Result<reqwest::Response, String> {
    const ATTEMPTS: u32 = 3;
    let mut last = String::new();

    for attempt in 0..ATTEMPTS {
        // Gövdesi kopyalanamayan isteklerde tek hakkımız var.
        let Some(deneme) = request.try_clone() else {
            return request
                .send()
                .await
                .map_err(|e| format!("{label}: {e}"));
        };

        match deneme.send().await {
            Ok(response) => return Ok(response),
            Err(e) => {
                let gecici = e.is_timeout() || e.is_connect();
                last = e.to_string();
                if gecici && attempt + 1 < ATTEMPTS {
                    tokio::time::sleep(Duration::from_millis(500 * u64::from(attempt + 1))).await;
                    continue;
                }
                break;
            }
        }
    }

    Err(format!("{label}: {last}"))
}

// ------------------------------------------------------- zincir işaretleri

const THREAD_EMOJI: char = '\u{1F9F5}';

/// Baştaki `1/6`, `(2/6)`, `3/` gibi sıra numarasını ayıklar.
///
/// Numaranın ardından boşluk ya da metin sonu gelmeli: `1/6 Bu konu…` ayıklanır
/// ama `3/4'lük kısmı` dokunulmadan kalır.
fn leading_numbering(s: &str) -> &str {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;

    let parantezli = matches!(chars.first(), Some('(') | Some('['));
    if parantezli {
        i += 1;
    }

    let ilk_rakam = i;
    while i < chars.len() && chars[i].is_ascii_digit() && i - ilk_rakam < 3 {
        i += 1;
    }
    if i == ilk_rakam {
        return s;
    }

    // Bölü işareti zorunlu: "1." ya da "2)" gerçek madde numarası olabiliyor.
    if chars.get(i) != Some(&'/') {
        return s;
    }
    i += 1;

    let ikinci_rakam = i;
    while i < chars.len() && chars[i].is_ascii_digit() && i - ikinci_rakam < 3 {
        i += 1;
    }

    if parantezli {
        match chars.get(i) {
            Some(')') | Some(']') => i += 1,
            _ => return s,
        }
    }

    if matches!(chars.get(i), Some(')') | Some('.') | Some(':') | Some('-')) {
        i += 1;
    }

    // Numaranın ardına harf yapışıksa bu bir sıra numarası değil.
    if let Some(c) = chars.get(i) {
        if !c.is_whitespace() {
            return s;
        }
    }

    let bayt = s.char_indices().nth(i).map(|(b, _)| b).unwrap_or(s.len());
    &s[bayt..]
}

/// Sondaki `1/6` biçimli sıra numarasını ayıklar — bazı yazarlar sona koyuyor.
fn trailing_numbering(s: &str) -> &str {
    let kirpilmis = s.trim_end();
    let Some(bosluk) = kirpilmis.rfind(char::is_whitespace) else {
        return s;
    };
    let son = kirpilmis[bosluk..].trim();
    // Son parçayı baştaki kuralla sınıyoruz: tamamı numaraysa geriye bir şey kalmaz.
    if !son.is_empty() && leading_numbering(son).is_empty() {
        return &kirpilmis[..bosluk];
    }
    s
}

/// Numaradan sonra ayraç olarak kullanılan tireyi atar.
///
/// Yalnızca ardından boşluk gelirse ayraç sayıyoruz: `- Devam` ayıklanır,
/// `-5 derece` dokunulmadan kalır.
fn strip_separator_dash(s: &str) -> &str {
    let mut chars = s.chars();
    match (chars.next(), chars.next()) {
        (Some('-' | '–' | '—'), Some(c)) if c.is_whitespace() => s[1..].trim_start(),
        _ => s,
    }
}

/// Thread numaralandırmasını seslendirme metninden ayıklar.
///
/// `🧵1/6 +Bu konu…` gibi işaretler yazarın sıralama notu; okunduğunda
/// "bir bölü altı artı" diye duyuluyor. Kart metnine dokunmuyoruz — orada
/// tweet'in özgün hâli görünsün.
pub fn strip_thread_marker(text: &str) -> String {
    let mut s = text.trim();

    // "🧵 1/6 +" gibi üst üste binen işaretler için birkaç tur.
    for _ in 0..4 {
        let onceki = s;
        s = s.trim_start();
        s = s.trim_start_matches(THREAD_EMOJI);
        s = s.trim_start();
        s = leading_numbering(s);
        s = s.trim_start();
        // "+", devam tweet'lerinde sık kullanılan bağlama işareti.
        s = s.trim_start_matches('+');
        s = strip_separator_dash(s);
        if s == onceki {
            break;
        }
    }

    s = s.trim_end_matches(THREAD_EMOJI).trim_end();
    s = trailing_numbering(s).trim();

    // Metin tamamen erirse özgün hâline dönüyoruz: sessiz kart üretmektense
    // işareti okumak yeğdir.
    if s.is_empty() {
        return text.trim().to_string();
    }
    s.to_string()
}

/// Metni seslendirilebilir parçalara böler.
///
/// Önce cümle sonlarında bölmeye çalışır; tek bir cümle bile sınırı aşıyorsa
/// kelime aralarına düşer. Hiçbir parça sınırı aşmaz.
#[cfg(test)]
pub fn split_for_tts(text: &str) -> Vec<String> {
    split_with_limit(text, MAX_CHUNK_CHARS)
}

/// Metni verilen karakter sınırına göre böler.
pub fn split_with_limit(text: &str, limit: usize) -> Vec<String> {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return Vec::new();
    }

    let mut sentences: Vec<String> = Vec::new();
    let mut current = String::new();

    for ch in normalized.chars() {
        current.push(ch);
        if matches!(ch, '.' | '!' | '?' | '…') {
            sentences.push(current.trim().to_string());
            current.clear();
        }
    }
    if !current.trim().is_empty() {
        sentences.push(current.trim().to_string());
    }

    let mut chunks: Vec<String> = Vec::new();
    let mut buffer = String::new();

    let push_buffer = |buffer: &mut String, chunks: &mut Vec<String>| {
        if !buffer.trim().is_empty() {
            chunks.push(buffer.trim().to_string());
        }
        buffer.clear();
    };

    for sentence in sentences {
        if sentence.chars().count() > limit {
            push_buffer(&mut buffer, &mut chunks);
            // Cümle tek başına sığmıyor: kelime kelime doldur.
            let mut word_buffer = String::new();
            for word in sentence.split(' ') {
                let candidate_len = word_buffer.chars().count() + word.chars().count() + 1;
                if !word_buffer.is_empty() && candidate_len > limit {
                    chunks.push(word_buffer.trim().to_string());
                    word_buffer.clear();
                }
                if !word_buffer.is_empty() {
                    word_buffer.push(' ');
                }
                word_buffer.push_str(word);
            }
            push_buffer(&mut word_buffer, &mut chunks);
            continue;
        }

        let candidate_len = buffer.chars().count() + sentence.chars().count() + 1;
        if !buffer.is_empty() && candidate_len > limit {
            push_buffer(&mut buffer, &mut chunks);
        }
        if !buffer.is_empty() {
            buffer.push(' ');
        }
        buffer.push_str(&sentence);
    }

    push_buffer(&mut buffer, &mut chunks);
    chunks
}

/// Sistem yolunda bir aracı bulur.
fn tool_path(name: &str) -> Result<String, String> {
    crate::toolpath::find_tool(name)
        .ok_or_else(|| format!("{name} bulunamadı. Kurulum ekranından yükleyebilirsin."))
}

/// Google Translate uç noktasından tek bir parça indirir.
/// `engines` modülü bunu Google motoru için çağırıyor.
pub async fn google_speak(
    client: &reqwest::Client,
    text: &str,
    lang: &str,
) -> Result<Vec<u8>, String> {
    fetch_chunk(client, text, lang, 0, 1).await
}

/// Tek bir parçayı indirir.
async fn fetch_chunk(
    client: &reqwest::Client,
    chunk: &str,
    lang: &str,
    index: usize,
    total: usize,
) -> Result<Vec<u8>, String> {
    let endpoint = format!(
        "https://translate.google.com/translate_tts?ie=UTF-8&q={}&tl={}&client=tw-ob&idx={}&total={}&textlen={}",
        urlencoding::encode(chunk),
        lang,
        index,
        total,
        chunk.chars().count()
    );

    const MAX_ATTEMPTS: u32 = 3;
    let mut last_status = 0u16;

    for attempt in 0..MAX_ATTEMPTS {
        let response = send_with_retry(
            client.get(&endpoint).header("Accept", "*/*"),
            "Seslendirme servisine ulaşılamadı",
        )
        .await?;

        let status = response.status();
        if status.is_success() {
            let bytes = response
                .bytes()
                .await
                .map_err(|e| format!("Ses verisi okunamadı: {e}"))?;
            if bytes.is_empty() {
                return Err("Seslendirme boş yanıt döndü.".to_string());
            }
            return Ok(bytes.to_vec());
        }

        last_status = status.as_u16();
        if matches!(last_status, 429 | 403 | 500..=599) && attempt + 1 < MAX_ATTEMPTS {
            tokio::time::sleep(Duration::from_millis(600 * u64::from(attempt + 1))).await;
            continue;
        }
        break;
    }

    Err(match last_status {
        400 => "Seslendirme metni kabul edilmedi (parça çok uzun olabilir).".to_string(),
        429 => "Seslendirme hız sınırına takıldı. Birkaç dakika sonra tekrar dene.".to_string(),
        code => format!("Seslendirme servisi {code} yanıtı verdi."),
    })
}

/// MP3 dosyasının süresini saniye cinsinden okur.
/// Diskteki bir mp3'ün süresini okur. Yeniden kullanılacak parçanın gerçekten
/// sağlam olduğunu da doğrulamış oluyoruz: yarım yazılmış dosyada ffprobe
/// süre veremez ve parça yeniden üretilir.
pub fn mp3_duration(path: &Path) -> Result<f64, String> {
    if !path.exists() {
        return Err("dosya yok".to_string());
    }
    let ffprobe = tool_path("ffprobe")?;
    let duration = probe_duration(&ffprobe, path)?;
    if duration <= 0.0 {
        return Err("süre sıfır".to_string());
    }
    Ok(duration)
}

fn probe_duration(ffprobe: &str, path: &Path) -> Result<f64, String> {
    let output = crate::toolpath::command(ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .output()
        .map_err(|e| format!("ffprobe çalıştırılamadı: {e}"))?;

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<f64>()
        .map_err(|_| "Ses süresi okunamadı.".to_string())
}

/// Konuşma hızı ve cümle arası sessizlik ayarları.
///
/// Google TTS'in hız parametresi yok; hızı sentez sonrası ffmpeg'in `atempo`
/// filtresiyle uyguluyoruz. Bu yaklaşım motordan bağımsız çalışır, yani
/// ElevenLabs/OpenAI gibi motorlar eklendiğinde de aynı kaydırıcı geçerli kalır.
#[derive(Clone, Copy, Debug)]
pub struct SpeechShape {
    pub speed: f64,
    pub silence_ms: u32,
}

impl Default for SpeechShape {
    fn default() -> Self {
        Self {
            speed: 1.0,
            silence_ms: 0,
        }
    }
}

/// `atempo` tek geçişte 0.5–2.0 aralığını destekler; dışına çıkarsa zincirlenir.
fn atempo_chain(speed: f64) -> Option<String> {
    let speed = speed.clamp(0.5, 3.0);
    if (speed - 1.0).abs() < 0.01 {
        return None;
    }

    let mut remaining = speed;
    let mut parts = Vec::new();
    while remaining > 2.0 {
        parts.push("atempo=2.0".to_string());
        remaining /= 2.0;
    }
    while remaining < 0.5 {
        parts.push("atempo=0.5".to_string());
        remaining /= 0.5;
    }
    parts.push(format!("atempo={remaining:.3}"));
    Some(parts.join(","))
}

/// İstenen uzunlukta sessizlik dosyası üretir.
fn make_silence(ffmpeg: &str, ms: u32, path: &Path) -> Result<(), String> {
    let seconds = f64::from(ms) / 1000.0;
    let output = crate::toolpath::command(ffmpeg)
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=24000:cl=mono",
            "-t",
            &format!("{seconds:.3}"),
            "-q:a",
            "9",
        ])
        .arg(path)
        .output()
        .map_err(|e| format!("Sessizlik üretilemedi: {e}"))?;

    if !output.status.success() {
        return Err("Sessizlik dosyası üretilemedi.".to_string());
    }
    Ok(())
}

/// Bir metni seslendirip tek bir MP3 dosyasına yazar ve süresini döndürür.
pub async fn synthesize_to_file(
    client: &reqwest::Client,
    engine: crate::engines::Engine,
    text: &str,
    voice: &str,
    lang: &str,
    out_path: &Path,
    shape: SpeechShape,
) -> Result<f64, String> {
    let chunks = split_with_limit(text, engine.max_chars());
    if chunks.is_empty() {
        return Err("Seslendirilecek metin boş.".to_string());
    }

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Çıktı klasörü oluşturulamadı: {e}"))?;
    }

    let ffprobe = tool_path("ffprobe")?;
    let tempo = atempo_chain(shape.speed);

    // Tek parça ve hız değişimi yoksa birleştirmeye hiç gerek yok.
    if chunks.len() == 1 && tempo.is_none() {
        let bytes = crate::engines::speak(client, engine, &chunks[0], voice, lang).await?;
        std::fs::write(out_path, &bytes).map_err(|e| format!("Ses dosyası yazılamadı: {e}"))?;
        return probe_duration(&ffprobe, out_path);
    }

    let ffmpeg = tool_path("ffmpeg")?;
    let work_dir = out_path.with_extension("parts");
    std::fs::create_dir_all(&work_dir)
        .map_err(|e| format!("Geçici klasör oluşturulamadı: {e}"))?;

    let mut part_paths: Vec<PathBuf> = Vec::with_capacity(chunks.len());
    for (index, chunk) in chunks.iter().enumerate() {
        let bytes = crate::engines::speak(client, engine, chunk, voice, lang).await?;
        let part = work_dir.join(format!("{index:03}.mp3"));
        std::fs::write(&part, &bytes).map_err(|e| format!("Ses parçası yazılamadı: {e}"))?;
        part_paths.push(part);
        // Uç noktayı yormamak için parçalar arasında kısa bir bekleme.
        tokio::time::sleep(Duration::from_millis(150)).await;
    }

    // Cümle arası sessizlik: parçaların arasına sessizlik dosyası serpiştiriyoruz.
    let silence_path = if shape.silence_ms > 0 && part_paths.len() > 1 {
        let path = work_dir.join("sessizlik.mp3");
        make_silence(&ffmpeg, shape.silence_ms, &path)?;
        Some(path)
    } else {
        None
    };

    // ffmpeg concat demuxer'ı için liste dosyası
    let list_path = work_dir.join("parts.txt");
    let mut lines: Vec<String> = Vec::new();
    for (index, part) in part_paths.iter().enumerate() {
        if index > 0 {
            if let Some(silence) = &silence_path {
                lines.push(format!("file '{}'", silence.display()));
            }
        }
        lines.push(format!("file '{}'", part.display()));
    }
    std::fs::write(&list_path, lines.join("\n"))
        .map_err(|e| format!("Birleştirme listesi yazılamadı: {e}"))?;

    let mut cmd = crate::toolpath::command(&ffmpeg);
    cmd.args(["-y", "-f", "concat", "-safe", "0", "-i"]).arg(&list_path);

    match &tempo {
        // Hız değişiyorsa yeniden kodlamak zorundayız.
        Some(filter) => {
            cmd.args(["-filter:a", filter, "-c:a", "libmp3lame", "-q:a", "4"]);
        }
        None => {
            cmd.args(["-c", "copy"]);
        }
    }

    let output = cmd
        .arg(out_path)
        .output()
        .map_err(|e| format!("ffmpeg çalıştırılamadı: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let tail = err.lines().rev().take(3).collect::<Vec<_>>().join(" · ");
        return Err(format!("Ses parçaları birleştirilemedi: {tail}"));
    }

    let _ = std::fs::remove_dir_all(&work_dir);
    probe_duration(&ffprobe, out_path)
}

#[cfg(test)]
mod ag_testleri {
    /// Ulaşılamayan bir adrese istek atıp yeniden denemenin gerçekten
    /// çalıştığını süreden ölçer: üç deneme + 0,5 sn ve 1,0 sn beklemeler,
    /// yani en az 1,5 saniye.
    #[test]
    #[ignore]
    fn canli_baglanti_hatasinda_yeniden_dener() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .unwrap();

        let basla = std::time::Instant::now();
        // Yönlendirilemeyen adres: bağlantı kurulamaz.
        let sonuc = rt.block_on(super::send_with_retry(
            client.get("http://127.0.0.1:9/hic"),
            "Test servisine ulaşılamadı",
        ));
        let gecen = basla.elapsed();

        println!("sonuç: {sonuc:?}");
        println!("geçen süre: {:.2} sn", gecen.as_secs_f64());
        assert!(sonuc.is_err(), "bağlantı kurulamamalıydı");
        assert!(
            sonuc.unwrap_err().starts_with("Test servisine ulaşılamadı"),
            "etiket hata mesajına geçmeli"
        );
        assert!(
            gecen.as_millis() >= 1_400,
            "iki bekleme yapılmalıydı, geçen: {} ms",
            gecen.as_millis()
        );
    }
}

#[cfg(test)]
mod imza_testleri {
    use super::clip_signature;

    fn imza(motor: &str, ses: &str, hiz: f64, sessizlik: u32, metin: &str) -> String {
        clip_signature(motor, ses, hiz, sessizlik, metin)
    }

    #[test]
    fn ayni_girdi_ayni_imza() {
        assert_eq!(
            imza("gemini", "Kore", 1.1, 320, "merhaba"),
            imza("gemini", "Kore", 1.1, 320, "merhaba")
        );
    }

    /// Asıl mesele bu: ses ayarı değiştiyse eski parça kullanılmamalı, yoksa
    /// video ortasında ses değişir.
    #[test]
    fn her_ayar_degisikligi_imzayi_degistirir() {
        let temel = imza("gemini", "Kore", 1.1, 320, "merhaba");
        assert_ne!(temel, imza("google", "Kore", 1.1, 320, "merhaba"), "motor");
        assert_ne!(temel, imza("gemini", "Puck", 1.1, 320, "merhaba"), "ses");
        assert_ne!(temel, imza("gemini", "Kore", 1.2, 320, "merhaba"), "hız");
        assert_ne!(temel, imza("gemini", "Kore", 1.1, 400, "merhaba"), "sessizlik");
        assert_ne!(temel, imza("gemini", "Kore", 1.1, 320, "selam"), "metin");
    }

    #[test]
    fn hizdaki_kucuk_kayma_imzayi_bozmaz() {
        // Kayan noktalı hız iki basamağa sabitleniyor; 1.1 ile 1.100000001
        // aynı parçayı göstermeli, yoksa her denemede yeniden üretilirdi.
        assert_eq!(
            imza("gemini", "Kore", 1.1, 320, "merhaba"),
            imza("gemini", "Kore", 1.100_000_001, 320, "merhaba")
        );
    }

    /// Komutun verdiği yeniden-kullanım kararını gerçek dosyayla sınar:
    /// `cargo test canli_yeniden_kullanim -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn canli_yeniden_kullanim() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = super::client().unwrap();
        let dir = std::env::temp_dir().join("rvmaker-surdurme");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let metin = "Bu parça yeniden üretilmemeli.";
        let sekil = super::SpeechShape { speed: 1.0, silence_ms: 0 };
        let yol = dir.join("p1.mp3");

        let sure = rt
            .block_on(super::synthesize_to_file(
                &client,
                crate::engines::Engine::Google,
                metin,
                "",
                "tr",
                &yol,
                sekil,
            ))
            .unwrap();
        println!("üretildi: {sure:.2} sn");

        let imza = super::clip_signature("google", "", sekil.speed, sekil.silence_ms, metin);
        let mut defter = std::collections::HashMap::new();
        defter.insert("p1".to_string(), imza.clone());
        super::save_manifest(&dir, &defter);

        // İkinci koşu: defter aynı imzayı veriyor ve dosya sağlam mı?
        let okunan = super::load_manifest(&dir);
        assert_eq!(okunan.get("p1"), Some(&imza), "defter geri okunmalı");
        let tekrar = super::mp3_duration(&yol).unwrap();
        println!("diskten okunan süre: {tekrar:.2} sn");
        assert!((tekrar - sure).abs() < 0.05, "süreler tutmalı");

        // Ses ayarı değişince imza tutmamalı — parça yeniden üretilir.
        let farkli = super::clip_signature("google", "", 1.25, sekil.silence_ms, metin);
        assert_ne!(okunan.get("p1"), Some(&farkli), "hız değişince kullanılmamalı");

        // Bozuk dosya yeniden kullanılmamalı.
        std::fs::write(&yol, b"bozuk").unwrap();
        assert!(super::mp3_duration(&yol).is_err(), "yarım dosya reddedilmeli");
        println!("bozuk dosya doğru şekilde reddedildi");
    }

    #[test]
    fn imza_sabit_uzunlukta() {
        assert_eq!(imza("gemini", "Kore", 1.1, 320, "merhaba").len(), 16);
    }

    #[test]
    fn alan_siniri_karisikligi_olmaz() {
        // Ayraç olmadan "ab"+"c" ile "a"+"bc" aynı imzayı verirdi.
        assert_ne!(
            imza("ab", "c", 1.0, 0, "x"),
            imza("a", "bc", 1.0, 0, "x")
        );
    }
}

#[cfg(test)]
mod zincir_isareti_testleri {
    use super::strip_thread_marker;

    #[test]
    fn olculen_ornegi_temizler() {
        // @polyperma zincirinden birebir alındı.
        let metin = "🧵2/6 +Zaten iki sarkida latin kulturunden esinlenmis gibi";
        assert_eq!(
            strip_thread_marker(metin),
            "Zaten iki sarkida latin kulturunden esinlenmis gibi"
        );
    }

    #[test]
    fn ilk_karti_temizler() {
        assert_eq!(
            strip_thread_marker("🧵1/6 Bu Crush Ateez'den caldi seyinin boku cikiyo"),
            "Bu Crush Ateez'den caldi seyinin boku cikiyo"
        );
    }

    #[test]
    fn parantezli_ve_bosluklu_bicimleri_tanir() {
        assert_eq!(strip_thread_marker("(2/6) Devam"), "Devam");
        assert_eq!(strip_thread_marker("[3/6] Devam"), "Devam");
        assert_eq!(strip_thread_marker("4/ Devam"), "Devam");
        assert_eq!(strip_thread_marker("5/6 - Devam"), "Devam");
        assert_eq!(strip_thread_marker("🧵 6/6  Devam"), "Devam");
    }

    #[test]
    fn sondaki_numarayi_da_alir() {
        assert_eq!(strip_thread_marker("Söyleyeceklerim bitti 6/6"), "Söyleyeceklerim bitti");
    }

    /// Asıl risk burada: metnin içindeki gerçek kesirler bozulmamalı.
    #[test]
    fn gercek_kesirlere_dokunmaz() {
        assert_eq!(strip_thread_marker("3/4'lük kısmı tamamlandı"), "3/4'lük kısmı tamamlandı");
        assert_eq!(
            strip_thread_marker("Nüfusun 2/3 kadarı şehirde yaşıyor"),
            "Nüfusun 2/3 kadarı şehirde yaşıyor"
        );
    }

    #[test]
    fn isaretli_sayiyi_bozmaz() {
        // Ayraç tiresi ile eksi işareti karışmamalı.
        assert_eq!(strip_thread_marker("-5 derece bekleniyor"), "-5 derece bekleniyor");
    }

    #[test]
    fn madde_numarasini_silmez() {
        // "1." bir sıra numarası değil, listenin ilk maddesi olabilir.
        assert_eq!(strip_thread_marker("1. Önce suyu kaynat"), "1. Önce suyu kaynat");
        assert_eq!(strip_thread_marker("2) Sonra tuz at"), "2) Sonra tuz at");
    }

    #[test]
    fn isaretten_ibaret_metni_bosaltmaz() {
        // Geriye okunacak bir şey kalmıyorsa özgün hâli dönüyor.
        assert_eq!(strip_thread_marker("🧵1/6"), "🧵1/6");
    }

    /// İşaretin gerçekten okunduğunu ve ayıklanınca sesten düştüğünü ölçer:
    /// `cargo test canli_isaret -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn canli_isaret() {
        let ham = "🧵2/6 +Zaten iki sarkida latin kulturunden esinlenmis gibi";
        let temiz = strip_thread_marker(ham);
        println!("ham   : {ham}");
        println!("temiz : {temiz}");

        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = super::client().unwrap();
        let dir = std::env::temp_dir().join("rvmaker-isaret");
        std::fs::create_dir_all(&dir).unwrap();
        let sekil = super::SpeechShape { speed: 1.0, silence_ms: 0 };

        let sure = |metin: &str, ad: &str| {
            let yol = dir.join(format!("{ad}.mp3"));
            rt.block_on(super::synthesize_to_file(
                &client,
                crate::engines::Engine::Google,
                metin,
                "",
                "tr",
                &yol,
                sekil,
            ))
            .unwrap()
        };

        let ham_sure = sure(ham, "ham");
        let temiz_sure = sure(&temiz, "temiz");
        println!("ham süre: {ham_sure:.2} sn · temiz süre: {temiz_sure:.2} sn");
        assert!(
            ham_sure > temiz_sure,
            "işaret ayıklanınca ses kısalmalı ({ham_sure:.2} → {temiz_sure:.2})"
        );
    }

    #[test]
    fn isaretsiz_metne_dokunmaz() {
        let metin = "Bugün hava çok güzel.";
        assert_eq!(strip_thread_marker(metin), metin);
    }
}

#[cfg(test)]
mod tests {
    use super::{split_for_tts, MAX_CHUNK_CHARS};

    #[test]
    fn normal_hizda_filtre_uygulanmaz() {
        assert_eq!(super::atempo_chain(1.0), None);
        assert_eq!(super::atempo_chain(1.004), None);
    }

    #[test]
    fn makul_hizi_tek_filtreye_cevirir() {
        assert_eq!(super::atempo_chain(1.15), Some("atempo=1.150".to_string()));
        assert_eq!(super::atempo_chain(0.75), Some("atempo=0.750".to_string()));
    }

    #[test]
    fn sinir_disi_hizi_zincirler() {
        // atempo tek geçişte 2.0'ı aşamaz; zincirlenmeli
        let chain = super::atempo_chain(2.5).unwrap();
        assert!(chain.starts_with("atempo=2.0,"), "zincir kurulmamış: {chain}");
    }

    #[test]
    fn kisa_metni_tek_parca_birakir() {
        let chunks = split_for_tts("Merhaba dünya.");
        assert_eq!(chunks, vec!["Merhaba dünya."]);
    }

    #[test]
    fn bos_metin_parca_uretmez() {
        assert!(split_for_tts("   ").is_empty());
    }

    #[test]
    fn cumle_sonlarindan_boler() {
        let sentence = format!("{} ", "Bu bir cümledir.".repeat(1));
        let text = sentence.repeat(20);
        let chunks = split_for_tts(&text);
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(
                chunk.chars().count() <= MAX_CHUNK_CHARS,
                "parça sınırı aştı: {}",
                chunk.chars().count()
            );
            assert!(chunk.ends_with('.'), "cümle ortasından bölünmüş: {chunk}");
        }
    }

    #[test]
    fn tek_uzun_cumleyi_kelimelerden_boler() {
        let text = format!("{}.", "kelime ".repeat(60));
        let chunks = split_for_tts(&text);
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.chars().count() <= MAX_CHUNK_CHARS);
            assert!(!chunk.contains("kelimekelime"), "kelime ortasından bölünmüş");
        }
    }

    /// Ağ + ffmpeg gerektirir: `cargo test --lib -- --ignored canli_seslendirme --nocapture`
    #[tokio::test]
    #[ignore]
    async fn canli_seslendirme() {
        let text = "Çoğu insan pirinci suyu berraklaşana kadar yıkamak gerektiğini sanıyor, \
                    oysa bu tam da işine yarayacak nişastayı alıp götürüyor. Yarasaların kör \
                    olduğu da yaygın bir yanılgı; gayet iyi görüyorlar ve ekolokasyon sadece \
                    üstüne eklenen fazladan bir duyu.";

        let chunks = super::split_for_tts(text);
        println!("parça sayısı: {} | karakter: {}", chunks.len(), text.chars().count());
        for (i, c) in chunks.iter().enumerate() {
            println!("  {i}: {} karakter", c.chars().count());
        }
        assert!(chunks.len() > 1, "bu metin bölünmeliydi");

        let client = super::client().unwrap();
        let out = std::env::temp_dir().join("rvmaker-test").join("deneme.mp3");
        let duration = super::synthesize_to_file(
            &client,
            crate::engines::Engine::Google,
            text,
            "",
            "tr",
            &out,
            super::SpeechShape { speed: 1.15, silence_ms: 250 },
        )
        .await
        .expect("seslendirme başarısız");

        let size = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
        println!("çıktı: {} | {size} bayt | {duration:.2} sn", out.display());
        assert!(duration > 5.0, "süre beklenenden kısa: {duration}");
        assert!(size > 10_000, "dosya beklenenden küçük: {size}");
    }

    #[test]
    fn butun_metni_korur() {
        let text = "Birinci cümle. İkinci cümle! Üçüncü cümle?";
        let joined = split_for_tts(text).join(" ");
        assert_eq!(joined, text);
    }
}
