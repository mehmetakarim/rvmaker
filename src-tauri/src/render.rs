//! Video birleştirme — ffmpeg.
//!
//! Girdi olarak sırayla eşleşen kart görselleri (1080×1920 saydam PNG) ve ses
//! parçaları alır. Her kart, kendi ses parçası çalarken ekranda durur; süreler
//! ses dosyalarından geldiği için görüntü ve ses kendiliğinden senkron olur.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

/// Varsayılan çıktı karesi — dikey 9:16. Ayarlardan değiştirilebilir.
const DEFAULT_WIDTH: u32 = 1080;
const DEFAULT_HEIGHT: u32 = 1920;

/// Arka plan verilmediğinde kullanılan düz zemin (tasarımın `--rv-bg-base` rengi).
const FALLBACK_BACKGROUND: &str = "0x14161b";

#[derive(Deserialize, Debug, Clone)]
pub struct Segment {
    /// Kart görselinin yolu (1080×1920 saydam PNG)
    pub card_path: String,
    /// Ses dosyasının yolu
    pub audio_path: String,
    pub duration_sec: f64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RenderOptions {
    pub segments: Vec<Segment>,
    /// Arka plan videosu; boşsa düz zemin üretilir.
    pub background_path: Option<String>,
    /// Arka plan müziği; boşsa eklenmez.
    pub music_path: Option<String>,
    /// 0-1 aralığında müzik seviyesi
    pub music_volume: f64,
    pub fps: u32,
    pub out_path: String,
    /// "1080x1920" biçiminde çözünürlük
    #[serde(default)]
    pub resolution: String,
    /// "h264" veya "h265"
    #[serde(default)]
    pub codec: String,
    #[serde(default)]
    pub bitrate_mbps: u32,
    #[serde(default)]
    pub hardware_accel: bool,
    /// Son karttan sonra yalnızca arka planın göründüğü kuyruk süresi.
    /// Video aniden kesilmesin diye; 0 verilirse eklenmez.
    #[serde(default)]
    pub outro_sec: f64,
}

/// "1080x1920" dizesini ayrıştırır; bozuksa varsayılana düşer.
fn parse_resolution(value: &str) -> (u32, u32) {
    let mut parts = value.split(['x', 'X', '×']);
    let width = parts.next().and_then(|p| p.trim().parse().ok());
    let height = parts.next().and_then(|p| p.trim().parse().ok());

    match (width, height) {
        (Some(w), Some(h)) if w > 0 && h > 0 => (w, h),
        _ => (DEFAULT_WIDTH, DEFAULT_HEIGHT),
    }
}

/// Kodek adını ve donanım hızlandırma tercihini ffmpeg kodlayıcısına çevirir.
fn encoder_for(codec: &str, hardware: bool) -> &'static str {
    match (codec, hardware) {
        ("h265", true) => "hevc_videotoolbox",
        ("h265", false) => "libx265",
        (_, true) => "h264_videotoolbox",
        _ => "libx264",
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct RenderResult {
    pub path: String,
    pub duration_sec: f64,
    pub size_bytes: u64,
}

/// Çalışan ffmpeg süreci — iptal edildiğinde öldürülebilsin diye tutuluyor.
static RUNNING: std::sync::Mutex<Option<u32>> = std::sync::Mutex::new(None);

/// Çalışan render'ı sonlandırır. İptal edilmiş bir işte çağrılır.
pub fn kill_running() {
    let pid = RUNNING.lock().ok().and_then(|g| *g);
    if let Some(pid) = pid {
        crate::toolpath::kill_pid(pid);
    }
}

fn tool_path(name: &str) -> Result<String, String> {
    crate::toolpath::find_tool(name)
        .ok_or_else(|| format!("{name} bulunamadı. Kurulum ekranından yükleyebilirsin."))
}

/// İstenen uzunlukta sessizlik dosyası üretir.
fn make_silence(ffmpeg: &str, seconds: f64, path: &Path) -> Result<(), String> {
    let output = Command::new(ffmpeg)
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
        .map_err(|e| format!("Kuyruk sessizliği üretilemedi: {e}"))?;

    if !output.status.success() {
        return Err("Kuyruk sessizliği üretilemedi.".to_string());
    }
    Ok(())
}

/// Ses parçalarını tek bir dosyada birleştirir; sonuna kuyruk sessizliği ekler.
fn concat_audio(
    ffmpeg: &str,
    segments: &[Segment],
    work_dir: &Path,
    outro_sec: f64,
) -> Result<PathBuf, String> {
    let list_path = work_dir.join("ses-listesi.txt");
    let mut lines: Vec<String> = segments
        .iter()
        .map(|s| format!("file '{}'", s.audio_path))
        .collect();

    // Son kartın ardından video birden kesilmesin diye sessiz bir kuyruk.
    if outro_sec > 0.0 {
        let silence = work_dir.join("kuyruk-sessizlik.mp3");
        make_silence(ffmpeg, outro_sec, &silence)?;
        lines.push(format!("file '{}'", silence.display()));
    }

    let body = lines.join("\n");
    std::fs::write(&list_path, body).map_err(|e| format!("Ses listesi yazılamadı: {e}"))?;

    let out = work_dir.join("ses.mp3");
    let output = Command::new(ffmpeg)
        .args(["-y", "-f", "concat", "-safe", "0", "-i"])
        .arg(&list_path)
        .args(["-c", "copy"])
        .arg(&out)
        .output()
        .map_err(|e| format!("ffmpeg çalıştırılamadı: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let tail = err.lines().rev().take(3).collect::<Vec<_>>().join(" · ");
        return Err(format!("Ses parçaları birleştirilemedi: {tail}"));
    }

    Ok(out)
}

/// Kartların ekranda kalacağı zaman aralıklarını üreten filtre zincirini kurar.
///
/// Her kart bir `overlay` katmanı; `enable` ifadesi kartın yalnızca kendi ses
/// parçası çalarken görünmesini sağlıyor.
fn build_filter(
    segments: &[Segment],
    has_background: bool,
    total: f64,
    music_volume: Option<f64>,
    width: u32,
    height: u32,
    fade_sec: f64,
) -> String {
    let mut filter = String::new();

    // Arka planı kareye oturt: kırpmadan doldur, sonra ortadan kes.
    if has_background {
        filter.push_str(&format!(
            "[0:v]scale={width}:{height}:force_original_aspect_ratio=increase,\
             crop={width}:{height},setsar=1,trim=duration={total:.3},setpts=PTS-STARTPTS[bg];"
        ));
    } else {
        filter.push_str(&format!(
            "[0:v]scale={width}:{height},setsar=1,trim=duration={total:.3},setpts=PTS-STARTPTS[bg];"
        ));
    }

    // Görsel girdiler ses girdisinden sonra geliyor: 0=arka plan, 1=ses, 2..=kartlar
    let mut current = "bg".to_string();
    let mut start = 0.0f64;

    for (index, segment) in segments.iter().enumerate() {
        let end = start + segment.duration_sec;
        let input = index + 2;
        let label = format!("v{index}");

        // Kartlar 1080 genişlikte üretiliyor; hedef kare farklıysa ölçekleniyor.
        if width != DEFAULT_WIDTH {
            filter.push_str(&format!("[{input}:v]scale={width}:-1[k{index}];"));
            filter.push_str(&format!(
                "[{current}][k{index}]overlay=(W-w)/2:(H-h)/2:enable='between(t,{start:.3},{end:.3})'[{label}];"
            ));
        } else {
            filter.push_str(&format!(
                "[{current}][{input}:v]overlay=(W-w)/2:(H-h)/2:enable='between(t,{start:.3},{end:.3})'[{label}];"
            ));
        }

        current = label;
        start = end;
    }

    // Müzik varsa konuşmanın altına karıştır. Müzik girdisi kartlardan sonra
    // geldiği için indisi 2 + kart sayısı.
    if let Some(volume) = music_volume {
        let music_input = segments.len() + 2;

        // Müzik kapanış payında birden kesilmesin; sona doğru yumuşakça sönüyor.
        let fade = if fade_sec > 0.05 {
            let start = (total - fade_sec).max(0.0);
            format!(",afade=t=out:st={start:.3}:d={fade_sec:.3}")
        } else {
            String::new()
        };

        // `dynaudnorm` KULLANMA: 15×250 ms'lik ön-belleğini çıktıya geri
        // vermiyor ve sesin son ~4 saniyesini yutuyor. Ölçüldü — 38,5 sn'lik
        // bir mikste yalnızca 34,5 sn örnek üretti, videonun sonunda
        // seslendirme ve müzik birden kesiliyordu. `speechnorm` aynı ses
        // düzeyini veriyor (-17,9 dB / -0,4 dB tepe) ama süreyi koruyor.
        filter.push_str(&format!(
            "[{music_input}:a]volume={volume:.3}[muzik];\
             [1:a][muzik]amix=inputs=2:duration=first:dropout_transition=0,\
             speechnorm=e=6.25:r=0.00001:l=1{fade}[ses];"
        ));
    }

    // Son etiketin sonundaki noktalı virgülü at
    if filter.ends_with(';') {
        filter.pop();
    }

    filter
}

fn probe_duration(ffprobe: &str, path: &Path) -> Result<f64, String> {
    let output = Command::new(ffprobe)
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0"])
        .arg(path)
        .output()
        .map_err(|e| format!("ffprobe çalıştırılamadı: {e}"))?;

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<f64>()
        .map_err(|_| "Video süresi okunamadı.".to_string())
}

/// Kartları, sesi ve arka planı tek bir MP4'te birleştirir.
pub fn render(options: &RenderOptions) -> Result<RenderResult, String> {
    if options.segments.is_empty() {
        return Err("Birleştirilecek bölüm yok.".to_string());
    }

    let ffmpeg = tool_path("ffmpeg")?;
    let ffprobe = tool_path("ffprobe")?;

    let out_path = PathBuf::from(&options.out_path);
    let work_dir = out_path
        .parent()
        .ok_or_else(|| "Çıktı yolu geçersiz.".to_string())?
        .to_path_buf();
    std::fs::create_dir_all(&work_dir)
        .map_err(|e| format!("Çıktı klasörü oluşturulamadı: {e}"))?;

    // Kuyruk süresi hem sese hem videoya ekleniyor; kart pencereleri
    // konuşma bitiminde kapandığı için son saniyelerde yalnızca arka plan kalıyor.
    let outro = options.outro_sec.clamp(0.0, 15.0);
    let audio_path = concat_audio(&ffmpeg, &options.segments, &work_dir, outro)?;
    let speech: f64 = options.segments.iter().map(|s| s.duration_sec).sum();
    let total = speech + outro;

    let background = options
        .background_path
        .as_deref()
        .filter(|p| !p.trim().is_empty() && Path::new(p).exists());

    let (width, height) = parse_resolution(&options.resolution);

    let mut cmd = Command::new(&ffmpeg);
    cmd.arg("-y");

    // 0 — arka plan
    match background {
        Some(path) => {
            cmd.args(["-stream_loop", "-1", "-i"]).arg(path);
        }
        None => {
            cmd.args([
                "-f",
                "lavfi",
                "-i",
                &format!(
                    "color=c={FALLBACK_BACKGROUND}:s={width}x{height}:r={}",
                    options.fps
                ),
            ]);
        }
    }

    // 1 — birleştirilmiş konuşma sesi
    cmd.arg("-i").arg(&audio_path);

    // 2.. — kart görselleri
    for segment in &options.segments {
        cmd.arg("-i").arg(&segment.card_path);
    }

    // Son girdi — arka plan müziği (varsa). Konuşmadan kısaysa döngüye alınır.
    let music = options
        .music_path
        .as_deref()
        .filter(|p| !p.trim().is_empty() && Path::new(p).exists());

    if let Some(path) = music {
        cmd.args(["-stream_loop", "-1", "-i"]).arg(path);
    }

    let music_volume = music.map(|_| options.music_volume.clamp(0.0, 1.0));

    // Kapanış payı varsa müzik o pay boyunca sönsün; yoksa son 1,5 saniyede.
    let fade_sec = if outro > 0.0 { outro } else { 1.5_f64.min(total) };

    let filter = build_filter(
        &options.segments,
        background.is_some(),
        total,
        music_volume,
        width,
        height,
        fade_sec,
    );
    let last_label = format!("v{}", options.segments.len() - 1);

    cmd.args(["-filter_complex", &filter]);
    cmd.args(["-map", &format!("[{last_label}]")]);
    cmd.args(["-map", if music_volume.is_some() { "[ses]" } else { "1:a" }]);
    let encoder = encoder_for(&options.codec, options.hardware_accel);
    cmd.args(["-c:v", encoder]);

    // `preset` yalnızca yazılım kodlayıcılarda var; VideoToolbox reddediyor.
    if encoder.starts_with("libx") {
        cmd.args(["-preset", "veryfast"]);
    }

    if options.bitrate_mbps > 0 {
        cmd.args(["-b:v", &format!("{}M", options.bitrate_mbps)]);
    }

    cmd.args([
        "-pix_fmt",
        "yuv420p",
        "-r",
        &options.fps.to_string(),
        "-c:a",
        "aac",
        "-b:a",
        "192k",
        "-shortest",
    ]);
    cmd.arg(&out_path);

    // Süreci elde tutuyoruz ki iptal edildiğinde öldürebilelim.
    let child = cmd
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("ffmpeg çalıştırılamadı: {e}"))?;

    if let Ok(mut guard) = RUNNING.lock() {
        *guard = Some(child.id());
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("ffmpeg beklenirken hata: {e}"))?;

    if let Ok(mut guard) = RUNNING.lock() {
        *guard = None;
    }

    if !output.status.success() {
        // Sinyalle sonlandırıldıysa bu bir hata değil, kullanıcı iptali.
        if output.status.code().is_none() {
            let _ = std::fs::remove_file(&out_path);
            return Err("İptal edildi.".to_string());
        }
        let err = String::from_utf8_lossy(&output.stderr);
        let tail = err
            .lines()
            .filter(|l| !l.trim().is_empty())
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .join(" · ");
        return Err(format!("Video birleştirilemedi: {tail}"));
    }

    let duration = probe_duration(&ffprobe, &out_path)?;
    let size_bytes = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);

    Ok(RenderResult {
        path: out_path.to_string_lossy().to_string(),
        duration_sec: duration,
        size_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::{build_filter, Segment};

    fn segment(duration: f64) -> Segment {
        Segment {
            card_path: "/tmp/kart.png".to_string(),
            audio_path: "/tmp/ses.mp3".to_string(),
            duration_sec: duration,
        }
    }

    /// Gerçek iş klasörüyle uçtan uca render.
    /// `cargo test --lib -- --ignored canli_render --nocapture`
    #[test]
    #[ignore]
    fn canli_render() {
        let job = std::env::var("RV_JOB_DIR").expect("RV_JOB_DIR verilmedi");
        let job = std::path::PathBuf::from(job);

        // Kart ve ses dosyalarını kimliklerine göre eşleştir.
        let mut segments: Vec<super::Segment> = Vec::new();
        let ffprobe = super::tool_path("ffprobe").unwrap();

        let mut ids: Vec<String> = std::fs::read_dir(&job)
            .unwrap()
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                if p.extension()?.to_str()? == "mp3" {
                    Some(p.file_stem()?.to_str()?.to_string())
                } else {
                    None
                }
            })
            .collect();
        // Başlık her zaman önce gelsin, kalanı ada göre sırala.
        ids.sort();
        ids.sort_by_key(|id| if id == "baslik" { 0 } else { 1 });

        for id in &ids {
            let audio = job.join(format!("{id}.mp3"));
            let card = job.join("kartlar").join(format!("{id}.png"));
            if !card.exists() {
                continue;
            }
            let duration = super::probe_duration(&ffprobe, &audio).unwrap();
            segments.push(super::Segment {
                card_path: card.to_string_lossy().to_string(),
                audio_path: audio.to_string_lossy().to_string(),
                duration_sec: duration,
            });
        }

        println!("bölüm sayısı: {}", segments.len());
        assert!(!segments.is_empty(), "eşleşen bölüm bulunamadı");

        let options = super::RenderOptions {
            segments,
            background_path: std::env::var("RV_BG").ok(),
            music_path: None,
            music_volume: 0.15,
            fps: 30,
            out_path: job.join("video.mp4").to_string_lossy().to_string(),
            resolution: "1080x1920".to_string(),
            codec: "h264".to_string(),
            bitrate_mbps: 8,
            hardware_accel: false,
            outro_sec: 5.0,
        };

        let started = std::time::Instant::now();
        let result = super::render(&options).expect("render başarısız");
        println!(
            "çıktı: {} | {:.1} sn | {:.1} MB | render süresi {:.0} sn",
            result.path,
            result.duration_sec,
            result.size_bytes as f64 / 1_048_576.0,
            started.elapsed().as_secs_f64()
        );
        assert!(result.duration_sec > 1.0);
    }

    #[test]
    fn zaman_araliklari_ardisik_ilerler() {
        let filter = build_filter(&[segment(2.0), segment(3.0)], true, 5.0, None, 1080, 1920, 0.0);
        assert!(filter.contains("between(t,0.000,2.000)"), "ilk aralık hatalı: {filter}");
        assert!(filter.contains("between(t,2.000,5.000)"), "ikinci aralık hatalı: {filter}");
    }

    #[test]
    fn kart_girdileri_sesten_sonra_numaralanir() {
        let filter = build_filter(&[segment(1.0), segment(1.0)], true, 2.0, None, 1080, 1920, 0.0);
        // 0 = arka plan, 1 = ses, kartlar 2'den başlar
        assert!(filter.contains("[2:v]overlay"), "ilk kart girdisi hatalı: {filter}");
        assert!(filter.contains("[3:v]overlay"), "ikinci kart girdisi hatalı: {filter}");
    }

    #[test]
    fn arkaplan_yokken_kirpma_uygulanmaz() {
        let filter = build_filter(&[segment(1.0)], false, 1.0, None, 1080, 1920, 0.0);
        assert!(!filter.contains("crop="), "düz zeminde kırpma olmamalı: {filter}");
    }

    #[test]
    fn kuyruk_suresi_kart_pencerelerini_uzatmaz() {
        // Kuyruk yalnızca toplam süreyi uzatır; son kart konuşma bitiminde kapanmalı.
        let filter = build_filter(&[segment(2.0), segment(3.0)], true, 10.0, None, 1080, 1920, 0.0);
        assert!(
            filter.contains("between(t,2.000,5.000)"),
            "son kart kuyruğa taşmış: {filter}"
        );
        assert!(
            filter.contains("trim=duration=10.000"),
            "arka plan kuyruk kadar uzamamış: {filter}"
        );
    }

    #[test]
    fn cozunurluk_ayristirilir() {
        assert_eq!(super::parse_resolution("720x1280"), (720, 1280));
        assert_eq!(super::parse_resolution("1440×2560"), (1440, 2560));
        // Bozuk değerde varsayılana düşer
        assert_eq!(super::parse_resolution("saçma"), (1080, 1920));
        assert_eq!(super::parse_resolution(""), (1080, 1920));
    }

    #[test]
    fn kodek_secimi_dogru() {
        assert_eq!(super::encoder_for("h264", false), "libx264");
        assert_eq!(super::encoder_for("h264", true), "h264_videotoolbox");
        assert_eq!(super::encoder_for("h265", false), "libx265");
        assert_eq!(super::encoder_for("h265", true), "hevc_videotoolbox");
    }

    #[test]
    fn farkli_cozunurlukte_kartlar_olceklenir() {
        let filter = build_filter(&[segment(1.0)], true, 1.0, None, 720, 1280, 0.0);
        assert!(filter.contains("scale=720:-1[k0]"), "kart ölçeklenmemiş: {filter}");
    }

    #[test]
    fn filtre_noktali_virgulle_bitmez() {
        let filter = build_filter(&[segment(1.0)], true, 1.0, None, 1080, 1920, 0.0);
        assert!(!filter.ends_with(';'), "filtre zinciri hatalı bitiyor: {filter}");
    }

    /// `dynaudnorm` ön-belleğini geri vermiyor ve sesin sonunu yutuyordu.
    /// Videonun son saniyelerinde seslendirme ve müzik birden kesiliyor,
    /// ekranda yalnızca arka plan kalıyordu.
    #[test]
    fn ses_normalizasyonu_dynaudnorm_kullanmaz() {
        let filter = build_filter(&[segment(2.0)], true, 2.0, Some(0.1), 1080, 1920, 1.0);
        assert!(
            !filter.contains("dynaudnorm"),
            "dynaudnorm sesin sonunu yutuyor, kullanılmamalı: {filter}"
        );
        assert!(filter.contains("speechnorm"), "normalizasyon eksik: {filter}");
    }

    /// Karışımdan geçen sesin süresi korunuyor mu?
    ///
    /// **Sentetik sesle bu hata tekrarlanmıyor** — sinüs ve sessizlik
    /// karışımları denendi, `dynaudnorm` onlarda süreyi koruyordu. Hatayı
    /// ancak gerçek üretim sesi ortaya çıkardı. Bu yüzden test, varsa
    /// `~/Movies/RVMaker` altındaki gerçek bir `ses.mp3`'ü kullanıyor.
    /// Bulamazsa atlıyor: yanlış bir güven vermesin.
    ///
    /// `cargo test canli_ses_suresi -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn canli_ses_suresi_korunuyor() {
        use std::process::Command;

        let Some(home) = crate::toolpath::home_dir() else {
            println!("ev dizini yok — atlanıyor");
            return;
        };
        let kok = home.join("Movies/RVMaker");
        let Some(konusma) = std::fs::read_dir(&kok).ok().and_then(|girdiler| {
            girdiler
                .flatten()
                .map(|e| e.path().join("ses.mp3"))
                .find(|p| p.exists())
        }) else {
            println!("gerçek ses.mp3 bulunamadı ({}) — atlanıyor", kok.display());
            return;
        };
        println!("kullanılan ses: {}", konusma.display());

        let ffmpeg = super::tool_path("ffmpeg").expect("ffmpeg gerekli");
        let dir = std::env::temp_dir().join("rvmaker-ses-suresi");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let muzik = dir.join("muzik.wav");
        let cikti = dir.join("karisim.m4a");

        assert!(
            Command::new(&ffmpeg)
                .args([
                    "-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i",
                    "aevalsrc=0.2*sin(220*2*PI*t):d=200:s=44100", "-y",
                ])
                .arg(&muzik)
                .status()
                .unwrap()
                .success(),
            "müzik üretilemedi"
        );

        let ham_sure = |yol: &std::path::Path| -> f64 {
            let out = Command::new(&ffmpeg)
                .args(["-hide_banner", "-loglevel", "error"])
                .arg("-i")
                .arg(yol)
                .args(["-ac", "1", "-ar", "8000", "-f", "s16le", "-"])
                .output()
                .unwrap();
            out.stdout.len() as f64 / 16_000.0
        };

        let girdi_sn = ham_sure(&konusma);

        // Üretimdeki zincirin aynısı.
        let filtre = "[1:a]volume=0.100[m];\
                      [0:a][m]amix=inputs=2:duration=first:dropout_transition=0,\
                      speechnorm=e=6.25:r=0.00001:l=1[ses]";

        assert!(
            Command::new(&ffmpeg)
                .args(["-hide_banner", "-loglevel", "error"])
                .arg("-i").arg(&konusma)
                .arg("-i").arg(&muzik)
                .args(["-filter_complex", filtre, "-map", "[ses]", "-c:a", "aac", "-y"])
                .arg(&cikti)
                .status()
                .unwrap()
                .success(),
            "karışım üretilemedi"
        );

        // Konteyner süresi doğru görünürken içeride delik olabiliyor; hata tam
        // da buydu. O yüzden ham örnek sayıyoruz.
        let cikti_sn = ham_sure(&cikti);
        println!("girdi {girdi_sn:.2} sn → çıktı {cikti_sn:.2} sn");

        assert!(
            cikti_sn > girdi_sn - 0.5,
            "ses kırpılmış: girdi {girdi_sn:.2} sn, çıktı {cikti_sn:.2} sn"
        );
    }

    #[test]
    fn muzik_kart_girdilerinden_sonra_numaralanir() {
        // 0=arka plan, 1=konuşma, 2-3=kartlar, 4=müzik
        let filter = build_filter(&[segment(1.0), segment(1.0)], true, 2.0, Some(0.15), 1080, 1920, 0.0);
        assert!(filter.contains("[4:a]volume=0.150"), "müzik girdisi hatalı: {filter}");
        assert!(filter.contains("amix=inputs=2"), "karışım eksik: {filter}");
    }

    #[test]
    fn muzik_kapanista_soner() {
        let filter = build_filter(&[segment(10.0)], true, 15.0, Some(0.15), 1080, 1920, 5.0);
        assert!(
            filter.contains("afade=t=out:st=10.000:d=5.000"),
            "sönme filtresi kurulmamış: {filter}"
        );
    }

    #[test]
    fn sonme_suresi_sifirsa_filtre_eklenmez() {
        let filter = build_filter(&[segment(1.0)], true, 1.0, Some(0.15), 1080, 1920, 0.0);
        assert!(!filter.contains("afade"), "gereksiz sönme eklenmiş: {filter}");
    }

    #[test]
    fn muziksizken_karisim_kurulmaz() {
        let filter = build_filter(&[segment(1.0)], true, 1.0, None, 1080, 1920, 0.0);
        assert!(!filter.contains("amix"), "müzik yokken karışım olmamalı: {filter}");
    }

    #[test]
    fn son_etiket_bolum_sayisiyla_uyusur() {
        let filter = build_filter(&[segment(1.0), segment(1.0), segment(1.0)], true, 3.0, None, 1080, 1920, 0.0);
        assert!(filter.contains("[v2]"), "son etiket eksik: {filter}");
    }
}
