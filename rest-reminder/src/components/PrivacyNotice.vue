<script setup lang="ts">
// 隐私说明（设置页常驻 + 首次启动引导复用）
</script>

<template>
  <div class="privacy">
    <p>我们以「隐私优先」设计本应用：</p>
    <ul>
      <li>仅监听键盘/鼠标的<strong>活动事件</strong>（用于判断您是否在使用电脑），<strong>不记录任何按键内容、不截屏、不录音</strong>。</li>
      <li>所有数据（设置、工作时长、喝水记录）仅保存在您本机 <code>%LOCALAPPDATA%/rest-reminder/</code>，<strong>不会上传到任何服务器</strong>。</li>
      <li>您可以随时在设置中「清空今日数据」，或手动删除本地数据文件。</li>
    </ul>
  </div>
</template>

<style scoped>
.privacy {
  font-size: 13px;
  color: var(--muted);
  line-height: 1.7;
}
.privacy ul {
  padding-left: 18px;
  margin-top: 6px;
}
.privacy code {
  background: var(--bg);
  padding: 1px 5px;
  border-radius: 4px;
  color: var(--text);
}
</style>
