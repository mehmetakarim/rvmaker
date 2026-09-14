# RVMaker

Reddit gönderilerinden ve X (Twitter) tweet'lerinden **dikey kısa video** üreten
bir masaüstü uygulaması. Bağlantıyı yapıştır, yorumları seç, sesi ve görünümü
ayarla — gerisini uygulama hallediyor.

macOS ve Windows için paketleri [Releases](../../releases) sayfasında.

---

## Ne işe yarar

Kısa video üretmek, aslında birbirini tekrar eden bir zincir: içeriği bul,
çevir, seslendir, kart görsellerini hazırla, arka planla birleştir, kes.
Elle yapıldığında bir video yarım saat alıyor ve her seferinde aynı adımlar
tekrarlanıyor. RVMaker bu zinciri tek ekrana indiriyor.

**Kimin işine yarar:**

- **İçerik üreticileri** — TikTok, Reels ve Shorts için düzenli akış üretmek
  isteyenler. Gündemdeki bir tweet'ten ya da popüler bir subreddit gönderisinden
  birkaç dakikada video çıkıyor.
- **Türkçe içerik yapanlar** — İngilizce kaynaklar otomatik çevriliyor, ama
  Türkçe içerik "çeviri" diye bozulmuyor: metin zaten hedef dildeyse çeviriye
  hiç gönderilmiyor.
- **Haber ve topluluk hesapları** — bir tweet'in yanıtları ya da bir thread,
  okunabilir kartlara dönüşüyor.
- **Erişilebilirlik** — uzun metin tartışmalarını sesli ve altyazılı biçime
  çevirmek isteyenler.
- **Kendi hattını kurmak isteyen geliştiriciler** — her adım ayrı bir modül;
  motorlar, kart tasarımı ve render ayarları değiştirilebilir.

**Neyi iyi yapıyor:**

- Önizleme ile çıktı birebir aynı — kartlar arayüzde hangi tuvalle çiziliyorsa
  videoya da o giriyor.
- Yarım kalan üretim kaldığı yerden sürüyor; üretilmiş ses parçaları yeniden
  üretilmiyor.
- Beş seslendirme motoru arasında geçiş yapılabiliyor, ücretsiz seçenek dahil.
- Kimlik bilgileri sistem anahtar zincirinde; arayüze bir daha okunmuyor.

---

## Kurulum

1. [Releases](../../releases) sayfasından işletim sistemine uygun paketi indir.
   - **macOS**: Apple Silicon için `aarch64`, Intel için `x64` dosyası
   - **Windows**: `.exe` kurulumu ya da `.msi`
2. **ffmpeg** kur — video birleştirme bunu kullanıyor:

   ```bash
   # macOS
   brew install ffmpeg

   # Windows
   winget install Gyan.FFmpeg
   ```

3. Uygulamayı aç. İlk açılışta **kurulum ekranı** geliyor ve neyin hazır,
   neyin eksik olduğunu gösteriyor; ffmpeg gibi zorunlu bir araç sonradan
   eksik kalırsa ekran açılışta yeniden çıkıyor.

Kutudan çıktığı gibi video üretebilmen için paketin içinde **varsayılan bir
arka plan videosu** (yumuşak bir gradyan) ve **telifsiz bir ambiyans müziği**
geliyor. Kendi videolarını ve müziklerini **Ayarlar → Video** altından klasör
seçerek ekleyebilirsin; varsayılanlar listede kalmaya devam ediyor.

> macOS'ta paket imzalı değil; ilk açılışta Gatekeeper uyarırsa
> **Sistem Ayarları → Gizlilik ve Güvenlik → Yine de aç** demen gerekiyor.

---

## Kullanım

1. **Kaynak** — Reddit gönderi bağlantısı ya da X tweet bağlantısı yapıştır.
   Bağlantı elinde yoksa **Bir subreddit'ten seç** ya da **X'te viral olanlar**
   ekranlarından gündemi tarayabilirsin.
2. **İçerik** — yorumları seç, sırala, düzenle; istersen kendi yorumunu ekle.
   Çeviri bu adımda çalışıyor.
3. **Ses** — motoru ve sesi seç, hızı ve cümle aralarındaki sessizliği ayarla,
   önizleme ile dinle.
4. **Görünüm** — arka plan videosu, fon müziği, kart teması ve yazı boyutu.
5. **Oluştur** — hat baştan sona koşuyor; ilerleme ve günlük ekranda.

Üretilen videolar **Kitaplık** ekranında; oradan oynatılabilir, klasörde
gösterilebilir ya da aynı kaynakla yeniden üretilebilir.

---

## İçerik kaynakları

### Reddit

İki yol var. **İstemci kimliği** (önerilen): reddit.com/prefs/apps adresinden
"installed app" türünde bir uygulama oluşturup kimliği gir — kalıcı, gizli
anahtar gerekmiyor. Uygulama oluşturamıyorsan **oturum çerezi** de çalışıyor;
tarayıcının geliştirici araçlarından `Cookie` başlığını kopyalayıp yapıştırman
yeterli.

### X (Twitter)

Reddit'teki "gönderi + yorumlar" yapısının karşılığı **tweet + yanıtları**.
Ayrıca **thread** modu var: yazarın kendi devam tweet'leri, atıldığı sırayla.

X içeriği [`bird`](https://www.npmjs.com/package/@steipete/bird) aracıyla
çekiliyor. [Node.js](https://nodejs.org) kurulu olmalı (macOS ve Windows):

```bash
npm install -g @steipete/bird
```

> `@steipete/bird` paketi npm'de hâlâ kurulabiliyor, ancak geliştiricisi
> tarafından bakımı bırakılmış durumda. X tarafı ileride bu araca bağlı
> kalmayacak şekilde değişebilir.

Ayarlardan `auth_token` ve `ct0` çerezleri giriliyor. İkisi **aynı oturumdan,
aynı anda** alınmalı — eşleşmezlerse X isteği reddediyor.

---

## Seslendirme motorları

| Motor | Anahtar | Not |
|---|---|---|
| **Google Translate** | gerekmez | Ücretsiz, her yerde çalışır. Varsayılan. |
| **Gemini** | Google AI Studio | Doğal ses. Ücretsiz katmanda kota var; uygulama modeller arasında geçiş yapıyor. |
| **ElevenLabs** | ElevenLabs | En doğal sonuç. Anahtarın `text_to_speech` ve `voices_read` izinleri açık olmalı. |
| **OpenAI** | OpenAI | Dengeli. |
| **Sistem sesi** | gerekmez | Yalnızca macOS (`say` komutu). |

Anahtarlar sistem anahtar zincirine yazılıyor, arayüze bir daha okunmuyor ve
yalnızca ilgili servise giden isteklerde kullanılıyor.

---

## Gizlilik

- API anahtarları ve oturum çerezleri **yalnızca sistem anahtar zincirinde**
  tutuluyor — ne dosyaya ne de depoya yazılıyor.
- Uygulama kendi sunucusuna bir şey göndermiyor; istekler doğrudan seçtiğin
  servise gidiyor.
- Üretilen videolar ve ara dosyalar yalnızca senin diskinde.

---

## Katkı ve geliştirme

Kaynaktan çalıştırmak, mimari, modül yapısı ve bilinen sınırlar için
[GELISTIRME.md](GELISTIRME.md) dosyasına bak.

Kısaca:

```bash
npm install
npm run tauri dev     # geliştirme
npm run tauri build   # paket üret
```

Gereksinimler: Node 20+, Rust (stable), ffmpeg.

---

## Teşekkür

Bu proje, [RedditVideoMakerBot](https://github.com/elebumm/RedditVideoMakerBot)
reposundaki fikirden esinlenerek geliştirildi. O proje Python tabanlı bir komut
satırı hattı; RVMaker ise aynı fikri masaüstü uygulamasına taşıyor — görsel
önizleme, adım adım akış, kuyruk, kitaplık ve birden çok içerik kaynağıyla.
Üretim hattı ve kart yaklaşımı sıfırdan yeniden yazıldı.

## Lisans

MIT — [LICENSE](LICENSE)
