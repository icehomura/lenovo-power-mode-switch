<template>
  <div class="floater-root" @mousedown.left="onDragStart">
    <div class="floater-drag-region" />

    <div
      class="floater-inner"
      :class="`shape-${shape}`"
      @click.stop="onMainClick"
      @contextmenu.prevent="toggleShape"
    >
      <!-- Capsule: icon + mode name (the wide, labelled style) -->
      <div v-if="shape === 'capsule'" class="floater-capsule">
        <span class="floater-icon">{{ currentMeta.icon }}</span>
        <span class="floater-name">{{ currentMeta.name }}</span>
      </div>

      <!-- Circle: icon only (the compact style) -->
      <div v-else class="floater-circle">
        <span class="floater-circle-icon">{{ currentMeta.icon }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { LogicalSize, PhysicalPosition } from '@tauri-apps/api/dpi';
import { listen } from '@tauri-apps/api/event';

type FloaterShape = 'capsule' | 'circle';

/** Window size per style, in logical pixels. */
const SHAPE_SIZE: Record<FloaterShape, { w: number; h: number }> = {
  capsule: { w: 72, h: 36 },
  circle: { w: 36, h: 36 },
};

/** Canonical cycle order used by click-to-switch. */
const MODE_META: Record<string, { name: string; icon: string }> = {
  intelligent: { name: '智能', icon: '⚖️' },
  saving:      { name: '省电', icon: '🌿' },
  performance: { name: '性能', icon: '⚡' },
  geek:        { name: '极客', icon: '🚀' },
};

const shape = ref<FloaterShape>('capsule');
const currentMode = ref('intelligent');

const currentMeta = computed(() => MODE_META[currentMode.value] ?? MODE_META.intelligent);

// ---------------------------------------------------------------------------
// Drag
//
// The browser fires `click` on mouseup no matter how far the pointer travelled,
// so a repositioning drag would also run the click handler and cycle the power
// mode. A press therefore has to be classified as either a click or a drag, and
// only the former may reach `onMainClick`.
// ---------------------------------------------------------------------------

/**
 * Pointer travel, in CSS pixels, past which a press counts as a drag.
 *
 * Small enough that any deliberate move registers — and since the window only
 * follows the pointer once this is exceeded, it doubles as the jitter filter
 * that keeps a shaky click from being swallowed.
 */
const DRAG_THRESHOLD = 4;

let dragging = false;
/** Set once the pointer travels past DRAG_THRESHOLD during the current press. */
let dragMoved = false;
let dragStartX = 0;
let dragStartY = 0;
let winStartX = 0;
let winStartY = 0;

function onDragStart(e: MouseEvent) {
  if (e.button !== 0) return;
  dragging = true;
  dragMoved = false;
  dragStartX = e.screenX;
  dragStartY = e.screenY;
  // Listeners are attached synchronously. Querying the window origin is an IPC
  // round-trip, and awaiting it here used to leave a gap in which the first
  // mouse movements — exactly the ones a quick flick produces — went unseen.
  document.addEventListener('mousemove', onDragMove);
  document.addEventListener('mouseup', onDragEnd);
  getCurrentWindow()
    .outerPosition()
    .then((pos) => {
      winStartX = pos.x;
      winStartY = pos.y;
    })
    .catch(() => {
      // Leave the cached origin in place; positioning just lags by one drag.
    });
}

function onDragMove(e: MouseEvent) {
  if (!dragging) return;

  const dx = e.screenX - dragStartX;
  const dy = e.screenY - dragStartY;

  // Below the threshold the window stays put, so a click never nudges it.
  if (!dragMoved) {
    if (Math.hypot(dx, dy) <= DRAG_THRESHOLD) return;
    dragMoved = true;
  }

  // `outerPosition()` returns PHYSICAL pixels while the pointer delta is in CSS
  // pixels, so the delta must be scaled. Mixing the two made the window drift
  // away from the cursor on scaled (HiDPI) displays.
  const scale = window.devicePixelRatio || 1;
  const x = Math.round(winStartX + dx * scale);
  const y = Math.round(winStartY + dy * scale);
  getCurrentWindow().setPosition(new PhysicalPosition(x, y));
}

function onDragEnd() {
  dragging = false;
  document.removeEventListener('mousemove', onDragMove);
  document.removeEventListener('mouseup', onDragEnd);
}

// ---------------------------------------------------------------------------
// A left click cycles to the next supported mode; right-click flips the style.
// ---------------------------------------------------------------------------
function onMainClick() {
  // The click that follows a drag is discarded here rather than prevented
  // upstream — there is no way to stop the browser synthesising it. The flag is
  // cleared on the next mousedown, so a drag that ends off-window (and so never
  // produces a click) cannot swallow a later, genuine one.
  if (dragMoved) {
    dragMoved = false;
    return;
  }
  invoke('cycle_mode').catch(() => {});
}

// ---------------------------------------------------------------------------
// Style toggle (right-click)
// ---------------------------------------------------------------------------
async function toggleShape() {
  shape.value = shape.value === 'capsule' ? 'circle' : 'capsule';
  applyWindowSize();
  await invoke('set_floater_style', { style: shape.value });
}

function applyWindowSize() {
  const { w, h } = SHAPE_SIZE[shape.value];
  getCurrentWindow().setSize(new LogicalSize(w, h));
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------
const unlisteners: (() => void)[] = [];

onMounted(async () => {
  try {
    currentMode.value = await invoke<string>('get_current_mode');
    const status = await invoke<{
      visible: boolean;
      style: FloaterShape;
      position: string;
    }>('get_floater_status');
    shape.value = status.style;
  } catch {
    // fall back to the defaults above
  }
  applyWindowSize();

  // The Rust side polls the hardware once and broadcasts every change, so the
  // floater only listens — polling here as well would just duplicate that.
  unlisteners.push(
    await listen<string>('mode-changed', (e) => {
      currentMode.value = e.payload;
    })
  );

  unlisteners.push(
    await listen<string>('floater-style-changed', (e) => {
      shape.value = e.payload as FloaterShape;
      applyWindowSize();
    })
  );
});

onUnmounted(() => {
  unlisteners.forEach(fn => fn());
  // A drag left in flight would otherwise keep its document listeners attached
  // for the lifetime of the page.
  onDragEnd();
});
</script>
