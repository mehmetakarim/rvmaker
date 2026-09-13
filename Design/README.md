# Design

Claude Design ile üretilen arayüz tasarımları buraya konulacak.

## Beklenen içerik

```
Design/
├── README.md
├── canvas-url.txt          # Claude Design canvas'ının Artifact bağlantısı
├── 01-onboarding-kurulum.dc.html
├── 02-yeni-video-kaynak.dc.html
├── 03-yeni-video-icerik.dc.html
├── 04-yeni-video-ses.dc.html
├── 05-yeni-video-gorunum.dc.html
├── 06-render-calisiyor.dc.html
├── 07-render-hata.dc.html
├── 08-kitaplik-dolu.dc.html
├── 09-kitaplik-bos.dc.html
├── 10-kuyruk.dc.html
├── 11-ayarlar-tts.dc.html
├── 12-ayarlar-video.dc.html
├── 13-acik-tema.dc.html
└── 14-tasarim-sistemi.dc.html
```

Dosya adları `../design-prompt.md` içindeki artboard adlarıyla birebir aynı olmalı;
kod tarafında rota ve bileşen adlarına bu isimlerle eşlenecek.

PNG dışa aktarımları da işe yarar, ancak `.dc.html` dosyaları tasarım
sistemini (CSS değişkenleri, ölçüler, durumlar) doğrudan koda çevirmeyi sağlar.
