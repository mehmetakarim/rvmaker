//! Anahtar zinciri yazma — geliştirme derlemelerindeki tuzağı ele alır.
//!
//! macOS'ta anahtar zinciri kaydı, onu oluşturan ikilinin kod imzasına bağlanır.
//! `tauri dev` her derlemede ikiliyi değiştirdiği için eski kayda erişim
//! "cannot find code object on disk" hatasıyla reddedilebiliyor. Bu durumda
//! kaydı silip yeniden yazmak sorunu çözüyor.

/// Kaydı yazar; kod imzası uyuşmazlığında kaydı silip bir kez daha dener.
pub fn write(service: &str, account: &str, value: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(service, account)
        .map_err(|e| format!("Anahtar zincirine erişilemedi: {e}"))?;

    match entry.set_password(value) {
        Ok(()) => Ok(()),
        Err(first) => {
            // Eski derlemeye ait kayıt engelliyor olabilir; silip yeniden dene.
            let _ = entry.delete_credential();

            let retry = keyring::Entry::new(service, account)
                .map_err(|e| format!("Anahtar zincirine erişilemedi: {e}"))?;

            retry.set_password(value).map_err(|second| {
                format!(
                    "Anahtar zincirine yazılamadı: {second}. \
                     Bu, geliştirme derlemesinde ikilinin her seferinde yeniden imzalanmasından \
                     kaynaklanabiliyor — uygulamayı tamamen kapatıp yeniden açmak genelde çözüyor. \
                     (İlk hata: {first})"
                )
            })
        }
    }
}
