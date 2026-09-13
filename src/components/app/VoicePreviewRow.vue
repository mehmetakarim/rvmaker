<script setup lang="ts">
import { Play, Pause, Check } from "@lucide/vue";
import type { Voice } from "@/lib/types";

defineProps<{
  voice: Voice;
  selected: boolean;
  playing: boolean;
  /** 0-1: dalga formunun ne kadarının çalındığı */
  played?: number;
  elapsed?: string;
  duration?: string;
}>();

defineEmits<{ select: []; toggle: [] }>();
</script>

<template>
  <div
    class="row"
    :class="{ selected }"
    data-component="VoicePreviewRow"
    @click="$emit('select')"
  >
    <button
      type="button"
      class="play"
      :class="{ active: playing }"
      :aria-label="playing ? 'Duraklat' : 'Önizlemeyi çal'"
      @click.stop="$emit('toggle')"
    >
      <Pause v-if="playing" :size="13" />
      <Play v-else :size="13" />
    </button>

    <div class="name-block">
      <span class="name">{{ voice.name }}</span>
      <span class="detail">{{ voice.detail }}</span>
    </div>

    <div class="wave" aria-hidden="true">
      <span
        v-for="(h, i) in voice.waveform"
        :key="i"
        :style="{
          height: `${h * 100}%`,
          background:
            playing && i / voice.waveform.length < (played ?? 0)
              ? 'var(--rv-accent)'
              : 'var(--rv-border-strong)',
        }"
      ></span>
    </div>

    <span class="time rv-tabular">
      <template v-if="playing">{{ elapsed }} / </template>{{ duration }}
    </span>

    <span v-if="selected" class="tick"><Check :size="11" /></span>
    <span v-else class="tick-space"></span>
  </div>
</template>

<style scoped>
.row {
  display: flex;
  align-items: center;
  gap: var(--rv-space-3);
  padding: 12px 14px;
  border-bottom: 1px solid var(--rv-border);
  cursor: pointer;
}

.row:last-child {
  border-bottom: none;
}

.row:hover:not(.selected) {
  background: var(--rv-bg-elevated);
}

.row.selected {
  background: var(--rv-accent-soft);
}

.play {
  width: 28px;
  height: 28px;
  flex: none;
  display: grid;
  place-items: center;
  border-radius: var(--rv-radius-pill);
  background: var(--rv-bg-elevated);
  color: var(--rv-text-muted);
  border: 1px solid var(--rv-border);
  cursor: pointer;
}

.play.active {
  background: var(--rv-accent);
  color: #fff;
  border-color: transparent;
}

.name-block {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 150px;
  flex: none;
}

.name {
  font-weight: 500;
}

.detail {
  font-size: 11px;
  color: var(--rv-text-faint);
}

.wave {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 2px;
  height: 26px;
  min-width: 0;
}

.wave span {
  flex: 1;
  border-radius: 2px;
}

.time {
  font-size: 11px;
  color: var(--rv-text-faint);
  flex: none;
}

.tick,
.tick-space {
  width: 16px;
  height: 16px;
  flex: none;
}

.tick {
  display: grid;
  place-items: center;
  border-radius: var(--rv-radius-pill);
  background: var(--rv-accent);
  color: #fff;
}
</style>
