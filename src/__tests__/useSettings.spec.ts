import { describe, it, expect, vi, beforeEach } from "vitest";

// We test the composable logic directly by importing the module
// with mocked dependencies.

// Override the global mock from setup.ts — we want the real composable
vi.unmock("@/composables/useSettings");

// Mock the Tauri invoke API
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string, _args?: unknown) => {
    if (cmd === "get_settings") return {};
    if (cmd === "save_settings") return undefined;
    return undefined;
  }),
}));

// Reset modules to get fresh composable state
beforeEach(async () => {
  vi.resetModules();
});

// ========================================================================
// useSettings composable
// ========================================================================

describe("useSettings - loadSettings", () => {
  it("loads default settings when store is empty", async () => {
    const { loadSettings, settings } = await import("@/composables/useSettings");
    await loadSettings();

    expect(settings.value).toBeDefined();
    expect(settings.value.autostart).toBe(false);
    expect(settings.value.floating).toBeDefined();
    expect(settings.value.floating.style).toBe("capsule");
    expect(settings.value.floating.enabled).toBe(false);
  });
});

describe("useSettings - updateSettings", () => {
  it("patches top-level settings", async () => {
    const { updateSettings, settings } = await import("@/composables/useSettings");

    updateSettings({ autostart: true });
    expect(settings.value.autostart).toBe(true);
  });

  it("preserves other fields when patching", async () => {
    const { updateSettings, settings } = await import("@/composables/useSettings");

    const originalStyle = settings.value.floating.style;
    updateSettings({ autostart: true });
    expect(settings.value.floating.style).toBe(originalStyle);
  });
});

describe("useSettings - updateFloating", () => {
  it("patches floating window settings", async () => {
    const { updateFloating, settings } = await import("@/composables/useSettings");

    updateFloating({ style: "circle" });
    expect(settings.value.floating.style).toBe("circle");
  });

  it("patches floating position", async () => {
    const { updateFloating, settings } = await import("@/composables/useSettings");

    updateFloating({ position: "bottom-left" });
    expect(settings.value.floating.position).toBe("bottom-left");
  });

  it("patches floating enabled", async () => {
    const { updateFloating, settings } = await import("@/composables/useSettings");

    updateFloating({ enabled: true });
    expect(settings.value.floating.enabled).toBe(true);
  });
});

describe("useSettings - resetSettings", () => {
  it("resets all settings to defaults", async () => {
    const { updateSettings, resetSettings, settings } = await import(
      "@/composables/useSettings"
    );

    // Modify settings first
    updateSettings({ autostart: true });
    expect(settings.value.autostart).toBe(true);

    resetSettings();
    expect(settings.value.autostart).toBe(false);
    expect(settings.value.floating.enabled).toBe(false);
    expect(settings.value.floating.style).toBe("capsule");
  });
});
