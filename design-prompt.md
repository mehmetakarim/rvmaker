# RVMaker — Claude Design Prompt

> Aşağıdaki bloğun tamamını kopyalayıp Claude Design'a (`/design`) yapıştır.
> Çıkan artboard'ları `RVMaker/Design/` klasörüne koy, sonra Tauri uygulamasını kurmaya başlıyoruz.

---

## PROMPT BAŞLANGICI

**RVMaker** adında bir masaüstü uygulaması için eksiksiz bir arayüz tasarımı hazırla.

### Ürün nedir

RVMaker, bir Reddit gönderisini ve yorumlarını alıp otomatik olarak dikey (9:16) kısa video üreten bir masaüstü uygulaması. Kullanıcı bir Reddit URL'si veriyor; uygulama gönderiyi çekiyor, metni Türkçeye çeviriyor, seslendiriyor, yorum kartlarını görsel olarak üretiyor, arka plan videosuyla birleştirip MP4 çıkarıyor. Çıktı TikTok / Reels / Shorts için kullanılıyor.

- **Teknoloji:** Tauri v2 kabuğu + Vue 3 arayüz. Yani bu bir web sitesi değil, **yerel bir masaüstü uygulaması**.
- **Birincil platform:** macOS (Windows ikincil).
- **Arayüz dili: Türkçe.** Tüm etiketler, butonlar, başlıklar, boş durum metinleri Türkçe olsun. Teknik terimlerde yaygın Türkçe kullanımı tercih et (Kitaplık, Kuyruk, Ayarlar, Oluştur, Önizleme, Çıktı klasörü).
- **Kullanıcı:** Tek kişilik içerik üreticisi. Teknik değil ama sabırsız. Günde 5-10 video üretmek istiyor, her seferinde 20 ayar kurcalamak istemiyor.

### Tasarımın çözmesi gereken asıl problem

Önceki sürüm tek bir URL kutusu ve akan bir terminal logundan ibaretti; kullanıcı ne olduğunu göremiyordu ve videoyu ancak bittikten sonra görüyordu. Yeni tasarımın üç şeyi çözmesi gerekiyor:

1. **Görünürlük** — üretim sırasında hangi aşamada olunduğu, ne kadar kaldığı bir bakışta anlaşılmalı.
2. **Kontrol** — kullanıcı render'dan *önce* hangi yorumların gireceğini, hangi sesin kullanılacağını, kartın nasıl görüneceğini görüp değiştirebilmeli.
3. **Sakinlik** — emoji yağmuru ve renkli terminal çıktısı yok. Sessiz, yoğun, profesyonel bir araç hissi.

### Tasarım yönü

- **Ton:** Linear / Raycast / CleanShot X ailesinden. Yoğun, sessiz, hızlı hissettiren bir üretkenlik aracı. Tüketici uygulaması değil, stüdyo aleti.
- **Tema:** Koyu tema birincil, açık tema da tasarlanmalı. Her ikisi de eksiksiz token seti üzerinden çalışsın.
- **Renk:** Nötr koyu bir zemin (mavi-gri veya kömür) + **tek** doygun vurgu rengi. Vurgu rengini sen seç, ama Reddit turuncusundan uzak dur — bu Reddit'in ürünü değil. Vurgu rengi yalnızca birincil eylem, aktif durum ve ilerleme göstergesinde kullanılsın; her yere serpiştirilmesin.
- **Yoğunluk:** Kompakt. Taban gövde metni 13px, ikincil 12px. Satır yükseklikleri sıkı. Bu bir masaüstü uygulaması, mobil dokunmatik hedefleri gerekmiyor.
- **Izgara:** 8px boşluk ölçeği. Köşe yarıçapı: küçük öğelerde 6px, kart/panelde 10px. Tek bir hafif gölge seviyesi, kenarlıklar gölgeden daha çok iş görsün.
- **Tipografi:** Inter (veya sistem yazı tipi yığını). Log/konsol alanlarında tek boşluklu (JetBrains Mono benzeri) bir yazı tipi.
- **İkonlar:** Lucide seti tarzında, ince çizgi, 16px/20px.
- **Emoji kullanma.** Durumlar ikon + renk ile anlatılsın.

### Pencere kabuğu (her artboard'da tutarlı)

- Varsayılan pencere: **1280 × 832**. En az bir artboard'u 1440 × 900'de de göster ki genişlemede düzenin nasıl davrandığı belli olsun.
- Üstte 38px yüksekliğinde, sürüklenebilir, kenarlıksız bir başlık çubuğu. Sol tarafta macOS trafik ışıkları için **78px** boşluk bırak.
- Solda **220px** sabit kenar çubuğu: ikon + etiketli gezinme. Öğeler: **Yeni Video**, **Kuyruk**, **Kitaplık**, **Ayarlar**. Altta ince bir durum satırı (ör. "ffmpeg hazır · Python 3.12").
- Sağda içerik alanı. İçerik alanı kendi içinde kaydırılabilir; pencere gövdesi asla yatay kaymasın.
- Tarayıcı öğesi kullanma: adres çubuğu, sekme şeridi, favicon, web sayfası altbilgisi yok.

### Tasarlanacak ekranlar (artboard listesi)

Her artboard'u tek bir tuvalde, aşağıdaki isimlendirmeyle üret. İsimleri birebir koru — kod tarafında rota ve bileşen adlarına bunlarla eşleyeceğim.

**`01-onboarding-kurulum`**
İlk açılış. Uygulamanın çalışması için gereken bileşenlerin kontrol listesi: ffmpeg, Python ortamı, arka plan video kitaplığı, çeviri servisi erişimi. Her satırın durumu var (hazır / indiriliyor / eksik). Eksik olan için tek bir "Kur" eylemi. Altta "Atla" seçeneği.

**`02-yeni-video-kaynak`**
Ana giriş ekranı. Büyük ve net bir Reddit URL alanı, yapıştır ve devam et. Altında son kullanılan URL'ler ve "Bir subreddit'ten seç" alternatifi. Sağda veya altta, seçili hazır ayar (preset) kartları — ör. "Türkçe · Kadın ses · Minecraft", "Türkçe · Erkek ses · Gradyan". Hazır ayar seçmek adımların çoğunu atlatır; bu ekranın kahramanı hız olmalı.

**`03-yeni-video-icerik`**
İçerik seçimi. Sol tarafta gönderi başlığı ve meta bilgisi (subreddit, oy sayısı, yorum sayısı). Ortada yorum listesi: her satırda onay kutusu, orijinal İngilizce metin ve altında Türkçe çevirisi, karakter sayısı ve tahmini ses süresi. Yorumlar sürüklenerek sıralanabilir. Üstte toplam tahmini video süresini gösteren canlı bir özet çubuğu. Çevirisi kullanıcı tarafından elle düzeltilebilir olmalı — bu durumu (düzenleme modundaki bir satır) da göster.

**`04-yeni-video-ses`**
Seslendirme ayarları. TTS motoru seçimi (Google Translate, ElevenLabs, OpenAI, TikTok, sistem sesi) segmentli bir kontrol veya liste olarak. Seçilen motorun sesleri liste halinde; her satırda oynat düğmesi ve küçük bir dalga formu. Konuşma hızı ve cümle arası sessizlik için kaydırıcılar. API anahtarı gerektiren motorlarda, anahtar eksikse satır içinde uyarı ve Ayarlar'a kısayol göster.

**`05-yeni-video-gorunum`**
Görsel ayarlar. Arka plan videosu seçimi küçük önizleme kareleriyle bir ızgara (Minecraft, gradyan, sörf, yerel dosya ekle). Arka plan müziği ve ses seviyesi. Kart teması (koyu / açık / saydam), yazı tipi boyutu, kart genişliği. **Sağda sabit duran, gerçek zamanlı 9:16 telefon önizlemesi** — ayarlar değiştikçe güncellenen bir örnek yorum kartı gösteriyor. Bu önizleme bu ekranın en önemli parçası.

**`06-render-calisiyor`**
Üretim ekranı. Solda büyük 9:16 canlı önizleme (o an işlenen kart). Sağda aşama listesi dikey bir stepper olarak: Gönderi çekiliyor → Çeviri → Seslendirme → Kartlar oluşturuluyor → Arka plan hazırlanıyor → Video render ediliyor. Tamamlananlar onaylı, aktif olan ilerleme yüzdesiyle, bekleyenler soluk. Üstte genel ilerleme ve kalan süre tahmini. Altta katlanmış bir "Ayrıntılı günlük" paneli — açık hâlini de göster, tek boşluklu yazı tipiyle, ama renkli terminal gürültüsü olmadan. Sağ üstte İptal.

**`07-render-hata`**
Aynı ekranın hata hâli. Hangi aşamada kırıldığı stepper'da net görünsün. Hata için insan diliyle yazılmış bir açıklama + teknik ayrıntıyı açan bir bağlantı. Eylemler: "Tekrar dene", "Günlüğü kopyala", "Ayarları düzenle". Hata kutusu kırmızı bir felaket alanı gibi değil, sakin ve çözüm odaklı görünsün.

**`08-kitaplik-dolu`**
Üretilmiş videoların ızgarası. Her kart: 9:16 küçük resim, başlık, subreddit rozeti, süre, tarih, dosya boyutu. Üstte arama ve subreddit / tarih filtreleri. Bir kart seçilince sağda ayrıntı paneli açılsın: büyük önizleme, kaynak URL, kullanılan ses ve arka plan, ve eylemler — Finder'da göster, Yeniden üret, Sil.

**`09-kitaplik-bos`**
Boş durum. Tek bir sakin illüstrasyon veya ikon, bir cümlelik açıklama, ve "İlk videonu oluştur" birincil eylemi. Sahte içerik doldurma.

**`10-kuyruk`**
Toplu üretim. Birden fazla URL'nin sıraya alındığı liste: sıra numarası, başlık, hedef ayar özeti, durum rozeti (bekliyor / çalışıyor / bitti / hata). Satırlar sürüklenerek yeniden sıralanabilir. Üstte "Tümünü başlat" / "Duraklat" ve aynı anda kaç iş çalışacağını belirleyen bir ayar. Çalışan satırda satır içi ilerleme çubuğu.

**`11-ayarlar-tts`**
Ayarlar ekranı, sol sekmeli düzen: Genel, Seslendirme, Çeviri, Video, Gelişmiş. Bu artboard'da **Seslendirme** sekmesi açık olsun: API anahtarı alanları (maskeli, göster/gizle düğmeli, "bağlantıyı test et" eylemli), varsayılan ses, varsayılan motor. Anahtarların yerel olarak saklandığını belirten ince bir güvenlik notu.

**`12-ayarlar-video`**
Aynı ayarlar kabuğunda **Video** sekmesi: çözünürlük, kare hızı, kodek, bit hızı, donanım hızlandırma anahtarı, çıktı klasörü seçici, dosya adlandırma şablonu. Ayrıca "Geçici dosyaları temizle" gibi bakım eylemleri ve kapladıkları alan.

**`13-acik-tema`**
`02-yeni-video-kaynak` veya `06-render-calisiyor` ekranlarından birinin açık tema karşılığı. Koyu temanın renk çevirisi olarak değil, kendi başına doğru görünen bir tasarım olsun.

**`14-tasarim-sistemi`**
Stil rehberi artboard'u: renk token'ları (isimleriyle birlikte), tipografi ölçeği, boşluk ölçeği, köşe yarıçapları, gölge, ve bileşen envanteri tüm varyant ve durumlarıyla — buton (birincil / ikincil / hayalet / tehlike; normal, hover, basılı, devre dışı, yükleniyor), metin alanı (normal, odaklı, hatalı, devre dışı), açılır liste, anahtar, kaydırıcı, onay kutusu, segmentli kontrol, etiket/rozet, ilerleme çubuğu ve dairesel ilerleme, stepper, kart, modal, bildirim (toast), araç ipucu, sekme, boş durum, günlük konsolu, ses önizleme satırı, arka plan seçim karesi.

### Uygulama sırasında işime yarayacak şeyler

- Renk, boşluk ve tipografi değerlerini **CSS özel değişkenleri** olarak tanımla ve `--rv-` önekiyle isimlendir (`--rv-bg-base`, `--rv-accent`, `--rv-text-muted`, `--rv-space-4`, `--rv-radius-md` gibi). Ham hex değerlerini bileşenlerin içine gömme.
- Yeniden kullanılan her parçaya, Vue bileşeni olarak yazacağım ismi ver: `AppSidebar`, `StepProgress`, `CommentRow`, `VoicePreviewRow`, `BackgroundTile`, `PhonePreview`, `LogConsole`, `SettingsTabs`, `EmptyState`, `QueueRow` gibi PascalCase adlar.
- Her ekranda odak halkasının nasıl göründüğünü en az bir kez göster; bu bir klavye ağırlıklı araç olacak.
- Metin/zemin kontrastı her iki temada da WCAG AA'yı geçsin, özellikle soluk ikincil metinlerde.
- Uzun içerik (yorum listesi, günlük, kitaplık ızgarası) kendi kapsayıcısı içinde kaysın; genel düzen sabit kalsın.

### Yapmamanı istediğim şeyler

- Pazarlama/açılış sayfası bölümleri, hero alanı, fiyatlandırma, özellik vitrini yok. Bu bir araç arayüzü.
- Gerçek marka logosu, Reddit logosu veya gerçek kişi ismi/avatarı kullanma. Yorum yazarları için jenerik takma adlar üret.
- Her yüzeye gradyan, cam efekti, neon parıltı serpiştirme. En fazla bir yerde, bilinçli bir vurgu olarak kullan.
- Emoji ile durum anlatma.
- Kalabalık gösterge paneli. Gösterilen her sayı bir karara hizmet etsin.

## PROMPT SONU

---

## Tasarım geldikten sonra

Artboard'ları `RVMaker/Design/` klasörüne koy. Benim için en kullanışlı hâli:

1. `.dc.html` artboard dosyaları (Claude Design canvas'ından dışa aktarılan hâli), **ve**
2. Canvas'ın Artifact bağlantısı — böylece gerektiğinde ayrıntıya ben bakabilirim.

Sadece PNG ekran görüntüsü de iş görür ama token değerlerini ve ölçüleri elle tahmin etmem gerekir; HTML varsa tasarım sistemi doğrudan koda dönüşür.
