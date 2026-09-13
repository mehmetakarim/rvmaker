import type { CardTheme } from "@/lib/types";

/** Kartlar bu tabanda üretilir; render gerekirse hedef çözünürlüğe ölçekler. */
export const FRAME_WIDTH = 1080;
export const FRAME_HEIGHT = 1920;

/** Önizleme 270 px genişlikte çiziliyor; oradaki ölçüler bu katsayıyla büyür. */
const PREVIEW_SCALE = FRAME_WIDTH / 270;

/** Kart ile kare kenarı arasında bırakılan en küçük boşluk. */
const MIN_VERTICAL_MARGIN = 80;

/** Otomatik küçültmenin ineceği taban punto. */
const MIN_FONT_SIZE = 24;

export interface CardOptions {
  author: string;
  upvotes: string;
  text: string;
  theme: CardTheme;
  /** 1080 px ölçeğinde yazı boyutu */
  fontSize: number;
  /** Kart genişliği, kare genişliğinin yüzdesi */
  cardWidth: number;
}

interface Palette {
  background: string;
  border: string;
  text: string;
  muted: string;
  avatar: string;
}

function palette(theme: CardTheme): Palette {
  if (theme === "light") {
    return {
      background: "rgba(246,247,249,0.94)",
      border: "rgba(0,0,0,0.08)",
      text: "#15171d",
      muted: "#565c6b",
      avatar: "#c9cdd7",
    };
  }
  if (theme === "transparent") {
    return {
      background: "rgba(20,22,27,0.35)",
      border: "rgba(255,255,255,0.16)",
      text: "#f2f4f8",
      muted: "#c3c8d4",
      avatar: "#3a4152",
    };
  }
  return {
    background: "rgba(20,22,27,0.92)",
    border: "rgba(255,255,255,0.08)",
    text: "#f2f4f8",
    muted: "#9aa1b1",
    avatar: "#3a4152",
  };
}

/** Metni verilen genişliğe göre satırlara böler. */
function wrapText(
  ctx: CanvasRenderingContext2D,
  text: string,
  maxWidth: number,
): string[] {
  const lines: string[] = [];
  let current = "";

  for (const word of text.split(/\s+/)) {
    const candidate = current ? `${current} ${word}` : word;
    if (ctx.measureText(candidate).width <= maxWidth || !current) {
      current = candidate;
    } else {
      lines.push(current);
      current = word;
    }
  }

  if (current) lines.push(current);
  return lines;
}

function roundedRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number,
) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
  ctx.closePath();
}

/** Yazı tipleri hazır olmadan ölçüm yanlış çıkar; bir kez bekleyip önbelleğe alıyoruz. */
let fontsReady: Promise<void> | null = null;

function ensureFonts(fontSize: number): Promise<void> {
  if (!fontsReady) {
    fontsReady = (async () => {
      await document.fonts.load(`500 ${fontSize}px Inter`);
      await document.fonts.load(`400 ${fontSize}px Inter`);
      await document.fonts.ready;
    })();
  }
  return fontsReady;
}

/**
 * Tek bir yorum kartını 1080×1920 saydam kare üzerine çizer.
 *
 * Arka plan videosu ffmpeg tarafında altına yerleştirileceği için kare
 * saydam bırakılır; yalnızca kart çizilir.
 */
export async function renderCard(options: CardOptions): Promise<Blob> {
  await ensureFonts(options.fontSize);

  const canvas = document.createElement("canvas");
  canvas.width = FRAME_WIDTH;
  canvas.height = FRAME_HEIGHT;

  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("Canvas bağlamı oluşturulamadı.");

  const colors = palette(options.theme);

  const cardW = (FRAME_WIDTH * options.cardWidth) / 100;
  const cardX = (FRAME_WIDTH - cardW) / 2;
  const padding = 14 * PREVIEW_SCALE;
  const radius = 12 * PREVIEW_SCALE;
  const bylineSize = 10 * PREVIEW_SCALE;
  const avatarSize = 14 * PREVIEW_SCALE;
  const gap = 8 * PREVIEW_SCALE;
  const bylineHeight = Math.max(avatarSize, bylineSize);
  const textMaxWidth = cardW - padding * 2;

  /** Kartın kare içinde kalması gereken en büyük yüksekliği. */
  const maxCardH = FRAME_HEIGHT - MIN_VERTICAL_MARGIN * 2;

  // Uzun yorumlarda kart kareyi taşmasın diye yazıyı kademeli küçültüyoruz.
  let fontSize = options.fontSize;
  let lines: string[] = [];
  let cardH = 0;

  while (true) {
    ctx.font = `400 ${fontSize}px Inter, sans-serif`;
    lines = wrapText(ctx, options.text, textMaxWidth);
    cardH = padding * 2 + bylineHeight + gap + lines.length * fontSize * 1.4;

    if (cardH <= maxCardH || fontSize <= MIN_FONT_SIZE) break;
    fontSize -= 2;
  }

  // En küçük punto ile bile sığmıyorsa metni kısaltıp üç nokta koyuyoruz;
  // yarısı kare dışında kalan bir kart üretmektense görünür biçimde kesiyoruz.
  if (cardH > maxCardH) {
    const maxLines = Math.max(
      1,
      Math.floor((maxCardH - padding * 2 - bylineHeight - gap) / (fontSize * 1.4)),
    );
    lines = lines.slice(0, maxLines);
    const last = lines.length - 1;
    if (last >= 0) lines[last] = `${lines[last].replace(/[\s.,;:]+$/, "")}…`;
    cardH = padding * 2 + bylineHeight + gap + lines.length * fontSize * 1.4;
  }

  const lineHeight = fontSize * 1.4;
  const cardY = (FRAME_HEIGHT - cardH) / 2;

  // Kart gövdesi
  ctx.fillStyle = colors.background;
  roundedRect(ctx, cardX, cardY, cardW, cardH, radius);
  ctx.fill();
  ctx.lineWidth = PREVIEW_SCALE;
  ctx.strokeStyle = colors.border;
  ctx.stroke();

  // Yazar satırı
  const bylineY = cardY + padding + bylineHeight / 2;
  ctx.fillStyle = colors.avatar;
  ctx.beginPath();
  ctx.arc(cardX + padding + avatarSize / 2, bylineY, avatarSize / 2, 0, Math.PI * 2);
  ctx.fill();

  ctx.fillStyle = colors.muted;
  ctx.font = `400 ${bylineSize}px Inter, sans-serif`;
  ctx.textBaseline = "middle";
  const bylineText = options.upvotes
    ? `${options.author} · ${options.upvotes}`
    : options.author;
  ctx.fillText(bylineText, cardX + padding + avatarSize + gap * 0.75, bylineY);

  // Gövde metni
  ctx.fillStyle = colors.text;
  ctx.font = `400 ${fontSize}px Inter, sans-serif`;
  ctx.textBaseline = "top";
  let y = cardY + padding + bylineHeight + gap;
  for (const line of lines) {
    ctx.fillText(line, cardX + padding, y);
    y += lineHeight;
  }

  return await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (blob) resolve(blob);
      else reject(new Error("Kart görseli üretilemedi."));
    }, "image/png");
  });
}
