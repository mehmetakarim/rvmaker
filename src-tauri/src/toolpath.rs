//! Harici araçların bulunabilmesi için süreç PATH'ini genişletir.
//!
//! macOS'ta Finder ya da Dock'tan başlatılan uygulamalar kabuğun PATH'ini
//! almıyor; `launchctl getenv PATH` tanımsızsa süreç yalnızca
//! `/usr/bin:/bin:/usr/sbin:/sbin` görüyor (ölçüldü). Homebrew'daki
//! `ffmpeg`/`ffprobe` ve nvm'deki `node`/`bird` bu listede olmadığı için
//! uygulama "ffmpeg eksik" diyor, `bird` de `env: node: No such file or
//! directory` ile düşüyordu. Terminalden açıldığında sorun görünmüyordu —
//! o yüzden geliştirme sırasında fark edilmedi.

use std::path::{Path, PathBuf};

/// Ev dizininden bağımsız, yaygın kurulum yerleri.
const SYSTEM_DIRS: [&str; 4] = [
    "/opt/homebrew/bin", // Apple Silicon Homebrew
    "/opt/homebrew/sbin",
    "/usr/local/bin", // Intel Homebrew ve elle kurulanlar
    "/usr/local/sbin",
];

/// Kullanıcının ev dizini.
///
/// Windows'ta `HOME` tanımlı olmuyor; orada `USERPROFILE` var.
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Bir aracın tam yolunu bulur.
///
/// `which` yalnızca Unix'te var; Windows'ta karşılığı `where` ve birden çok
/// satır dönebiliyor, ilkini alıyoruz.
pub fn find_tool(name: &str) -> Option<String> {
    let arayici = if cfg!(target_os = "windows") { "where" } else { "which" };
    let out = std::process::Command::new(arayici).arg(name).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let yol = String::from_utf8_lossy(&out.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    if yol.is_empty() {
        None
    } else {
        Some(yol)
    }
}

/// Çalışan bir süreci sonlandırır. Windows'ta `kill` yok.
pub fn kill_pid(pid: u32) {
    let _ = if cfg!(target_os = "windows") {
        std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output()
    } else {
        // ffmpeg SIGTERM ile temiz kapanır ve yarım dosyayı bırakır.
        std::process::Command::new("kill").arg(pid.to_string()).output()
    };
}

/// Ev dizinine göreli aday dizinler.
const HOME_DIRS: [&str; 3] = [".local/bin", "bin", ".cargo/bin"];

/// PATH girdilerini sırayı bozmadan birleştirir ve yinelenenleri atar.
///
/// Mevcut girdiler önde kalıyor: kullanıcının kendi kurduğu bir sürüm varsa
/// bizim eklediğimiz dizin onu gölgelemesin.
pub fn merge(current: &str, extra: &[PathBuf]) -> String {
    let mut out: Vec<String> = Vec::new();

    let mut push = |value: String| {
        if value.is_empty() || out.iter().any(|existing| *existing == value) {
            return;
        }
        out.push(value);
    };

    for entry in current.split(':') {
        push(entry.to_string());
    }
    for dir in extra {
        push(dir.to_string_lossy().to_string());
    }

    out.join(":")
}

/// nvm'nin kurduğu sürümlerin `bin` klasörlerini toplar.
fn nvm_bin_dirs(home: &Path) -> Vec<PathBuf> {
    let versions = home.join(".nvm/versions/node");
    let Ok(entries) = std::fs::read_dir(versions) else {
        return Vec::new();
    };

    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path().join("bin"))
        .filter(|dir| dir.is_dir())
        .collect();

    // readdir sırası garanti değil; sabit bir sıra yeniden üretilebilirlik için.
    dirs.sort();
    dirs
}

/// Diskte gerçekten var olan aday dizinleri döndürür.
fn candidates(home: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = SYSTEM_DIRS.iter().map(PathBuf::from).collect();
    dirs.extend(HOME_DIRS.iter().map(|rel| home.join(rel)));
    dirs.extend(nvm_bin_dirs(home));
    dirs.retain(|dir| dir.is_dir());
    dirs
}

/// Süreç PATH'ini genişletir. Uygulama açılırken bir kez çağrılıyor; bundan
/// sonra başlatılan her alt süreç genişletilmiş listeyi miras alıyor.
pub fn augment() {
    let Some(home) = home_dir() else {
        return;
    };
    let current = std::env::var("PATH").unwrap_or_default();
    let merged = merge(&current, &candidates(&home));
    std::env::set_var("PATH", merged);
}

#[cfg(test)]
mod tests {
    use super::merge;
    use std::path::PathBuf;

    #[test]
    fn mevcut_girdileri_onde_tutar() {
        let birlesik = merge("/usr/bin:/bin", &[PathBuf::from("/opt/homebrew/bin")]);
        assert_eq!(birlesik, "/usr/bin:/bin:/opt/homebrew/bin");
    }

    #[test]
    fn yinelenenleri_atar() {
        let birlesik = merge(
            "/usr/bin:/opt/homebrew/bin:/bin",
            &[PathBuf::from("/opt/homebrew/bin"), PathBuf::from("/usr/bin")],
        );
        assert_eq!(birlesik, "/usr/bin:/opt/homebrew/bin:/bin");
    }

    #[test]
    fn bos_girdiler_sizmaz() {
        // PATH'te art arda iki iki nokta olması boş girdi üretir; bunu
        // taşımak "geçerli dizin" anlamına gelir ve istemediğimiz bir şey.
        let birlesik = merge("/usr/bin::/bin:", &[]);
        assert_eq!(birlesik, "/usr/bin:/bin");
    }

    /// Finder'dan açılan uygulamanın gördüğü çıplak PATH'i taklit eder ve
    /// araçların gerçekten bulunabildiğini doğrular. Süreç genelindeki PATH'i
    /// değiştirdiği için elle çalıştırılıyor:
    /// `cargo test canli_path -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn canli_path() {
        fn bulunur(arac: &str) -> bool {
            super::find_tool(arac).is_some()
        }

        std::env::set_var("PATH", "/usr/bin:/bin:/usr/sbin:/sbin");
        for arac in ["ffmpeg", "ffprobe", "node"] {
            println!("genişletme öncesi {arac}: {}", bulunur(arac));
        }

        super::augment();
        println!("PATH: {}", std::env::var("PATH").unwrap_or_default());

        for arac in ["ffmpeg", "ffprobe", "node"] {
            let ok = bulunur(arac);
            println!("genişletme sonrası {arac}: {ok}");
            assert!(ok, "{arac} genişletilmiş PATH'te bulunamadı");
        }
    }

    #[test]
    fn bos_pathi_kurtarir() {
        let birlesik = merge("", &[PathBuf::from("/opt/homebrew/bin")]);
        assert_eq!(birlesik, "/opt/homebrew/bin");
    }
}
