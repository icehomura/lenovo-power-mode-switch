<template>
  <Settings
    v-if="page === 'settings'"
    :modes="modes"
    @back="page = 'main'"
  />

  <div v-else class="panel">
    <TitleBar>
      <button class="icon-btn" title="设置" @click="page = 'settings'">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
      </button>
      <button class="icon-btn" :disabled="busy" title="重新读取" @click="refresh">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12a9 9 0 1 1-2.64-6.36" />
          <path d="M21 3v6h-6" />
        </svg>
      </button>
      <!-- Floating window: picture-in-picture frame. The inner pane fills in
           while the floater is visible, so on/off reads from the icon itself. -->
      <button
        class="icon-btn"
        :class="{ active: floaterVisible }"
        :title="floaterVisible ? '隐藏悬浮窗' : '显示悬浮窗'"
        @click="toggleFloater"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round">
          <rect x="3" y="4" width="18" height="16" rx="2" />
          <rect
            x="12" y="11" width="7" height="6" rx="1"
            :fill="floaterVisible ? 'currentColor' : 'none'"
          />
        </svg>
      </button>
    </TitleBar>

    <div class="mode-list">
      <button
        v-for="mode in modes"
        :key="mode.id"
        class="mode-btn"
        :class="{ active: currentMode === mode.id, disabled: !selectable(mode) }"
        :style="{ '--mode-color': mode.color, '--mode-color-rgb': hexToRgb(mode.color) }"
        :disabled="busy || !selectable(mode)"
        @click="switchMode(mode)"
      >
        <span class="icon">{{ mode.icon }}</span>
        <span class="label">
          <span class="name">{{ mode.name }}</span>
          <span class="desc">{{ modeHint(mode) }}</span>
        </span>
        <span v-if="currentMode === mode.id" class="check">✓</span>
      </button>
    </div>

    <footer class="status-bar" :class="{ error: isError }">{{ status }}</footer>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import Settings from './views/Settings.vue';
import TitleBar from './components/TitleBar.vue';
import { loadSettings, updateFloating } from './composables/useSettings';

export interface ModeInfo {
  id: string;
  name: string;
  desc: string;
  icon: string;
  color: string;
  abi: number;
  supported: boolean;
  greyed: boolean;
}

const modes = ref<ModeInfo[]>([]);
const currentMode = ref<string | null>(null);
const status = ref('正在初始化…');
const isError = ref(false);
const busy = ref(false);
const page = ref<'main' | 'settings'>('main');
const floaterVisible = ref(false);

let unlistenModeChanged: (() => void) | undefined;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function hexToRgb(hex: string): string {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `${r}, ${g}, ${b}`;
}

function selectable(mode: ModeInfo): boolean {
  return mode.supported && !mode.greyed;
}

function modeHint(mode: ModeInfo): string {
  if (mode.greyed) return '硬件支持，但当前被系统置灰';
  if (!mode.supported) return '本机硬件不支持';
  return mode.desc;
}

function setStatus(text: string, error = false) {
  status.value = text;
  isError.value = error;
}

// ---------------------------------------------------------------------------
// Floating window
// ---------------------------------------------------------------------------

async function toggleFloater() {
  try {
    // The backend owns visibility, so let it flip the window and adopt the
    // answer — that keeps FloaterState and this button in lockstep.
    floaterVisible.value = await invoke<boolean>('toggle_floater');
    updateFloating({ enabled: floaterVisible.value });
  } catch (e) {
    setStatus(`悬浮窗切换失败：${e}`, true);
  }
}

// ---------------------------------------------------------------------------
// Mode operations
// ---------------------------------------------------------------------------

async function syncCurrentMode() {
  currentMode.value = await invoke<string>('get_current_mode');
}

async function refresh() {
  if (busy.value) return;
  busy.value = true;
  try {
    modes.value = await invoke<ModeInfo[]>('get_modes');
    await syncCurrentMode();
    const mode = modes.value.find(m => m.id === currentMode.value);
    setStatus(`当前模式：${mode?.name ?? currentMode.value}`);
  } catch (e) {
    setStatus(`读取失败：${e}`, true);
  } finally {
    busy.value = false;
  }
}

async function switchMode(mode: ModeInfo) {
  if (busy.value || !selectable(mode)) return;

  busy.value = true;
  setStatus(`正在切换到 ${mode.name}…`);
  try {
    // The Rust side broadcasts `mode-changed` to every window on its own,
    // so there is nothing to fan out from here.
    await invoke<string>('set_mode', { mode: mode.id });
    await syncCurrentMode();
    setStatus(`已切换到：${mode.name}`);
  } catch (e) {
    setStatus(`切换失败：${e}`, true);
    try {
      await syncCurrentMode();
    } catch {
      currentMode.value = null;
    }
  } finally {
    busy.value = false;
  }
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

onMounted(async () => {
  await loadSettings();

  // Seed the floater button from the backend, which owns that state.
  try {
    const st = await invoke<{ visible: boolean }>('get_floater_status');
    floaterVisible.value = st.visible;
  } catch {
    // floater window not configured — leave the button inactive
  }

  unlistenModeChanged = await listen<string>('mode-changed', async () => {
    await syncCurrentMode();
  });

  const conn = await invoke<string>('check_connection');
  if (conn !== 'connected') {
    setStatus(`桥接不可用：${conn.replace(/^disconnected:\s*/, '')}`, true);
    return;
  }
  await refresh();

  // The backend polls the hardware and broadcasts every change, so the window
  // only needs to re-read when it regains focus.
  window.addEventListener('focus', refresh);
});

onUnmounted(() => {
  window.removeEventListener('focus', refresh);
  unlistenModeChanged?.();
});
</script>
