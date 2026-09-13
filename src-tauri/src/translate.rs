//! Metin çevirisi.
//!
//! Google Translate'in Chrome sözlük uzantısı uç noktasını kullanır —
//! API anahtarı gerektirmez. (`translate_a/single` uç noktası bu makineden
//! 429 döndürüyor; `clients5.../translate_a/t` çalışıyor.)
//! Resmî bir sözleşmesi olmadığı için hız sınırına takılabilir; bu yüzden
//! istekler sırayla ve zaman aşımıyla yapılır.

const USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// Varsayılan zaman aşımı — ayar verilmediğinde kullanılıyor.
const DEFAULT_TIMEOUT_SEC: u64 = 20;

/// Ayarlardaki "istek zaman aşımı" değeriyle istemci kurar.
pub fn client_with_timeout(seconds: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(seconds.clamp(5, 300)))
        .build()
        .map_err(|e| format!("HTTP istemcisi kurulamadı: {e}"))
}

pub fn client() -> Result<reqwest::Client, String> {
    client_with_timeout(DEFAULT_TIMEOUT_SEC)
}

/// Metnin zaten hedef dilde olup olmadığını kestirir.
///
/// Reddit'te Türkçe topluluklar da var; Türkçe bir metni "İngilizceden Türkçeye"
/// çevirtmek kelimeleri bozuyor (ölçülen bir örnek: "Haklısın" → "Htalksın").
/// Bu yüzden çeviriye göndermeden önce eleme yapıyoruz.
pub fn is_probably_turkish(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.chars().count() < 8 {
        // Çok kısa metinde güvenilir karar veremeyiz; dokunmamak daha güvenli.
        return true;
    }

    // Türkçeye özgü harfler
    if trimmed.chars().any(|c| "çÇğĞıİöÖşŞüÜ".contains(c)) {
        return true;
    }

    // Sık geçen Türkçe kelimeler (harf içermeyen metinlerde de işe yarar)
    let lowered = format!(" {} ", trimmed.to_lowercase());
    const COMMON: [&str; 18] = [
        " ve ", " bir ", " için ", " bu ", " ile ", " da ", " de ", " gibi ",
        " kadar ", " sonra ", " olan ", " çok ", " en ", " ama ", " ya ",
        " ki ", " değil ", " var ",
    ];

    COMMON.iter().filter(|w| lowered.contains(**w)).count() >= 2
}

/// Tek bir metni çevirir. Boş metin olduğu gibi döner.
/// Bir metinde `needle`'ı tam kelime olarak, büyük/küçük harf gözetmeden
/// değiştirir. "AI" → "yapay zeka" yaparken "SAID" içindeki harfleri bozmaz.
fn replace_whole_word(haystack: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return haystack.to_string();
    }

    let hay: Vec<char> = haystack.chars().collect();
    let pat: Vec<char> = needle.chars().collect();

    // Karakter karakter karşılaştırıyoruz. Metnin tamamını `to_lowercase()`
    // ile çevirmek işe yaramıyor: "İ" iki karaktere açılıyor ve dizin hizası
    // bozuluyor — Türkçe metinlerde bu neredeyse her zaman olur.
    fn ayni(a: char, b: char) -> bool {
        a == b || a.to_lowercase().eq(b.to_lowercase())
    }

    let mut out = String::with_capacity(haystack.len());
    let mut i = 0;

    while i < hay.len() {
        let son = i + pat.len();
        let eslesti = son <= hay.len()
            && hay[i..son]
                .iter()
                .zip(pat.iter())
                .all(|(a, b)| ayni(*a, *b));

        if eslesti {
            let onu_bos = i == 0 || !hay[i - 1].is_alphanumeric();
            let sonu_bos = son >= hay.len() || !hay[son].is_alphanumeric();
            if onu_bos && sonu_bos {
                out.push_str(replacement);
                i = son;
                continue;
            }
        }

        out.push(hay[i]);
        i += 1;
    }

    out
}

/// Kullanıcının tanımladığı terim sözlüğünü **çevrilmiş** metne uygular.
///
/// Biçim: her satırda `kaynak=hedef`. Çeviri sonrasına uyguluyoruz çünkü
/// Google bu tür kısaltmaları (AI, OP) çoğunlukla olduğu gibi bırakıyor;
/// çeviriden önce değiştirmek ise ikinci kez çevrilmelerine yol açardı.
pub fn apply_glossary(text: &str, glossary: &str) -> String {
    let mut out = text.to_string();

    for line in glossary.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((from, to)) = line.split_once('=') else {
            continue;
        };
        let (from, to) = (from.trim(), to.trim());
        if from.is_empty() {
            continue;
        }
        out = replace_whole_word(&out, from, to);
    }

    out
}

pub async fn translate_text(
    client: &reqwest::Client,
    text: &str,
    source: &str,
    target: &str,
    skip_same_language: bool,
) -> Result<String, String> {
    if text.trim().is_empty() {
        return Ok(text.to_string());
    }

    // Zaten hedef dildeyse çeviriye gönderme — bozmaktan iyidir.
    if skip_same_language && target.starts_with("tr") && is_probably_turkish(text) {
        return Ok(text.to_string());
    }

    let endpoint = format!(
        "https://clients5.google.com/translate_a/t?client=dict-chrome-ex&sl={}&tl={}&q={}",
        source,
        target,
        urlencoding::encode(text)
    );

    let response = client
        .get(&endpoint)
        .send()
        .await
        .map_err(|e| format!("Çeviri servisine ulaşılamadı: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Çeviri servisi {} yanıtı verdi.",
            response.status().as_u16()
        ));
    }

    let payload: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Çeviri yanıtı okunamadı: {e}"))?;

    let translated = extract_translation(&payload)
        .ok_or_else(|| "Çeviri yanıtı beklenen biçimde değil.".to_string())?;

    if translated.trim().is_empty() {
        return Err("Çeviri boş döndü.".to_string());
    }

    Ok(translated)
}

/// Uç nokta iki biçimden birini döndürür:
///   `["çeviri"]`                       (kaynak dil açıkça verildiğinde)
///   `[["çeviri","algılanan-dil"]]`     (`sl=auto` kullanıldığında)
fn extract_translation(payload: &serde_json::Value) -> Option<String> {
    let items = payload.as_array()?;
    if items.is_empty() {
        return None;
    }

    if items.iter().all(|item| item.is_string()) {
        return Some(items.iter().filter_map(|i| i.as_str()).collect());
    }

    let joined: String = items
        .iter()
        .filter_map(|item| item.as_array()?.first()?.as_str())
        .collect();

    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
}

#[cfg(test)]
mod sozluk_testleri {
    use super::apply_glossary;

    #[test]
    fn tam_kelimeyi_degistirir() {
        let out = apply_glossary("AI çok hızlı gelişiyor.", "AI=yapay zeka");
        assert_eq!(out, "yapay zeka çok hızlı gelişiyor.");
    }

    #[test]
    fn kelime_icinde_gecen_harfleri_bozmaz() {
        // "SAID" içinde "AI" var; dokunulmamalı.
        let out = apply_glossary("SAID ve MAIL bozulmamalı, AI değişmeli.", "AI=yapay zeka");
        assert_eq!(out, "SAID ve MAIL bozulmamalı, yapay zeka değişmeli.");
    }

    #[test]
    fn buyuk_kucuk_harf_gozetmez() {
        let out = apply_glossary("op böyle demiş, OP haklı.", "OP=gönderi sahibi");
        assert_eq!(out, "gönderi sahibi böyle demiş, gönderi sahibi haklı.");
    }

    #[test]
    fn birden_cok_kural_uygular() {
        let out = apply_glossary("OP dedi ki AI geliyor.", "AI=yapay zeka\nOP=gönderi sahibi");
        assert_eq!(out, "gönderi sahibi dedi ki yapay zeka geliyor.");
    }

    #[test]
    fn bozuk_satirlari_atlar() {
        // Eşittir içermeyen satır, boş satır ve yorum satırı yok sayılmalı.
        let out = apply_glossary("AI test", "  \n# yorum\nesittir yok\nAI=yapay zeka");
        assert_eq!(out, "yapay zeka test");
    }

    /// "İ" harfi `to_lowercase()` ile iki karaktere açılıyor. Eski uygulama
    /// bu yüzden böyle metinlerde sözlüğü tümden atlıyordu.
    #[test]
    fn buyuk_i_iceren_metinde_de_calisir() {
        let out = apply_glossary("İyi ki doğdun, OP haklıymış.", "OP=gönderi sahibi");
        assert_eq!(out, "İyi ki doğdun, gönderi sahibi haklıymış.");
    }

    #[test]
    fn bos_sozluk_metni_degistirmez() {
        let metin = "Hiçbir şey değişmesin.";
        assert_eq!(apply_glossary(metin, ""), metin);
    }

    #[test]
    fn turkce_harfli_terimleri_tasir() {
        let out = apply_glossary("Şirket büyüyor.", "şirket=kurum");
        assert_eq!(out, "kurum büyüyor.");
    }
}

#[cfg(test)]
mod tests {
    use super::extract_translation;

    #[test]
    fn istemci_kurulabiliyor() {
        assert!(super::client().is_ok());
    }

    #[test]
    fn turkce_metni_taniyor() {
        assert!(super::is_probably_turkish(
            "Haklısın gazına gelmişsin gibi, o kadar olumsuz konuşuyor."
        ));
    }

    #[test]
    fn ozel_harf_olmayan_turkceyi_de_taniyor() {
        assert!(super::is_probably_turkish(
            "Bu bir test cumlesi ve icinde ozel harf yok ama Turkce"
        ));
    }

    #[test]
    fn ingilizce_metni_turkce_saymiyor() {
        assert!(!super::is_probably_turkish(
            "Most people think you need to rinse rice until the water runs clear."
        ));
    }

    #[test]
    fn cok_kisa_metne_dokunmuyor() {
        assert!(super::is_probably_turkish("Evet"));
    }

    #[test]
    fn duz_dizi_bicimini_okur() {
        let payload = serde_json::json!(["Selam Dünya."]);
        assert_eq!(
            extract_translation(&payload),
            Some("Selam Dünya.".to_string())
        );
    }

    #[test]
    fn ic_ice_dizi_bicimini_okur() {
        let payload = serde_json::json!([["Selam Dünya.", "en"]]);
        assert_eq!(
            extract_translation(&payload),
            Some("Selam Dünya.".to_string())
        );
    }

    #[test]
    fn bos_yaniti_reddeder() {
        assert_eq!(extract_translation(&serde_json::json!([])), None);
    }

    /// Ağ gerektirir: `cargo test -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn canli_ceviri_yapar() {
        let client = super::client().unwrap();
        let out = super::translate_text(&client, "The cat sat on the mat.", "en", "tr", true)
            .await
            .expect("çeviri başarısız");
        assert!(!out.is_empty());
        assert_ne!(out, "The cat sat on the mat.");
        println!("çeviri: {out}");
    }
}
