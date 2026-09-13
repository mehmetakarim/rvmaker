<template>
  <div class="card"><slot /></div>
</template>

<style scoped>
.card {
  background: var(--rv-bg-surface);
  border: 1px solid var(--rv-border);
  border-radius: var(--rv-radius-md);
}
</style>
