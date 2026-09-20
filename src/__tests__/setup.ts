/**
 * Vitest global setup file.
 *
 * Provides jsdom globals and Tauri API mocks needed by all frontend tests.
 */
import { vi } from "vitest";

// ---------------------------------------------------------------------------
// Tauri API mock
// ---------------------------------------------------------------------------

const commandHandlers = new Map<string, (...args: unknown[]) => unknown>();

export function resetMocks() {
  commandHandlers.clear();
}

export function mockInvoke(
  command: string,
  handler: (...args: unknown[]) => unknown
) {
  commandHandlers.set(command, handler);
}

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (command: string, args?: Record<string, unknown>) => {
    const handler = commandHandlers.get(command);
    if (!handler) {
      throw new Error(`[TauriMock] 未注册的命令: ${command}`);
    }
    return handler(args);
  }),
}));

// ---------------------------------------------------------------------------
// Event system mock
// ---------------------------------------------------------------------------

const eventListeners = new Map<string, ((payload: unknown) => void)[]>();

export function emitToListeners(event: string, payload: unknown) {
  const listeners = eventListeners.get(event) || [];
  for (const fn of listeners) fn(payload);
}

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (event: string, cb: (...args: unknown[]) => void) => {
    if (!eventListeners.has(event)) eventListeners.set(event, []);
    eventListeners.get(event)!.push(cb as (payload: unknown) => void);
    return () => {
      const arr = eventListeners.get(event);
      if (arr) {
        const idx = arr.indexOf(cb as (payload: unknown) => void);
        if (idx >= 0) arr.splice(idx, 1);
      }
    };
  }),
  emit: vi.fn().mockResolvedValue(undefined),
}));

// ---------------------------------------------------------------------------
// WebviewWindow mock
// ---------------------------------------------------------------------------

vi.mock("@tauri-apps/api/webviewWindow", () => ({
  WebviewWindow: {
    getByLabel: vi.fn().mockResolvedValue(null),
  },
}));

// ---------------------------------------------------------------------------
// Tauri window mock (for Floater drag)
// ---------------------------------------------------------------------------

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: vi.fn().mockReturnValue({
    outerPosition: vi.fn().mockResolvedValue({ x: 0, y: 0 }),
    setPosition: vi.fn().mockResolvedValue(undefined),
    setSize: vi.fn().mockResolvedValue(undefined),
    // Window controls used by TitleBar
    startDragging: vi.fn().mockResolvedValue(undefined),
    minimize: vi.fn().mockResolvedValue(undefined),
    maximize: vi.fn().mockResolvedValue(undefined),
    unmaximize: vi.fn().mockResolvedValue(undefined),
    isMaximized: vi.fn().mockResolvedValue(false),
    setAlwaysOnTop: vi.fn().mockResolvedValue(undefined),
    close: vi.fn().mockResolvedValue(undefined),
    onResized: vi.fn().mockResolvedValue(() => {}),
  }),
}));

vi.mock("@tauri-apps/api/dpi", () => ({
  // Must be regular functions (not arrows) so `new X(...)` works
  LogicalSize: function LogicalSize(w: number, h: number) { return { w, h }; },
  LogicalPosition: function LogicalPosition(x: number, y: number) { return { x, y }; },
  PhysicalPosition: function PhysicalPosition(x: number, y: number) { return { x, y }; },
}));

// ---------------------------------------------------------------------------
// Plugin mocks
// ---------------------------------------------------------------------------

vi.mock("@tauri-apps/plugin-autostart", () => ({
  isEnabled: vi.fn().mockResolvedValue(false),
  enable: vi.fn().mockResolvedValue(undefined),
  disable: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@tauri-apps/plugin-store", () => {
  const store = new Map<string, unknown>();
  const api = {
    get: vi.fn(async (key: string) => store.get(key) ?? null),
    set: vi.fn(async (key: string, value: unknown) => {
      store.set(key, value);
    }),
    save: vi.fn(async () => {}),
  };
  return {
    LazyStore: vi.fn().mockImplementation(function () { return api; }),
    load: vi.fn().mockResolvedValue(api),
  };
});

vi.mock("@/store/settings", () => {
  const _floaterSettings = {
    autostart: false,
    showFloater: false,
    floaterStyle: "capsule" as "capsule" | "circle",
    floaterX: 100,
    floaterY: 100,
  };
  return {
    settings: _floaterSettings,
    loadSettings: vi.fn().mockResolvedValue(undefined),
  };
});

// ---------------------------------------------------------------------------
// jsdom globals that Tauri WebView runtime expects
// ---------------------------------------------------------------------------

Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});
