<script setup lang="ts">
import { computed } from 'vue'
import MainWindow from './views/MainWindow.vue'
import RestPopup from './components/RestPopup.vue'
import WaterCard from './components/WaterCard.vue'

// 通过 URL hash 区分主窗口 / 休息提醒窗口 / 喝水提醒窗口
const view = computed(() => {
  const h = location.hash.replace(/^#/, '') || '/main'
  if (h.startsWith('/rest')) return RestPopup
  if (h.startsWith('/water')) return WaterCard
  return MainWindow
})
</script>

<template>
  <component :is="view" />
</template>
