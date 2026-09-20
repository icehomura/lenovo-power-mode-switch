<template>
  <header class="titlebar" @mousedown="onMouseDown" @dblclick="onDblClick">
    <div class="titlebar-brand">
      <slot name="brand">
        <span class="brand-dot" />
        <h1 class="brand-title">{{ title }}</h1>
      </slot>
    </div>

    <div class="titlebar-actions">
      <!-- App-specific actions supplied by the parent. They deliberately share
           one row with the window controls — no separator, no grouping. -->
      <slot />

      <!-- Window controls. Order matches Windows: pin, min, max, close. -->
      <button
        class="win-btn"
        :class="{ on: pinned }"
        title="置顶"
        @click="togglePin"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 17v5" />
          <path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1z" />
        </svg>
      </button>

      <button class="win-btn" title="最小化" @click="onMinimize">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <path d="M5 12h14" />
        </svg>
      </button>

      <button class="win-btn" title="最大化" @click="toggleMaximize">
        <svg v-if="!isMaximized" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round">
          <rect x="5" y="5" width="14" height="14" rx="1.5" />
        </svg>
        <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round">
          <rect x="8" y="8" width="11" height="11" rx="1.5" />
          <path d="M5 16V6.5A1.5 1.5 0 0 1 6.5 5H16" />
        </svg>
      </button>

      <button class="win-btn win-btn--close" title="关闭到托盘" @click="onClose">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <path d="M18 6 6 18M6 6l12 12" />
        </svg>
      </button>
    </div>
  </header>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';

withDefaults(defineProps<{ title?: string }>(), {
  title: 'Lenovo Power Mode Switch',
});

const win = getCurrentWindow();
const pinned = ref(false);
const isMaximized = ref(false);

let unlistenResized: (() => void) | undefined;

/**
 * Interactive elements must not start a window drag — otherwise clicking a
 * button would also move the window.
 */
function isInteractive(target: EventTarget | null): boolean {
  return !!(target as HTMLElement | null)?.closest(
    'button, a, input, select, textarea'
  );
}

function onMouseDown(e: MouseEvent) {
  if (e.button !== 0 || isInteractive(e.target)) return;
  e.preventDefault();
  win?.startDragging?.();
}

function onDblClick(e: MouseEvent) {
  if (isInteractive(e.target)) return;
  toggleMaximize();
}

async function togglePin() {
  pinned.value = !pinned.value;
  try {
    await win?.setAlwaysOnTop?.(pinned.value);
  } catch {
    pinned.value = !pinned.value; // revert if the OS refused
  }
}

function onMinimize() {
  win?.minimize?.();
}

async function toggleMaximize() {
  try {
    if (await win?.isMaximized?.()) {
      await win?.unmaximize?.();
    } else {
      await win?.maximize?.();
    }
  } catch {
    // window controls unavailable (e.g. in tests)
  }
  await syncMaximized();
}

async function onClose() {
  // The Rust side intercepts this and hides to tray instead of exiting.
  await win?.close?.();
}

async function syncMaximized() {
  try {
    isMaximized.value = !!(await win?.isMaximized?.());
  } catch {
    // ignore
  }
}

onMounted(async () => {
  await syncMaximized();
  try {
    unlistenResized = await win?.onResized?.(() => {
      syncMaximized();
    });
  } catch {
    // ignore
  }
});

onUnmounted(() => {
  unlistenResized?.();
});
</script>
