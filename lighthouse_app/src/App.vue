<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, nextTick } from "vue";
import { Icon } from "@iconify/vue";
import gsap from "gsap";
import { invoke } from "@tauri-apps/api/core";
import Titlebar from "./components/Titlebar.vue";

// GSAP button hover animations
function onButtonMouseEnter(event: MouseEvent) {
  const target = event.currentTarget as HTMLButtonElement;
  if (target.disabled) return;
  gsap.to(target, {
    scale: 1.02,
    duration: 0.2,
    ease: "power2.out",
    overwrite: "auto",
  });
}

function onButtonMouseLeave(event: MouseEvent) {
  const target = event.currentTarget as HTMLButtonElement;
  gsap.killTweensOf(target);
  gsap.set(target, { scale: 1 });
}

function onButtonMouseDown(event: MouseEvent) {
  const target = event.currentTarget as HTMLButtonElement;
  if (target.disabled) return;
  gsap.to(target, {
    scale: 0.98,
    duration: 0.1,
    ease: "power2.out",
    overwrite: "auto",
  });
}

function onButtonMouseUp(event: MouseEvent) {
  const target = event.currentTarget as HTMLButtonElement;
  if (target.disabled) return;
  gsap.to(target, {
    scale: 1.02,
    duration: 0.1,
    ease: "power2.out",
    overwrite: "auto",
  });
}

const currentView = ref<"main" | "settings">("main");
const mainViewRef = ref<HTMLElement | null>(null);
const settingsViewRef = ref<HTMLElement | null>(null);

interface DeviceInfo {
  name: string;
  address: string;
}

const lighthouses = ref<DeviceInfo[]>([]);
const isScanning = ref(false);
const isPoweringOn = ref(false);
const isStandingBy = ref(false);
const steamvrRegistered = ref(false);

type DeviceStatus = "online" | "standby" | "transitioning";
const deviceStatus = ref<Map<string, DeviceStatus>>(new Map());

const showScrollButton = ref(false);
const cardsContainerRef = ref<HTMLElement | null>(null);

const showInstallPrompt = ref(false);
const showClearDevicesModal = ref(false);
const showUninstallModal = ref(false);
const showResetModal = ref(false);
const isInstalled = ref(false);
const isInstallerSupported = ref(false);
const isInstalling = ref(false);
const isUninstalling = ref(false);
const isClearingDevices = ref(false);
const isResetting = ref(false);
const createDesktopShortcut = ref(true);
const doNotShowInstallPrompt = ref(false);
const theme = ref("dark");
const appDataPath = ref("");

function checkScroll() {
  if (!cardsContainerRef.value) return;
  const { scrollTop, scrollHeight, clientHeight } = cardsContainerRef.value;
  // Show button if there is more content to scroll to (with a small buffer)
  showScrollButton.value = scrollHeight - scrollTop - clientHeight > 10;
}

function scrollDown() {
  if (!cardsContainerRef.value) return;
  cardsContainerRef.value.scrollBy({ top: 200, behavior: "smooth" });
}

async function checkInstallation() {
  try {
    // Check config first
    const config: { do_not_show_install_prompt: boolean; theme: string } =
      await invoke("get_app_config");
    doNotShowInstallPrompt.value = config.do_not_show_install_prompt;
    // Ensure theme defaults to dark if not set or invalid
    theme.value = config.theme || "dark";

    isInstallerSupported.value = await invoke("is_installer_supported");
    if (isInstallerSupported.value) {
      isInstalled.value = await invoke("check_installation_status");
      if (!isInstalled.value && !doNotShowInstallPrompt.value) {
        showInstallPrompt.value = true;
      }
    }
  } catch (e) {
    console.error("Failed to check installation status:", e);
  }
}

async function installApp() {
  isInstalling.value = true;
  try {
    await invoke("install_application", {
      createDesktopShortcut: createDesktopShortcut.value,
    });
    isInstalled.value = true;
    showInstallPrompt.value = false;
  } catch (e) {
    console.error("Failed to install application:", e);
    alert("Failed to install application: " + e);
  } finally {
    isInstalling.value = false;
  }
}

async function skipInstall() {
  try {
    await invoke("save_app_config", {
      config: {
        do_not_show_install_prompt: true,
        theme: theme.value,
      },
    });
  } catch (e) {
    console.error("Failed to save config:", e);
  }
  showInstallPrompt.value = false;
}

async function toggleTheme() {
  theme.value = theme.value === "dark" ? "light" : "dark";
  try {
    // We need to get the current config to preserve other values
    // Or we can just send the partial update if the backend supported it, but it doesn't.
    // However, save_app_config overwrites the file.
    // So we should probably read it first or keep a local copy of the full config.
    // For now, let's assume we have the latest values in our refs.
    await invoke("save_app_config", {
      config: {
        do_not_show_install_prompt: doNotShowInstallPrompt.value,
        theme: theme.value,
      },
    });
  } catch (e) {
    console.error("Failed to save config:", e);
  }
}

function openUninstallModal() {
  showUninstallModal.value = true;
}

async function openResetModal() {
  try {
    appDataPath.value = await invoke("get_app_data_dir");
    showResetModal.value = true;
  } catch (e) {
    console.error("Failed to get app data dir:", e);
    alert("Failed to get application data directory: " + e);
  }
}

async function confirmUninstall() {
  isUninstalling.value = true;
  try {
    await invoke("uninstall_application");
    // The app will exit after uninstall, so this code won't execute
  } catch (e) {
    console.error("Failed to uninstall application:", e);
    alert("Failed to uninstall application: " + e);
    isUninstalling.value = false;
    showUninstallModal.value = false;
  }
}

async function confirmReset() {
  isResetting.value = true;
  try {
    await invoke("reset_application_data");
    await invoke("restart_application");
  } catch (e) {
    console.error("Failed to reset application:", e);
    alert("Failed to reset application: " + e);
    isResetting.value = false;
    showResetModal.value = false;
  }
}

function openSettings() {
  currentView.value = "settings";
  checkSteamVRStatus();
}

function goBack() {
  currentView.value = "main";
}

async function fetchDevices() {
  try {
    lighthouses.value = await invoke("get_devices");
  } catch (e) {
    console.error("Failed to fetch devices:", e);
  }
}

function openClearDevicesModal() {
  showClearDevicesModal.value = true;
}

async function confirmClearDevices() {
  isClearingDevices.value = true;
  try {
    await invoke("clear_saved_devices");
    lighthouses.value = [];
    deviceStatus.value.clear();
    showClearDevicesModal.value = false;
  } catch (e) {
    console.error("Failed to clear saved devices:", e);
    alert("Failed to clear saved devices: " + e);
  } finally {
    isClearingDevices.value = false;
  }
}

async function scan() {
  isScanning.value = true;
  try {
    lighthouses.value = await invoke("scan_for_devices");
  } catch (e) {
    console.error("Failed to scan:", e);
  } finally {
    isScanning.value = false;
  }
}

async function powerOn() {
  isPoweringOn.value = true;
  try {
    // Set existing devices to transitioning while we scan
    lighthouses.value.forEach((device) => {
      deviceStatus.value.set(device.address, "transitioning");
    });

    // Power on returns the discovered and saved devices
    const devices: DeviceInfo[] = await invoke("power_on_all");

    // Update the device list with found devices
    if (devices.length > 0) {
      lighthouses.value = devices;
      // Set all found devices to online status
      devices.forEach((device) => {
        deviceStatus.value.set(device.address, "online");
      });
    }

    isPoweringOn.value = false;
  } catch (e) {
    console.error("Failed to power on:", e);
    // Reset to standby on error
    lighthouses.value.forEach((device) => {
      deviceStatus.value.set(device.address, "standby");
    });
    isPoweringOn.value = false;
  }
}

async function standby() {
  isStandingBy.value = true;
  try {
    // Set existing devices to transitioning while we scan
    lighthouses.value.forEach((device) => {
      deviceStatus.value.set(device.address, "transitioning");
    });

    // Standby returns the discovered and saved devices
    const devices: DeviceInfo[] = await invoke("standby_all");

    // Update the device list with found devices
    if (devices.length > 0) {
      lighthouses.value = devices;
      // Set all found devices to standby status
      devices.forEach((device) => {
        deviceStatus.value.set(device.address, "standby");
      });
    }

    isStandingBy.value = false;
  } catch (e) {
    console.error("Failed to standby:", e);
    // Reset to online on error
    lighthouses.value.forEach((device) => {
      deviceStatus.value.set(device.address, "online");
    });
    isStandingBy.value = false;
  }
}

async function checkSteamVRStatus() {
  try {
    steamvrRegistered.value = await invoke("get_steamvr_status");
  } catch (e) {
    console.error("Failed to check SteamVR status:", e);
  }
}

async function toggleSteamVR(enabled: boolean) {
  try {
    await invoke("set_steamvr_registration", { enabled });
    steamvrRegistered.value = enabled;
  } catch (e) {
    console.error("Failed to toggle SteamVR:", e);
    // Revert on error
    steamvrRegistered.value = !enabled;
  }
}

function animateViewIn(viewRef: HTMLElement | null) {
  if (!viewRef) return;

  const header = viewRef.querySelector(".content-header");
  const title = viewRef.querySelector(".page-title");
  const buttons = viewRef.querySelectorAll(".icon-button");
  const cards = viewRef.querySelectorAll(".lighthouse-card");
  const actionButtons = viewRef.querySelector(".action-buttons");
  const settingsContent = viewRef.querySelector(".settings-content");

  // Set initial states
  gsap.set(header, { opacity: 0, y: -20 });
  gsap.set(title, { opacity: 0, x: -30 });
  if (buttons.length > 0)
    gsap.set(buttons, { opacity: 0, scale: 0.5, rotation: -180 });
  if (cards.length > 0) gsap.set(cards, { opacity: 0, y: 30, scale: 0.9 });
  if (actionButtons) gsap.set(actionButtons, { opacity: 0, y: 20 });
  if (settingsContent) gsap.set(settingsContent, { opacity: 0, y: 20 });

  // Create timeline for staggered animation
  const tl = gsap.timeline({ defaults: { ease: "power3.out" } });

  tl.to(header, { opacity: 1, y: 0, duration: 0.4 }).to(
    title,
    { opacity: 1, x: 0, duration: 0.5 },
    "-=0.2"
  );

  if (buttons.length > 0) {
    tl.to(
      buttons,
      { opacity: 1, scale: 1, rotation: 0, duration: 0.6 },
      "-=0.3"
    );
  }

  if (cards.length > 0) {
    tl.to(
      cards,
      { opacity: 1, y: 0, scale: 1, duration: 0.5, stagger: 0.1 },
      "-=0.3"
    );
  }

  if (actionButtons) {
    tl.to(actionButtons, { opacity: 1, y: 0, duration: 0.4 }, "-=0.2");
  }

  if (settingsContent) {
    tl.to(settingsContent, { opacity: 1, y: 0, duration: 0.4 }, "-=0.2");
  }
}

onMounted(async () => {
  await fetchDevices();
  await checkInstallation();
  nextTick(() => {
    animateViewIn(mainViewRef.value);
    checkScroll();
  });

  // Recalculate scroll button visibility on window resize
  window.addEventListener("resize", checkScroll);

  // Disable context menu (right-click)
  window.addEventListener("contextmenu", (e) => e.preventDefault());
});

onUnmounted(() => {
  window.removeEventListener("resize", checkScroll);
  window.removeEventListener("contextmenu", (e) => e.preventDefault());
});

watch(currentView, () => {
  nextTick(() => {
    if (currentView.value === "main") {
      animateViewIn(mainViewRef.value);
    } else {
      animateViewIn(settingsViewRef.value);
    }
  });
});

watch(
  theme,
  (newTheme) => {
    document.documentElement.setAttribute("data-theme", newTheme);
  },
  { immediate: true }
);

// Watch for lighthouse changes and animate new cards
watch(
  lighthouses,
  (newDevices) => {
    // Only animate if we're on the main view and devices were added
    if (currentView.value === "main" && newDevices.length > 0) {
      nextTick(() => {
        const cards = mainViewRef.value?.querySelectorAll(".lighthouse-card");
        if (cards && cards.length > 0) {
          // Animate all cards to be visible
          gsap.to(cards, {
            opacity: 1,
            y: 0,
            scale: 1,
            duration: 0.4,
            stagger: 0.1,
            ease: "power3.out",
          });
        }
        // Also update scroll button visibility
        checkScroll();
      });
    }
  },
  { deep: true }
);
</script>

<template>
  <Titlebar />

  <!-- Installation Prompt Modal -->
  <div v-if="showInstallPrompt" class="modal-overlay">
    <div class="modal-content">
      <h2 class="modal-title">Welcome to Lighthouse Manager</h2>
      <p class="modal-message">
        This application is not currently installed. Would you like to install
        it to your system?
      </p>
      <p class="modal-info">
        Installing will copy the application to your local app data folder and
        create a Start Menu shortcut.
      </p>
      <label class="checkbox-option">
        <input
          type="checkbox"
          v-model="createDesktopShortcut"
          :disabled="isInstalling"
        />
        <span>Create desktop shortcut</span>
      </label>

      <div class="modal-actions">
        <button
          class="modal-btn modal-btn-secondary"
          @click="skipInstall"
          :disabled="isInstalling"
          @mouseenter="onButtonMouseEnter"
          @mouseleave="onButtonMouseLeave"
          @mousedown="onButtonMouseDown"
          @mouseup="onButtonMouseUp"
        >
          Skip
        </button>
        <button
          class="modal-btn modal-btn-primary"
          @click="installApp"
          :disabled="isInstalling"
          @mouseenter="onButtonMouseEnter"
          @mouseleave="onButtonMouseLeave"
          @mousedown="onButtonMouseDown"
          @mouseup="onButtonMouseUp"
        >
          <Icon v-if="isInstalling" icon="mdi:loading" class="spin" />
          {{ isInstalling ? "Installing..." : "Install" }}
        </button>
      </div>
    </div>
  </div>

  <!-- Clear Devices Confirmation Modal -->
  <div v-if="showClearDevicesModal" class="modal-overlay">
    <div class="modal-content">
      <h2 class="modal-title">Clear Saved Lighthouses</h2>
      <p class="modal-message">
        Are you sure you want to delete all saved lighthouses?
      </p>
      <p class="modal-info">
        You will need to scan again to discover them. This action cannot be
        undone.
      </p>
      <div class="modal-actions">
        <button
          class="modal-btn modal-btn-secondary"
          @click="showClearDevicesModal = false"
          :disabled="isClearingDevices"
          @mouseenter="onButtonMouseEnter"
          @mouseleave="onButtonMouseLeave"
          @mousedown="onButtonMouseDown"
          @mouseup="onButtonMouseUp"
        >
          Cancel
        </button>
        <button
          class="modal-btn modal-btn-primary"
          @click="confirmClearDevices"
          :disabled="isClearingDevices"
          @mouseenter="onButtonMouseEnter"
          @mouseleave="onButtonMouseLeave"
          @mousedown="onButtonMouseDown"
          @mouseup="onButtonMouseUp"
        >
          <Icon v-if="isClearingDevices" icon="mdi:loading" class="spin" />
          {{ isClearingDevices ? "Clearing..." : "Clear All" }}
        </button>
      </div>
    </div>
  </div>

  <!-- Uninstall Confirmation Modal -->
  <div v-if="showUninstallModal" class="modal-overlay">
    <div class="modal-content">
      <h2 class="modal-title">Uninstall Lighthouse Manager</h2>
      <p class="modal-message">
        Are you sure you want to uninstall Lighthouse Manager?
      </p>
      <p class="modal-info">
        This will remove all application files, shortcuts, and saved data. This
        action cannot be undone.
      </p>
      <div class="modal-actions">
        <button
          class="modal-btn modal-btn-secondary"
          @click="showUninstallModal = false"
          :disabled="isUninstalling"
          @mouseenter="onButtonMouseEnter"
          @mouseleave="onButtonMouseLeave"
          @mousedown="onButtonMouseDown"
          @mouseup="onButtonMouseUp"
        >
          Cancel
        </button>
        <button
          class="modal-btn modal-btn-primary"
          @click="confirmUninstall"
          :disabled="isUninstalling"
          @mouseenter="onButtonMouseEnter"
          @mouseleave="onButtonMouseLeave"
          @mousedown="onButtonMouseDown"
          @mouseup="onButtonMouseUp"
        >
          <Icon v-if="isUninstalling" icon="mdi:loading" class="spin" />
          {{ isUninstalling ? "Uninstalling..." : "Uninstall" }}
        </button>
      </div>
    </div>
  </div>

  <!-- Reset Confirmation Modal -->
  <div v-if="showResetModal" class="modal-overlay">
    <div class="modal-content">
      <h2 class="modal-title">Reset Application Data</h2>
      <p class="modal-message">
        Are you sure you want to reset all application data?
      </p>
      <p class="modal-info">
        This will delete all configuration files, saved lighthouses, and
        preferences. The application will restart.
      </p>
      <p class="modal-info">
        This will delete the following directory and all its contents:<br />
        <code class="path-display">{{ appDataPath }}</code>
      </p>
      <div class="modal-actions">
        <button
          class="modal-btn modal-btn-secondary"
          @click="showResetModal = false"
          :disabled="isResetting"
          @mouseenter="onButtonMouseEnter"
          @mouseleave="onButtonMouseLeave"
          @mousedown="onButtonMouseDown"
          @mouseup="onButtonMouseUp"
        >
          Cancel
        </button>
        <button
          class="modal-btn modal-btn-primary"
          @click="confirmReset"
          :disabled="isResetting"
          @mouseenter="onButtonMouseEnter"
          @mouseleave="onButtonMouseLeave"
          @mousedown="onButtonMouseDown"
          @mouseup="onButtonMouseUp"
        >
          <Icon v-if="isResetting" icon="mdi:loading" class="spin" />
          {{ isResetting ? "Resetting..." : "Reset" }}
        </button>
      </div>
    </div>
  </div>

  <main class="container">
    <!-- Main View -->
    <div v-if="currentView === 'main'" ref="mainViewRef" class="view">
      <header class="content-header">
        <h1 class="page-title">Lighthouses</h1>
        <div class="header-actions">
          <button
            class="icon-button theme-button"
            @click="toggleTheme"
            :aria-label="
              theme === 'dark' ? 'Switch to Light Mode' : 'Switch to Dark Mode'
            "
          >
            <Icon
              :icon="
                theme === 'dark'
                  ? 'mdi:white-balance-sunny'
                  : 'mdi:weather-night'
              "
            />
          </button>
          <button
            class="icon-button cog-button"
            @click="openSettings"
            aria-label="Settings"
            style="position: relative"
          >
            <Icon icon="mdi:cog" />
          </button>
        </div>
      </header>

      <!-- Lighthouse Cards -->
      <div class="cards-container-wrapper">
        <div
          class="cards-container"
          ref="cardsContainerRef"
          @scroll="checkScroll"
        >
          <div v-if="lighthouses.length === 0" class="no-devices">
            <span>No devices found</span>
          </div>
          <div
            v-for="lighthouse in lighthouses"
            :key="lighthouse.address"
            class="lighthouse-card"
          >
            <div class="lighthouse-info">
              <span class="lighthouse-name">{{ lighthouse.name }}</span>
              <span class="lighthouse-address">{{ lighthouse.address }}</span>
            </div>
            <div
              class="device-status"
              :class="{
                'status-online':
                  deviceStatus.get(lighthouse.address) === 'online',
                'status-standby':
                  deviceStatus.get(lighthouse.address) === 'standby',
                'status-transitioning':
                  deviceStatus.get(lighthouse.address) === 'transitioning',
              }"
            ></div>
          </div>
        </div>
        <transition name="fade">
          <button
            v-if="showScrollButton"
            class="scroll-more-btn"
            @click="scrollDown"
          >
            Show more
            <Icon icon="mdi:chevron-down" />
          </button>
        </transition>
      </div>

      <!-- Action Buttons -->
      <div class="action-buttons">
        <button
          class="action-btn"
          @click="scan"
          :disabled="isScanning"
          @mouseenter="onButtonMouseEnter"
          @mouseleave="onButtonMouseLeave"
          @mousedown="onButtonMouseDown"
          @mouseup="onButtonMouseUp"
        >
          <Icon icon="mdi:radar" :class="{ spin: isScanning }" />
          <span>{{ isScanning ? "Scanning..." : "Scan" }}</span>
        </button>
        <button
          class="action-btn"
          @click="powerOn"
          :disabled="isPoweringOn"
          @mouseenter="onButtonMouseEnter"
          @mouseleave="onButtonMouseLeave"
          @mousedown="onButtonMouseDown"
          @mouseup="onButtonMouseUp"
        >
          <Icon
            icon="mdi:loading"
            :class="{ spin: isPoweringOn }"
            v-if="isPoweringOn"
          />
          <Icon icon="mdi:power" v-else />
          <span>{{ isPoweringOn ? "Powering On..." : "Power On" }}</span>
        </button>
        <button
          class="action-btn"
          @click="standby"
          :disabled="isStandingBy"
          @mouseenter="onButtonMouseEnter"
          @mouseleave="onButtonMouseLeave"
          @mousedown="onButtonMouseDown"
          @mouseup="onButtonMouseUp"
        >
          <Icon
            icon="mdi:loading"
            :class="{ spin: isStandingBy }"
            v-if="isStandingBy"
          />
          <Icon icon="mdi:power-sleep" v-else />
          <span>{{ isStandingBy ? "Standing By..." : "Standby" }}</span>
        </button>
      </div>
    </div>

    <!-- Settings View -->
    <div
      v-else-if="currentView === 'settings'"
      ref="settingsViewRef"
      class="view"
    >
      <header class="content-header">
        <button
          class="icon-button back-button"
          @click="goBack"
          aria-label="Go back"
        >
          <Icon icon="mdi:arrow-left" />
        </button>
        <h1 class="page-title">Settings</h1>
        <div class="header-spacer"></div>
      </header>

      <div class="settings-content">
        <!-- SteamVR Integration -->
        <div class="setting-item">
          <div class="setting-label">
            <span class="setting-name">SteamVR Integration</span>
            <span class="setting-desc"
              >Automatically manage lighthouses with SteamVR</span
            >
          </div>
          <label class="switch">
            <input
              type="checkbox"
              :checked="steamvrRegistered"
              @change="
                toggleSteamVR(($event.target as HTMLInputElement).checked)
              "
            />
            <span class="slider round"></span>
          </label>
        </div>

        <!-- Clear Saved Devices -->
        <div class="setting-item">
          <div class="setting-label">
            <span class="setting-name">Saved Lighthouses</span>
            <span class="setting-desc"
              >{{ lighthouses.length }} device(s) saved</span
            >
          </div>
          <button
            class="action-btn-small"
            @click="openClearDevicesModal"
            :disabled="isClearingDevices || lighthouses.length === 0"
            title="Delete all saved lighthouses"
            @mouseenter="onButtonMouseEnter"
            @mouseleave="onButtonMouseLeave"
            @mousedown="onButtonMouseDown"
            @mouseup="onButtonMouseUp"
          >
            <Icon v-if="isClearingDevices" icon="mdi:loading" class="spin" />
            <Icon v-else icon="mdi:delete-sweep" />
            Clear All
          </button>
        </div>

        <!-- Spacer to push uninstall button to bottom -->
        <div class="settings-spacer"></div>

        <!-- Uninstall / Install / Reset Buttons -->
        <div class="uninstall-container">
          <template v-if="isInstallerSupported">
            <template v-if="isInstalled">
              <button
                class="action-btn-small uninstall-btn"
                @click="openUninstallModal"
                :disabled="isUninstalling"
                @mouseenter="onButtonMouseEnter"
                @mouseleave="onButtonMouseLeave"
                @mousedown="onButtonMouseDown"
                @mouseup="onButtonMouseUp"
              >
                <Icon v-if="isUninstalling" icon="mdi:loading" class="spin" />
                <Icon v-else icon="mdi:delete" />
                {{ isUninstalling ? "Uninstalling..." : "Uninstall" }}
              </button>
            </template>
            <template v-else>
              <div class="install-actions">
                <button
                  class="action-btn-small reset-btn"
                  @click="openResetModal"
                  :disabled="isResetting"
                  title="Reset application data"
                  @mouseenter="onButtonMouseEnter"
                  @mouseleave="onButtonMouseLeave"
                  @mousedown="onButtonMouseDown"
                  @mouseup="onButtonMouseUp"
                >
                  <Icon v-if="isResetting" icon="mdi:loading" class="spin" />
                  <Icon v-else icon="mdi:restore" />
                  Reset
                </button>
                <button
                  class="action-btn-small install-btn"
                  @click="installApp"
                  :disabled="isInstalling"
                  @mouseenter="onButtonMouseEnter"
                  @mouseleave="onButtonMouseLeave"
                  @mousedown="onButtonMouseDown"
                  @mouseup="onButtonMouseUp"
                >
                  <Icon v-if="isInstalling" icon="mdi:loading" class="spin" />
                  <Icon v-else icon="mdi:download" />
                  {{ isInstalling ? "Installing..." : "Install" }}
                </button>
              </div>
            </template>
          </template>
          <template v-else>
            <button
              class="action-btn-small reset-btn"
              @click="openResetModal"
              :disabled="isResetting"
              title="Reset application data"
              @mouseenter="onButtonMouseEnter"
              @mouseleave="onButtonMouseLeave"
              @mousedown="onButtonMouseDown"
              @mouseup="onButtonMouseUp"
            >
              <Icon v-if="isResetting" icon="mdi:loading" class="spin" />
              <Icon v-else icon="mdi:restore" />
              Reset
            </button>
          </template>
        </div>
      </div>
    </div>
  </main>
</template>

<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

body {
  margin: 0;
  padding: 0;
  overflow: hidden;
  width: 100vw;
  height: 100vh;
}

.container {
  margin: 0;
  padding-top: 30px;
  display: flex;
  flex-direction: column;
  justify-content: flex-start;
  text-align: center;
  height: 100vh;
  box-sizing: border-box;
}

.view {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
}

/* Initial hidden state for animated elements */
.content-header,
.page-title,
.icon-button,
.lighthouse-card,
.action-buttons,
.settings-content {
  opacity: 0;
}

.content-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  width: 100%;
  box-sizing: border-box;
}

.page-title {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
  text-align: left;
}

.header-spacer {
  width: 36px;
  height: 36px;
}

/* Cards Container */
.cards-container-wrapper {
  position: relative;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  margin-bottom: 24px;
  padding: 0 16px;
}

.cards-container {
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow-y: auto;
  flex: 1;
  min-height: 0;
  padding-right: 4px; /* Space for scrollbar */
}

/* Custom Scrollbar */
.cards-container::-webkit-scrollbar {
  width: 6px;
}

.cards-container::-webkit-scrollbar-track {
  background: transparent;
}

.cards-container::-webkit-scrollbar-thumb {
  background-color: rgba(128, 128, 128, 0.3);
  border-radius: 3px;
}

.cards-container::-webkit-scrollbar-thumb:hover {
  background-color: rgba(128, 128, 128, 0.5);
}

.uninstall-container {
  margin-top: auto;
  padding-top: 20px;
  display: flex;
  justify-content: center;
  width: 100%;
}

.install-actions {
  display: flex;
  gap: 10px;
  width: 100%;
  justify-content: center;
}

.uninstall-btn {
  background-color: #ffebee;
  color: #d32f2f;
  border: 1px solid #ffcdd2;
  width: 100%;
  justify-content: center;
}

.uninstall-btn:hover:not(:disabled) {
  background-color: #ffcdd2;
  border-color: #ef9a9a;
}

.reset-btn {
  background-color: #fff3e0;
  color: #ef6c00;
  border: 1px solid #ffe0b2;
  flex: 1;
  justify-content: center;
}

.reset-btn:hover:not(:disabled) {
  background-color: #ffe0b2;
  border-color: #ffcc80;
}

.install-btn {
  background-color: #e8f5e9;
  color: #2e7d32;
  border: 1px solid #c8e6c9;
  flex: 1;
  justify-content: center;
}

.install-btn:hover:not(:disabled) {
  background-color: #c8e6c9;
  border-color: #a5d6a7;
}

/* Scrollbar styling */
::-webkit-scrollbar {
  width: 8px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: #e0e0e0;
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: #bdbdbd;
}

.scroll-more-btn {
  position: absolute;
  bottom: 10px;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(255, 255, 255, 0.9);
  border: 1px solid rgba(0, 0, 0, 0.1);
  border-radius: 20px;
  padding: 6px 16px;
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  font-weight: 500;
  color: #333;
  cursor: pointer;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  transition: all 0.2s ease;
  z-index: 10;
}

.scroll-more-btn:hover {
  background: white;
  transform: translateX(-50%) translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

@media (prefers-color-scheme: dark) {
  .scroll-more-btn {
    background: rgba(50, 50, 50, 0.9);
    border-color: rgba(255, 255, 255, 0.1);
    color: #eee;
  }

  .scroll-more-btn:hover {
    background: #3a3a3a;
  }
}

.no-devices {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px;
  color: #888;
  gap: 10px;
}

.lighthouse-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px;
  background: rgba(128, 128, 128, 0.1);
  border-radius: 12px;
  transition: all 0.1s ease;
}

.lighthouse-card:hover {
  background: rgba(128, 128, 128, 0.2);
  transform: translateX(4px);
}

.lighthouse-info {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
}

.lighthouse-name {
  font-size: 16px;
  font-weight: 500;
}

.lighthouse-address {
  font-size: 12px;
  opacity: 0.7;
}

/* Device Status Indicator */
.device-status {
  width: 14px;
  height: 100%;
  border-radius: 12px;
  margin-left: auto;
  transition: background-color 0.1s ease;
}

.status-online {
  background-color: #4caf50;
}

.status-standby {
  background-color: #ff9800;
}

.status-transitioning {
  background: linear-gradient(180deg, #4caf50 0%, #ff9800 100%);
  animation: pulse-status 1s ease-in-out infinite;
}

@keyframes pulse-status {
  0%,
  100% {
    opacity: 1;
    transform: scaleY(1);
  }
  50% {
    opacity: 0.7;
    transform: scaleY(0.95);
  }
}

/* Action Buttons */
.action-buttons {
  display: flex;
  gap: 12px;
  padding: 16px;
  margin-top: auto;
  background: rgba(255, 255, 255, 0.5);
  backdrop-filter: blur(10px);
}

.action-btn {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 14px 10px;
  border: none;
  border-radius: 10px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  background: #1a1a1a;
  color: white;
  transition: background-color 0.15s ease;
}

.action-btn:hover:not(:disabled) {
  background: #404040;
}

.action-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.action-btn svg {
  width: 20px;
  height: 20px;
}

/* Small action button variant for settings */
.action-btn-small {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  background: #1a1a1a;
  color: white;
  border: none;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: background-color 0.15s ease;
}

.action-btn-small:hover:not(:disabled) {
  background: #404040;
}

.action-btn-small:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.action-btn-small svg {
  width: 16px;
  height: 16px;
}

/* Small icon-only button */
.icon-btn-small {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  padding: 0;
  background: #1a1a1a;
  color: white;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  transition: background-color 0.15s ease;
}

.icon-btn-small:hover:not(:disabled) {
  background: #404040;
}

.icon-btn-small:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.icon-btn-small svg {
  width: 18px;
  height: 18px;
}

.icon-button {
  display: flex;
  justify-content: center;
  align-items: center;
  width: 36px;
  height: 36px;
  padding: 8px;
  border: none;
  border-radius: 8px;
  background: transparent;
  cursor: pointer;
  transition: background-color 0.1s ease;
  font-size: 20px;
  color: #0f0f0f;
}

.icon-button svg {
  color: inherit;
  width: 20px;
  height: 20px;
}

.icon-button:hover {
  background-color: rgba(128, 128, 128, 0.3);
}

[data-theme="dark"] .icon-button {
  color: #f6f6f6;
}

[data-theme="dark"] .icon-button:hover {
  background-color: rgba(255, 255, 255, 0.2);
}

.cog-button:hover svg {
  animation: spin 0.5s ease-in-out;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(90deg);
  }
}

.spin {
  animation: spin 1s linear infinite;
}

.back-button:hover svg {
  animation: bounceLeft 0.3s ease-in-out;
}

@keyframes bounceLeft {
  0%,
  100% {
    transform: translateX(0);
  }
  50% {
    transform: translateX(-3px);
  }
}

.icon-button svg {
  width: 20px;
  height: 20px;
  color: inherit;
}

/* Settings Styles */

.setting-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  background: rgba(128, 128, 128, 0.1);
  border-radius: 12px;
}

.setting-label {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  text-align: left;
}

.setting-name {
  font-weight: 600;
  font-size: 16px;
}

.setting-desc {
  font-size: 12px;
  opacity: 0.7;
}

/* Switch Toggle */
.switch {
  position: relative;
  display: inline-block;
  width: 50px;
  height: 28px;
}

.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}

.slider {
  position: absolute;
  cursor: pointer;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: #ccc;
  -webkit-transition: 0.4s;
  transition: 0.4s;
}

.slider:before {
  position: absolute;
  content: "";
  height: 20px;
  width: 20px;
  left: 4px;
  bottom: 4px;
  background-color: white;
  -webkit-transition: 0.4s;
  transition: 0.4s;
}

input:checked + .slider {
  background-color: #2196f3;
}

input:focus + .slider {
  box-shadow: 0 0 1px #2196f3;
}

input:checked + .slider:before {
  -webkit-transform: translateX(22px);
  -ms-transform: translateX(22px);
  transform: translateX(22px);
}

/* Rounded sliders */
.slider.round {
  border-radius: 34px;
}

.slider.round:before {
  border-radius: 50%;
}

/* Modal Styles */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

.modal-content {
  background: white;
  border-radius: 16px;
  padding: 32px;
  max-width: 450px;
  width: 90%;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
}

.modal-title {
  margin: 0 0 16px 0;
  font-size: 24px;
  font-weight: 600;
  color: #0f0f0f;
}

.modal-message {
  margin: 0 0 8px 0;
  font-size: 16px;
  line-height: 1.5;
  color: #333;
}

.modal-info {
  margin: 0 0 24px 0;
  font-size: 14px;
  color: #666;
  line-height: 1.5;
}

.path-display {
  display: block;
  margin-top: 8px;
  padding: 8px;
  background: rgba(0, 0, 0, 0.05);
  border-radius: 4px;
  font-family: monospace;
  word-break: break-all;
  font-size: 12px;
}

[data-theme="dark"] .path-display {
  background: rgba(255, 255, 255, 0.1);
}

.modal-actions {
  display: flex;
  gap: 12px;
  justify-content: flex-end;
}

.checkbox-option {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 16px;
  cursor: pointer;
  font-size: 14px;
  color: #333;
}

.checkbox-option input[type="checkbox"] {
  width: 18px;
  height: 18px;
  cursor: pointer;
  accent-color: #1a1a1a;
}

.checkbox-option input[type="checkbox"]:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

@media (prefers-color-scheme: dark) {
  .checkbox-option {
    color: #d0d0d0;
  }

  .checkbox-option input[type="checkbox"] {
    accent-color: #4a4a4a;
  }
}

.modal-btn {
  padding: 10px 20px;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: background-color 0.15s ease;
  display: flex;
  align-items: center;
  gap: 6px;
}

.modal-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.modal-btn-primary {
  background: #1a1a1a;
  color: white;
}

.modal-btn-primary:hover:not(:disabled) {
  background: #404040;
}

.modal-btn-secondary {
  background: #e0e0e0;
  color: #1a1a1a;
}

.modal-btn-secondary:hover:not(:disabled) {
  background: #c0c0c0;
}

/* Settings Content */
.settings-content {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
  flex: 1;
  min-height: 0;
}

/* Spacer to push uninstall to bottom */
.settings-spacer {
  flex: 1;
}

/* Update Available Section */
.update-available {
  background: rgba(76, 175, 80, 0.1) !important;
  border: 1px solid rgba(76, 175, 80, 0.3);
}

.update-icon {
  width: 18px;
  height: 18px;
  color: #4caf50;
  margin-right: 6px;
  vertical-align: middle;
}

/* Version Info */
.version-info {
  opacity: 0.8;
}

:root {
  --bg-color: #f6f6f6;
  --text-color: #0f0f0f;
  --card-bg: rgba(128, 128, 128, 0.1);
  --card-hover: rgba(128, 128, 128, 0.2);
  --action-bg: rgba(255, 255, 255, 0.5);
  --setting-bg: rgba(128, 128, 128, 0.1);
  --btn-bg: #1a1a1a;
  --btn-hover: #404040;
  --modal-bg: white;
  --modal-text: #333;
  --modal-info: #666;
}

[data-theme="dark"] {
  --bg-color: #2f2f2f;
  --text-color: #f6f6f6;
  --card-bg: rgba(255, 255, 255, 0.1);
  --card-hover: rgba(255, 255, 255, 0.15);
  --action-bg: rgba(47, 47, 47, 0.8);
  --setting-bg: rgba(255, 255, 255, 0.1);
  --btn-bg: #2a2a2a;
  --btn-hover: #4a4a4a;
  --modal-bg: #2f2f2f;
  --modal-text: #d0d0d0;
  --modal-info: #a0a0a0;
}

body {
  background-color: var(--bg-color);
  color: var(--text-color);
}

.lighthouse-card {
  background: var(--card-bg);
}

.lighthouse-card:hover {
  background: var(--card-hover);
}

.action-buttons {
  background: var(--action-bg);
}

.setting-item {
  background: var(--setting-bg);
}

.action-btn,
.action-btn-small,
.icon-btn-small,
.modal-btn-primary {
  background: var(--btn-bg);
  color: white;
}

.action-btn:hover:not(:disabled),
.action-btn-small:hover:not(:disabled),
.icon-btn-small:hover:not(:disabled),
.modal-btn-primary:hover:not(:disabled) {
  background: var(--btn-hover);
}

.modal-content {
  background: var(--modal-bg);
}

.modal-title {
  color: var(--text-color);
}

.modal-message {
  color: var(--modal-text);
}

.modal-btn-secondary {
  background: #e0e0e0;
  color: #1a1a1a;
}

.modal-btn-secondary:hover:not(:disabled) {
  background: #c0c0c0;
}

[data-theme="dark"] .modal-btn-secondary {
  background: #4a4a4a;
  color: #f6f6f6;
}

[data-theme="dark"] .modal-btn-secondary:hover:not(:disabled) {
  background: #5a5a5a;
}

[data-theme="dark"] .checkbox-option {
  color: #d0d0d0;
}

[data-theme="dark"] .checkbox-option input[type="checkbox"] {
  accent-color: #4a4a4a;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>
