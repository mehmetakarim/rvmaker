<script setup lang="ts">
import { ref } from "vue";

withDefaults(defineProps<{ text: string; placement?: "top" | "bottom" }>(), {
  placement: "top",
});

const shown = ref(false);
let timer: number | null = null;

/** Kısa gecikme, imleç gezinirken ipuçlarının parlamasını önlüyor. */
function show() {
  timer = window.setTimeout(() => (shown.value = true), 350);
}

function hide() {
  if (timer !== null) window.clearTimeout(timer);
  timer = null;
  shown.value = false;
}
</script>

<template>
  <span
    class="anchor"
    @mouseenter="show"
    @mouseleave="hide"
    @focusin="shown = true"
    @focusout="hide"
  >
    <slot />
    <span v-if="shown" class="bubble" :class="placement" role="tooltip">{{ text }}</span>
  </span>
</template>

<style scoped>
.anchor {
  position: relative;
  display: inline-flex;
}

.bubble {
  position: absolute;
  left: 50%;
  transform: translateX(-50%);
  padding: 5px 9px;
  border-radius: var(--rv-radius-sm);
  background: #000;
  color: #fff;
  font-size: 11px;
  line-height: 1.35;
  white-space: nowrap;
  box-shadow: var(--rv-shadow-1);
  pointer-events: none;
  z-index: 40;
}

.bubble.top {
  bottom: calc(100% + 6px);
}

.bubble.bottom {
  top: calc(100% + 6px);
}
</style>
