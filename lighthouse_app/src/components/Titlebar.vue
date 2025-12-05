<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Icon } from "@iconify/vue";

const appWindow = getCurrentWindow();

function minimize() {
  appWindow.minimize();
}

function close() {
  appWindow.close();
}
</script>

<template>
  <div data-tauri-drag-region class="titlebar">
    <div class="title-text">Lighthouse Manager</div>
    <div class="titlebar-button" id="titlebar-minimize" @click="minimize">
      <Icon icon="mdi:window-minimize" />
    </div>
    <div class="titlebar-button" id="titlebar-close" @click="close">
      <Icon icon="mdi:close" />
    </div>
  </div>
</template>

<style scoped>
.titlebar {
  height: 30px;
  background: #323233;
  user-select: none;
  display: flex;
  justify-content: flex-end;
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: 1000;
}

.title-text {
  flex-grow: 1;
  text-align: center;
  color: white;
  line-height: 30px;
  font-size: 12px;
  font-family: "Segoe UI", Tahoma, Geneva, Verdana, sans-serif;
  pointer-events: none; /* Let clicks pass through to drag region */
}

.titlebar-button {
  display: inline-flex;
  justify-content: center;
  align-items: center;
  width: 30px;
  height: 30px;
  cursor: default; /* Windows buttons use default cursor */
}

.titlebar-button:hover {
  background: #5f5f5f;
}

#titlebar-close:hover {
  background: #e81123;
}

.titlebar-button svg {
  color: white;
  width: 10px;
  height: 10px;
}
</style>
