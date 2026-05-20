<script setup lang="ts">
import { ref, onMounted, provide } from "vue";
import type { LarkConfig, TabType } from "./types/transfer";
import SettingsPanel from "./components/SettingsPanel.vue";
import SendPanel from "./components/SendPanel.vue";
import ReceivePanel from "./components/ReceivePanel.vue";
import SpeedTestPanel from "./components/SpeedTestPanel.vue";
import { useLang } from "./i18n";

const { lang, toggleLang, t } = useLang();
provide("lang", lang);
provide("t", t);

const DEFAULT_LARK_CONFIG: LarkConfig = {
  appId: "cli_aa871fad79f8de15",
  appSecret: "FMVDzOY6TVErA94tzzuFHeDlnigRui72",
};

const activeTab = ref<TabType>("send");
const showSettings = ref(false);
const larkConfig = ref<LarkConfig>({ ...DEFAULT_LARK_CONFIG });
const isDark = ref(false);

function hasLarkConfig(config: LarkConfig): boolean {
  return Boolean(config.appId?.trim() && config.appSecret?.trim());
}

function toggleTheme() {
  isDark.value = !isDark.value;
  document.documentElement.setAttribute("data-theme", isDark.value ? "dark" : "light");
  localStorage.setItem("ft-theme", isDark.value ? "dark" : "light");
}

function saveSettings(config: LarkConfig) {
  larkConfig.value = { ...config };
  localStorage.setItem("ft-lark-config", JSON.stringify(config));
}

onMounted(() => {
  const savedTheme = localStorage.getItem("ft-theme");
  if (savedTheme === "dark") {
    isDark.value = true;
    document.documentElement.setAttribute("data-theme", "dark");
  } else {
    document.documentElement.setAttribute("data-theme", "light");
  }

  const savedConfig = localStorage.getItem("ft-lark-config");
  if (savedConfig) {
    try {
      larkConfig.value = { ...DEFAULT_LARK_CONFIG, ...JSON.parse(savedConfig) };
    } catch {
      larkConfig.value = { ...DEFAULT_LARK_CONFIG };
    }
  }
});
</script>

<template>
  <div class="app-wrapper">
    <div class="app-container">
      <header class="app-header">
        <div class="header-left">
          <h1 class="app-title">
            <span class="title-icon"><svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="17 8 12 3 7 8" /><line x1="12" y1="3" x2="12" y2="15" /></svg></span>
            {{ t.appTitle }}
          </h1>
        </div>
        <div class="header-actions">
          <button class="lang-btn" @click="toggleLang" :title="lang === 'zh' ? 'English' : '中文'">{{ lang === 'zh' ? 'EN' : '中' }}</button>
          <button class="icon-btn" @click="showSettings = true" :title="t.settings"><svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" /></svg></button>
          <button class="icon-btn" @click="toggleTheme" :title="isDark ? t.lightMode : t.darkMode"><svg v-if="isDark" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="5" /><line x1="12" y1="1" x2="12" y2="3" /><line x1="12" y1="21" x2="12" y2="23" /><line x1="4.22" y1="4.22" x2="5.64" y2="5.64" /><line x1="18.36" y1="18.36" x2="19.78" y2="19.78" /><line x1="1" y1="12" x2="3" y2="12" /><line x1="21" y1="12" x2="23" y2="12" /><line x1="4.22" y1="19.78" x2="5.64" y2="18.36" /></svg><svg v-else width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" /></svg></button>
        </div>
      </header>

      <div class="token-warning" v-if="!hasLarkConfig(larkConfig)" @click="showSettings = true">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" /><line x1="12" y1="9" x2="12" y2="13" /><line x1="12" y1="17" x2="12.01" y2="17" /></svg>
        {{ t.configWarning }}
      </div>

      <div class="tab-bar">
        <button class="tab-btn" :class="{ active: activeTab === 'send' }" @click="activeTab = 'send'"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="17 8 12 3 7 8" /><line x1="12" y1="3" x2="12" y2="15" /></svg>{{ t.send }}</button>
        <button class="tab-btn" :class="{ active: activeTab === 'receive' }" @click="activeTab = 'receive'"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="7 10 12 15 17 10" /><line x1="12" y1="15" x2="12" y2="3" /></svg>{{ t.receive }}</button>
        <button class="tab-btn" :class="{ active: activeTab === 'speed' }" @click="activeTab = 'speed'"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 12a9 9 0 1 0 18 0" /><path d="M12 12l4-4" /></svg>{{ t.speedTest }}</button>
      </div>

      <main class="app-main">
        <Transition name="fade" mode="out-in">
          <SendPanel v-if="activeTab === 'send'" :larkConfig="larkConfig" :key="'send'" />
          <ReceivePanel v-else-if="activeTab === 'receive'" :larkConfig="larkConfig" :key="'receive'" />
          <SpeedTestPanel v-else :larkConfig="larkConfig" :key="'speed'" />
        </Transition>
      </main>
    </div>
    <SettingsPanel :visible="showSettings" :config="larkConfig" @close="showSettings = false" @save="saveSettings" />
  </div>
</template>

<style scoped>
.app-wrapper {
  min-height: 100vh;
  display: flex;
  justify-content: center;
  align-items: flex-start;
  padding: 20px;
  background: var(--bg);
  transition: background 0.3s;
}
.app-container {
  width: 100%;
  max-width: 500px;
  background: var(--card-bg);
  border-radius: 16px;
  box-shadow: 0 4px 24px var(--shadow);
  overflow: hidden;
  transition: background 0.3s, box-shadow 0.3s;
}
.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 20px;
  border-bottom: 1px solid var(--border);
}
.header-left {
  display: flex;
  align-items: center;
  gap: 4px;
}
.app-title {
  font-size: 18px;
  font-weight: 700;
  color: var(--text);
  margin: 0;
  display: flex;
  align-items: center;
  gap: 8px;
}
.title-icon {
  display: flex;
  color: var(--primary);
}
.header-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}
.lang-btn {
  background: none;
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 4px 8px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s;
}
.lang-btn:hover {
  border-color: var(--primary);
  color: var(--primary);
}
.icon-btn {
  background: none;
  border: none;
  border-radius: 8px;
  padding: 6px;
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  transition: all 0.2s;
}
.icon-btn:hover {
  background: var(--hover);
  color: var(--text);
}
.token-warning {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 20px;
  font-size: 13px;
  color: #d97706;
  background: #fffbeb;
  cursor: pointer;
  transition: background 0.2s;
}
.token-warning:hover {
  background: #fef3c7;
}
.tab-bar {
  display: flex;
  gap: 4px;
  padding: 8px 12px;
  background: var(--bg);
  border-bottom: 1px solid var(--border);
}
.tab-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 8px 12px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}
.tab-btn:hover {
  background: var(--hover);
  color: var(--text);
}
.tab-btn.active {
  background: var(--primary);
  color: #fff;
}
.app-main {
  padding: 16px 20px 20px;
}
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.15s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
