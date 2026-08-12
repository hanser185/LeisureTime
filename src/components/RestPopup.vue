<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useAppStore } from '../stores/appStore'

const store = useAppStore()
const params = new URLSearchParams(location.hash.split('?')[1] || '')
const workedMin = Number(params.get('min') || '60')
const isFull = params.get('mode') === 'fullscreen'
const remain = ref(20)
let timer: number

// 关闭弹窗：先尽力上报状态（失败不影响关闭），再关闭窗口；close 失败则强制 destroy 兜底
async function dismiss(auto: boolean, snoozeMin = 0) {
  window.clearInterval(timer)
  if (!auto) {
    try {
      if (snoozeMin > 0) await store.snooze(snoozeMin)
      else await store.skip()
    } catch (e) {
      console.warn('[RestPopup] 状态上报失败，仍关闭弹窗', e)
    }
  }
  try {
    await getCurrentWindow().close()
  } catch (e) {
    console.error('[RestPopup] close 失败，尝试 destroy', e)
    try {
      await getCurrentWindow().destroy()
    } catch (e2) {
      console.error('[RestPopup] destroy 也失败', e2)
    }
  }
}

onMounted(() => {
  timer = window.setInterval(() => {
    remain.value -= 1
    if (remain.value <= 0) dismiss(true)
  }, 1000)
})
</script>

<template>
  <div class="rest" :class="{ full: isFull }">
    <div class="emoji">🌿</div>
    <h2>该休息一下啦</h2>
    <p>你已经连续工作了 <b>{{ workedMin }}</b> 分钟，起来走走、看看远处吧～</p>
    <div class="cd">{{ remain }} 秒后自动关闭</div>
    <div class="btns">
      <button class="btn-ghost" @click="dismiss(false, 5)">稍后 5 分钟</button>
      <button class="btn-ghost" @click="dismiss(false, 10)">稍后 10 分钟</button>
      <button class="btn-primary" @click="dismiss(false, 0)">跳过本次</button>
    </div>
  </div>
</template>

<style scoped>
.rest {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 18px;
  text-align: center;
  background: linear-gradient(135deg, #ecfeff, #f0fdfa);
  color: #134e4a;
}
[data-theme='dark'] .rest {
  background: linear-gradient(135deg, #0f172a, #134e4a);
  color: #ccfbf1;
}
.emoji {
  font-size: 38px;
}
h2 {
  font-size: 20px;
}
p {
  font-size: 14px;
  color: inherit;
  opacity: 0.85;
}
.cd {
  font-size: 12px;
  opacity: 0.7;
}
.btns {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  justify-content: center;
  margin-top: 6px;
}
.btn-primary {
  background: var(--accent);
  color: #063;
}
/* 全屏遮罩模式：深色半透明蒙版覆盖整屏，居中放大面板，强制用户休息 */
.rest.full {
  background: rgba(15, 23, 42, 0.92);
  color: #e2e8f0;
  backdrop-filter: blur(6px);
}
[data-theme='dark'] .rest.full {
  background: rgba(2, 6, 23, 0.95);
}
.rest.full .emoji {
  font-size: 84px;
}
.rest.full h2 {
  font-size: 38px;
}
.rest.full p {
  font-size: 21px;
  opacity: 0.92;
}
.rest.full .cd {
  font-size: 18px;
  opacity: 0.8;
}
.rest.full .btns {
  margin-top: 20px;
}
.rest.full .btns button {
  font-size: 16px;
  padding: 11px 20px;
}
</style>
