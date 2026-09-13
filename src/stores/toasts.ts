import { defineStore } from "pinia";
import { ref } from "vue";

export type ToastTone = "success" | "error" | "info" | "warning";

export interface Toast {
  id: number;
  tone: ToastTone;
  text: string;
  /** Tıklanabilir eylem — ör. "Klasörde göster" */
  actionLabel?: string;
  action?: () => void;
}

/** Kısa bir bildirim sesi. Dosya taşımamak için Web Audio ile üretiliyor. */
function playChime(tone: ToastTone) {
  try {
    const Ctx = window.AudioContext ?? (window as unknown as { webkitAudioContext: typeof AudioContext }).webkitAudioContext;
    if (!Ctx) return;
    const ctx = new Ctx();

    // Başarıda yükselen iki nota, hatada alçalan iki nota.
    const notes = tone === "error" ? [660, 440] : [660, 880];
    const now = ctx.currentTime;

    notes.forEach((freq, i) => {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = "sine";
      osc.frequency.value = freq;

      const start = now + i * 0.12;
      const end = start + 0.16;
      // Yumuşak giriş ve çıkış; ani başlayan ses tıklama gibi duyuluyor.
      gain.gain.setValueAtTime(0, start);
      gain.gain.linearRampToValueAtTime(0.14, start + 0.02);
      gain.gain.exponentialRampToValueAtTime(0.0001, end);

      osc.connect(gain).connect(ctx.destination);
      osc.start(start);
      osc.stop(end + 0.02);
    });

    setTimeout(() => ctx.close().catch(() => {}), 800);
  } catch {
    /* ses üretilemezse bildirim yine görünür */
  }
}

export const useToastStore = defineStore("toasts", () => {
  const items = ref<Toast[]>([]);
  let nextId = 1;

  function dismiss(id: number) {
    items.value = items.value.filter((t) => t.id !== id);
  }

  /**
   * Bildirim gösterir.
   * @param sound Ses çalınsın mı — kullanıcının bildirim tercihi burada karar veriyor.
   */
  function push(
    toast: Omit<Toast, "id">,
    options: { sound?: boolean; durationMs?: number } = {},
  ) {
    const id = nextId++;
    items.value.push({ ...toast, id });

    if (options.sound) playChime(toast.tone);

    const duration = options.durationMs ?? (toast.tone === "error" ? 8000 : 5000);
    setTimeout(() => dismiss(id), duration);
    return id;
  }

  return { items, push, dismiss };
});
