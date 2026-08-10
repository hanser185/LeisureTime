<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useAppStore } from '../stores/appStore'

const store = useAppStore()

async function drank() {
  await store.recordWater()
  await getCurrentWindow().close()
}
async function later() {
  // 关闭即可，调度器会在下一次间隔到达时再次提醒
  await getCurrentWindow().close()
}
</script>

<template>
  <div class="water">
    <div class="emoji">💧</div>
    <h2>该喝水啦</h2>
    <p>起身接杯水，给身体补点水分～</p>
    <div class="btns">
      <button class="btn-primary" @click="drank">已喝水</button>
      <button class="btn-ghost" @click="later">稍后</button>
    </div>
  </div>
</template>

<style scoped>
.water {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 16px;
  text-align: center;
  background: linear-gradient(135deg, #eff6ff, #ecfeff);
  color: #0c4a6e;
}
[data-theme='dark'] .water {
  background: linear-gradient(135deg, #0f172a, #0c4a6e);
  color: #bae6fd;
}
.emoji {
  font-size: 34px;
}
h2 {
  font-size: 18px;
}
p {
  font-size: 13px;
  opacity: 0.85;
}
.btns {
  display: flex;
  gap: 8px;
  margin-top: 6px;
}
.btn-primary {
  background: #38bdf8;
  color: #042;
}
</style>
