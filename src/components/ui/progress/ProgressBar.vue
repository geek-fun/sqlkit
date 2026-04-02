<script setup lang="ts">
type Props = {
  visible?: boolean
  indeterminate?: boolean
}

withDefaults(defineProps<Props>(), {
  visible: false,
  indeterminate: true,
})
</script>

<template>
  <div v-if="visible" class="progress-bar">
    <div class="progress-bar-inner" :class="{ indeterminate }" />
  </div>
</template>

<style scoped>
.progress-bar {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  height: 2px;
  background-color: transparent;
  overflow: hidden;
  z-index: 1000;
}

.progress-bar-inner {
  height: 100%;
  background-color: hsl(var(--primary));
  transition: width 0.3s ease;
}

.progress-bar-inner.indeterminate {
  width: 30%;
  animation: indeterminate 1.5s infinite;
}

@keyframes indeterminate {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(400%);
  }
}
</style>
