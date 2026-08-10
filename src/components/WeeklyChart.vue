<script setup lang="ts">
import { computed } from 'vue'
import type { WeekDay } from '../types'

const props = defineProps<{ weekly: WeekDay[] }>()

const max = computed(() => Math.max(30, ...props.weekly.map((w) => w.work_min)))
const bars = computed(() =>
  [...props.weekly].reverse().map((w) => ({
    label: w.date.slice(5),
    min: w.work_min,
    height: Math.round((w.work_min / max.value) * 100),
  })),
)
</script>

<template>
  <div class="chart">
    <div v-for="(b, i) in bars" :key="i" class="col">
      <div class="bar" :style="{ height: b.height + '%' }" :title="b.min + ' 分钟'"></div>
      <div class="lab">{{ b.label }}</div>
    </div>
  </div>
</template>

<style scoped>
.chart {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  height: 110px;
  gap: 6px;
}
.col {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  height: 100%;
  justify-content: flex-end;
}
.bar {
  width: 70%;
  background: linear-gradient(var(--accent), var(--accent-d));
  border-radius: 6px 6px 0 0;
  min-height: 2px;
  transition: height 0.3s;
}
.lab {
  font-size: 10px;
  color: var(--muted);
  margin-top: 4px;
}
</style>
