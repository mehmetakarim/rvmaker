//! X (Twitter) erişimi.
//!
//! İki katman var:
//!
//! * **Tweet içeriği** — `api.fxtwitter.com` aynası üzerinden, kimlik gerekmez.
//!   Metin, yazar, beğeni/RT/yanıt sayıları buradan geliyor.
//! * **Yanıt zinciri** — X'in kendi API'si gerekiyor, o da oturum çerezlerini
//!   (`auth_token` + `ct0`) istiyor. Kullanıcı bunları Ayarlar'dan giriyor.
//!
//! Reddit'teki gibi çerezler sistem anahtar zincirinde saklanıyor.

use serde::{Deserialize, Serialize};

const KEYCHAIN_SERVICE: &str = "com.rvmaker.desktop";
const AUTH_TOKEN_ENTRY: &str = "x-auth-token";
const CT0_ENTRY: &str = "x-ct0";

const BROWSER_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36";

// ------------------------------------------------------------------ kimlik

fn entry(account: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, account)
        .map_err(|e| format!("Anahtar zincirine erişilemedi: {e}"))
}

fn read_entry(account: &str) -> Option<String> {
    let value = entry(account).ok()?.get_password().ok()?;
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Kullanıcının yapıştırdığı metinden `auth_token` ve `ct0` değerlerini çıkarır.
///
/// Üç biçimi de kabul ediyor: Cookie-Editor JSON çıktısı, tarayıcıdan kopyalanan
/// `ad=değer; ad=değer` başlık dizesi, ya da yalnız iki değerin kendisi.
pub fn extract_credentials(raw: &str) -> Option<(String, String)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut auth = String::new();
    let mut ct0 = String::new();

    // Cookie-Editor JSON çıktısı
    if trimmed.starts_with('[') || trimmed.starts_with('{') {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
            let items: Vec<&serde_json::Value> = match &parsed {
                serde_json::Value::Array(list) => list.iter().collect(),
                other => vec![other],
            };
            for item in items {
                let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let value = item.get("value").and_then(|v| v.as_str()).unwrap_or("");
                match name {
                    "auth_token" => auth = value.trim().to_string(),
                    "ct0" => ct0 = value.trim().to_string(),
                    _ => {}
                }
            }
        }
    }

    // Başlık dizesi ya da serbest metin
    if auth.is_empty() || ct0.is_empty() {
        for part in trimmed.split([';', '\n', ' ']) {
            let part = part.trim().trim_start_matches("Cookie:").trim();
            if let Some(value) = part.strip_prefix("auth_token=") {
                auth = value.trim().to_string();
            } else if let Some(value) = part.strip_prefix("ct0=") {
                ct0 = value.trim().to_string();
            }
        }
    }

    if auth.is_empty() || ct0.is_empty() {
        None
    } else {
        Some((auth, ct0))
    }
}

pub fn store_credentials(raw: &str) -> Result<(), String> {
    let (auth, ct0) = extract_credentials(raw).ok_or_else(|| {
        "Çerezlerde `auth_token` ve `ct0` bulunamadı. Cookie-Editor'ün JSON çıktısını ya da \
         tarayıcıdan kopyaladığın Cookie başlığının tamamını yapıştır."
            .to_string()
    })?;

    crate::keychain::write(KEYCHAIN_SERVICE, AUTH_TOKEN_ENTRY, &auth)?;
    crate::keychain::write(KEYCHAIN_SERVICE, CT0_ENTRY, &ct0)
}

pub fn clear_credentials() -> Result<(), String> {
    for account in [AUTH_TOKEN_ENTRY, CT0_ENTRY] {
        if let Ok(e) = entry(account) {
            match e.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => {}
                Err(err) => return Err(format!("Çerez silinemedi: {err}")),
            }
        }
    }
    Ok(())
}

pub fn credentials_present() -> bool {
    read_entry(AUTH_TOKEN_ENTRY).is_some() && read_entry(CT0_ENTRY).is_some()
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(BROWSER_USER_AGENT)
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| format!("HTTP istemcisi kurulamadı: {e}"))
}

/// `bird` CLI'ı bulur. X'in iç API'si sık değiştiği için kendi istemcimizi
/// yazmak yerine, bu değişiklikleri takip eden bakımlı bir aracı çağırıyoruz.
/// nvm ile kurulmuş sürümler PATH'te olmayabiliyor, o yüzden orayı da tarıyoruz.
pub fn bird_path() -> Option<String> {
    if let Some(path) = crate::toolpath::find_tool("bird") {
        return Some(path);
    }

    // nvm kurulumları
    let home = crate::toolpath::home_dir()?;
    let versions = home.join(".nvm/versions/node");
    let entries = std::fs::read_dir(versions).ok()?;
    for entry in entries.flatten() {
        let candidate = entry.path().join("bin/bird");
        if candidate.exists() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

/// bird'ün nasıl çalıştırılacağı: program ve ondan önce gelecek argümanlar.
#[derive(Debug, Clone, PartialEq)]
pub struct BirdCommand {
    pub program: String,
    pub prefix: Vec<String>,
}

/// `package.json`'daki `bin` alanından bir komutun giriş dosyasını çıkarır.
/// Alan hem düz dize (`"dist/index.js"`) hem ad→yol eşlemesi olabiliyor.
pub fn bin_entry(package_json: &str, bin_name: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(package_json).ok()?;
    match v.get("bin")? {
        serde_json::Value::String(yol) => Some(yol.clone()),
        serde_json::Value::Object(harita) => harita
            .get(bin_name)
            .and_then(|x| x.as_str())
            .map(str::to_string),
        _ => None,
    }
}

/// npm kabuğunun durduğu klasörden `node <giriş dosyası>` komutunu kurar.
///
/// Windows'ta `bird.cmd`'yi doğrudan çalıştırmak iki yerden kırılgan: uzantısız
/// Unix betiği seçilirse os error 193, `.cmd` seçilirse argümanlar toplu iş
/// kaçış kurallarından geçiyor ve arama sorgusundaki boşluk/tırnak/iki nokta
/// bozulabiliyor. Gerçek JS dosyasını node ile çalıştırmak ikisinden de kaçıyor.
pub fn node_command_for(shim_dir: &std::path::Path, node: &str) -> Option<BirdCommand> {
    let paket = shim_dir.join("node_modules").join("@steipete").join("bird");
    let json = std::fs::read_to_string(paket.join("package.json")).ok()?;
    let giris = paket.join(bin_entry(&json, "bird")?);
    if !giris.is_file() || node.is_empty() {
        return None;
    }
    Some(BirdCommand {
        program: node.to_string(),
        prefix: vec![giris.to_string_lossy().to_string()],
    })
}

/// bird'ü çalıştıracak komutu belirler.
pub fn bird_command() -> Option<BirdCommand> {
    let bird = bird_path()?;
    if cfg!(target_os = "windows") {
        let dizin = std::path::Path::new(&bird).parent();
        let node = crate::toolpath::find_tool("node");
        if let (Some(dizin), Some(node)) = (dizin, node) {
            if let Some(komut) = node_command_for(dizin, &node) {
                return Some(komut);
            }
        }
    }
    Some(BirdCommand { program: bird, prefix: Vec::new() })
}

/// Saklanan çerezlerle `bird` çalıştırır.
///
/// Çerezleri açıkça geçiyoruz; bird'ün tarayıcıdan kendi okuması, `ct0`
/// eşleşmediğinde 353 hatası veriyor (ölçüldü).
fn run_bird(args: &[&str]) -> Result<String, String> {
    let bird = bird_command().ok_or_else(|| {
        "bird CLI bulunamadı. Kurmak için: npm install -g @steipete/bird".to_string()
    })?;
    let auth = read_entry(AUTH_TOKEN_ENTRY)
        .ok_or_else(|| "X çerezleri tanımlı değil.".to_string())?;
    let ct0 = read_entry(CT0_ENTRY).ok_or_else(|| "X çerezleri tanımlı değil.".to_string())?;

    // bird bir node betiği ve `#!/usr/bin/env node` ile başlıyor. Kendi
    // sürümünün `node` ikilisi yanı başında duruyor; onu PATH'in başına
    // koyuyoruz ki başka bir node sürümüne düşmesin ya da hiç bulunamasın.
    let mut child_path = std::env::var("PATH").unwrap_or_default();
    if let Some(dir) = std::path::Path::new(&bird.program).parent() {
        // Ayırıcı platforma göre: Windows'ta `:` sürücü harflerini bölüyordu.
        child_path = crate::toolpath::prepend(&dir.to_string_lossy(), &child_path);
    }

    let output = crate::toolpath::command(&bird.program)
        .env("PATH", &child_path)
        .args(&bird.prefix)
        .args(["--auth-token", &auth, "--ct0", &ct0, "--plain"])
        .args(args)
        .output()
        .map_err(|e| format!("bird çalıştırılamadı: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() || stdout.trim().is_empty() {
        let combined = format!("{stdout}{stderr}");
        let detail: String = combined
            .lines()
            .filter(|l| !l.contains("No Twitter cookies found"))
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(200)
            .collect();

        return Err(if detail.contains("env: node") || detail.contains("node: command not found") {
            "X aracı çalıştırılamadı: `node` bulunamadı. Node.js kurulu değilse kur, \
             kuruluysa uygulamayı yeniden başlat."
                .to_string()
        } else if detail.contains("353") || detail.contains("csrf") {
            "X çerezleri eşleşmedi — `auth_token` ve `ct0` aynı oturumdan, aynı anda alınmalı.              Tarayıcıdan ikisini birlikte yeniden kopyala."
                .to_string()
        } else if detail.contains("401") || detail.contains("code\":32") {
            "X çerezleri geçersiz veya süresi dolmuş.".to_string()
        } else {
            format!("X isteği başarısız: {detail}")
        });
    }

    Ok(stdout)
}

/// Saklanan çerezlerin hâlâ geçerli olup olmadığını sınar; hesabın kullanıcı
/// adını döndürür.
pub async fn verify_credentials() -> Result<String, String> {
    let output = tokio::task::spawn_blocking(|| run_bird(&["whoami"]))
        .await
        .map_err(|e| format!("Doğrulama görevi başlatılamadı: {e}"))??;

    // `bird whoami --plain` çıktısı: "user: @ad (Görünen Ad)"
    let account = output
        .lines()
        .find_map(|line| line.trim().strip_prefix("user:"))
        .map(|v| v.trim().to_string())
        .unwrap_or_else(|| "hesap adı okunamadı".to_string());

    Ok(format!("{account} olarak bağlanıldı"))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Reply {
    pub id: String,
    pub author: String,
    pub text: String,
    pub likes: i64,
}

/// Bir tweet'in yanıtlarını getirir — video için "yorumlar" bunlar.
pub async fn fetch_replies(url_or_id: &str, limit: usize) -> Result<Vec<Reply>, String> {
    let id = extract_tweet_id(url_or_id)
        .ok_or_else(|| "Bağlantıdan tweet kimliği çıkarılamadı.".to_string())?;

    let raw = tokio::task::spawn_blocking(move || run_bird(&["replies", "--json", &id]))
        .await
        .map_err(|e| format!("Yanıt görevi başlatılamadı: {e}"))??;

    // bird'ün çıktısında uyarı satırları olabiliyor; ilk JSON gövdesini alıyoruz.
    let json_start = raw
        .find(['[', '{'])
        .ok_or_else(|| "bird beklenen JSON çıktısını vermedi.".to_string())?;
    let payload: serde_json::Value = serde_json::from_str(raw[json_start..].trim())
        .map_err(|e| format!("bird çıktısı okunamadı: {e}"))?;

    let items: Vec<&serde_json::Value> = match &payload {
        serde_json::Value::Array(list) => list.iter().collect(),
        serde_json::Value::Object(map) => map
            .get("replies")
            .or_else(|| map.get("tweets"))
            .and_then(|v| v.as_array())
            .map(|l| l.iter().collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    };

    let mut replies: Vec<Reply> = items
        .iter()
        .filter_map(|item| {
            let text = item
                .get("text")
                .or_else(|| item.get("full_text"))
                .and_then(|v| v.as_str())?
                .trim()
                .to_string();
            if text.is_empty() {
                return None;
            }
            let author = item
                .get("author")
                .and_then(|a| a.get("screen_name").or_else(|| a.get("username")))
                .or_else(|| item.get("username"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            Some(Reply {
                id: item
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                author: format!("@{author}"),
                text: crate::reddit::clean_comment_body(strip_leading_mentions(&text)),
                likes: bird_count(item, "likeCount", "likes"),
            })
        })
        .filter(|r| r.text.chars().count() >= 3)
        .collect();

    replies.sort_by(|a, b| b.likes.cmp(&a.likes));
    replies.truncate(limit.clamp(1, 50));
    Ok(replies)
}

// ------------------------------------------------------------------ içerik

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tweet {
    pub id: String,
    pub author: String,
    pub author_name: String,
    pub text: String,
    pub likes: i64,
    pub retweets: i64,
    pub replies: i64,
    pub created_utc: f64,
    pub url: String,
}

/// Bir X bağlantısından tweet kimliğini çıkarır.
pub fn extract_tweet_id(input: &str) -> Option<String> {
    let cleaned = input.trim().trim_end_matches('/');

    // .../status/<id> veya .../statuses/<id>
    for marker in ["/status/", "/statuses/"] {
        if let Some(rest) = cleaned.split(marker).nth(1) {
            let id: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !id.is_empty() {
                return Some(id);
            }
        }
    }

    // Çıplak kimlik
    if !cleaned.is_empty() && cleaned.len() <= 25 && cleaned.chars().all(|c| c.is_ascii_digit()) {
        return Some(cleaned.to_string());
    }

    None
}

/// Tweet içeriğini getirir. Kimlik gerektirmez — herkese açık ayna kullanılıyor.
pub async fn fetch_tweet(url_or_id: &str) -> Result<Tweet, String> {
    let id = extract_tweet_id(url_or_id)
        .ok_or_else(|| "Bağlantıdan tweet kimliği çıkarılamadı.".to_string())?;

    let response = client()?
        .get(format!("https://api.fxtwitter.com/status/{id}"))
        .send()
        .await
        .map_err(|e| format!("X aynasına ulaşılamadı: {e}"))?;

    if !response.status().is_success() {
        return Err(match response.status().as_u16() {
            404 => "Tweet bulunamadı. Silinmiş ya da gizli bir hesaba ait olabilir.".to_string(),
            code => format!("X aynası {code} yanıtı verdi."),
        });
    }

    let payload: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("X yanıtı okunamadı: {e}"))?;

    let tweet = payload
        .get("tweet")
        .ok_or_else(|| "Yanıtta tweet verisi yok.".to_string())?;

    let author = tweet.get("author").cloned().unwrap_or_default();
    let text = crate::reddit::clean_comment_body(
        tweet.get("text").and_then(|v| v.as_str()).unwrap_or_default(),
    );

    if text.trim().is_empty() {
        return Err("Tweet metni boş.".to_string());
    }

    Ok(Tweet {
        id: tweet
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string(),
        author: format!(
            "@{}",
            author.get("screen_name").and_then(|v| v.as_str()).unwrap_or("")
        ),
        author_name: author
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        text,
        likes: tweet.get("likes").and_then(|v| v.as_i64()).unwrap_or(0),
        retweets: tweet.get("retweets").and_then(|v| v.as_i64()).unwrap_or(0),
        replies: tweet.get("replies").and_then(|v| v.as_i64()).unwrap_or(0),
        created_utc: tweet
            .get("created_timestamp")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        url: tweet
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("https://x.com/i/status/{id}"))
            .to_string(),
    })
}

// ------------------------------------------------------- bird JSON yardımcıları

/// bird'ün sayaç alanları sürümden sürüme ad değiştirebiliyor; ikisini de dene.
fn bird_count(item: &serde_json::Value, primary: &str, fallback: &str) -> i64 {
    item.get(primary)
        .or_else(|| item.get(fallback))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

/// Yanıt metinleri "@hesap @hesap2 asıl söz" diye başlıyor. Kartta bu mention
/// zinciri yer kaplamaktan başka bir işe yaramıyor, baştakileri atıyoruz.
fn strip_leading_mentions(text: &str) -> &str {
    let mut rest = text.trim_start();
    loop {
        if !rest.starts_with('@') {
            return rest;
        }
        let Some(space) = rest.find(char::is_whitespace) else {
            // Yalnızca mention'dan ibaret — atacak bir şey kalmıyor.
            return rest;
        };
        let candidate = rest[space..].trim_start();
        if candidate.is_empty() {
            return rest;
        }
        rest = candidate;
    }
}

/// Takvim gününü unix epoch'a olan gün farkına çevirir (Howard Hinnant algoritması).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Twitter'ın `"Thu Sep 03 21:08:33 +0000 2026"` biçimini unix saniyeye çevirir.
/// Twitter bu alanı her zaman UTC veriyor, o yüzden offset'i okumuyoruz.
pub fn parse_twitter_date(raw: &str) -> f64 {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    let parts: Vec<&str> = raw.split_whitespace().collect();
    if parts.len() < 6 {
        return 0.0;
    }
    let Some(month) = MONTHS.iter().position(|m| *m == parts[1]) else {
        return 0.0;
    };
    let (Ok(day), Ok(year)) = (parts[2].parse::<i64>(), parts[5].parse::<i64>()) else {
        return 0.0;
    };

    let clock: Vec<i64> = parts[3]
        .split(':')
        .filter_map(|v| v.parse::<i64>().ok())
        .collect();
    if clock.len() != 3 {
        return 0.0;
    }

    let days = days_from_civil(year, month as i64 + 1, day);
    (days * 86_400 + clock[0] * 3_600 + clock[1] * 60 + clock[2]) as f64
}

/// bird'ün JSON tweet nesnesini ortak `Tweet` biçimine çevirir.
fn tweet_from_bird(item: &serde_json::Value) -> Option<Tweet> {
    // t.co bağlantıları neredeyse her tweet'te var; kartta yer kaplıyor ve
    // seslendirmede okunuyor. Yanıtlarla aynı temizlikten geçiriyoruz.
    let text = crate::reddit::clean_comment_body(
        item.get("text")
            .or_else(|| item.get("full_text"))
            .and_then(|v| v.as_str())?,
    );
    if text.trim().is_empty() {
        return None;
    }

    let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let author = item.get("author").cloned().unwrap_or_default();
    let username = author
        .get("username")
        .or_else(|| author.get("screen_name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Some(Tweet {
        url: format!("https://x.com/{username}/status/{id}"),
        id,
        author: format!("@{username}"),
        author_name: author
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        text,
        likes: bird_count(item, "likeCount", "likes"),
        retweets: bird_count(item, "retweetCount", "retweets"),
        replies: bird_count(item, "replyCount", "replies"),
        created_utc: item
            .get("createdAt")
            .and_then(|v| v.as_str())
            .map(parse_twitter_date)
            .unwrap_or(0.0),
    })
}

/// bird çıktısındaki ilk JSON gövdesini ayrıştırır — aracın uyarı satırları
/// stdout'a karışabiliyor.
fn bird_json(raw: &str) -> Result<serde_json::Value, String> {
    let start = raw
        .find(['[', '{'])
        .ok_or_else(|| "bird beklenen JSON çıktısını vermedi.".to_string())?;
    serde_json::from_str(raw[start..].trim())
        .map_err(|e| format!("bird çıktısı okunamadı: {e}"))
}

// -------------------------------------------------------------- viral arama

/// "Viral" ekranının arama ölçütleri.
#[derive(Deserialize, Debug, Clone)]
pub struct ViralQuery {
    /// Boşsa yalnızca eşik ölçütleriyle aranır.
    pub keyword: String,
    /// `tr`, `en`… Boşsa dil süzgeci uygulanmaz.
    pub lang: String,
    pub min_faves: i64,
    pub min_replies: i64,
    /// Son kaç saat içinde atılmış olsun; 0 = sınırsız.
    pub hours: i64,
    /// Bağlantı ve medya içeren tweet'leri ele — kart olarak metin okunabilsin.
    pub only_text: bool,
    pub limit: usize,
}

/// Ölçütleri X'in arama sözdizimine çevirir.
pub fn build_search_query(q: &ViralQuery, now: i64) -> String {
    let mut parts: Vec<String> = Vec::new();

    let keyword = q.keyword.trim();
    if !keyword.is_empty() {
        parts.push(keyword.to_string());
    }
    if q.min_faves > 0 {
        parts.push(format!("min_faves:{}", q.min_faves));
    }
    if q.min_replies > 0 {
        parts.push(format!("min_replies:{}", q.min_replies));
    }
    let lang = q.lang.trim();
    if !lang.is_empty() {
        parts.push(format!("lang:{lang}"));
    }
    if q.hours > 0 {
        parts.push(format!("since_time:{}", now - q.hours * 3_600));
    }
    if q.only_text {
        parts.push("-filter:links".to_string());
    }

    // Yanıtları ve retweet'leri eliyoruz: ikisi de kendi başına gönderi değil.
    parts.push("-filter:replies".to_string());
    parts.push("-filter:retweets".to_string());
    parts.join(" ")
}

/// Ölçütlere uyan viral tweet'leri getirir.
pub async fn search_viral(query: ViralQuery) -> Result<Vec<Tweet>, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let search = build_search_query(&query, now);
    let count = query.limit.clamp(1, 100).to_string();

    let raw = tokio::task::spawn_blocking(move || {
        run_bird(&["search", "--json", "-n", &count, &search])
    })
    .await
    .map_err(|e| format!("Arama görevi başlatılamadı: {e}"))??;

    let payload = bird_json(&raw)?;
    let items: Vec<&serde_json::Value> = match &payload {
        serde_json::Value::Array(list) => list.iter().collect(),
        serde_json::Value::Object(map) => map
            .get("tweets")
            .and_then(|v| v.as_array())
            .map(|l| l.iter().collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    };

    let mut tweets: Vec<Tweet> = items.iter().filter_map(|i| tweet_from_bird(i)).collect();
    tweets.sort_by(|a, b| b.likes.cmp(&a.likes));
    Ok(tweets)
}

// ------------------------------------------------------------ yazar zinciri

/// `bird thread` çıktısındaki tek bir düğüm.
struct ChainNode {
    tweet: Tweet,
    in_reply_to: Option<String>,
    conversation_id: Option<String>,
}

/// Zincirin makul üst sınırı — bozuk veride sonsuz döngüyü engelliyor.
const MAX_CHAIN: usize = 200;

/// Kök yazarın kendi zincirini kurar: kökten başlayıp her adımda bir öncekine
/// yanıt veren **aynı yazara ait** tweet'i takip ediyor.
///
/// `bird thread` düz bir dizi döndürüyor ve içinde başka yazarların yanıtları
/// da var (ölçüldü: kök `@lemarcaspors_`, hemen ardından `@torresgala9`).
/// Sırayı ancak `inReplyToStatusId` zincirini izleyerek kurabiliyoruz —
/// yanıtlarda yaptığımız gibi beğeniye göre sıralamak zinciri anlamsızlaştırır.
fn build_chain(nodes: &[ChainNode], root_id: &str) -> Vec<Tweet> {
    let Some(root) = nodes.iter().find(|n| n.tweet.id == root_id) else {
        return Vec::new();
    };

    let author = root.tweet.author.clone();
    let mut chain = vec![root.tweet.clone()];
    let mut seen: Vec<String> = vec![root.tweet.id.clone()];

    while chain.len() < MAX_CHAIN {
        let current = chain.last().expect("zincir hiç boş olmuyor").id.clone();

        // Aynı tweet'e yazar birden çok kez yanıt vermişse en eskisi zincirin
        // devamıdır; sonrakiler ayrı bir dal oluyor.
        let next = nodes
            .iter()
            .filter(|n| {
                n.tweet.author == author
                    && n.in_reply_to.as_deref() == Some(current.as_str())
                    && !seen.contains(&n.tweet.id)
            })
            .min_by(|a, b| {
                a.tweet
                    .created_utc
                    .partial_cmp(&b.tweet.created_utc)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        let Some(next) = next else { break };
        seen.push(next.tweet.id.clone());
        chain.push(next.tweet.clone());
    }

    chain
}

/// Bir kullanıcının kendi thread'ini getirir: kök tweet başlık kartı, yazarın
/// kendi devam tweet'leri de sıradaki kartlar oluyor.
///
/// Zincirin ortasındaki bir bağlantı verilse bile baştan başlıyoruz —
/// `conversationId` kökü gösteriyor.
pub async fn fetch_author_thread(url_or_id: &str, limit: usize) -> Result<TweetThread, String> {
    let id = extract_tweet_id(url_or_id)
        .ok_or_else(|| "Bağlantıdan tweet kimliği çıkarılamadı.".to_string())?;

    let arg = id.clone();
    let raw = tokio::task::spawn_blocking(move || run_bird(&["thread", "--json", &arg]))
        .await
        .map_err(|e| format!("Zincir görevi başlatılamadı: {e}"))??;

    let payload = bird_json(&raw)?;
    let items: Vec<&serde_json::Value> = match &payload {
        serde_json::Value::Array(list) => list.iter().collect(),
        serde_json::Value::Object(map) => map
            .get("tweets")
            .and_then(|v| v.as_array())
            .map(|l| l.iter().collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    };

    let nodes: Vec<ChainNode> = items
        .iter()
        .filter_map(|item| {
            Some(ChainNode {
                tweet: tweet_from_bird(item)?,
                in_reply_to: item
                    .get("inReplyToStatusId")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                conversation_id: item
                    .get("conversationId")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
            })
        })
        .collect();

    // Ortadaki bir tweet verilmişse zinciri kökünden kuruyoruz.
    let root_id = nodes
        .iter()
        .find(|n| n.tweet.id == id)
        .and_then(|n| n.conversation_id.clone())
        .filter(|c| nodes.iter().any(|n| n.tweet.id == *c))
        .unwrap_or(id.clone());

    let chain = build_chain(&nodes, &root_id);
    let mut chain = chain.into_iter();

    let root = chain
        .next()
        .ok_or_else(|| "Zincirin kök tweet'i okunamadı.".to_string())?;

    let replies: Vec<Reply> = chain
        .take(limit.clamp(1, 50))
        .map(|t| Reply {
            id: t.id,
            author: t.author,
            text: t.text,
            likes: t.likes,
        })
        .filter(|r| r.text.chars().count() >= 3)
        .collect();

    Ok(TweetThread {
        tweet: root,
        replies,
    })
}

// ---------------------------------------------------------------- zincir

/// Bir tweet ve yanıtları — Reddit'teki "gönderi + yorumlar" karşılığı.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TweetThread {
    pub tweet: Tweet,
    pub replies: Vec<Reply>,
}

/// Video için gereken her şeyi tek seferde toplar.
///
/// Tweet'in kendisini bird üzerinden okuyoruz; bird ulaşılamazsa kimlik
/// istemeyen aynaya düşüyoruz, böylece en azından başlık kartı çıkıyor.
pub async fn fetch_thread(url_or_id: &str, limit: usize) -> Result<TweetThread, String> {
    let id = extract_tweet_id(url_or_id)
        .ok_or_else(|| "Bağlantıdan tweet kimliği çıkarılamadı.".to_string())?;

    let read_id = id.clone();
    let tweet = match tokio::task::spawn_blocking(move || run_bird(&["read", "--json", &read_id]))
        .await
        .map_err(|e| format!("Tweet görevi başlatılamadı: {e}"))?
    {
        Ok(raw) => bird_json(&raw)
            .ok()
            .as_ref()
            .and_then(tweet_from_bird)
            .ok_or_else(|| "Tweet içeriği okunamadı.".to_string())?,
        // bird yoksa ya da çerez süresi dolduysa içerik yine de okunabiliyor.
        Err(_) => fetch_tweet(&id).await?,
    };

    let replies = fetch_replies(&id, limit).await.unwrap_or_default();
    Ok(TweetThread { tweet, replies })
}

#[cfg(test)]
mod bird_komut_testleri {
    use super::{bin_entry, node_command_for, BirdCommand};

    /// bird'ü çözüp kimlik gerektirmeyen `--version` ile çalıştırır. Windows
    /// CI'da kullanıcının yaşadığı os error 193'ün gerçekten kapandığını sınıyor.
    /// `cargo test canli_bird_cozumleme -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn canli_bird_cozumleme() {
        let komut = super::bird_command().expect("bird bulunamadı");
        println!("program: {}", komut.program);
        println!("önek   : {:?}", komut.prefix);
        if cfg!(target_os = "windows") {
            let p = komut.program.to_ascii_lowercase();
            assert!(
                p.ends_with(".exe") || p.ends_with(".cmd") || p.ends_with(".bat"),
                "Windows'ta uzantısız betik seçildi: {}",
                komut.program
            );
        }
        let cikti = crate::toolpath::command(&komut.program)
            .args(&komut.prefix)
            .arg("--version")
            .output()
            .expect("bird çalıştırılamadı");
        let surum = String::from_utf8_lossy(&cikti.stdout);
        println!("bird --version: {}", surum.trim());
        assert!(cikti.status.success(), "bird --version düştü: {}", String::from_utf8_lossy(&cikti.stderr));
    }

    #[test]
    fn bin_alani_eslemeden_okunur() {
        // @steipete/bird 0.4.0'ın gerçek package.json'u bu biçimde.
        let json = r#"{"name":"@steipete/bird","bin":{"bird":"dist/index.js"}}"#;
        assert_eq!(bin_entry(json, "bird").as_deref(), Some("dist/index.js"));
    }

    #[test]
    fn bin_alani_duz_dize_de_olabilir() {
        assert_eq!(bin_entry(r#"{"bin":"cli.js"}"#, "bird").as_deref(), Some("cli.js"));
        assert_eq!(bin_entry(r#"{"name":"x"}"#, "bird"), None);
        assert_eq!(bin_entry("bozuk json", "bird"), None);
    }

    /// Windows'taki npm global düzenini geçici klasörde kurup çözümlemeyi
    /// sınar — klasör yapısı platformdan bağımsız olduğu için her yerde koşuyor.
    #[test]
    fn npm_kabugunun_yanindaki_paketi_bulur() {
        let kok = std::env::temp_dir().join(format!("rvmaker-bird-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&kok);
        let paket = kok.join("node_modules").join("@steipete").join("bird");
        std::fs::create_dir_all(paket.join("dist")).unwrap();
        std::fs::write(paket.join("package.json"), r#"{"bin":{"bird":"dist/index.js"}}"#).unwrap();
        std::fs::write(paket.join("dist").join("index.js"), "// bird").unwrap();
        std::fs::write(kok.join("bird.cmd"), "@echo off").unwrap();

        let komut = node_command_for(&kok, "node.exe").expect("komut kurulmalı");
        assert_eq!(komut.program, "node.exe");
        assert_eq!(komut.prefix.len(), 1);
        assert!(komut.prefix[0].ends_with("index.js"), "giriş: {:?}", komut.prefix);

        let _ = std::fs::remove_dir_all(&kok);
    }

    #[test]
    fn paket_yoksa_kabuga_duser() {
        let bos = std::env::temp_dir().join(format!("rvmaker-bird-bos-{}", std::process::id()));
        std::fs::create_dir_all(&bos).unwrap();
        assert_eq!(node_command_for(&bos, "node.exe"), None::<BirdCommand>);
        let _ = std::fs::remove_dir_all(&bos);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_search_query, extract_credentials, extract_tweet_id, parse_twitter_date,
        strip_leading_mentions, ViralQuery,
    };

    fn olcut() -> ViralQuery {
        ViralQuery {
            keyword: String::new(),
            lang: "tr".into(),
            min_faves: 2000,
            min_replies: 50,
            hours: 24,
            only_text: false,
            limit: 25,
        }
    }

    /// Gerçek X hesabına bağlanır; elle çalıştırmak için:
    /// `cargo test canli_viral -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn canli_viral() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tweets = rt.block_on(super::search_viral(olcut())).unwrap();
        println!("viral tweet: {}", tweets.len());
        let first = tweets.first().expect("en az bir tweet beklenir");
        println!("  {} · {} beğeni · {} yanıt", first.author, first.likes, first.replies);
        println!("  {}", first.url);

        let thread = rt.block_on(super::fetch_thread(&first.url, 8)).unwrap();
        println!("zincir: {} yanıt", thread.replies.len());
        for r in thread.replies.iter().take(3) {
            println!("  {} ({}): {}", r.author, r.likes, r.text.chars().take(60).collect::<String>());
        }
    }

    fn dugum(id: &str, yazar: &str, yanit: Option<&str>, zaman: f64) -> super::ChainNode {
        super::ChainNode {
            tweet: super::Tweet {
                id: id.into(),
                author: yazar.into(),
                author_name: String::new(),
                text: format!("{id} metni"),
                likes: 0,
                retweets: 0,
                replies: 0,
                created_utc: zaman,
                url: String::new(),
            },
            in_reply_to: yanit.map(str::to_string),
            conversation_id: Some("1".into()),
        }
    }

    /// Gerçek bir thread'le sınar:
    /// `cargo test canli_zincir -- --ignored --nocapture <tweet-url>`
    #[test]
    #[ignore]
    fn canli_zincir() {
        let hedef = std::env::var("RV_THREAD_URL")
            .unwrap_or_else(|_| "1990840742800744470".to_string());
        let rt = tokio::runtime::Runtime::new().unwrap();
        match rt.block_on(super::fetch_author_thread(&hedef, 20)) {
            Ok(z) => {
                println!("kök: {} · {} beğeni", z.tweet.author, z.tweet.likes);
                println!("  {}", z.tweet.text.chars().take(80).collect::<String>());
                println!("devam tweet sayısı: {}", z.replies.len());
                for r in z.replies.iter().take(5) {
                    println!("  {} ({}): {}", r.author, r.likes,
                             r.text.chars().take(70).collect::<String>());
                }
            }
            Err(e) => println!("HATA: {e}"),
        }
    }

    #[test]
    fn zinciri_yazari_takip_ederek_kurar() {
        // Kök @ben; araya @baskasi'nın yanıtı giriyor, zincire girmemeli.
        let nodes = vec![
            dugum("1", "@ben", None, 100.0),
            dugum("9", "@baskasi", Some("1"), 110.0),
            dugum("2", "@ben", Some("1"), 120.0),
            dugum("3", "@ben", Some("2"), 130.0),
        ];
        let zincir = super::build_chain(&nodes, "1");
        let idler: Vec<&str> = zincir.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(idler, vec!["1", "2", "3"]);
    }

    #[test]
    fn siralama_zamana_gore_dalda_en_eskiyi_secer() {
        // Yazar aynı tweet'e iki kez yanıt vermiş; zincir en eskisini izler.
        let nodes = vec![
            dugum("1", "@ben", None, 100.0),
            dugum("3", "@ben", Some("1"), 300.0),
            dugum("2", "@ben", Some("1"), 200.0),
        ];
        let zincir = super::build_chain(&nodes, "1");
        let idler: Vec<&str> = zincir.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(idler, vec!["1", "2"]);
    }

    #[test]
    fn devami_olmayan_tweette_yalniz_kok_doner() {
        let nodes = vec![
            dugum("1", "@ben", None, 100.0),
            dugum("5", "@baskasi", Some("1"), 110.0),
        ];
        assert_eq!(super::build_chain(&nodes, "1").len(), 1);
    }

    #[test]
    fn kok_bulunamazsa_bos_doner() {
        let nodes = vec![dugum("2", "@ben", Some("1"), 100.0)];
        assert!(super::build_chain(&nodes, "1").is_empty());
    }

    #[test]
    fn dongude_takilmaz() {
        // Bozuk veri: iki tweet birbirini yanıtlıyor gösteriyor.
        let nodes = vec![
            dugum("1", "@ben", Some("2"), 100.0),
            dugum("2", "@ben", Some("1"), 200.0),
        ];
        let zincir = super::build_chain(&nodes, "1");
        assert_eq!(zincir.len(), 2, "her tweet zincire en fazla bir kez girmeli");
    }

    #[test]
    fn twitter_tarihini_unix_saniyeye_cevirir() {
        // 2026-09-03 21:08:33 UTC
        assert_eq!(parse_twitter_date("Thu Sep 03 21:08:33 +0000 2026"), 1_788_469_713.0);
        // 1970-01-01 00:00:00 UTC — epoch'un kendisi
        assert_eq!(parse_twitter_date("Thu Jan 01 00:00:00 +0000 1970"), 0.0);
        // Bozuk girdi sıfır döner, çökmez.
        assert_eq!(parse_twitter_date("bilinmeyen"), 0.0);
    }

    #[test]
    fn bastaki_mentionlari_atar() {
        assert_eq!(strip_leading_mentions("@a @b asıl söz"), "asıl söz");
        assert_eq!(strip_leading_mentions("mention yok"), "mention yok");
        // Yalnızca mention'dan ibaretse metni boşaltmıyoruz.
        assert_eq!(strip_leading_mentions("@yalnizca"), "@yalnizca");
    }

    #[test]
    fn arama_sorgusunu_kurar() {
        let q = build_search_query(&olcut(), 1_000_000);
        assert!(q.contains("min_faves:2000"));
        assert!(q.contains("min_replies:50"));
        assert!(q.contains("lang:tr"));
        assert!(q.contains("since_time:913600"));
        assert!(q.contains("-filter:replies"));
        assert!(q.contains("-filter:retweets"));
        // Medya süzgeci kapalıyken eklenmemeli.
        assert!(!q.contains("-filter:links"));
    }

    #[test]
    fn bos_olcutler_sorguyu_kirletmez() {
        let mut q = olcut();
        q.lang = String::new();
        q.hours = 0;
        q.min_replies = 0;
        q.only_text = true;
        q.keyword = "  deprem  ".into();
        let built = build_search_query(&q, 1_000_000);
        assert!(built.starts_with("deprem "));
        assert!(!built.contains("lang:"));
        assert!(!built.contains("since_time:"));
        assert!(!built.contains("min_replies:"));
        assert!(built.contains("-filter:links"));
    }

    #[test]
    fn baglantidan_tweet_kimligi_cikarir() {
        assert_eq!(
            extract_tweet_id("https://x.com/jack/status/20"),
            Some("20".to_string())
        );
        assert_eq!(
            extract_tweet_id("https://twitter.com/user/status/1878123456789?s=20"),
            Some("1878123456789".to_string())
        );
    }

    #[test]
    fn ciplak_kimligi_kabul_eder() {
        assert_eq!(extract_tweet_id("20"), Some("20".to_string()));
    }

    #[test]
    fn gecersiz_baglantiyi_reddeder() {
        assert_eq!(extract_tweet_id("https://example.com/bir-yazi"), None);
    }

    #[test]
    fn json_cerezinden_kimlik_cikarir() {
        let json = r#"[{"name":"auth_token","value":"abc123"},
                       {"name":"ct0","value":"def456"},
                       {"name":"guest_id","value":"yok"}]"#;
        assert_eq!(
            extract_credentials(json),
            Some(("abc123".to_string(), "def456".to_string()))
        );
    }

    #[test]
    fn baslik_dizesinden_kimlik_cikarir() {
        assert_eq!(
            extract_credentials("guest_id=x; auth_token=abc123; ct0=def456"),
            Some(("abc123".to_string(), "def456".to_string()))
        );
    }

    /// Ağ + anahtar zincirindeki X çerezleri gerektirir:
    /// `cargo test --lib -- --ignored canli_x --nocapture`
    #[tokio::test]
    #[ignore]
    async fn canli_x() {
        println!("bird: {:?}", super::bird_path());

        match super::verify_credentials().await {
            Ok(v) => println!("doğrulama: {v}"),
            Err(e) => println!("doğrulama HATA: {e}"),
        }

        match super::fetch_replies("https://x.com/jack/status/20", 5).await {
            Ok(list) => {
                println!("yanıt sayısı: {}", list.len());
                for r in list.iter().take(3) {
                    println!("  {} ({} beğeni): {}", r.author, r.likes,
                             r.text.chars().take(70).collect::<String>());
                }
            }
            Err(e) => println!("yanıtlar HATA: {e}"),
        }
    }

    #[test]
    fn eksik_deger_reddedilir() {
        // ct0 olmadan oturum doğrulanamaz
        assert_eq!(extract_credentials("auth_token=abc123"), None);
        assert_eq!(extract_credentials(""), None);
    }
}
