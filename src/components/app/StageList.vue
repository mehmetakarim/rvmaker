<script setup lang="ts">
import { Check, X, Minus } from "@lucide/vue";
import type { Stage } from "@/lib/types";

defineProps<{ stages: Stage[] }>();
</script>

<template>
  <div class="stages rv-scroll" data-component="StageList">
    <div class="label">Aşamalar</div>

    <div
      v-for="stage in stages"
      :key="stage.id"
      class="stage"
      :class="stage.state"
    >
      <span class="marker">
        <Check v-if="stage.state === 'done'" :size="12" />
        <X v-else-if="stage.state === 'failed'" :size="12" />
        <Minus v-else-if="stage.state === 'blocked'" :size="12" />
      </span>

      <div class="info">
        <div class="line">
          <span class="name">{{ stage.label }}</span>
          <span v-if="stage.state === 'active'" class="percent rv-tabular">
            %{{ Math.round((stage.progress ?? 0) * 100) }}
          </span>
        </div>

        <div v-if="stage.state === 'active'" class="bar">
          <div class="fill" :style="{ width: `${(stage.progress ?? 0) * 100}%` }"></div>
        </div>

        <span v-if="stage.detail && stage.state !== 'done'" class="detail">{{ stage.detail }}</span>
      </div>

      <span v-if="stage.state === 'done'" class="elapsed rv-tabular">{{ stage.detail }}</span>
    </div>

    <div class="footer"><slot name="footer" /></div>
  </div>
</template>

<style scoped>
.stages {
  width: 340px;
  flex: none;
  border-left: 1px solid var(--rv-border);
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.label {
  font-size: 11px;
  font-weight: 500;
  color: var(--rv-text-faint);
  letter-spacing: 0.02em;
  text-transform: uppercase;
  padding-bottom: 10px;
}

.stage {
  display: flex;
  gap: 10px;
  padding: 10px var(--rv-space-2);
  border: 1px solid transparent;
  border-radius: var(--rv-radius-sm);
}

.stage.pending {
  opacity: 0.5;
}

/* Bağlanmamış aşama: soluk ama "bekliyor" gibi durmuyor —
   çizgili işaret ve kesik kenarlık bunun henüz var olmadığını söylüyor. */
.stage.blocked {
  opacity: 0.6;
}

.stage.blocked .marker {
  border-style: dashed;
  color: var(--rv-text-faint);
}

.stage.blocked .name {
  color: var(--rv-text-faint);
}

.stage.active {
  padding: 12px 10px;
  background: var(--rv-accent-soft);
  border-color: color-mix(in srgb, var(--rv-accent) 30%, transparent);
}

.stage.failed {
  padding: 12px 10px;
  background: var(--rv-danger-soft);
  border-color: color-mix(in srgb, var(--rv-danger) 30%, transparent);
}

.marker {
  width: 18px;
  height: 18px;
  flex: none;
  display: grid;
  place-items: center;
  border-radius: var(--rv-radius-pill);
  border: 1px solid var(--rv-border-strong);
}

.stage.done .marker {
  background: var(--rv-success-soft);
  color: var(--rv-success);
  border-color: transparent;
}

.stage.active .marker {
  border: 2px solid var(--rv-accent);
  border-top-color: transparent;
  animation: rv-spin 1.2s linear infinite;
}

.stage.failed .marker {
  background: var(--rv-danger);
  color: #fff;
  border-color: transparent;
}

.info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--rv-space-2);
}

.line {
  display: flex;
  align-items: baseline;
}

.name {
  color: var(--rv-text-muted);
}

.stage.active .name,
.stage.failed .name {
  color: var(--rv-text);
  font-weight: 500;
}

.percent {
  margin-left: auto;
  font-size: 11px;
  color: var(--rv-accent-quiet);
}

.bar {
  height: 4px;
  border-radius: var(--rv-radius-pill);
  background: rgba(0, 0, 0, 0.28);
  overflow: hidden;
}

.fill {
  height: 100%;
  background: var(--rv-accent);
}

.detail {
  font-size: 11px;
  color: var(--rv-text-muted);
}

.elapsed {
  font-size: 11px;
  color: var(--rv-text-faint);
}

.footer {
  margin-top: auto;
}
</style>
