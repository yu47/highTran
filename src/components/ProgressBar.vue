<script setup lang="ts">
defineProps<{
  progress: number;
  label?: string;
}>();
</script>

<template>
  <div class="progress-wrapper">
    <div class="progress-header" v-if="label">
      <span class="progress-label">{{ label }}</span>
      <span class="progress-value">{{ progress.toFixed(1) }}%</span>
    </div>
    <div class="progress-track">
      <div
        class="progress-fill"
        :style="{ width: Math.min(progress, 100) + '%' }"
      ></div>
    </div>
  </div>
</template>

<style scoped>
.progress-wrapper {
  width: 100%;
}

.progress-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}

.progress-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.progress-value {
  font-size: 12px;
  font-weight: 700;
  color: var(--primary);
}

.progress-track {
  width: 100%;
  height: 6px;
  background: var(--border-color);
  border-radius: 3px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--primary), var(--primary-light));
  border-radius: 3px;
  transition: width 0.3s ease;
  position: relative;
}

.progress-fill::after {
  content: "";
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(
    90deg,
    transparent 0%,
    rgba(255, 255, 255, 0.25) 50%,
    transparent 100%
  );
  animation: shimmer 1.8s infinite;
}

@keyframes shimmer {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(100%); }
}
</style>
