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

/// Alt süreç komutu kurar.
///
/// Windows'ta GUI uygulamasından başlatılan her konsol programı (ffmpeg,
/// where, node) kendi konsol penceresini açıp kapatıyor — kullanıcı her
/// seslendirme parçasında siyah bir pencerenin yanıp söndüğünü görüyor.
/// `CREATE_NO_WINDOW` bunu engelliyor.
pub fn command<S: AsRef<std::ffi::OsStr>>(program: S) -> std::process::Command {
    #[allow(unused_mut)]
    let mut cmd = std::process::Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// PATH girdilerini ayıran karakter. Windows'ta `;` — orada `:` sürücü
/// harflerinde (`C:\\`) geçtiği için ayırıcı olarak kullanılamaz.
pub const PATH_SEP: char = if cfg!(target_os = "windows") { ';' } else { ':' };

/// `which`/`where` çıktısından çalıştırılabilir yolu seçer.
///
/// Windows'ta npm global kurulumu aynı araç için hem uzantısız bir Unix
/// betiği (`bird`) hem de `bird.cmd` üretiyor ve `where` ilk satırda
/// uzantısızı veriyor. Onu çalıştırmak "geçerli bir Win32 uygulaması değil"
/// (os error 193) hatasıyla düşüyordu. Windows'ta gerçekten çalıştırılabilir
/// uzantılara öncelik veriyoruz.
pub fn pick_where_line(stdout: &str, windows: bool) -> Option<String> {
    let satirlar: Vec<&str> = stdout.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let secilen = if windows {
        [".exe", ".cmd", ".bat"]
            .iter()
            .find_map(|uzanti| {
                satirlar
                    .iter()
                    .find(|l| l.to_ascii_lowercase().ends_with(uzanti))
            })
            .or_else(|| satirlar.first())
    } else {
        satirlar.first()
    };
    secilen.map(|s| s.to_string())
}

/// Bir aracın tam yolunu bulur. Unix'te `which`, Windows'ta `where`.
pub fn find_tool(name: &str) -> Option<String> {
    let windows = cfg!(target_os = "windows");
    let arayici = if windows { "where" } else { "which" };
    let out = command(arayici).arg(name).output().ok()?;
    if !out.status.success() {
        return None;
    }
    pick_where_line(&String::from_utf8_lossy(&out.stdout), windows)
}

/// Bir dizini PATH'in başına ekler — platformun ayırıcısıyla.
pub fn prepend(dir: &str, current: &str) -> String {
    prepend_with(dir, current, PATH_SEP)
}

/// `prepend`'in ayırıcısı açıkça verilen, test edilebilir hâli.
pub fn prepend_with(dir: &str, current: &str, sep: char) -> String {
    if dir.is_empty() {
        return current.to_string();
    }
    if current.is_empty() {
        return dir.to_string();
    }
    format!("{dir}{sep}{current}")
}

/// Çalışan bir süreci sonlandırır. Windows'ta `kill` yok.
pub fn kill_pid(pid: u32) {
    let _ = if cfg!(target_os = "windows") {
        command("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output()
    } else {
        // ffmpeg SIGTERM ile temiz kapanır ve yarım dosyayı bırakır.
        command("kill").arg(pid.to_string()).output()
    };
}

/// Ev dizinine göreli aday dizinler.
const HOME_DIRS: [&str; 3] = [".local/bin", "bin", ".cargo/bin"];

/// PATH girdilerini sırayı bozmadan birleştirir ve yinelenenleri atar.
///
/// Mevcut girdiler önde kalıyor: kullanıcının kendi kurduğu bir sürüm varsa
/// bizim eklediğimiz dizin onu gölgelemesin.
pub fn merge(current: &str, extra: &[PathBuf]) -> String {
    merge_with(current, extra, PATH_SEP)
}

/// `merge`'ün ayırıcısı açıkça verilen, test edilebilir hâli.
pub fn merge_with(current: &str, extra: &[PathBuf], sep: char) -> String {
    let mut out: Vec<String> = Vec::new();

    let mut push = |value: String| {
        if value.is_empty() || out.iter().any(|existing| *existing == value) {
            return;
        }
        out.push(value);
    };

    for entry in current.split(sep) {
        push(entry.to_string());
    }
    for dir in extra {
        push(dir.to_string_lossy().to_string());
    }

    out.join(&sep.to_string())
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
    use super::{merge_with, pick_where_line, prepend_with};
    use std::path::PathBuf;

    #[test]
    fn mevcut_girdileri_onde_tutar() {
        let birlesik = merge_with("/usr/bin:/bin", &[PathBuf::from("/opt/homebrew/bin")], ':');
        assert_eq!(birlesik, "/usr/bin:/bin:/opt/homebrew/bin");
    }

    #[test]
    fn yinelenenleri_atar() {
        let birlesik = merge_with(
            "/usr/bin:/opt/homebrew/bin:/bin",
            &[PathBuf::from("/opt/homebrew/bin"), PathBuf::from("/usr/bin")],
            ':',
        );
        assert_eq!(birlesik, "/usr/bin:/opt/homebrew/bin:/bin");
    }

    #[test]
    fn bos_girdiler_sizmaz() {
        // PATH'te art arda iki iki nokta olması boş girdi üretir; bunu
        // taşımak "geçerli dizin" anlamına gelir ve istemediğimiz bir şey.
        let birlesik = merge_with("/usr/bin::/bin:", &[], ':');
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

    /// Windows PATH'inde sürücü harfleri `:` içeriyor; eskiden `:` ile
    /// bölündüğü için `C:\\Windows` "C" ve "\\Windows" diye parçalanıyordu.
    #[test]
    fn windows_pathinde_surucu_harfleri_bozulmaz() {
        let birlesik = merge_with(
            r"C:\Windows;C:\Program Files\nodejs;C:\Windows",
            &[PathBuf::from(r"C:\Users\m\AppData\Roaming\npm")],
            ';',
        );
        assert_eq!(
            birlesik,
            r"C:\Windows;C:\Program Files\nodejs;C:\Users\m\AppData\Roaming\npm"
        );
    }

    #[test]
    fn basa_ekleme_platform_ayiricisini_kullanir() {
        assert_eq!(prepend_with("/a", "/b:/c", ':'), "/a:/b:/c");
        assert_eq!(
            prepend_with(r"C:\npm", r"C:\Windows", ';'),
            r"C:\npm;C:\Windows"
        );
        assert_eq!(prepend_with("", "/b", ':'), "/b");
        assert_eq!(prepend_with("/a", "", ':'), "/a");
    }

    /// Kullanıcının Windows makinesinde ölçülen `where bird` çıktısı.
    #[test]
    fn windowsta_calistirilabilir_uzantiyi_secer() {
        let cikti = "C:\\Users\\m\\AppData\\Roaming\\npm\\bird\r\n\
                     C:\\Users\\m\\AppData\\Roaming\\npm\\bird.cmd\r\n";
        assert_eq!(
            pick_where_line(cikti, true).as_deref(),
            Some(r"C:\Users\m\AppData\Roaming\npm\bird.cmd")
        );
    }

    #[test]
    fn windowsta_exe_cmd_den_once_gelir() {
        let cikti = "C:\\x\\ffmpeg.cmd\nC:\\y\\ffmpeg.exe\n";
        assert_eq!(pick_where_line(cikti, true).as_deref(), Some(r"C:\y\ffmpeg.exe"));
    }

    #[test]
    fn unixte_ilk_satiri_alir() {
        assert_eq!(
            pick_where_line("/opt/homebrew/bin/ffmpeg\n/usr/local/bin/ffmpeg\n", false).as_deref(),
            Some("/opt/homebrew/bin/ffmpeg")
        );
        assert_eq!(pick_where_line("", false), None);
    }

    /// Windows'ta konsol penceresi açılmaması için bütün alt süreçler
    /// `toolpath::command`'dan geçmeli. Bu kural bir kez kaçtı: `tts.rs`'in
    /// başındaki tek satırlık bir `#[cfg(test)]` yüzünden oradaki üretim
    /// çağrıları test sanıldı ve her seslendirme parçasında pencere açıldı.
    /// Kaynağı tarayıp test modülleri dışındaki `Command::new`'ı yakalıyoruz.
    #[test]
    fn alt_surecler_yalnizca_yardimcidan_gecer() {
        let kok = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut ihlaller = Vec::new();
        for giris in std::fs::read_dir(&kok).unwrap().flatten() {
            let yol = giris.path();
            if yol.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let ad = yol.file_name().unwrap().to_string_lossy().to_string();
            if ad == "toolpath.rs" {
                continue;
            }
            let metin = std::fs::read_to_string(&yol).unwrap();
            let satirlar: Vec<&str> = metin.lines().collect();

            // `#[cfg(test)] mod x { ... }` bloklarını kapsamıyla birlikte dışarıda bırak.
            let mut test_satiri = vec![false; satirlar.len()];
            let mut i = 0;
            while i < satirlar.len() {
                if satirlar[i].trim() == "#[cfg(test)]" && i + 1 < satirlar.len() {
                    let sonraki = satirlar[i + 1].trim_start();
                    if sonraki.starts_with("mod ") || sonraki.starts_with("pub mod ") {
                        let mut derinlik = 0i32;
                        let mut k = i + 1;
                        while k < satirlar.len() {
                            derinlik += satirlar[k].matches('{').count() as i32;
                            derinlik -= satirlar[k].matches('}').count() as i32;
                            test_satiri[k] = true;
                            if derinlik == 0 && k > i + 1 {
                                break;
                            }
                            k += 1;
                        }
                        i = k;
                    }
                }
                i += 1;
            }

            for (n, satir) in satirlar.iter().enumerate() {
                if !test_satiri[n] && satir.contains("Command::new(") {
                    ihlaller.push(format!("{ad}:{} {}", n + 1, satir.trim()));
                }
            }
        }
        assert!(
            ihlaller.is_empty(),
            "toolpath::command yerine Command::new kullanılmış:\n{}",
            ihlaller.join("\n")
        );
    }

    #[test]
    fn bos_pathi_kurtarir() {
        let birlesik = merge_with("", &[PathBuf::from("/opt/homebrew/bin")], ':');
        assert_eq!(birlesik, "/opt/homebrew/bin");
    }
}
