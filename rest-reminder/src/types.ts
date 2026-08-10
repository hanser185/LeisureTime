export interface Settings {
  work_threshold_min: number
  rest_threshold_min: number
  reminder_mode: 'toast' | 'popup' | 'fullscreen'
  water_enabled: boolean
  water_interval_min: number
  work_hours_only: boolean
  work_start: string
  work_end: string
  paused: boolean
  autostart: boolean
  theme: 'light' | 'dark' | 'system'
}

export interface Segment {
  start_ms: number
  end_ms: number
  duration_sec: number
}

export interface DailyData {
  date: string
  work_segments: Segment[]
  rest_segments: Segment[]
  water_intakes: string[]
  rest_count: number
  rest_reminders: number
  last_activity_ms: number
  last_water_prompt_ms: number
}

export interface Status {
  state: 'idle' | 'working' | 'resting'
  current_segment_sec: number
  paused: boolean
  snooze_until_ms: number
}

export interface WeekDay {
  date: string
  work_min: number
  rest_min: number
  water: number
}
