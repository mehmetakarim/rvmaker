<script setup lang="ts">
import { computed } from "vue";
import type { CardTheme } from "@/lib/types";

const props = withDefaults(
  defineProps<{
    width?: number;
    author: string;
    upvotes: string;
    text: string;
    theme?: CardTheme;
    /** Kart görselindeki yazı boyutu (1080 px genişlik ölçeğinde) */
    fontSize?: number;
    /** Kart genişliği, video genişliğinin yüzdesi */
    cardWidth?: number;
    progress?: number;
    elapsed?: string;
    total?: string;
  }>(),
  {
    width: 270,
    theme: "dark",
    fontSize: 40,
    cardWidth: 86,
    progress: 0.34,
    elapsed: "0:22",
    total: "1:04",
  },
);

const height = computed(() => Math.round((props.width * 16) / 9));

/** 1080 px genişlikteki kart ölçüsünü önizleme genişliğine indirger. */
const scaledFont = computed(() => Math.max(9, (props.fontSize * props.width) / 1080));

const cardStyle = computed(() => {
  const inset = ((100 - props.cardWidth) / 2 / 100) * props.width;
  const base = {
    left: `${inset}px`,
    right: `${inset}px`,
    fontSize: `${scaledFont.value}px`,
  };
  if (props.theme === "light") {
    return { ...base, background: "rgba(246,247,249,.94)", color: "#15171d", borderColor: "rgba(0,0,0,.08)" };
  }
  if (props.theme === "transparent") {
    return { ...base, background: "rgba(20,22,27,.35)", color: "#f2f4f8", borderColor: "rgba(255,255,255,.16)" };
  }
  return { ...base, background: "rgba(20,22,27,.92)", color: "#f2f4f8", borderColor: "rgba(255,255,255,.08)" };
});
</script>

<template>
  <div class="phone" :style="{ width: `${width}px`, height: `${height}px` }">
    <div class="scrim"></div>

    <div class="card" :style="cardStyle">
      <div class="byline">
        <span class="avatar"></span>
        {{ author }}
        <span>· {{ upvotes }}</span>
      </div>
      <div class="text">{{ text }}</div>
    </div>

    <div class="timeline">
      <div class="track">
        <div class="fill" :style="{ width: `${progress * 100}%` }"></div>
      </div>
      <div class="times rv-mono">
        <span>{{ elapsed }}</span>
        <span>{{ total }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.phone {
  position: relative;
  overflow: hidden;
  border-radius: 20px;
  border: 1px solid var(--rv-border-strong);
  background: repeating-linear-gradient(135deg, #242b3a 0 8px, #1e2431 8px 16px);
  box-shadow: var(--rv-shadow-1);
  flex: none;
}

.scrim {
  position: absolute;
  inset: 0;
  background: linear-gradient(
    180deg,
    rgba(10, 11, 15, 0.35),
    rgba(10, 11, 15, 0.05) 40%,
    rgba(10, 11, 15, 0.5)
  );
}

.card {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  padding: 14px;
  border-radius: 12px;
  border: 1px solid;
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-2);
}

.byline {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 10px;
  color: #9aa1b1;
}

.avatar {
  width: 14px;
  height: 14px;
  border-radius: var(--rv-radius-pill);
  background: #3a4152;
}

.text {
  line-height: 1.4;
}

.timeline {
  position: absolute;
  left: 16px;
  right: 16px;
  bottom: 16px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.track {
  height: 3px;
  border-radius: var(--rv-radius-pill);
  background: rgba(255, 255, 255, 0.18);
  overflow: hidden;
}

.fill {
  height: 100%;
  background: #fff;
}

.times {
  display: flex;
  justify-content: space-between;
  font-size: 10px;
  color: rgba(255, 255, 255, 0.7);
}
</style>
