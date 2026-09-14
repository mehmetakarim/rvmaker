"""RVMaker için özgün, telifsiz varsayılan fon müziği üretir.

Yumuşak bir akor pad'i, derin bir bas ve seyrek çan tınılarından oluşan 60
saniyelik sakin bir ambiyans. Tamamen prosedürel sentez — hiçbir kayıt ya da
var olan eserden örnek kullanılmıyor.
"""

import sys
import wave

import numpy as np

SR = 44100
SLOT = 5.0          # her akorun süresi (sn)
TEKRAR = 3          # dört akorluk dizinin kaç kez döneceği
ATAK = 1.4
SALIM = 1.8

# (bas, [pad sesleri]) — Hz
AKORLAR = [
    (110.00, [220.00, 261.63, 329.63, 493.88]),   # La minör (add9)
    (87.31,  [174.61, 220.00, 261.63, 329.63]),   # Fa maj7
    (130.81, [261.63, 329.63, 392.00, 587.33]),   # Do (add9)
    (98.00,  [196.00, 246.94, 293.66, 329.63]),   # Sol 6
]

sure = SLOT * len(AKORLAR) * TEKRAR
n = int(SR * sure)
t_tum = np.arange(n) / SR
sol = np.zeros(n)
sag = np.zeros(n)
rng = np.random.default_rng(20260915)


def zarf(uzunluk: int, atak: float, salim: float) -> np.ndarray:
    z = np.ones(uzunluk)
    a = min(int(atak * SR), uzunluk // 2)
    r = min(int(salim * SR), uzunluk // 2)
    # yumuşak (kosinüs) giriş ve çıkış — tıkırtı olmasın
    z[:a] = 0.5 - 0.5 * np.cos(np.linspace(0, np.pi, a))
    z[-r:] = 0.5 + 0.5 * np.cos(np.linspace(0, np.pi, r))
    return z


for dongu in range(TEKRAR):
    for i, (bas, pad) in enumerate(AKORLAR):
        baslangic = (dongu * len(AKORLAR) + i) * SLOT
        # akorlar birbirine taşsın: önceki bitmeden sonraki yükselsin
        b = max(0, int((baslangic - 0.8) * SR))
        e = min(n, int((baslangic + SLOT + SALIM) * SR))
        t = t_tum[b:e] - t_tum[b]
        z = zarf(e - b, ATAK, SALIM)
        tremolo = 1.0 - 0.12 * (0.5 + 0.5 * np.sin(2 * np.pi * 0.18 * t + i))

        for k, f in enumerate(pad):
            genlik = 0.085 / (1 + 0.35 * k)
            faz = rng.uniform(0, 2 * np.pi)
            # sol ve sağ kanal hafif farklı akortlu — koro sıcaklığı
            sol[b:e] += genlik * np.sin(2 * np.pi * (f - 0.45) * t + faz) * z * tremolo
            sag[b:e] += genlik * np.sin(2 * np.pi * (f + 0.45) * t + faz * 1.3) * z * tremolo
            # ikinci harmonik, çok zayıf — sesi biraz doldursun
            sol[b:e] += genlik * 0.12 * np.sin(2 * np.pi * 2 * f * t) * z
            sag[b:e] += genlik * 0.12 * np.sin(2 * np.pi * 2 * f * t + 0.4) * z

        bas_ses = 0.11 * np.sin(2 * np.pi * bas * t) * z
        sol[b:e] += bas_ses
        sag[b:e] += bas_ses

        # seyrek çan: akorun iki sesinden biri, iki oktav üstte, sönümlü
        for adim, gecikme in enumerate((0.35, 2.6)):
            if (dongu + i + adim) % 3 == 2:
                continue  # her çanı çalma; seyrek kalsın
            ton = pad[(i + adim * 2) % len(pad)] * 2
            cb = int((baslangic + gecikme) * SR)
            ce = min(n, cb + int(3.2 * SR))
            if cb >= n:
                continue
            ct = t_tum[cb:ce] - t_tum[cb]
            can = 0.05 * np.sin(2 * np.pi * ton * ct) * np.exp(-ct / 1.1)
            can *= 1 - np.exp(-ct / 0.004)  # tıkırtısız giriş
            pan = 0.3 if adim == 0 else 0.7
            sol[cb:ce] += can * (1 - pan)
            sag[cb:ce] += can * pan

stereo = np.stack([sol, sag], axis=1)
stereo /= np.max(np.abs(stereo)) / 0.7
pcm = (stereo * 32767).astype(np.int16)

hedef = sys.argv[1]
with wave.open(hedef, "wb") as w:
    w.setnchannels(2)
    w.setsampwidth(2)
    w.setframerate(SR)
    w.writeframes(pcm.tobytes())

print(f"{hedef}: {sure:.0f} sn, {len(AKORLAR) * TEKRAR} akor")
