<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useAppStore } from '../stores/appStore'

const store = useAppStore()

// 关闭弹窗：先尽力上报状态（失败不影响关闭），再关闭窗口；close 失败则强制 destroy 兜底
async function closeWindow() {
  try {
    await getCurrentWindow().close()
  } catch (e) {
    console.error('[WaterCard] close 失败，尝试 destroy', e)
    try {
      await getCurrentWindow().destroy()
    } catch (e2) {
      console.error('[WaterCard] destroy 也失败', e2)
    }
  }
}

async function drank() {
  try {
    await store.recordWater()
  } catch (e) {
    console.warn('[WaterCard] 记录喝水失败，仍关闭弹窗', e)
  }
  await closeWindow()
}

async function later() {
  // 先推迟下次喝水提醒（避免关闭后 1 秒内立即重复弹出），再关闭窗口
  try {
    await invoke('defer_water')
  } catch (e) {
    console.warn('[WaterCard] 推迟喝水提醒失败', e)
  }
  await closeWindow()
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
