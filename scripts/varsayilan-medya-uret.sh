#!/usr/bin/env bash
# RVMaker'la gelen varsayılan arka plan videosunu ve fon müziğini yeniden üretir.
# İkisi de bu depoda özgün olarak üretilmiştir; üçüncü taraf içerik kullanılmaz.
set -euo pipefail
cd "$(dirname "$0")/.."
OUT=src-tauri/resources/media
TMP=$(mktemp -d)
mkdir -p "$OUT"

# Yumuşak, tüm kareyi kaplayan gradyan: 10 sn ileri + 10 sn geri = dikişsiz döngü
ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i "gradients=s=720x1280:x0=0:y0=0:x1=720:y1=1280:c0=0x4338ca:c1=0x0e7490:c2=0xbe185d:c3=0x6d28d9:nb_colors=4:speed=0.02:duration=10:rate=30:seed=3" \
  -filter_complex "[0:v]gblur=sigma=40,eq=brightness=-0.10:saturation=0.85,vignette=PI/4,split[a][b];[b]reverse[r];[a][r]concat=n=2:v=1:a=0,format=yuv420p[v]" \
  -map "[v]" -c:v libx264 -preset slow -crf 30 -movflags +faststart -an \
  "$OUT/varsayilan-gradyan.mp4"

# Prosedürel ambiyans (numpy gerekir), ardından yankı ve yumuşak giriş/çıkış
python3 scripts/varsayilan-ambiyans-uret.py "$TMP/ham.wav"
ffmpeg -hide_banner -loglevel error -y -i "$TMP/ham.wav" \
  -af "lowpass=f=7000,aecho=0.8:0.6:190|340:0.22|0.12,afade=t=in:d=2,afade=t=out:st=57:d=3" \
  -c:a libmp3lame -b:a 128k "$OUT/varsayilan-ambiyans.mp3"

rm -rf "$TMP"
ls -lh "$OUT"
