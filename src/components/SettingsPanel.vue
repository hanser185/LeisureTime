<script setup lang="ts">
import { ref } from 'vue'
import { useAppStore } from '../stores/appStore'
import PrivacyNotice from './PrivacyNotice.vue'
import type { Settings } from '../types'

const store = useAppStore()
const form = ref<Settings>({ ...(store.settings as Settings) })
const showPath = ref(false)
const path = ref('')

async function save() {
  await store.saveSettings({ ...form.value })
}
function resetDefault() {
  form.value = {
    work_threshold_min: 60,
    rest_threshold_min: 10,
    reminder_mode: 'toast',
    water_enabled: true,
    water_interval_min: 60,
    work_hours_only: false,
    work_start: '09:00',
    work_end: '18:00',
    paused: false,
    autostart: false,
    theme: 'system',
  }
}
async function clearToday() {
  if (confirm('确定清空今日所有工作/休息/喝水记录？')) {
    await store.clearToday()
  }
}
async function showDataPath() {
  path.value = await store.dataPath()
  showPath.value = true
}
</script>

<template>
  <div class="set">
    <div class="card">
      <div class="cap">检测与提醒</div>
      <label class="item">
        <span>工作时长阈值（分钟）</span>
        <input type="number" min="1" v-model.number="form.work_threshold_min" />
      </label>
      <label class="item">
        <span>休息判定时长（分钟）</span>
        <input type="number" min="1" v-model.number="form.rest_threshold_min" />
      </label>
      <label class="item">
        <span>提醒方式</span>
        <select v-model="form.reminder_mode">
          <option value="toast">系统通知（Toast）</option>
          <option value="popup">弹窗</option>
          <option value="fullscreen">全屏遮罩（首版回退弹窗）</option>
        </select>
      </label>
    </div>

    <div class="card">
      <div class="cap">喝水提醒</div>
      <label class="item">
        <span>启用喝水提醒</span>
        <input type="checkbox" v-model="form.water_enabled" />
      </label>
      <label class="item" v-if="form.water_enabled">
        <span>喝水间隔（分钟）</span>
        <input type="number" min="1" v-model.number="form.water_interval_min" />
      </label>
    </div>

    <div class="card">
      <div class="cap">工作时段</div>
      <label class="item">
        <span>仅工作时段提醒（工作日 9–18）</span>
        <input type="checkbox" v-model="form.work_hours_only" />
      </label>
      <div class="time" v-if="form.work_hours_only">
        <input type="time" v-model="form.work_start" />
        <span>至</span>
        <input type="time" v-model="form.work_end" />
      </div>
    </div>

    <div class="card">
      <div class="cap">通用</div>
      <label class="item">
        <span>开机自启动</span>
        <input type="checkbox" v-model="form.autostart" />
      </label>
      <label class="item">
        <span>主题外观</span>
        <select v-model="form.theme">
          <option value="system">跟随系统</option>
          <option value="light">浅色</option>
          <option value="dark">深色</option>
        </select>
      </label>
    </div>

    <div class="card">
      <div class="cap">数据与隐私</div>
      <PrivacyNotice />
      <div class="acts">
        <button class="btn-ghost" @click="clearToday">清空今日数据</button>
        <button class="btn-ghost" @click="showDataPath">查看数据文件位置</button>
        <button class="btn-ghost" @click="resetDefault">恢复默认设置</button>
      </div>
      <div v-if="showPath" class="path">{{ path }}</div>
    </div>

    <button class="btn-primary save" @click="save">保存设置</button>
  </div>
</template>

<style scoped>
.set {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.cap {
  font-weight: 600;
  margin-bottom: 8px;
}
.item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 0;
  font-size: 14px;
  gap: 10px;
}
.item input[type='number'],
.item select,
.time input {
  padding: 6px 8px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--text);
}
.time {
  display: flex;
  gap: 8px;
  align-items: center;
  padding: 6px 0;
  color: var(--muted);
  font-size: 13px;
}
.acts {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 10px;
}
.path {
  margin-top: 8px;
  font-size: 12px;
  color: var(--muted);
  word-break: break-all;
  background: var(--bg);
  padding: 8px;
  border-radius: 8px;
}
.save {
  margin-top: 4px;
}
</style>
