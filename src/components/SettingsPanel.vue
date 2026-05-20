<script setup lang="ts">
import { ref, inject, watch, type ComputedRef } from "vue";
import type { LarkConfig } from "../types/transfer";

const t = inject<ComputedRef<Record<string, string>>>("t")!;

const props = defineProps<{
  visible: boolean;
  config: LarkConfig;
}>();

const emit = defineEmits<{
  close: [];
  save: [config: LarkConfig];
}>();

const localAppId = ref(props.config.appId || "");
const localAppSecret = ref(props.config.appSecret || "");
const showAppSecret = ref(false);

watch(
  () => props.visible,
  (v) => {
    if (v) {
      localAppId.value = props.config.appId || "";
      localAppSecret.value = props.config.appSecret || "";
    }
  }
);

function handleSave() {
  emit("save", {
    appId: localAppId.value.trim(),
    appSecret: localAppSecret.value.trim(),
  });
  emit("close");
}
</script>

<template>
  <Transition name="overlay">
    <div class="overlay" v-if="visible" @click.self="emit('close')">
      <div class="settings-card">
        <div class="settings-header">
          <h3>{{ t.settings }}</h3>
          <button class="close-btn" @click="emit('close')"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" /></svg></button>
        </div>
        <div class="settings-body">
          <div class="field-group">
            <label class="field-label">{{ t.appIdLabel }}</label>
            <input v-model="localAppId" class="token-input" type="text" :placeholder="t.appIdPlaceholder" spellcheck="false" />
          </div>

          <div class="field-group">
            <label class="field-label">{{ t.appSecretLabel }}</label>
            <div class="secret-row">
              <input v-if="showAppSecret" v-model="localAppSecret" class="token-input secret-input" type="text" :placeholder="t.appSecretPlaceholder" spellcheck="false" />
              <input v-else v-model="localAppSecret" class="token-input secret-input" type="password" autocomplete="new-password" :placeholder="t.appSecretPlaceholder" spellcheck="false" />
              <button class="toggle-btn" @click="showAppSecret = !showAppSecret" type="button" :title="showAppSecret ? t.hide : t.show"><svg v-if="showAppSecret" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94" /><path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19" /><line x1="1" y1="1" x2="23" y2="23" /></svg><svg v-else width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" /><circle cx="12" cy="12" r="3" /></svg></button>
            </div>
          </div>

          <p class="hint">{{ t.appCredentialHint }}</p>
        </div>
        <div class="settings-footer">
          <button class="btn btn-secondary" @click="emit('close')">{{ t.cancel }}</button>
          <button class="btn btn-primary" @click="handleSave">{{ t.save }}</button>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 1000;
  padding: 20px;
}
.settings-card {
  width: 100%;
  max-width: 420px;
  background: var(--card-bg);
  border-radius: 16px;
  box-shadow: 0 8px 32px var(--shadow);
  overflow: hidden;
  animation: slideUp 0.25s ease;
}
@keyframes slideUp {
  from { opacity: 0; transform: translateY(20px); }
  to { opacity: 1; transform: translateY(0); }
}
.settings-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border);
}
.settings-header h3 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--text);
}
.close-btn {
  background: none;
  border: none;
  border-radius: 6px;
  padding: 4px;
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  transition: all 0.2s;
}
.close-btn:hover {
  background: var(--hover);
  color: var(--text);
}
.settings-body {
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.field-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.field-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}
.text-input {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  font-size: 14px;
  background: var(--input-bg);
  color: var(--text);
  outline: none;
  transition: border-color 0.2s;
  box-sizing: border-box;
}
.text-input:focus {
  border-color: var(--primary);
}
.secret-row {
  display: flex;
  gap: 6px;
}
.secret-input {
  flex: 1;
}
.secret-input.masked {
  -webkit-text-security: disc;
  font-family: text-security-disc, sans-serif;
}
.toggle-btn {
  background: none;
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 6px 8px;
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  transition: all 0.2s;
  flex-shrink: 0;
}
.toggle-btn:hover {
  border-color: var(--primary);
  color: var(--primary);
}
.divider {
  height: 1px;
  background: var(--border);
  margin: 4px 0;
}
.token-input {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  font-size: 14px;
  font-family: monospace;
  background: var(--input-bg);
  color: var(--text);
  outline: none;
  resize: vertical;
  transition: border-color 0.2s;
  box-sizing: border-box;
}
.token-input:focus {
  border-color: var(--primary);
}
.token-input.masked {
  color: transparent;
  text-shadow: 0 0 8px var(--text);
}
.btn {
  padding: 8px 20px;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}
.btn-primary {
  background: var(--primary);
  color: #fff;
}
.btn-primary:hover {
  opacity: 0.9;
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
.hint {
  font-size: 12px;
  color: var(--text-secondary);
  margin: 0;
  line-height: 1.5;
}
.settings-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 20px;
  border-top: 1px solid var(--border);
}
.overlay-enter-active,
.overlay-leave-active {
  transition: opacity 0.2s ease;
}
.overlay-enter-from,
.overlay-leave-to {
  opacity: 0;
}
</style>
