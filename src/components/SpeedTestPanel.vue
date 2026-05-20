<script setup lang="ts">
import { ref, inject, type ComputedRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { LarkConfig, LarkCredential, SpeedTestResult } from "../types/transfer";

const t = inject<ComputedRef<Record<string, string>>>("t")!;

const props = defineProps<{
  larkConfig: LarkConfig;
}>();

const SIZES = [1, 8, 16];
const testSize = ref(8);
const result = ref<SpeedTestResult | null>(null);
const error = ref("");
const isRunning = ref(false);

function hasCredential(config: LarkConfig): boolean {
  return Boolean(config.appId?.trim() && config.appSecret?.trim());
}

function toCredential(config: LarkConfig): LarkCredential {
  return {
    type: "app",
    appId: config.appId?.trim() || "",
    appSecret: config.appSecret?.trim() || "",
  };
}

async function startSpeedTest() {
  if (!hasCredential(props.larkConfig)) {
    error.value = t.value.configFirst;
    return;
  }
  error.value = "";
  result.value = null;
  isRunning.value = true;
  try {
    result.value = await invoke<SpeedTestResult>("run_speed_test", {
      credential: toCredential(props.larkConfig),
      testSizeMb: testSize.value,
    });
  } catch (e: any) {
    error.value = typeof e === "string" ? e : e.message || t.value.speedTestFailed;
  } finally {
    isRunning.value = false;
  }
}

function formatSpeed(bps: number): string {
  if (bps < 1024) return bps.toFixed(0) + " B/s";
  if (bps < 1048576) return (bps / 1024).toFixed(1) + " KB/s";
  return (bps / 1048576).toFixed(2) + " MB/s";
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1048576).toFixed(1)} MB`;
}
</script>

<template>
  <div class="speed-panel">
    <div class="section">
      <div class="test-box">
        <div class="test-icon"><svg width="38" height="38" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6"><path d="M3 12a9 9 0 1 0 18 0" /><path d="M12 12l5-5" /><path d="M8 21h8" /></svg></div>
        <p class="test-title">{{ t.speedTestTitle }}</p>
        <p class="test-subtitle">{{ t.speedTestSubtitle }}</p>
      </div>
      <div class="size-selector">
        <button v-for="s in SIZES" :key="s" class="size-btn" :class="{ active: testSize === s }" :disabled="isRunning" @click="testSize = s">{{ s }}MB</button>
      </div>
      <button class="btn btn-primary btn-full" :disabled="isRunning" @click="startSpeedTest"><span v-if="isRunning" class="spinner"></span><svg v-else width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="5 3 19 12 5 21 5 3" /></svg>{{ isRunning ? t.speedTesting : t.startSpeedTest }}</button>
    </div>
    <div class="result-card" v-if="result">
      <div class="result-header"><span>{{ t.speedTestResult }}</span><strong>{{ formatSize(result.file_size) }}</strong></div>
      <div class="metric-grid">
        <div class="metric"><span class="metric-label">{{ t.uploadSpeed }}</span><strong>{{ formatSpeed(result.upload_bps) }}</strong><small>{{ result.upload_secs.toFixed(2) }}s</small></div>
        <div class="metric"><span class="metric-label">{{ t.downloadSpeed }}</span><strong>{{ formatSpeed(result.download_bps) }}</strong><small>{{ result.download_secs.toFixed(2) }}s</small></div>
      </div>
    </div>
    <div class="error-box" v-if="error"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10" /><line x1="15" y1="9" x2="9" y2="15" /><line x1="9" y1="9" x2="15" y2="15" /></svg>{{ error }}</div>
  </div>
</template>

<style scoped>
.speed-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.test-box {
  text-align: center;
  padding: 28px 20px;
  border: 2px dashed var(--border);
  border-radius: 12px;
}
.test-icon {
  color: var(--primary);
  margin-bottom: 10px;
}
.test-title {
  margin: 0 0 4px;
  font-size: 16px;
  font-weight: 600;
  color: var(--text);
}
.test-subtitle {
  margin: 0;
  font-size: 13px;
  color: var(--text-secondary);
}
.btn {
  padding: 10px 20px;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  transition: all 0.2s;
}
.btn-primary {
  background: var(--primary);
  color: #fff;
}
.btn-primary:hover:not(:disabled) {
  opacity: 0.9;
}
.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.btn-full {
  width: 100%;
}
.spinner {
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255,255,255,0.3);
  border-top-color: #fff;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}
@keyframes spin {
  to { transform: rotate(360deg); }
}
.result-card {
  background: var(--bg);
  border-radius: 12px;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.result-header {
  display: flex;
  justify-content: space-between;
  font-size: 13px;
  color: var(--text-secondary);
}
.result-header strong {
  color: var(--text);
  font-size: 14px;
}
.metric-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}
.metric {
  background: var(--card-bg);
  border-radius: 8px;
  padding: 12px;
  text-align: center;
}
.metric-label {
  display: block;
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 4px;
}
.metric strong {
  display: block;
  font-size: 18px;
  color: var(--text);
  margin-bottom: 2px;
}
.metric small {
  font-size: 11px;
  color: var(--text-secondary);
}
.size-selector {
  display: flex;
  gap: 6px;
}
.size-btn {
  flex: 1;
  padding: 6px 0;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--card-bg);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}
.size-btn:hover:not(:disabled) {
  border-color: var(--primary);
  color: var(--primary);
}
.size-btn.active {
  background: var(--primary);
  border-color: var(--primary);
  color: #fff;
}
.size-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.error-box {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 14px;
  background: #fef2f2;
  color: #dc2626;
  border-radius: 8px;
  font-size: 13px;
}
</style>
