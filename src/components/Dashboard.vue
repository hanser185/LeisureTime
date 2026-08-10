<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useAppStore } from '../stores/appStore'
import WeeklyChart from './WeeklyChart.vue'

const store = useAppStore()
const nowMs = ref(Date.now())
let timer: number

onMounted(() => {
  timer = window.setInterval(() => (nowMs.value = Date.now()), 1000)
})
onUnmounted(() => window.clearInterval(timer))

function fmt(sec: number): string {
  const h = Math.floor(sec / 3600)
  const m = Math.floor((sec % 3600) / 60)
  if (h > 0) return `${h}小时${m}分`
  if (m > 0) return `${m}分`
  return `${sec}秒`
}

const stateText = computed(() => {
  if (store.status.paused) return '已暂停'
  switch (store.status.state) {
    case 'working':
      return '工作中'
    case 'resting':
      return '休息中'
    default:
      return '未开始'
  }
})
const stateCls = computed(() => {
  if (store.status.paused) return 'paused'
  return store.status.state
})

// 当前片段已连续时长
const curSec = computed(() => store.status.current_segment_sec)
// 距下次喝水倒计时
const waterRemainMs = computed(() => {
  const d = store.daily
  const s = store.settings
  if (!d || !s || !s.water_enabled) return 0
  const interval = s.water_interval_min * 60000
  return Math.max(0, interval - (nowMs.value - d.last_water_prompt_ms))
})
function mmss(ms: number): string {
  const s = Math.floor(ms / 1000)
  return `${String(Math.floor(s / 60)).padStart(2, '0')}:${String(s % 60).padStart(2, '0')}`
}

// 时间轴：工作/休息片段按开始时间排序
const segments = computed(() => {
  const d = store.daily
  if (!d) return []
  const all = [
    ...d.work_segments.map((s) => ({ ...s, kind: 'work' })),
    ...d.rest_segments.map((s) => ({ ...s, kind: 'rest' })),
  ]
  return all.sort((a, b) => a.start_ms - b.start_ms)
})
</script>

<template>
  <div class="dash">
    <!-- 当前状态 -->
    <div class="card state" :class="stateCls">
      <div class="dot"></div>
      <div>
        <div class="big">{{ stateText }}</div>
        <div class="sub">
          <template v-if="stateCls === 'working'">已连续 {{ fmt(curSec) }}</template>
          <template v-else-if="stateCls === 'resting'">放松一下~</template>
          <template v-else>动动键盘/鼠标开始计时</template>
        </div>
      </div>
    </div>

    <!-- 指标卡 -->
    <div class="grid">
      <div class="card metric">
        <div class="num">{{ fmt(store.workTotalSec) }}</div>
        <div class="lbl">累计工作</div>
      </div>
      <div class="card metric">
        <div class="num">{{ store.daily?.rest_count ?? 0 }}</div>
        <div class="lbl">休息次数</div>
      </div>
      <div class="card metric">
        <div class="num">{{ fmt(store.restTotalSec) }}</div>
        <div class="lbl">累计休息</div>
      </div>
      <div class="card metric">
        <div class="num">{{ store.waterCount }}</div>
        <div class="lbl">喝水次数</div>
      </div>
    </div>

    <div class="card">
      <div class="row">
        <span class="lbl">最长连续工作</span>
        <span class="val">{{ fmt(store.maxWorkSec) }}</span>
      </div>
      <div class="row" v-if="store.settings?.water_enabled">
        <span class="lbl">距下次喝水</span>
        <span class="val">{{ mmss(waterRemainMs) }}</span>
      </div>
    </div>

    <!-- 今日时间轴 -->
    <div class="card">
      <div class="cap">今日时间轴</div>
      <div class="timeline">
        <span
          v-for="(s, i) in segments"
          :key="i"
          class="seg"
          :class="s.kind"
          :style="{ flexGrow: Math.max(1, s.duration_sec / 60) }"
          :title="(s.kind === 'work' ? '工作 ' : '休息 ') + fmt(s.duration_sec)"
        ></span>
        <span v-if="segments.length === 0" class="empty">今天还没有记录，开始工作吧～</span>
      </div>
    </div>

    <!-- 本周趋势 -->
    <div class="card">
      <div class="cap">本周趋势（每日工作时长）</div>
      <WeeklyChart :weekly="store.weekly" />
    </div>

    <!-- 今日活动明细 -->
    <div class="card">
      <div class="cap">今日活动</div>
      <div class="acts">
        <div v-for="(s, i) in segments" :key="'a' + i" class="act">
          <span class="tag" :class="s.kind">{{ s.kind === 'work' ? '工作' : '休息' }}</span>
          <span class="t">{{ new Date(s.start_ms).toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' }) }}</span>
          <span class="d">→ {{ fmt(s.duration_sec) }}</span>
        </div>
        <div v-if="segments.length === 0" class="empty">暂无活动</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.dash {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.state {
  display: flex;
  align-items: center;
  gap: 12px;
}
.state .dot {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--muted);
}
.state.working .dot {
  background: var(--work);
}
.state.resting .dot {
  background: var(--rest);
}
.state.paused .dot {
  background: var(--muted);
}
.big {
  font-size: 20px;
  font-weight: 700;
}
.sub {
  color: var(--muted);
  font-size: 13px;
}
.grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px;
}
.metric .num {
  font-size: 22px;
  font-weight: 700;
  color: var(--accent-d);
}
.metric .lbl {
  color: var(--muted);
  font-size: 13px;
  margin-top: 2px;
}
.row {
  display: flex;
  justify-content: space-between;
  padding: 4px 0;
}
.row .val {
  font-weight: 600;
}
.cap {
  font-size: 13px;
  color: var(--muted);
  margin-bottom: 8px;
}
.timeline {
  display: flex;
  gap: 3px;
  height: 26px;
  align-items: stretch;
}
.seg {
  border-radius: 4px;
  min-width: 4px;
}
.seg.work {
  background: var(--work);
}
.seg.rest {
  background: var(--rest);
}
.empty {
  color: var(--muted);
  font-size: 13px;
}
.acts {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.act {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}
.tag {
  font-size: 11px;
  padding: 1px 6px;
  border-radius: 4px;
  color: #fff;
}
.tag.work {
  background: var(--work);
}
.tag.rest {
  background: var(--rest);
}
.t {
  color: var(--muted);
}
.d {
  margin-left: auto;
}
</style>
