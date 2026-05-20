<script setup lang="ts">
import { ref, inject, onMounted, onUnmounted, type ComputedRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { LarkConfig, LarkCredential, SendStatus } from "../types/transfer";
import ProgressBar from "./ProgressBar.vue";

const t = inject<ComputedRef<Record<string, string>>>("t")!;

const props = defineProps<{
  larkConfig: LarkConfig;
}>();

const selectedFile = ref<string | null>(null);
const fileName = ref("");
const fileSize = ref(0);
const status = ref<SendStatus | null>(null);
const error = ref("");
const isSending = ref(false);
const copied = ref(false);

let unlisten: (() => void) | null = null;

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

onMounted(async () => {
  unlisten = await listen<SendStatus>("send-status", (event) => {
    status.value = event.payload;
    if (event.payload.state === "error") {
      error.value = event.payload.message;
      isSending.value = false;
    }
    if (event.payload.state === "completed") {
      isSending.value = false;
    }
  });
});

onUnmounted(() => {
  unlisten?.();
});

async function selectFile() {
  const result = await open({ multiple: false, directory: false });
  if (result) {
    selectedFile.value = result as string;
    const parts = (result as string).replace(/\\/g, "/").split("/");
    fileName.value = parts[parts.length - 1];
    fileSize.value = 0;
  }
}

async function startSend() {
  if (!selectedFile.value) return;
  if (!hasCredential(props.larkConfig)) {
    error.value = t.value.configFirst;
    return;
  }
  error.value = "";
  isSending.value = true;
  try {
    const code = await invoke<string>("start_send", {
      credential: toCredential(props.larkConfig),
      filePath: selectedFile.value,
    });
    status.value = {
      state: "waiting",
      code,
      progress: 0,
      message: `${t.value.pickupCode}: ${code} - ${t.value.waitingCode}`,
      filename: fileName.value,
      file_size: 0,
      speed_bps: 0,
      eta_secs: -1,
    };
  } catch (e: any) {
    error.value = typeof e === "string" ? e : e.message || t.value.sendFailed;
    isSending.value = false;
  }
}

async function copyCode() {
  if (!status.value?.code) return;
  try {
    await navigator.clipboard.writeText(status.value.code);
    copied.value = true;
    setTimeout(() => { copied.value = false; }, 2000);
  } catch {
    const ta = document.createElement("textarea");
    ta.value = status.value.code;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand("copy");
    document.body.removeChild(ta);
    copied.value = true;
    setTimeout(() => { copied.value = false; }, 2000);
  }
}

function formatEta(secs: number): string {
  if (secs < 0 || !isFinite(secs)) return t.value.calculating;
  const s = Math.round(secs);
  if (s < 60) return `${s}${t.value.secUnit}`;
  if (s < 3600) return `${Math.floor(s / 60)}${t.value.minUnit}${s % 60}${t.value.secUnit}`;
  return `${Math.floor(s / 3600)}${t.value.hourUnit}${Math.floor((s % 3600) / 60)}${t.value.minUnit}`;
}

function formatSpeed(bps: number): string {
  if (bps < 1024) return bps.toFixed(0) + " B/s";
  if (bps < 1048576) return (bps / 1024).toFixed(1) + " KB/s";
  return (bps / 1048576).toFixed(2) + " MB/s";
}

function formatSize(bytes: number): string {
  if (bytes === 0) return "";
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1048576) return (bytes / 1024).toFixed(1) + " KB";
  if (bytes < 1073741824) return (bytes / 1048576).toFixed(1) + " MB";
  return (bytes / 1073741824).toFixed(2) + " GB";
}

function reset() {
  selectedFile.value = null;
  fileName.value = "";
  fileSize.value = 0;
  status.value = null;
  error.value = "";
  isSending.value = false;
}
</script>

<template>
  <div class="send-panel">
    <div class="section" v-if="!status || status.state === 'completed'">
      <div class="file-select" @click="selectFile">
        <div class="file-icon"><svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="17 8 12 3 7 8" /><line x1="12" y1="3" x2="12" y2="15" /></svg></div>
        <p class="file-hint" v-if="!selectedFile">{{ t.selectFile }}</p>
        <div class="file-info" v-else>
          <p class="file-name">{{ fileName }}</p>
          <p class="file-size" v-if="fileSize">{{ formatSize(fileSize) }}</p>
        </div>
      </div>
      <button class="btn btn-primary btn-full" :disabled="!selectedFile || isSending" @click="startSend"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="22" y1="2" x2="11" y2="13" /><polygon points="22 2 15 22 11 13 2 9 22 2" /></svg>{{ t.confirmSend }}</button>
    </div>
    <div class="section" v-if="status">
      <div class="code-display" v-if="status.code">
        <span class="code-label">{{ t.pickupCode }}</span>
        <div class="code-value"><span v-for="(ch, i) in status.code.split('')" :key="i" class="code-char">{{ ch }}</span></div>
        <button class="copy-btn" @click="copyCode"><svg v-if="!copied" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2" /><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" /></svg><svg v-else width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="20 6 9 17 4 12" /></svg>{{ copied ? t.copied : t.copyCode }}</button>
        <p class="code-hint">{{ t.codeHint }}</p>
      </div>
      <div class="state-badge" :class="status.state"><span class="state-dot"></span>{{ status.state === 'waiting' ? t.waiting : status.state === 'transferring' ? t.transferring : status.state === 'completed' ? t.completed : t.errorState }}</div>
      <ProgressBar v-if="status.state === 'transferring'" :progress="status.progress" :label="t.uploadProgress" />
      <div class="speed-info" v-if="status.state === 'transferring' && status.speed_bps > 0"><span>{{ formatSpeed(status.speed_bps) }}</span><span class="separator">|</span><span>{{ t.remaining }} {{ formatEta(status.eta_secs) }}</span></div>
      <p class="status-message">{{ status.message }}</p>
      <button v-if="status.state === 'completed'" class="btn btn-secondary btn-full" @click="reset">{{ t.sendNewFile }}</button>
    </div>
    <div class="error-box" v-if="error"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10" /><line x1="15" y1="9" x2="9" y2="15" /><line x1="9" y1="9" x2="15" y2="15" /></svg>{{ error }}</div>
  </div>
</template>

<style scoped>
.send-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.file-select {
  border: 2px dashed var(--border);
  border-radius: 12px;
  padding: 32px 20px;
  text-align: center;
  cursor: pointer;
  transition: all 0.2s;
}
.file-select:hover {
  border-color: var(--primary);
  background: var(--hover);
}
.file-icon {
  color: var(--text-secondary);
  margin-bottom: 8px;
}
.file-hint {
  margin: 0;
  color: var(--text-secondary);
  font-size: 14px;
}
.file-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.file-name {
  margin: 0;
  font-weight: 600;
  color: var(--text);
  font-size: 14px;
  word-break: break-all;
}
.file-size {
  margin: 0;
  color: var(--text-secondary);
  font-size: 12px;
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
.btn-secondary {
  background: var(--hover);
  color: var(--text);
}
.btn-secondary:hover {
  background: var(--border);
}
.btn-full {
  width: 100%;
}
.code-display {
  text-align: center;
  padding: 16px;
  background: var(--bg);
  border-radius: 12px;
}
.code-label {
  display: block;
  font-size: 13px;
  color: var(--text-secondary);
  margin-bottom: 10px;
}
.code-value {
  display: flex;
  justify-content: center;
  gap: 4px;
  margin-bottom: 12px;
}
.code-char {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 44px;
  background: var(--card-bg);
  border: 2px solid var(--border);
  border-radius: 8px;
  font-size: 22px;
  font-weight: 700;
  font-family: monospace;
  color: var(--text);
}
.code-hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-secondary);
}
.copy-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--card-bg);
  color: var(--text-secondary);
  font-size: 13px;
  cursor: pointer;
  transition: all 0.2s;
}
.copy-btn:hover {
  border-color: var(--primary);
  color: var(--primary);
}
.state-badge {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 500;
  padding: 6px 12px;
  border-radius: 20px;
  background: var(--bg);
  align-self: flex-start;
}
.state-badge.waiting {
  color: #f59e0b;
}
.state-badge.transferring {
  color: var(--primary);
}
.state-badge.completed {
  color: #10b981;
}
.state-badge.error {
  color: #ef4444;
}
.state-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: currentColor;
}
.speed-info {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-secondary);
}
.separator {
  color: var(--border);
}
.status-message {
  margin: 0;
  font-size: 13px;
  color: var(--text-secondary);
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
