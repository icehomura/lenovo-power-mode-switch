<template>
  <div class="panel settings-panel">
    <TitleBar>
      <template #brand>
        <button class="back-btn" title="返回" @click="$emit('back')">←</button>
        <h1 class="brand-title">设置</h1>
      </template>
      <button class="reset-btn" title="重置所有设置" @click="onReset">↺</button>
    </TitleBar>

    <div class="settings-scroll">
      <!-- 快捷键绑定 -->
      <section class="setting-section">
        <h2 class="section-title">快捷键绑定</h2>
        <p class="section-desc">点击输入框后按下快捷键组合，Esc 取消</p>
        <div class="hotkey-list">
          <div
            v-for="mode in modes"
            :key="mode.id"
            class="hotkey-row"
            :class="{ disabled: !mode.supported || mode.greyed }"
          >
            <span class="hotkey-label">
              <span class="mode-dot" :style="{ background: mode.color }"></span>
              {{ mode.name }}
            </span>
            <div class="hotkey-input-wrap">
              <input
                class="hotkey-input"
                :class="{ recording: recordingMode === mode.id, bound: shortcutFor(mode.id) }"
                type="text"
                readonly
                :value="shortcutFor(mode.id) || '未绑定'"
                :disabled="!mode.supported || mode.greyed"
                @focus="startRecording(mode.id)"
                @keydown.prevent="onKeyDown($event, mode.id)"
              />
              <button
                v-if="shortcutFor(mode.id)"
                class="hotkey-clear"
                title="解除绑定"
                @click="unbind(mode.id)"
              >✕</button>
            </div>
          </div>

          <!-- Cycle binding: one shortcut that walks through the modes -->
          <div class="hotkey-row">
            <span class="hotkey-label">
              <span class="mode-dot" style="background: #8E8E93"></span>
              循环切换模式
            </span>
            <div class="hotkey-input-wrap">
              <input
                class="hotkey-input"
                :class="{ recording: recordingMode === CYCLE_KEY, bound: shortcutFor(CYCLE_KEY) }"
                type="text"
                readonly
                :value="shortcutFor(CYCLE_KEY) || '未绑定'"
                @focus="startRecording(CYCLE_KEY)"
                @keydown.prevent="onKeyDown($event, CYCLE_KEY)"
              />
              <button
                v-if="shortcutFor(CYCLE_KEY)"
                class="hotkey-clear"
                title="解除绑定"
                @click="unbind(CYCLE_KEY)"
              >✕</button>
            </div>
          </div>
        </div>
        <button
          v-if="hasAnyShortcut"
          class="clear-all-btn"
          @click="clearAll"
        >清除全部快捷键</button>
      </section>

      <!-- 悬浮窗设置 -->
      <section class="setting-section">
        <h2 class="section-title">悬浮窗</h2>
        <div class="setting-row">
          <span class="setting-label">启用悬浮窗</span>
          <button
            class="toggle"
            :class="{ on: settings.floating.enabled }"
            @click="toggleFloating"
          >
            <span class="toggle-thumb"></span>
          </button>
        </div>
        <div class="setting-row">
          <span class="setting-label">快速显示 / 隐藏</span>
          <button class="mini-btn" @click="toggleFloaterNow">
            {{ floaterVisible ? "隐藏" : "显示" }}
          </button>
        </div>
        <template v-if="settings.floating.enabled">
          <div class="setting-row">
            <span class="setting-label">样式</span>
            <div class="segmented">
              <button
                class="seg-btn"
                :class="{ active: settings.floating.style === 'capsule' }"
                @click="setStyle('capsule')"
              >胶囊型</button>
              <button
                class="seg-btn"
                :class="{ active: settings.floating.style === 'circle' }"
                @click="setStyle('circle')"
              >圆形</button>
            </div>
          </div>
          <div class="setting-row">
            <span class="setting-label">悬浮窗初始位置</span>
            <div class="segmented seg-position">
              <button
                v-for="pos in positionOptions"
                :key="pos.value"
                class="seg-btn"
                :class="{ active: settings.floating.position === pos.value }"
                @click="setPosition(pos.value)"
              >{{ pos.label }}</button>
            </div>
          </div>
          <p class="setting-hint">
            新建悬浮窗时停靠的角落；选择后悬浮窗会立即移过去。
            之后可自由拖动，隐藏再显示仍会保留你拖动的位置。
          </p>
        </template>
      </section>

      <!-- 开机自启动 -->
      <section class="setting-section">
        <h2 class="section-title">系统</h2>
        <div class="setting-row">
          <span class="setting-label">开机自启动</span>
          <button
            class="toggle"
            :class="{ on: settings.autostart }"
            @click="toggleAutostart"
          >
            <span class="toggle-thumb"></span>
          </button>
        </div>
      </section>
    </div>

    <footer class="settings-footer">
      <span v-if="lastError" class="error-indicator">{{ lastError }}</span>
      <span v-else-if="saved" class="save-indicator">已保存</span>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import {
  settings,
  updateSettings,
  updateFloating,
  resetSettings,
} from '../composables/useSettings';
import { DEFAULT_SETTINGS } from '../types/settings';
import TitleBar from '../components/TitleBar.vue';
import {
  enable,
  isEnabled,
  disable,
} from '@tauri-apps/plugin-autostart';
import type { ModeInfo } from '../App.vue';

defineProps<{
  modes: ModeInfo[];
}>();

defineEmits<{
  back: [];
}>();

interface ShortcutBinding {
  mode: string;
  shortcut: string;
}

/** Pseudo mode key for the "cycle through modes" binding. */
const CYCLE_KEY = 'cycle';

const saved = ref(false);
const lastError = ref<string | null>(null);
const shortcuts = ref<ShortcutBinding[]>([]);
const recordingMode = ref<string | null>(null);
/** Mirrors the backend's floater visibility, which the window itself owns. */
const floaterVisible = ref(false);

const positionOptions = [
  { value: 'top-left' as const, label: '左上' },
  { value: 'top-right' as const, label: '右上' },
  { value: 'bottom-left' as const, label: '左下' },
  { value: 'bottom-right' as const, label: '右下' },
];

function shortcutFor(modeId: string): string | undefined {
  return shortcuts.value.find(s => s.mode === modeId)?.shortcut;
}

const hasAnyShortcut = computed(() => shortcuts.value.length > 0);

// ---------------------------------------------------------------------------
// Shortcut backend operations
// ---------------------------------------------------------------------------

async function loadShortcuts() {
  try {
    shortcuts.value = await invoke<ShortcutBinding[]>('get_shortcuts');
  } catch {
    shortcuts.value = [];
  }
}

async function registerShortcut(modeId: string, shortcut: string) {
  try {
    await invoke<string>('register_shortcut', { mode: modeId, shortcut });
    await loadShortcuts();
    flashSaved();
  } catch (e) {
    console.error('绑定失败:', e);
  }
}

async function unbind(modeId: string) {
  try {
    await invoke<string>('unregister_shortcut', { mode: modeId });
    await loadShortcuts();
    flashSaved();
  } catch (e) {
    console.error('解除失败:', e);
  }
}

async function clearAll() {
  try {
    await invoke<string>('clear_shortcuts');
    await loadShortcuts();
    flashSaved();
  } catch (e) {
    console.error('清除失败:', e);
  }
}

// ---------------------------------------------------------------------------
// Keyboard capture
// ---------------------------------------------------------------------------

/** Map KeyboardEvent.code to the key name Rust's parse_shortcut expects. */
function codeToKey(code: string): string | null {
  if (code.startsWith('Digit')) return code.slice(5);
  if (code.startsWith('Key')) return code.slice(3);
  if (code.startsWith('F') && code.length <= 3) return code; // F1..F12
  switch (code) {
    case 'Space': return 'Space';
    case 'Backspace': return 'Backspace';
    case 'Delete': return 'Delete';
    case 'Insert': return 'Insert';
    case 'Enter': return 'Enter';
    case 'Tab': return 'Tab';
    case 'Escape': return 'Escape';
    case 'ArrowUp': return 'Up';
    case 'ArrowDown': return 'Down';
    case 'ArrowLeft': return 'Left';
    case 'ArrowRight': return 'Right';
    case 'Home': return 'Home';
    case 'End': return 'End';
    case 'PageUp': return 'PageUp';
    case 'PageDown': return 'PageDown';
    default: return null;
  }
}

function startRecording(modeId: string) {
  recordingMode.value = modeId;
}

function onKeyDown(e: KeyboardEvent, modeId: string) {
  if (e.key === 'Escape') {
    recordingMode.value = null;
    (e.target as HTMLElement).blur();
    return;
  }

  // 跳过纯修饰键
  if (['Control', 'Alt', 'Shift', 'Meta'].includes(e.key)) return;
  // 至少需要一个修饰键
  if (!e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey) return;

  const keyName = codeToKey(e.code);
  if (!keyName) return;

  const parts: string[] = [];
  if (e.ctrlKey) parts.push('Ctrl');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  if (e.metaKey) parts.push('Super');

  parts.push(keyName);

  const shortcut = parts.join('+');
  recordingMode.value = null;
  (e.target as HTMLElement).blur();
  registerShortcut(modeId, shortcut);
}

// ---------------------------------------------------------------------------
// Floating window & autostart
// ---------------------------------------------------------------------------

/**
 * Record a failed backend call and surface it in the footer.
 *
 * These used to only `console.error`, which is invisible in a packaged desktop
 * app — the toggle simply looked inert.
 */
function fail(action: string, e: unknown): void {
  lastError.value = `${action}失败：${e}`;
  console.error(`${action}失败:`, e);
}

async function toggleFloating(): Promise<void> {
  lastError.value = null;
  try {
    const next = !settings.value.floating.enabled;
    // Enabling really creates the window and disabling tears it down, so
    // turning it back on is a genuine "new floater" rather than a re-show.
    await invoke(next ? "show_floater" : "destroy_floater");
    updateFloating({ enabled: next });
    floaterVisible.value = next;
    flashSaved();
  } catch (e) {
    fail("悬浮窗切换", e);
  }
}

/**
 * Flip the floater's visibility through the backend, which owns that state.
 *
 * Also covers the case where the window was closed (Alt+F4) while the
 * "enabled" switch stayed on: the switch alone cannot bring it back.
 */
async function toggleFloaterNow(): Promise<void> {
  lastError.value = null;
  try {
    floaterVisible.value = await invoke<boolean>("toggle_floater");
    updateFloating({ enabled: floaterVisible.value });
    flashSaved();
  } catch (e) {
    fail("悬浮窗显隐", e);
  }
}

async function setStyle(style: "capsule" | "circle"): Promise<void> {
  lastError.value = null;
  try {
    await invoke("set_floater_style", { style });
    updateFloating({ style });
    flashSaved();
  } catch (e) {
    fail("悬浮窗样式切换", e);
  }
}

async function setPosition(
  position: "top-left" | "top-right" | "bottom-left" | "bottom-right"
): Promise<void> {
  lastError.value = null;
  try {
    await invoke("set_floater_position", { position });
    updateFloating({ position });
    flashSaved();
  } catch (e) {
    fail("悬浮窗位置切换", e);
  }
}

async function toggleAutostart(): Promise<void> {
  lastError.value = null;
  try {
    const next = !settings.value.autostart;
    if (next) {
      await enable();
    } else {
      await disable();
    }
    updateSettings({ autostart: next });
    flashSaved();
  } catch (e) {
    fail("自启动切换", e);
  }
}

function onReset(): void {
  resetSettings();
  disable().catch(() => {});
  // Reset backend-owned floater state too, not just the persisted settings.
  invoke("set_floater_style", { style: DEFAULT_SETTINGS.floating.style }).catch(() => {});
  invoke("set_floater_position", { position: DEFAULT_SETTINGS.floating.position }).catch(() => {});
  // 清除所有快捷键
  invoke<string>('clear_shortcuts').catch(() => {});
  loadShortcuts();
  flashSaved();
}

function flashSaved(): void {
  saved.value = true;
  setTimeout(() => { saved.value = false; }, 1500);
}

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------

onMounted(async () => {
  await loadShortcuts();

  // The backend's floater state starts at its defaults on every launch, so
  // push the persisted preferences across before anything reads them.
  invoke("set_floater_style", { style: settings.value.floating.style }).catch(() => {});
  invoke("set_floater_position", { position: settings.value.floating.position }).catch(
    () => {}
  );
  if (settings.value.floating.enabled) {
    invoke("show_floater").catch(() => {});
  }

  // Seed the quick show/hide button from the backend, which owns the window.
  try {
    const st = await invoke<{ visible: boolean }>("get_floater_status");
    floaterVisible.value = st.visible;
  } catch {
    // floater window not configured — leave the button showing "显示"
  }

  // 同步自启动状态
  try {
    const osEnabled = await isEnabled();
    if (osEnabled !== settings.value.autostart) {
      updateSettings({ autostart: osEnabled });
    }
  } catch (e) {
    fail("读取自启动状态", e);
  }
});
</script>
