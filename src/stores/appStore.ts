import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { DailyData, Settings, Status, WeekDay } from '../types'

// ponytail: 模块级句柄——init 幂等且可彻底回收，避免 HMR/重复挂载叠加轮询与事件监听
let navOff: (() => void) | null = null
let timers: number[] = []
let initialized = false
let initSeq = 0 // 代际令牌：dispose 时自增，使在途的异步 init 失效，杜绝重复注册

export const useAppStore = defineStore('app', {
  state: () => ({
    settings: null as Settings | null,
    daily: null as DailyData | null,
    status: { state: 'idle', current_segment_sec: 0, paused: false, snooze_until_ms: 0 } as Status,
    weekly: [] as WeekDay[],
    activeTab: 'today' as 'today' | 'settings',
    privacyAck: typeof localStorage !== 'undefined' && localStorage.getItem('privacy_ack') === '1',
  }),
  getters: {
    workTotalSec: (s) => (s.daily?.work_segments ?? []).reduce((a, b) => a + b.duration_sec, 0),
    restTotalSec: (s) => (s.daily?.rest_segments ?? []).reduce((a, b) => a + b.duration_sec, 0),
    maxWorkSec: (s) => (s.daily?.work_segments ?? []).reduce((a, b) => Math.max(a, b.duration_sec), 0),
    waterCount: (s) => s.daily?.water_intakes.length ?? 0,
  },
  actions: {
    async init() {
      if (initialized) return
      const seq = ++initSeq
      initialized = true
      this.settings = await invoke('get_settings')
      this.applyTheme()
      this.daily = await invoke('get_daily')
      this.weekly = await invoke('get_weekly')
      this.status = await invoke('get_status')
      // 异步等待期间若发生 dispose（HMR 卸载），在途 init 应作废，避免重复注册
      if (seq !== initSeq) return
      navOff = await listen('navigate', (e: { payload: unknown }) => {
        if (e.payload === 'settings') this.activeTab = 'settings'
      })
      if (seq !== initSeq) {
        navOff?.()
        navOff = null
        return
      }
      // 状态 1s 轮询；当日数据 5s；周报变动极少，60s 足以，避免每 5s 读 7 个文件
      timers.push(
        window.setInterval(async () => {
          this.status = await invoke('get_status')
        }, 1000),
        window.setInterval(async () => {
          this.daily = await invoke('get_daily')
        }, 5000),
        window.setInterval(async () => {
          this.weekly = await invoke('get_weekly')
        }, 60_000),
      )
    },
    dispose() {
      initialized = false
      initSeq++ // 让任何进行中的 init 失效
      navOff?.()
      navOff = null
      timers.forEach((t) => window.clearInterval(t))
      timers = []
    },
    async saveSettings(s: Settings) {
      await invoke('save_settings', { settings: s })
      this.settings = s
      this.applyTheme()
    },
    async togglePause() {
      const p = (await invoke('toggle_pause')) as boolean
      if (this.settings) this.settings.paused = p
      this.status = await invoke('get_status')
    },
    async snooze(min: number) {
      await invoke('snooze_rest', { minutes: min })
    },
    async skip() {
      await invoke('skip_rest')
    },
    async recordWater() {
      await invoke('record_water')
      await this.refresh()
    },
    async clearToday() {
      await invoke('clear_today')
      await this.refresh()
    },
    async openDataFolder() {
      await invoke('open_data_folder')
    },
    async dataPath(): Promise<string> {
      return await invoke('data_path')
    },
    async refresh() {
      this.daily = await invoke('get_daily')
      this.weekly = await invoke('get_weekly')
      this.status = await invoke('get_status')
    },
    ackPrivacy() {
      this.privacyAck = true
      try {
        localStorage.setItem('privacy_ack', '1')
      } catch {
        /* ignore */
      }
    },
    applyTheme() {
      let t = this.settings?.theme ?? 'system'
      if (t === 'system' && typeof window !== 'undefined' && window.matchMedia) {
        t = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
      }
      document.documentElement.dataset.theme = t
    },
  },
})
