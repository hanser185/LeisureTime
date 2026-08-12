<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useAppStore } from '../stores/appStore'
import Dashboard from '../components/Dashboard.vue'
import SettingsPanel from '../components/SettingsPanel.vue'
import PrivacyNotice from '../components/PrivacyNotice.vue'

const store = useAppStore()
const showPrivacy = ref(false)

onMounted(async () => {
  await store.init()
  if (!store.privacyAck) showPrivacy.value = true
})
</script>

<template>
  <div class="main">
    <header class="hd">
      <div class="title">
        <span class="logo">🌿</span>
        <span>休息提醒助手</span>
      </div>
      <div class="win-ctrl">
        <button class="icon" title="暂停/恢复" @click="store.togglePause()">
          {{ store.status.paused ? '▶' : '⏸' }}
        </button>
      </div>
    </header>

    <nav class="tabs">
      <button :class="{ on: store.activeTab === 'today' }" @click="store.activeTab = 'today'">
        今日
      </button>
      <button :class="{ on: store.activeTab === 'settings' }" @click="store.activeTab = 'settings'">
        设置
      </button>
    </nav>

    <main class="body">
      <Dashboard v-if="store.activeTab === 'today'" />
      <SettingsPanel v-else />
    </main>

    <!-- 首次启动隐私引导 -->
    <div v-if="showPrivacy" class="modal-mask">
      <div class="modal card">
        <h3>欢迎使用 · 隐私须知</h3>
        <PrivacyNotice />
        <button class="btn-primary" @click="(store.ackPrivacy(), (showPrivacy = false))">
          我已知晓，开始使用
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.main {
  height: 100%;
  display: flex;
  flex-direction: column;
}
.hd {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
}
.title {
  font-weight: 700;
  font-size: 16px;
  display: flex;
  align-items: center;
  gap: 8px;
}
.logo {
  font-size: 20px;
}
.win-ctrl .icon {
  background: transparent;
  color: var(--muted);
  font-size: 16px;
  padding: 4px 8px;
}
.win-ctrl .icon:hover {
  color: var(--text);
}
.tabs {
  display: flex;
  gap: 8px;
  padding: 10px 16px 0;
}
.tabs button {
  background: transparent;
  color: var(--muted);
  border-bottom: 2px solid transparent;
  border-radius: 0;
}
.tabs button.on {
  color: var(--accent-d);
  border-bottom-color: var(--accent);
  font-weight: 600;
}
.body {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}
.modal-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
}
.modal {
  max-width: 380px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.modal h3 {
  color: var(--accent-d);
}
</style>
