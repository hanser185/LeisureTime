<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useAppStore } from '../stores/appStore'

const store = useAppStore()
const params = new URLSearchParams(location.hash.split('?')[1] || '')
const workedMin = Number(params.get('min') || '60')
const remain = ref(20)
let timer: number

async function close(auto: boolean) {
  window.clearInterval(timer)
  if (!auto) await store.skip()
  await getCurrentWindow().close()
}

onMounted(() => {
  timer = window.setInterval(() => {
    remain.value -= 1
    if (remain.value <= 0) close(true)
  }, 1000)
})
</script>

<template>
  <div class="rest">
    <div class="emoji">🌿</div>
    <h2>该休息一下啦</h2>
    <p>你已经连续工作了 <b>{{ workedMin }}</b> 分钟，起来走走、看看远处吧～</p>
    <div class="cd">{{ remain }} 秒后自动关闭</div>
    <div class="btns">
      <button class="btn-ghost" @click="store.snooze(5).then(() => getCurrentWindow().close())">
        稍后 5 分钟
      </button>
      <button class="btn-ghost" @click="store.snooze(10).then(() => getCurrentWindow().close())">
        稍后 10 分钟
      </button>
      <button class="btn-primary" @click="close(false)">跳过本次</button>
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
</style>
