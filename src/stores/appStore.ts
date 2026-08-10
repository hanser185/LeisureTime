import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { DailyData, Settings, Status, WeekDay } from '../types'

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
      this.settings = await invoke('get_settings')
      this.applyTheme()
      this.daily = await invoke('get_daily')
      this.weekly = await invoke('get_weekly')
      this.status = await invoke('get_status')
      listen('navigate', (e: { payload: unknown }) => {
        if (e.payload === 'settings') this.activeTab = 'settings'
      })
      // 状态每秒轮询；数据每 5 秒轮询
      setInterval(async () => {
        this.status = await invoke('get_status')
      }, 1000)
      setInterval(async () => {
        this.daily = await invoke('get_daily')
        this.weekly = await invoke('get_weekly')
      }, 5000)
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
      const t = this.settings?.theme ?? 'system'
      document.documentElement.dataset.theme = t
    },
  },
})
