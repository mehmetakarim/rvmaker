<script setup lang="ts">
import { ref } from "vue";
import { ChevronDown, ChevronRight, Copy } from "@lucide/vue";
import type { LogLine } from "@/lib/types";

const props = defineProps<{ lines: LogLine[]; totalLines: number }>();

const open = ref(true);

async function copyLog() {
  const text = props.lines.map((l) => `${l.time}  ${l.text}`).join("\n");
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    /* pano erişimi yoksa sessizce geç */
  }
}
</script>

<template>
  <div class="console" data-component="LogConsole">
    <button type="button" class="head" @click="open = !open">
      <ChevronDown v-if="open" :size="14" />
      <ChevronRight v-else :size="14" />
      Ayrıntılı günlük
      <span class="count rv-mono">{{ totalLines }} satır</span>
      <span class="copy" role="button" title="Günlüğü kopyala" @click.stop="copyLog">
        <Copy :size="14" />
      </span>
    </button>

    <div v-if="open" class="body rv-mono">
      <div v-for="(line, i) in lines" :key="i" class="line" :class="line.level">
        <span class="time">{{ line.time }}</span>
        {{ line.text }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.console {
  flex: none;
  border-top: 1px solid var(--rv-border);
  background: var(--rv-bg-surface);
}

.head {
  display: flex;
  align-items: center;
  gap: var(--rv-space-2);
  width: 100%;
  height: 36px;
  padding: 0 var(--rv-space-4);
  background: none;
  border: none;
  color: var(--rv-text-muted);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}

.count {
  margin-left: auto;
  font-size: 11px;
  color: var(--rv-text-faint);
}

.copy {
  color: var(--rv-text-faint);
  display: grid;
  place-items: center;
}

.copy:hover {
  color: var(--rv-text);
}

.body {
  height: 158px;
  overflow: auto;
  padding: 10px var(--rv-space-4) 14px;
  background: var(--rv-bg-inset);
  font-size: 11.5px;
  line-height: 1.75;
  color: var(--rv-text-muted);
}

.time {
  color: var(--rv-text-faint);
  margin-right: var(--rv-space-2);
}

.line.error {
  color: var(--rv-danger);
}

.line.warn {
  color: var(--rv-warning);
}
</style>
