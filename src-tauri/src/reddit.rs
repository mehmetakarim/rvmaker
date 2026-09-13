//! Reddit gönderisi ve yorumlarını API kimlik bilgisi olmadan çeker.
//!
//! Reddit'in herkese açık `.json` uç noktasını kullanır; HTML ayrıştırma yok,
//! bu yüzden sayfa düzeni değişikliklerinden etkilenmez.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Reddit'in kurallarına uygun istemci tanımı.
const USER_AGENT: &str = "macos:com.rvmaker.desktop:v0.1.0 (by /u/rvmaker)";

/// Yüklü uygulama (installed app) yetkilendirmesi — istemci gizli anahtarı gerektirmez.
const INSTALLED_CLIENT_GRANT: &str = "https://oauth.reddit.com/grants/installed_client";

struct CachedToken {
    value: String,
    fetched_at: Instant,
    lifetime: Duration,
}

fn token_cache() -> &'static Mutex<Option<CachedToken>> {
    static CACHE: OnceLock<Mutex<Option<CachedToken>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

/// Önbellekteki jeton hâlâ geçerliyse döner. Süresine 60 sn pay bırakılır.
fn cached_token() -> Option<String> {
    let guard = token_cache().lock().ok()?;
    let token = guard.as_ref()?;
    if token.fetched_at.elapsed() + Duration::from_secs(60) < token.lifetime {
        Some(token.value.clone())
    } else {
        None
    }
}

fn store_token(value: &str, lifetime_secs: u64) {
    if let Ok(mut guard) = token_cache().lock() {
        *guard = Some(CachedToken {
            value: value.to_string(),
            fetched_at: Instant::now(),
            lifetime: Duration::from_secs(lifetime_secs),
        });
    }
}

const KEYCHAIN_SERVICE: &str = "com.rvmaker.desktop";
const KEYCHAIN_COOKIE_ENTRY: &str = "reddit-session-cookie";

fn cookie_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_COOKIE_ENTRY)
        .map_err(|e| format!("Anahtar zincirine erişilemedi: {e}"))
}

/// Oturum çerezini sistem anahtar zincirine yazar. Değer hiçbir zaman
/// arayüze geri dönmez; yalnızca varlığı sorgulanabilir.
pub fn store_cookie(value: &str) -> Result<(), String> {
    let normalized = normalize_cookie(value);
    if normalized.is_empty() {
        return Err("Çerez okunamadı. Başlık dizesi ya da Cookie-Editor JSON çıktısı yapıştır.".to_string());
    }

    crate::keychain::write(KEYCHAIN_SERVICE, KEYCHAIN_COOKIE_ENTRY, &normalized)
}

pub fn clear_cookie() -> Result<(), String> {
    let entry = cookie_entry()?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Çerez silinemedi: {e}")),
    }
}

fn read_cookie() -> Option<String> {
    let entry = cookie_entry().ok()?;
    let value = entry.get_password().ok()?;
    let normalized = normalize_cookie(&value);
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

/// Çerezi `ad=değer; ad2=değer2` biçimine getirir.
///
/// Cookie-Editor gibi eklentiler JSON olarak dışa aktarabiliyor; kullanıcıyı
/// doğru biçimi seçmeye zorlamak yerine ikisini de kabul ediyoruz.
pub fn normalize_cookie(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if trimmed.starts_with('[') || trimmed.starts_with('{') {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
            let items: Vec<&serde_json::Value> = match &parsed {
                serde_json::Value::Array(list) => list.iter().collect(),
                other => vec![other],
            };

            let pairs: Vec<String> = items
                .iter()
                .filter_map(|item| {
                    let name = item.get("name")?.as_str()?.trim();
                    let value = item.get("value")?.as_str()?.trim();
                    if name.is_empty() {
                        None
                    } else {
                        Some(format!("{name}={value}"))
                    }
                })
                .collect();

            if !pairs.is_empty() {
                return pairs.join("; ");
            }
        }
    }

    // Tarayıcıdan kopyalanan başlık bazen "Cookie:" önekiyle geliyor.
    trimmed
        .strip_prefix("Cookie:")
        .or_else(|| trimmed.strip_prefix("cookie:"))
        .unwrap_or(trimmed)
        .trim()
        .to_string()
}

pub fn cookie_present() -> bool {
    read_cookie().is_some()
}

/// Uygulama düzeyinde (kullanıcı oturumu olmayan) bir erişim jetonu alır.
async fn access_token(client: &reqwest::Client, client_id: &str) -> Result<String, String> {
    if let Some(token) = cached_token() {
        return Ok(token);
    }

    let response = client
        .post("https://www.reddit.com/api/v1/access_token")
        .basic_auth(client_id, Some(""))
        .form(&[
            ("grant_type", INSTALLED_CLIENT_GRANT),
            ("device_id", "DO_NOT_TRACK_THIS_DEVICE"),
        ])
        .send()
        .await
        .map_err(|e| format!("Reddit yetkilendirmesine ulaşılamadı: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 => "Reddit istemci kimliği reddedildi. Ayarlar → Genel → Reddit erişimi altındaki kimliği kontrol et.".to_string(),
            429 => "Reddit hız sınırına takıldı. Birkaç dakika sonra tekrar dene.".to_string(),
            code => format!("Reddit yetkilendirmesi {code} yanıtı verdi."),
        });
    }

    let payload: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Yetkilendirme yanıtı okunamadı: {e}"))?;

    let token = payload
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Yetkilendirme yanıtında erişim jetonu yok.".to_string())?;

    let lifetime = payload
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .unwrap_or(3600);

    store_token(token, lifetime);
    Ok(token.to_string())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Comment {
    pub id: String,
    pub author: String,
    pub upvotes: i64,
    pub body: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Post {
    pub id: String,
    pub subreddit: String,
    pub title: String,
    pub selftext: String,
    pub url: String,
    pub upvotes: i64,
    pub comment_count: i64,
    pub is_nsfw: bool,
    pub created_utc: f64,
    /// Gönderi kaldırılmış ya da silinmişse kullanıcıya gösterilecek not.
    pub removal_note: Option<String>,
    pub comments: Vec<Comment>,
}

/// Bir Reddit bağlantısından gönderi kimliğini çıkarır.
/// `/comments/<id>/...`, `redd.it/<id>` ve çıplak kimlik biçimlerini kabul eder.
pub fn extract_post_id(input: &str) -> Option<String> {
    let cleaned = input.trim().trim_end_matches('/');

    if let Some(rest) = cleaned.split("/comments/").nth(1) {
        let id = rest.split('/').next().unwrap_or("");
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }

    if let Some(rest) = cleaned.split("redd.it/").nth(1) {
        let id = rest.split(['/', '?']).next().unwrap_or("");
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }

    // Çıplak kimlik: yalnızca harf ve rakam, makul uzunlukta
    if !cleaned.is_empty()
        && cleaned.len() <= 12
        && cleaned.chars().all(|c| c.is_ascii_alphanumeric())
    {
        return Some(cleaned.to_string());
    }

    None
}

fn string_field(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Markdown bağlantı ve görsel gömülerini ayıklar.
/// `![gif](giphy|xyz)` tamamen atılır, `[metin](url)` yalnızca metne indirgenir.
fn strip_markdown_links(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;

    let find_from = |from: usize, target: char| -> Option<usize> {
        (from..chars.len()).find(|&j| chars[j] == target)
    };

    while i < chars.len() {
        let is_image = chars[i] == '!' && chars.get(i + 1) == Some(&'[');
        let is_link = chars[i] == '[';

        if is_image || is_link {
            let text_start = if is_image { i + 2 } else { i + 1 };
            if let Some(close) = find_from(text_start, ']') {
                if chars.get(close + 1) == Some(&'(') {
                    if let Some(paren) = find_from(close + 2, ')') {
                        if !is_image {
                            out.extend(chars[text_start..close].iter());
                        }
                        i = paren + 1;
                        continue;
                    }
                }
            }
        }

        out.push(chars[i]);
        i += 1;
    }

    out
}

/// Yorum gövdesini seslendirilebilir düz metne çevirir.
///
/// Reddit yorumları markdown içeriyor: gif gömüleri, bağlantılar, alıntı
/// işaretleri, kalın/italik yıldızları. Bunlar hem kartta çirkin duruyor hem de
/// TTS tarafından okunuyor.
pub fn clean_comment_body(raw: &str) -> String {
    let without_links = strip_markdown_links(raw);

    let mut cleaned = String::with_capacity(without_links.len());
    for line in without_links.lines() {
        // Alıntı işaretlerini ve madde imlerini at
        let trimmed = line.trim_start_matches(['>', '*', '-', '#', ' ']).trim();
        if trimmed.is_empty() {
            continue;
        }
        if !cleaned.is_empty() {
            cleaned.push(' ');
        }
        cleaned.push_str(trimmed);
    }

    // Çıplak bağlantıları at
    let no_urls: Vec<&str> = cleaned
        .split_whitespace()
        .filter(|token| {
            let t = token.trim_start_matches(['(', '[']);
            !(t.starts_with("http://") || t.starts_with("https://") || t.starts_with("www."))
        })
        .collect();

    let joined = no_urls.join(" ");

    // Vurgu işaretlerini ve kalan kod tırnaklarını at
    let stripped: String = joined
        .chars()
        .filter(|c| !matches!(c, '*' | '_' | '`' | '~' | '^'))
        .collect();

    stripped.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Bir yorum düğümünü ayrıştırır; kullanılamaz olanlar için `None` döner.
fn parse_comment(node: &serde_json::Value) -> Option<Comment> {
    if node.get("kind").and_then(|k| k.as_str()) != Some("t1") {
        return None;
    }

    let data = node.get("data")?;

    // Sabitlenmiş ve moderatör yorumları anlatıya girmemeli.
    if data.get("stickied").and_then(|v| v.as_bool()).unwrap_or(false) {
        return None;
    }
    if data.get("distinguished").and_then(|v| v.as_str()).is_some() {
        return None;
    }

    let body = string_field(data, "body");
    let trimmed = body.trim();
    if trimmed.is_empty() || trimmed == "[removed]" || trimmed == "[deleted]" {
        return None;
    }

    // Yalnızca gif/bağlantıdan ibaret yorumlar temizlik sonrası boş kalır;
    // seslendirilecek bir şey olmadığı için listeye alınmazlar.
    let cleaned = clean_comment_body(trimmed);
    if cleaned.chars().count() < 3 {
        return None;
    }

    Some(Comment {
        id: string_field(data, "id"),
        author: format!("u/{}", string_field(data, "author")),
        upvotes: data.get("score").and_then(|v| v.as_i64()).unwrap_or(0),
        body: cleaned,
    })
}

/// Tarayıcı kimliği — çerez yolunda Reddit bunu bekliyor.
const BROWSER_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36";

/// Yalnızca `application/json` istemek Reddit'in kenar sunucusunda bot işareti
/// sayılıyor; tarayıcının gönderdiği geniş listeyi taklit ediyoruz.
const BROWSER_ACCEPT: &str =
    "application/json,text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8";

const BROWSER_ACCEPT_LANGUAGE: &str = "tr-TR,tr;q=0.9,en-US;q=0.8,en;q=0.7";

/// Resmî OAuth uç noktasından ham listeyi çeker.
async fn fetch_via_oauth(
    client_id: &str,
    post_id: &str,
    limit: usize,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP istemcisi kurulamadı: {e}"))?;

    let token = access_token(&client, client_id).await?;

    let endpoint = format!(
        "https://oauth.reddit.com/comments/{post_id}?limit={limit}&sort=top&raw_json=1"
    );

    let response = client
        .get(&endpoint)
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| format!("Reddit'e ulaşılamadı: {e}"))?;

    if !response.status().is_success() {
        return Err(match response.status().as_u16() {
            401 | 403 => "Reddit erişimi reddedildi. İstemci kimliğini kontrol et.".to_string(),
            404 => "Gönderi bulunamadı. Silinmiş veya özel bir topluluğa ait olabilir.".to_string(),
            429 => "Reddit hız sınırına takıldı. Birkaç dakika sonra tekrar dene.".to_string(),
            code => format!("Reddit {code} yanıtı verdi."),
        });
    }

    response
        .json()
        .await
        .map_err(|e| format!("Reddit yanıtı okunamadı: {e}"))
}

/// Kullanıcının kendi oturum çereziyle old.reddit.com üzerinden çeker.
///
/// Yönlendirme izlenmez: giriş sayfasına yönlendirme, çerezin artık
/// geçerli olmadığının en net işaretidir ve bunu kullanıcıya öyle söyleriz.
async fn fetch_via_cookie(
    cookie: &str,
    post_id: &str,
    limit: usize,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .user_agent(BROWSER_USER_AGENT)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP istemcisi kurulamadı: {e}"))?;

    let endpoint = format!(
        "https://old.reddit.com/comments/{post_id}.json?limit={limit}&sort=top&raw_json=1"
    );

    // Reddit'in kenar sunucusu taze bağlantıdaki ilk isteği sabit bir engelleme
    // sayfasıyla (403) karşılıyor; aynı istemci üzerinden yapılan ikinci istek
    // geçiyor. Ölçtüğümüz davranış bu, o yüzden 403'te kısa bir bekleyip yeniden
    // deniyoruz. Kalıcı bir yetki sorunu ise üç denemenin sonunda yine 403 gelir.
    const MAX_ATTEMPTS: u32 = 3;
    let mut last_status = 0u16;

    for attempt in 0..MAX_ATTEMPTS {
        let response = client
            .get(&endpoint)
            .header("Cookie", cookie.trim())
            .header("Accept", BROWSER_ACCEPT)
            .header("Accept-Language", BROWSER_ACCEPT_LANGUAGE)
            .send()
            .await
            .map_err(|e| format!("Reddit'e ulaşılamadı: {e}"))?;

        let status = response.status();

        if status.is_success() {
            return response
                .json()
                .await
                .map_err(|e| format!("Reddit yanıtı okunamadı: {e}. Çerez geçerli olmayabilir."));
        }

        if status.is_redirection() {
            let target = response
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_string();
            if target.contains("/login") {
                return Err(
                    "Reddit oturum çerezi geçersiz veya süresi dolmuş. Tarayıcıdan yeni bir çerez kopyalayıp Ayarlar → Genel → Reddit erişimi altında güncelle."
                        .to_string(),
                );
            }
            return Err(format!("Reddit beklenmedik bir yönlendirme verdi: {target}"));
        }

        last_status = status.as_u16();

        let retriable = matches!(last_status, 403 | 429 | 500..=599);
        if retriable && attempt + 1 < MAX_ATTEMPTS {
            tokio::time::sleep(Duration::from_millis(400 * u64::from(attempt + 1))).await;
            continue;
        }

        break;
    }

    Err(match last_status {
        401 | 403 => "Reddit isteği engelledi. Çerezin süresi dolmuş olabilir; tarayıcıdan yenisini kopyalamayı dene.".to_string(),
        404 => "Gönderi bulunamadı. Silinmiş veya özel bir topluluğa ait olabilir.".to_string(),
        429 => "Reddit hız sınırına takıldı. Birkaç dakika sonra tekrar dene.".to_string(),
        code => format!("Reddit {code} yanıtı verdi."),
    })
}

/// Gönderi kaldırılmış ya da silinmişse okunur bir not döndürür.
///
/// Dikkat: `author == "[deleted]"` tek başına yeterli **değil**. Hesabı
/// silinmiş ama metni ve yorumları yerinde duran gönderiler var — ölçtüm:
/// r/AmItheAsshole/comments/1wexj1j yazarı `[deleted]`, buna karşın gövdesi
/// tam ve 71 yorumu var. Sadece yazara bakan bir kontrol bunu yanlışlıkla
/// kullanılamaz sayardı.
pub fn removal_note(post_data: &serde_json::Value) -> Option<String> {
    let selftext = string_field(post_data, "selftext");
    let selftext = selftext.trim();

    // Moderatör ya da Reddit kaldırdıysa gövde `[removed]` oluyor.
    if selftext == "[removed]" {
        let by = post_data
            .get("removed_by_category")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        return Some(match by {
            "moderator" => "Bu gönderiyi moderatör kaldırmış; metni artık okunamıyor.",
            "deleted" => "Bu gönderiyi yazarı silmiş; metni artık okunamıyor.",
            "automod_filtered" | "reddit" | "anti_evil_ops" | "community_ops" => {
                "Bu gönderiyi Reddit kaldırmış; metni artık okunamıyor."
            }
            _ => "Bu gönderi kaldırılmış; metni artık okunamıyor.",
        }
        .to_string());
    }

    if selftext == "[deleted]" {
        return Some("Bu gönderiyi yazarı silmiş; metni artık okunamıyor.".to_string());
    }

    // Gövdesi olmayan bağlantı gönderilerinde `removed_by_category` tek ipucu.
    if let Some(by) = post_data.get("removed_by_category").and_then(|v| v.as_str()) {
        if !by.is_empty() {
            return Some("Bu gönderi kaldırılmış; başlık dışında içerik yok.".to_string());
        }
    }

    None
}

/// Ham liste yanıtını `Post` yapısına dönüştürür.
/// Hem OAuth hem çerez yolu aynı biçimi döndürür.
fn parse_listing(payload: serde_json::Value, limit: usize) -> Result<Post, String> {
    let listings = payload
        .as_array()
        .ok_or_else(|| "Beklenmeyen Reddit yanıtı.".to_string())?;

    let post_data = listings
        .first()
        .and_then(|l| l.get("data"))
        .and_then(|d| d.get("children"))
        .and_then(|c| c.as_array())
        .and_then(|c| c.first())
        .and_then(|c| c.get("data"))
        .ok_or_else(|| "Gönderi bulunamadı.".to_string())?;

    let title = string_field(post_data, "title");
    if title.trim().is_empty() {
        return Err("Gönderi başlığı boş; bağlantı geçersiz olabilir.".to_string());
    }

    let mut comments: Vec<Comment> = listings
        .get(1)
        .and_then(|l| l.get("data"))
        .and_then(|d| d.get("children"))
        .and_then(|c| c.as_array())
        .map(|children| children.iter().filter_map(parse_comment).collect())
        .unwrap_or_default();

    comments.sort_by(|a, b| b.upvotes.cmp(&a.upvotes));
    comments.truncate(limit);

    Ok(Post {
        id: string_field(post_data, "id"),
        subreddit: string_field(post_data, "subreddit"),
        title,
        selftext: string_field(post_data, "selftext"),
        url: format!("https://www.reddit.com{}", string_field(post_data, "permalink")),
        upvotes: post_data.get("score").and_then(|v| v.as_i64()).unwrap_or(0),
        comment_count: post_data
            .get("num_comments")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        is_nsfw: post_data
            .get("over_18")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        created_utc: post_data
            .get("created_utc")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0),
        removal_note: removal_note(post_data),
        comments,
    })
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostSummary {
    pub id: String,
    pub subreddit: String,
    pub title: String,
    pub url: String,
    pub upvotes: i64,
    pub comment_count: i64,
    pub created_utc: f64,
    pub is_nsfw: bool,
    pub is_video: bool,
    /// Metin gönderisi mi (yorumlar için daha uygun)
    pub is_self: bool,
}

/// Bir subreddit'in gönderi listesini getirir.
///
/// Aynı kimlik yollarını (OAuth veya çerez) kullanır; tek fark uç nokta.
pub async fn fetch_subreddit(
    client_id: &str,
    subreddit: &str,
    sort: &str,
    limit: usize,
) -> Result<Vec<PostSummary>, String> {
    let sub = subreddit
        .trim()
        .trim_start_matches("r/")
        .trim_start_matches('/')
        .to_string();
    if sub.is_empty() {
        return Err("Subreddit adı boş.".to_string());
    }

    let sort = match sort {
        "new" | "top" | "rising" | "hot" => sort,
        _ => "hot",
    };
    let limit = limit.clamp(1, 100);
    let client_id = client_id.trim();

    let payload = if !client_id.is_empty() {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("HTTP istemcisi kurulamadı: {e}"))?;
        let token = access_token(&client, client_id).await?;

        let response = client
            .get(format!(
                "https://oauth.reddit.com/r/{sub}/{sort}?limit={limit}&raw_json=1"
            ))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| format!("Reddit'e ulaşılamadı: {e}"))?;

        if !response.status().is_success() {
            return Err(match response.status().as_u16() {
                403 => format!("r/{sub} erişime kapalı."),
                404 => format!("r/{sub} bulunamadı."),
                code => format!("Reddit {code} yanıtı verdi."),
            });
        }
        response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| format!("Reddit yanıtı okunamadı: {e}"))?
    } else if let Some(cookie) = read_cookie() {
        let client = reqwest::Client::builder()
            .user_agent(BROWSER_USER_AGENT)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("HTTP istemcisi kurulamadı: {e}"))?;

        let endpoint =
            format!("https://old.reddit.com/r/{sub}/{sort}.json?limit={limit}&raw_json=1");

        // Kenar sunucusu taze bağlantıdaki ilk isteği engelleyebiliyor; tekrar deniyoruz.
        let mut last_status = 0u16;
        let mut result = None;

        for attempt in 0..3u32 {
            let response = client
                .get(&endpoint)
                .header("Cookie", cookie.trim())
                .header("Accept", BROWSER_ACCEPT)
                .header("Accept-Language", BROWSER_ACCEPT_LANGUAGE)
                .send()
                .await
                .map_err(|e| format!("Reddit'e ulaşılamadı: {e}"))?;

            if response.status().is_success() {
                result = Some(
                    response
                        .json::<serde_json::Value>()
                        .await
                        .map_err(|e| format!("Reddit yanıtı okunamadı: {e}"))?,
                );
                break;
            }

            if response.status().is_redirection() {
                return Err(
                    "Reddit oturum çerezi geçersiz veya süresi dolmuş. Ayarlar → Genel'den güncelle."
                        .to_string(),
                );
            }

            last_status = response.status().as_u16();
            if matches!(last_status, 403 | 429 | 500..=599) && attempt < 2 {
                tokio::time::sleep(Duration::from_millis(400 * u64::from(attempt + 1))).await;
                continue;
            }
            break;
        }

        result.ok_or_else(|| match last_status {
            404 => format!("r/{sub} bulunamadı."),
            429 => "Reddit hız sınırına takıldı. Birkaç dakika sonra dene.".to_string(),
            code => format!("Reddit {code} yanıtı verdi."),
        })?
    } else {
        return Err(
            "Reddit erişimi kurulmamış. Ayarlar → Genel → Reddit erişimi altından ekle."
                .to_string(),
        );
    };

    let children = payload
        .get("data")
        .and_then(|d| d.get("children"))
        .and_then(|c| c.as_array())
        .ok_or_else(|| "Beklenmeyen Reddit yanıtı.".to_string())?;

    Ok(children
        .iter()
        .filter_map(|child| {
            let data = child.get("data")?;
            let title = string_field(data, "title");
            if title.trim().is_empty() {
                return None;
            }
            // Sabitlenmiş duyurular listeyi kirletiyor.
            if data.get("stickied").and_then(|v| v.as_bool()).unwrap_or(false) {
                return None;
            }

            Some(PostSummary {
                id: string_field(data, "id"),
                subreddit: string_field(data, "subreddit"),
                title,
                url: format!(
                    "https://www.reddit.com{}",
                    string_field(data, "permalink")
                ),
                upvotes: data.get("score").and_then(|v| v.as_i64()).unwrap_or(0),
                comment_count: data
                    .get("num_comments")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                created_utc: data.get("created_utc").and_then(|v| v.as_f64()).unwrap_or(0.0),
                is_nsfw: data.get("over_18").and_then(|v| v.as_bool()).unwrap_or(false),
                is_video: data.get("is_video").and_then(|v| v.as_bool()).unwrap_or(false),
                is_self: data.get("is_self").and_then(|v| v.as_bool()).unwrap_or(false),
            })
        })
        .collect())
}

/// Gönderiyi ve en üst düzey yorumlarını getirir.
///
/// Reddit kimliksiz erişimi kapattığı için iki yoldan biri gerekir:
/// tercih edilen resmî OAuth istemci kimliği, ya da kullanıcının kendi
/// tarayıcı oturum çerezi (anahtar zincirinde saklanır).
pub async fn fetch_post(client_id: &str, url_or_id: &str, limit: usize) -> Result<Post, String> {
    let post_id = extract_post_id(url_or_id)
        .ok_or_else(|| "Bağlantıdan gönderi kimliği çıkarılamadı.".to_string())?;

    let limit = limit.clamp(1, 100);
    let client_id = client_id.trim();

    let payload = if !client_id.is_empty() {
        fetch_via_oauth(client_id, &post_id, limit).await?
    } else if let Some(cookie) = read_cookie() {
        fetch_via_cookie(&cookie, &post_id, limit).await?
    } else {
        return Err(
            "Reddit erişimi kurulmamış. Ayarlar → Genel → Reddit erişimi altından ya bir istemci kimliği ya da oturum çerezi ekle."
                .to_string(),
        );
    };

    parse_listing(payload, limit)
}

#[cfg(test)]
mod kaldirma_testleri {
    use super::removal_note;
    use serde_json::json;

    #[test]
    fn moderator_kaldirmasini_tanir() {
        let not = removal_note(&json!({
            "selftext": "[removed]",
            "removed_by_category": "moderator",
            "author": "birisi",
        }));
        assert_eq!(
            not.as_deref(),
            Some("Bu gönderiyi moderatör kaldırmış; metni artık okunamıyor.")
        );
    }

    #[test]
    fn yazarin_silmesini_tanir() {
        let not = removal_note(&json!({ "selftext": "[deleted]", "author": "[deleted]" }));
        assert!(not.unwrap().contains("yazarı silmiş"));
    }

    #[test]
    fn govdesiz_kaldirilmis_baglantiyi_tanir() {
        let not = removal_note(&json!({
            "selftext": "",
            "removed_by_category": "automod_filtered",
        }));
        assert!(not.unwrap().contains("kaldırılmış"));
    }

    /// Gerçek örnek: r/AmItheAsshole/comments/1wexj1j — yazarı `[deleted]`,
    /// ama gövdesi tam ve 71 yorumu var. Uyarı vermemeli.
    #[test]
    fn silinmis_hesap_ama_iceriği_yerinde_olani_isaretlemez() {
        let not = removal_note(&json!({
            "selftext": "I’m, 37F. My sister (36F) has a 16 year old daughter…",
            "removed_by_category": serde_json::Value::Null,
            "banned_by": serde_json::Value::Null,
            "author": "[deleted]",
            "num_comments": 71,
        }));
        assert_eq!(not, None, "hesabı silinmiş ama içeriği duran gönderi işaretlenmemeli");
    }

    #[test]
    fn saglam_gonderiyi_isaretlemez() {
        let not = removal_note(&json!({
            "selftext": "normal bir metin",
            "author": "birisi",
        }));
        assert_eq!(not, None);
    }
}

#[cfg(test)]
mod tests {
    use super::extract_post_id;

    #[test]
    fn tam_baglantidan_kimlik_cikarir() {
        assert_eq!(
            extract_post_id("https://old.reddit.com/r/AskReddit/comments/1f2k9x/baslik/"),
            Some("1f2k9x".to_string())
        );
    }

    #[test]
    fn kisa_baglantidan_kimlik_cikarir() {
        assert_eq!(
            extract_post_id("https://redd.it/1f2k9x"),
            Some("1f2k9x".to_string())
        );
    }

    #[test]
    fn ciplak_kimligi_kabul_eder() {
        assert_eq!(extract_post_id("1f2k9x"), Some("1f2k9x".to_string()));
    }

    #[test]
    fn gecersiz_baglantiyi_reddeder() {
        assert_eq!(extract_post_id("https://example.com/bir-yazi"), None);
    }

    /// Ağa çıkmadan, kimlikten bağımsız: bağlantı çözülemiyorsa hemen hata döner.
    #[tokio::test]
    async fn cozulemeyen_baglanti_hata_verir() {
        let err = super::fetch_post("", "https://example.com/bir-yazi", 10)
            .await
            .unwrap_err();
        assert!(
            err.contains("gönderi kimliği çıkarılamadı"),
            "beklenmeyen hata: {err}"
        );
    }

    /// Yalnızca ortamda kayıtlı çerez yokken anlamlı; varsa bu dal denenmez.
    #[tokio::test]
    async fn kimlik_yoksa_kurulum_uyarisi_verir() {
        if super::cookie_present() {
            return;
        }
        let err = super::fetch_post("", "https://www.reddit.com/r/x/comments/abc123/y/", 10)
            .await
            .unwrap_err();
        assert!(err.contains("Reddit erişimi kurulmamış"), "beklenmeyen hata: {err}");
    }

    #[test]
    fn gif_gomusunu_atar() {
        let raw = "Next thing he sees\n\n![gif](giphy|VeerK4hE9sjoB8e6OQ)";
        assert_eq!(super::clean_comment_body(raw), "Next thing he sees");
    }

    #[test]
    fn baglantiyi_metne_indirger() {
        let raw = "Şuna bak [bu videoya](https://example.com/v) ve gör.";
        assert_eq!(
            super::clean_comment_body(raw),
            "Şuna bak bu videoya ve gör."
        );
    }

    #[test]
    fn alinti_ve_vurgu_isaretlerini_temizler() {
        let raw = "> **Kesinlikle** katılıyorum\n> ikinci satır";
        assert_eq!(
            super::clean_comment_body(raw),
            "Kesinlikle katılıyorum ikinci satır"
        );
    }

    #[test]
    fn ciplak_baglantiyi_atar() {
        let raw = "Kaynak burada https://reddit.com/r/x kontrol et";
        assert_eq!(super::clean_comment_body(raw), "Kaynak burada kontrol et");
    }

    #[test]
    fn baslik_dizesini_oldugu_gibi_birakir() {
        assert_eq!(
            super::normalize_cookie("  reddit_session=abc; token_v2=def  "),
            "reddit_session=abc; token_v2=def"
        );
    }

    #[test]
    fn cookie_onekini_atar() {
        assert_eq!(
            super::normalize_cookie("Cookie: reddit_session=abc"),
            "reddit_session=abc"
        );
    }

    #[test]
    fn json_ciktisini_baslik_dizesine_cevirir() {
        let json = r#"[{"name":"reddit_session","value":"abc","domain":".reddit.com"},
                       {"name":"token_v2","value":"def"}]"#;
        assert_eq!(
            super::normalize_cookie(json),
            "reddit_session=abc; token_v2=def"
        );
    }

    /// Ağ + anahtar zincirindeki çerez gerektirir:
    /// `cargo test --lib -- --ignored canli_gonderi_ceker --nocapture`
    #[tokio::test]
    #[ignore]
    async fn canli_gonderi_ceker() {
        let post = super::fetch_post("", "https://old.reddit.com/r/GuysBeingDudes/comments/1slbvm9/", 8)
            .await
            .expect("gönderi çekilemedi");
        println!("r/{} · {} oy · {} yorum getirildi", post.subreddit, post.upvotes, post.comments.len());
        println!("başlık: {}", post.title);
        for c in post.comments.iter().take(3) {
            println!("  {} ({} oy): {}", c.author, c.upvotes, c.body.chars().take(90).collect::<String>());
        }
        assert!(!post.title.is_empty());
    }

    /// Ağ gerektirir: `cargo test -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn gecersiz_kimlik_reddedilir() {
        let err = super::fetch_post(
            "gecersiz_istemci_kimligi",
            "https://www.reddit.com/r/AskReddit/comments/abc123/y/",
            5,
        )
        .await
        .unwrap_err();
        println!("hata: {err}");
        assert!(!err.is_empty());
    }
}
