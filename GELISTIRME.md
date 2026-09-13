# RVMaker — geliştirme notları

Kullanım ve kurulum için [README.md](README.md). Bu dosya mimariyi, ölçülen
davranışları ve bilinen sınırları tutuyor.

Reddit gönderilerinden dikey (9:16) video üreten masaüstü uygulaması.
**Tauri v2 + Vue 3 + Vite + TypeScript.**

Arayüz, `Design/design-system-with-14-artboards/` altındaki Claude Design
artboard'larından birebir uygulanmıştır.

## Çalıştırma

Yalnızca arayüz (tarayıcıda, hızlı geliştirme):

```bash
npm run dev
```

Masaüstü uygulaması olarak:

```bash
npm run tauri dev
```

Üretim derlemesi (.app / .dmg):

```bash
npm run tauri build
```

## Klasör yapısı

```
src/
├── main.ts                 # uygulama girişi
├── App.vue                 # kabuk: TitleBar + AppSidebar + RouterView
├── router/index.ts         # rotalar (artboard adlarıyla eşleşir)
├── styles/
│   ├── tokens.css          # tasarım token'ları (--rv-*), koyu + açık tema
│   └── base.css            # sıfırlama, yazı tipleri, kaydırma çubuğu, odak halkası
├── lib/types.ts            # paylaşılan tipler
├── stores/                 # Pinia: draft, jobs, library, settings
├── components/
│   ├── ui/                 # tasarım sistemi bileşenleri (RvButton, RvSlider, …)
│   └── app/                # ürüne özel bileşenler (PhonePreview, StageList, …)
└── views/                  # ekranlar
src-tauri/                  # Rust tarafı (pencere, ortam denetimi komutu)
```

## Rota ↔ artboard eşlemesi

| Rota | Artboard | Görünüm |
|---|---|---|
| `/kurulum` | `01-onboarding-kurulum` | `SetupView.vue` |
| `/yeni/kaynak` | `02-yeni-video-kaynak` | `new/SourceStep.vue` |
| `/yeni/icerik` | `03-yeni-video-icerik` | `new/ContentStep.vue` |
| `/yeni/ses` | `04-yeni-video-ses` | `new/VoiceStep.vue` |
| `/yeni/gorunum` | `05-yeni-video-gorunum` | `new/LookStep.vue` |
| `/render/:id` | `06-render-calisiyor` / `07-render-hata` | `RenderView.vue` |
| `/kitaplik` | `08-kitaplik-dolu` / `09-kitaplik-bos` | `LibraryView.vue` |
| `/kuyruk` | `10-kuyruk` | `QueueView.vue` |
| `/yeni/subreddit` | — (tasarım sonrası eklendi) | `new/SubredditStep.vue` |
| `/yeni/viral` | — (tasarım sonrası eklendi) | `new/ViralStep.vue` |
| `/ayarlar/:tab` | `11-ayarlar-tts` / `12-ayarlar-video` | `SettingsView.vue` |
| — (tema anahtarı) | `13-acik-tema` | `stores/settings.ts` |
| — | `14-tasarim-sistemi` | `components/ui/*` + `styles/tokens.css` |

`06` ve `07` aynı görünümün iki hâli: `jobs.renderState` `"running"` / `"failed"`.
`08` ve `09` aynı şekilde kitaplık dolu/boş durumu.

## Reddit erişimi (tek seferlik kurulum)

Reddit, kimlik doğrulaması olmayan erişimi kapattı — `.json` uç noktası 403,
`old.reddit.com` giriş sayfasına yönlendiriyor, `www.reddit.com` ise
JavaScript olmadan boş bir kabuk döndürüyor. Bu yüzden iki yoldan biri gerekiyor.
İkisi de **Ayarlar → Genel → Reddit erişimi** altında; ikisi de doluysa istemci
kimliği öncelikli olur.

### 1. İstemci kimliği (önerilen)

Kalıcıdır, süresi dolmaz, resmî yoldur.

1. https://www.reddit.com/prefs/apps → **create another app…**
2. Tür: **installed app**
3. redirect uri: `http://localhost:1420` (kullanılmıyor ama alan zorunlu)
4. Uygulama adının altındaki kimliği kopyalayıp ayarlara yapıştır

Gizli anahtar gerekmez. Kimlik `localStorage`'da tutulur.

### 2. Oturum çerezi (uygulama oluşturulamıyorsa)

Reddit bazı hesaplarda uygulama oluşturmaya izin vermiyor. O durumda kendi
tarayıcı oturumun kullanılabilir:

1. Tarayıcıda reddit.com'da oturum açıkken geliştirici araçları → **Network**
2. Herhangi bir isteğe tıkla → istek başlıklarındaki `Cookie` satırının tamamını kopyala
3. Ayarlardaki oturum çerezi alanına yapıştır → **Kaydet**

Reddit'in kenar sunucusu taze bağlantıdaki **ilk isteği** sabit bir engelleme
sayfasıyla (403, tam 189908 bayt) karşılıyor; aynı istemci üzerinden yapılan
ikinci istek geçiyor. Bu yüzden çerez yolu 403/429/5xx durumlarında kısa
beklemelerle en fazla üç kez deniyor. Ayrıca istek başlıkları tarayıcıyı
taklit ediyor — yalnızca `Accept: application/json` istemek bot işareti
sayılıyor.

Çerez **macOS anahtar zincirine** yazılır (`com.rvmaker.desktop` /
`reddit-session-cookie`), arayüze bir daha okunmaz ve yalnızca Reddit
isteklerinde `Cookie` başlığı olarak kullanılır. Hesaba tam erişim verdiği için
kullanılmadığında **Sil** ile kaldırılmalı. Çerezin süresi dolduğunda uygulama
bunu açıkça söyler ve yenilemeni ister.

## X (Twitter) erişimi

Reddit'teki "gönderi + yorumlar" yapısının X'teki karşılığı **tweet +
yanıtları**. Hat aynı; yalnızca kaynak etiketi `r/AskReddit` yerine
`@kullanici` oluyor.

X'te iki kaynak türü var ve kaynak adımından seçiliyor:

| Mod | Başlık kartı | Sonraki kartlar |
|---|---|---|
| **Yanıtlar** | Tweet'in kendisi | Tweet'e gelen yanıtlar, beğeniye göre sıralı |
| **Thread** | Zincirin kök tweet'i | Yazarın kendi devam tweet'leri, **atıldığı sırayla** |

Zincir tweet'lerindeki `🧵1/6`, `(2/6)`, `+` gibi sıra işaretleri **yalnızca
seslendirmeden** ayıklanıyor (`tts::strip_thread_marker`); kartta tweet'in
özgün hâli kalıyor. Ayıklama dar tutuldu: yalnızca bölü içeren biçimler ve
yalnızca metnin başı/sonu hedefleniyor, böylece `3/4'lük kısmı` ya da
`1. Önce suyu kaynat` gibi gerçek içerik bozulmuyor. İşaretten ibaret bir
metinde ayıklama yapılmıyor — sessiz kart üretmektense işareti okumak yeğ.

Thread'de sıra anlam taşıdığı için içerik adımında sıralama seçeneği tek
seçeneğe iniyor ("Zincir sırası"); beğeniye göre dizmek `1/6, 2/6…` akışını
bozardı. Zincirin ortasındaki bir bağlantı yapıştırılsa bile baştan
başlanıyor — `conversationId` kökü gösteriyor.

Zinciri kurmak `bird thread` çıktısını süzmeyi gerektiriyor: araç düz bir dizi
döndürüyor ve içinde **başka yazarların** yanıtları da var. Kökten başlayıp her
adımda bir öncekine yanıt veren *aynı yazara ait* tweet takip ediliyor. Yazarın
yorumculara verdiği cevaplar bu yüzden zincire girmiyor — ölçüldü: bir tweet'te
yazarın 5 kendi yanıtı vardı ama hiçbiri zincirin devamı değildi, hepsi farklı
kişilere cevaptı. Kaynak adımına bir `x.com/…/status/…` bağlantısı
yapıştırmak yeterli; **X'te viral olanlar** ekranı ise ölçüt vererek gündemden
tweet seçmeyi sağlıyor.

X'in iç API'si sık değiştiği için kendi istemcimizi yazmıyoruz; bakımlı
[`bird`](https://www.npmjs.com/package/@steipete/bird) aracını çağırıyoruz:

```bash
npm install -g @steipete/bird
```

Ayarlar → Genel'den `auth_token` ve `ct0` çerezleri giriliyor (Cookie-Editor
JSON çıktısı ya da `Cookie` başlığı dizesi kabul ediliyor). İkisi **aynı
oturumdan, aynı anda** alınmalı — eşleşmezlerse X isteği `csrf 353` ile
reddediyor. Çerezler anahtar zincirinde saklanıyor (`com.rvmaker.desktop` /
`x-auth-token`, `x-ct0`) ve bird'e komut satırında açıkça geçiriliyor; bird'ün
kendi tarayıcı çerezi okuması bu makinede aynı 353 hatasını veriyordu.

Viral ekranının ölçütleri X'in arama sözdizimine çevriliyor
(`min_faves:`, `min_replies:`, `lang:`, `since_time:`, `-filter:links`,
`-filter:replies`, `-filter:retweets`). Yanıtı ondan az olan tweet'ler
listelenmiyor: videoda yalnızca başlık kartı kalırdı.

## Üretim hattı

"Oluştur" tek başına baştan sona video üretir. Altı aşamanın hepsi gerçek:

| Aşama | Ne yapar |
|---|---|
| Gönderi çekme | Reddit'ten gönderi + üst düzey yorumlar; sabitlenmiş/moderatör yorumları ve silinmiş gövdeler elenir, markdown temizlenir |
| Çeviri | Sırayla çevirir, ilerlemeyi olayla bildirir. Metin zaten hedef dildeyse atlar |
| Seslendirme | Motora göre parçalara böler, indirir, ffmpeg ile birleştirir; hız ve cümle arası sessizlik uygulanır |
| Kartlar | 1080×1920 saydam PNG; uzun metinde punto otomatik küçülür, sığmazsa kesilir |
| Arka plan | Seçilen video döngüye alınır ve kareye oturtulur; yoksa düz zemin |
| Render | ffmpeg ile birleştirme — kart süreleri ses dosyalarından gelir, senkron kendiliğinden doğrudur |

Çıktılar `~/Movies/RVMaker/<iş>/` altına yazılır: video, ses parçaları,
`kartlar/` klasörü ve kitaplığın okuduğu `bilgi.json`.

## Seslendirme motorları

| Motor | Anahtar | Not |
|---|---|---|
| Google Translate | gerekmez | Dil başına tek ses. İstek sınırı **200 karakter** (ölçüldü) |
| Sistem sesi (`say`) | gerekmez | Çevrimdışı; Türkçe için yalnızca Yelda kurulu geliyor |
| ElevenLabs | gerekir | Ses listesi API'den canlı çekilir |
| OpenAI | gerekir | Sabit altı ses |
| Gemini | gerekir | Ham PCM döndürür, ffmpeg ile MP3'e çevrilir |

Konuşma hızı motordan bağımsız: sentez sonrası ffmpeg `atempo` filtresiyle
uygulanır, dolayısıyla hız kaydırıcısı her motorda çalışır.

Anahtarlar sistem anahtar zincirinde saklanır ve kaydedildikten sonra arayüze
bir daha okunmaz.

## Araç yolları (PATH)

macOS'ta Finder ya da Dock'tan başlatılan uygulamalar kabuğun PATH'ini almıyor;
süreç yalnızca `/usr/bin:/bin:/usr/sbin:/sbin` görüyor. Homebrew'daki
`ffmpeg`/`ffprobe` ve nvm'deki `node`/`bird` bu listede olmadığı için uygulama
"ffmpeg eksik" diyor, `bird` de `env: node: No such file or directory` ile
düşüyordu. Terminalden `open -a` ile açıldığında sorun görünmüyor — bu yüzden
geliştirme sırasında fark edilmedi.

`src-tauri/src/toolpath.rs` açılışta PATH'i genişletiyor: Homebrew dizinleri,
`~/.local/bin`, `~/bin`, `~/.cargo/bin` ve nvm'nin kurduğu her sürümün `bin`
klasörü. Yalnızca diskte var olanlar ekleniyor, mevcut girdiler önde kalıyor.
`bird` ayrıca kendi sürümünün `node` ikilisini bulsun diye alt sürece kendi
dizini PATH'in başında geçiriliyor.

## Çeviri ayarları

Ayarlar → Çeviri sekmesindeki üç kontrol de gerçek:

| Ayar | Ne yapıyor |
|---|---|
| **Çeviri servisi** | `Google Translate` normal akış; `Çevirme` seçilirse metin özgün dilinde kalıyor — Türkçe kaynaklarda (X gündemi) çeviri zaten zarar veriyor. |
| **Zaten hedef dildeyse atla** | `is_probably_turkish()` elemesini açıp kapatıyor. Kapatılırsa Türkçe metin de Google'a gidiyor. |
| **Terim sözlüğü** | Satır satır `kaynak=hedef`. Çeviri **sonrasına** uygulanıyor: Google `AI`, `OP` gibi kısaltmaları çoğunlukla olduğu gibi bırakıyor, çeviriden önce değiştirmek ise ikinci kez çevrilmelerine yol açardı. Eşleşme tam kelime ve büyük/küçük harf duyarsız — `SAID` içindeki `AI` bozulmuyor. |

Sözlük eşleştirmesi metnin tamamını küçük harfe çevirmiyor: `İ` harfi
`to_lowercase()` ile iki karaktere açılıyor ve dizin hizası bozuluyor. Bunun
yerine karakter karakter karşılaştırılıyor.

## Dayanıklılık

**Yarım kalan iş çöpe gitmiyor.** "Yeniden dene" artık yeni bir `job-<zaman>`
klasörü açmıyor, aynı klasörde sürdürüyor. Her ses parçası üretildiğinde iş
klasörüne `ses-imza.json` yazılıyor; ikinci koşuda aynı imzaya sahip parçalar
diskten alınıyor, yalnızca eksikler yeniden seslendiriliyor.

İmza `motor|ses|hız|sessizlik|metin` değerlerinden üretiliyor. Bunlardan biri
değişirse imza tutmaz ve parça yeniden üretilir — bu kontrol olmadan yarım bir
işi farklı bir sesle sürdürmek videonun ortasında sesi değiştirirdi. Dosya
yarım yazılmışsa `ffprobe` süre veremez ve parça yine yeniden üretilir.

**Yarım iş oturumlar arası sürdürülüyor.** İş başlarken klasöre `taslak.json`
yazılıyor (gönderi, yorumlar, ses ve görünüm ayarları). Uygulama kapansa bile
kitaplığın üstünde "Yarım kalan üretim" şeridi çıkıyor ve **Devam et** aynı
klasörde kaldığı yerden sürdürüyor. `bilgi.json` yalnızca iş bittiğinde
yazıldığı için tek başına yetmiyordu.

Taslağı olmayan eski yarım klasörler listede görünmüyor: sürdürecek bir girdi
yok, yalnızca ses dosyaları var. Taslak sürümü tutmuyorsa da yüklenmiyor —
eksik alanlarla devam etmek işi yanlış ayarlarla sürdürmek olurdu.

**Geçici ağ hataları işi düşürmüyor.** Durum kodları (429, 5xx) için zaten
tekrar deneme vardı, ama `send()` hatası hattı anında kesiyordu: kısa bir Wi-Fi
kesintisi ya da DNS takılması tüm işe mal oluyordu. `tts::send_with_retry`
bağlantı ve zaman aşımı hatalarını 0,5 sn ve 1,0 sn bekleyerek üç kez deniyor;
Google, Gemini, ElevenLabs ve OpenAI çağrılarının hepsi buradan geçiyor.
Sertifika ya da istek kurma hatalarında beklenmiyor — onlarda tekrarın anlamı
yok.

## Yerel derleme yolu

`src-tauri/.cargo/config.toml` depoya **dahil değil** ve olmamalı: içinde
makineye özel mutlak bir yol var. Proje yavaş bir harici diskte duruyorsa ara
çıktıları dahili diske yönlendirmek derlemeyi belirgin biçimde hızlandırıyor:

```toml
[build]
target-dir = "/Users/<kullanici>/Library/Caches/rvmaker-target"
```

Bu dosya sürüm iş akışında bir kez depoya sızdı ve macOS paketleri
`Permission denied` ile düştü — CI çalıştırıcısında o yol yok.

## Ses normalizasyonu — `dynaudnorm` kullanma

`dynaudnorm=f=250:g=15` ön-belleğini (15×250 ms) çıktıya geri vermiyor ve
karışımın **son ~4 saniyesini yutuyor**. Sonuç: videonun sonunda seslendirme
ve müzik birden kesiliyor, ekranda yalnızca arka plan kalıyor. Konteyner
süresi doğru görünüyor — delik ancak ham örnek sayılınca ortaya çıkıyor:

```
mp4 video  : 38,13 sn
mp4 sesi   : 34,13 sn   ← 4 sn eksik
ses paketi : 29,49 → 33,50 sn arası boş
```

Yerine `speechnorm=e=6.25:r=0.00001:l=1` kullanılıyor: ses düzeyi neredeyse
birebir aynı (-17,9 dB ortalama / -0,4 dB tepe; eskisi -18,0 / -0,8) ama süre
korunuyor.

Hata **sentetik sesle tekrarlanmıyor** — sinüs ve sessizlik karışımlarında
`dynaudnorm` süreyi koruyor. Bu yüzden `canli_ses_suresi_korunuyor` testi
gerçek bir üretim `ses.mp3`'ü arıyor; bulamazsa atlıyor.

## Sürüm numarası

`package.json`, `src-tauri/tauri.conf.json` ve `src-tauri/Cargo.toml` aynı
sürümü taşımalı ve git etiketiyle eşleşmeli. Sürüm iş akışı paket adlarını ve
sürüm başlığını `tauri.conf.json`'dan alıyor: bir kez `v0.1.1` etiketi
atılırken dosya 0.1.0'da kalmıştı ve yayın "RVMaker v0.1.0" adıyla,
`RVMaker_0.1.0_*` dosyalarıyla çıktı.

## Bilinen sınırlar

- **Kuyrukta eşzamanlı iş 1'e sabit.** ffmpeg zaten tüm çekirdekleri kullandığı
  için paralel render toplam süreyi kısaltmıyor. Ayar arayüzde duruyor ama
  gerçek bir fayda ölçülene kadar bağlanmadı.
- **Geliştirme derlemesinde anahtar zinciri kırılgan.** macOS kaydı oluşturan
  ikilinin kod imzasına bağlıyor; `tauri dev` her derlemede yeniden imzaladığı
  için erişim reddedilebiliyor. Kod bunu yakalayıp kaydı silip yeniden yazıyor.
  İmzalı sürüm derlemesinde sorun ortadan kalkar.
- **CSP'yi değiştirirken dikkat.** `tauri.conf.json` içindeki güvenlik
  politikasında `script-src` ve `connect-src` **mutlaka** bulunmalı. Eksik
  olurlarsa uygulama geliştirme derlemesinde sorunsuz çalışır ama sürüm
  derlemesinde **bomboş bir pencere** açar ve konsola hiçbir şey düşmez:
  Tauri'nin satır içi başlatma betiği ve `ipc` kanalı engellenir. Sebebi,
  geliştirmede `http://localhost:1420`, sürümde `tauri://localhost`
  kullanılması ve politikanın ancak ikincisinde gerçekten uygulanması.
- **Tarayıcıda** (`npm run dev`) arka uç yok; ekranlar tohum veriyle çalışır,
  üretim yapılamaz. Gerçek kullanım için `npm run app`.

