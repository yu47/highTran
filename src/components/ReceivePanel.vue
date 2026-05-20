<script setup lang="ts">
import { ref, inject, onMounted, onUnmounted, type ComputedRef } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { LarkConfig, LarkCredential, ReceiveStatus } from "../types/transfer";
import ProgressBar from "./ProgressBar.vue";

const t = inject<ComputedRef<Record<string, string>>>("t")!;

const props = defineProps<{
  larkConfig: LarkConfig;
}>();

const code = ref("");
const saveDir = ref("");
const status = ref<ReceiveStatus | null>(null);
const error = ref("");
const isReceiving = ref(false);
const showErrorDialog = ref(false);
const errorDialogTitle = ref("");
const errorDialogMessage = ref("");

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

function showError(title: string, message: string) {
  errorDialogTitle.value = title;
  errorDialogMessage.value = message;
  showErrorDialog.value = true;
}

function closeErrorDialog() {
  showErrorDialog.value = false;
}

function parseError(e: any): { title: string; message: string } {
  const raw = typeof e === "string" ? e : e?.message || t.value.receiveFailed;
  if (raw === "INVALID_CODE" || raw.includes("INVALID_CODE")) return { title: t.value.invalidCode, message: t.value.invalidCodeMsg };
  if (raw.includes("NOT_FOUND")) return { title: t.value.fileNotFound, message: t.value.fileNotFoundMsg };
  if (raw.includes("Lark code") || raw.includes("Lark")) return { title: t.value.receiveFailed, message: raw };
  if (raw.includes("Token")) return { title: t.value.authFailed, message: raw };
  if (raw.includes("Decryption failed")) return { title: t.value.decryptFailed, message: t.value.decryptFailedMsg };
  return { title: t.value.receiveFailed, message: raw };
}

onMounted(async () => {
  unlisten = await listen<ReceiveStatus>("receive-status", (event) => {
    status.value = event.payload;
    if (event.payload.state === "error") {
      const { title, message } = parseError(event.payload.message);
      showError(title, message);
      isReceiving.value = false;
    }
    if (event.payload.state === "completed") {
      isReceiving.value = false;
    }
  });
});

onUnmounted(() => { unlisten?.(); });

async function selectDir() {
  const result = await open({ directory: true, multiple: false });
  if (result) saveDir.value = result as string;
}

async function startReceive() {
  if (!code.value.trim()) { showError(t.value.hint, t.value.enterCode); return; }
  if (!saveDir.value) { showError(t.value.hint, t.value.selectDir); return; }
  if (!hasCredential(props.larkConfig)) { showError(t.value.hint, t.value.configFirst); return; }
  error.value = "";
  isReceiving.value = true;
  status.value = null;
  try {
    await invoke<string>("start_receive", {
      credential: toCredential(props.larkConfig),
      code: code.value.trim().toLowerCase(),
      saveDir: saveDir.value,
    });
  } catch (e: any) {
    const { title, message } = parseError(e);
    showError(title, message);
    isReceiving.value = false;
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

function reset() {
  code.value = ""; saveDir.value = ""; status.value = null; error.value = ""; isReceiving.value = false;
}
</script>

<template>
  <div class="receive-panel">
    <div class="section" v-if="!status || status.state === 'completed'">
      <div class="input-group">
        <label class="field-label">{{ t.pickupCodeLabel }}</label>
        <div class="code-input-row"><input v-model="code" type="text" class="code-input" :placeholder="t.pickupCodePlaceholder" maxlength="8" :disabled="isReceiving" @keydown.enter="startReceive" /></div>
      </div>
      <div class="input-group">
        <label class="field-label">{{ t.saveLocation }}</label>
        <button class="dir-select" @click="selectDir" :disabled="isReceiving"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" /></svg><span v-if="saveDir" class="dir-path">{{ saveDir }}</span><span v-else class="dir-hint">{{ t.selectSaveDir }}</span></button>
      </div>
      <button class="btn btn-primary btn-full" :disabled="!code.trim() || !saveDir || isReceiving" @click="startReceive"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="7 10 12 15 17 10" /><line x1="12" y1="15" x2="12" y2="3" /></svg>{{ t.startReceive }}</button>
    </div>
    <div class="section" v-if="status">
      <div class="file-info-card" v-if="status.filename"><svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" /><polyline points="14 2 14 8 20 8" /></svg><div><p class="info-filename">{{ status.filename }}</p><p class="info-size" v-if="status.file_size">{{ (status.file_size / 1048576).toFixed(1) }} MB</p></div></div>
      <div class="state-badge" :class="status.state"><span class="state-dot"></span>{{ status.state === 'connecting' ? t.connecting : status.state === 'downloading' ? t.downloading : status.state === 'completed' ? t.completed : t.errorState }}</div>
      <ProgressBar v-if="status.state === 'downloading'" :progress="status.progress" :label="t.downloadProgress" />
      <div class="speed-info" v-if="status.state === 'downloading' && status.speed_bps > 0"><span>{{ formatSpeed(status.speed_bps) }}</span><span class="separator">|</span><span>{{ t.remaining }} {{ formatEta(status.eta_secs) }}</span></div>
      <p class="status-message">{{ status.message }}</p>
      <button v-if="status.state === 'completed'" class="btn btn-secondary btn-full" @click="reset">{{ t.receiveNewFile }}</button>
    </div>
    <Teleport to="body">
      <Transition name="dialog-fade">
        <div class="dialog-overlay" v-if="showErrorDialog" @click.self="closeErrorDialog">
          <div class="dialog-box"><div class="dialog-icon"><svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" /></svg></div><h3 class="dialog-title">{{ errorDialogTitle }}</h3><p class="dialog-message">{{ errorDialogMessage }}</p><button class="dialog-btn" @click="closeErrorDialog">{{ t.gotIt }}</button></div>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.receive-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.section {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.input-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.field-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}
.code-input-row {
  display: flex;
  gap: 8px;
}
.code-input {
  flex: 1;
  padding: 10px 14px;
  border: 2px solid var(--border);
  border-radius: 10px;
  font-size: 20px;
  font-weight: 700;
  font-family: monospace;
  text-align: center;
  letter-spacing: 6px;
  text-transform: lowercase;
  background: var(--input-bg);
  color: var(--text);
  outline: none;
  transition: border-color 0.2s;
}
.code-input:focus {
  border-color: var(--primary);
}
.code-input:disabled {
  opacity: 0.6;
}
.dir-select {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 10px 14px;
  border: 2px dashed var(--border);
  border-radius: 10px;
  background: none;
  color: var(--text-secondary);
  font-size: 14px;
  cursor: pointer;
  transition: all 0.2s;
  box-sizing: border-box;
}
.dir-select:hover:not(:disabled) {
  border-color: var(--primary);
  color: var(--primary);
}
.dir-select:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
.dir-path {
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.dir-hint {
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
.file-info-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  background: var(--bg);
  border-radius: 10px;
}
.file-info-card svg {
  flex-shrink: 0;
  color: var(--primary);
}
.info-filename {
  margin: 0;
  font-weight: 600;
  font-size: 14px;
  color: var(--text);
  word-break: break-all;
}
.info-size {
  margin: 2px 0 0;
  font-size: 12px;
  color: var(--text-secondary);
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
.state-badge.connecting {
  color: #f59e0b;
}
.state-badge.downloading {
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
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 2000;
  padding: 20px;
}
.dialog-box {
  background: var(--card-bg);
  border-radius: 16px;
  padding: 28px 24px 20px;
  text-align: center;
  max-width: 320px;
  width: 100%;
  box-shadow: 0 8px 32px var(--shadow);
}
.dialog-icon {
  color: #f59e0b;
  margin-bottom: 12px;
}
.dialog-title {
  margin: 0 0 8px;
  font-size: 17px;
  font-weight: 600;
  color: var(--text);
}
.dialog-message {
  margin: 0 0 20px;
  font-size: 14px;
  color: var(--text-secondary);
  line-height: 1.5;
  word-break: break-word;
}
.dialog-btn {
  padding: 8px 28px;
  border: none;
  border-radius: 8px;
  background: var(--primary);
  color: #fff;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: opacity 0.2s;
}
.dialog-btn:hover {
  opacity: 0.9;
}
.dialog-fade-enter-active,
.dialog-fade-leave-active {
  transition: opacity 0.2s ease;
}
.dialog-fade-enter-from,
.dialog-fade-leave-to {
  opacity: 0;
}
</style>
