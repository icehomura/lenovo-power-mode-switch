import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import Settings from "@/views/Settings.vue";
import { resetMocks, mockInvoke } from "./setup";
import { TEST_MODES } from "./fixtures/modes";
import { settings } from "@/composables/useSettings";

function setupDefaults() {
  mockInvoke("get_shortcuts", () => []);
  mockInvoke("register_shortcut", () => "ok");
  mockInvoke("unregister_shortcut", () => "ok");
  mockInvoke("clear_shortcuts", () => "ok");
  mockInvoke("get_settings", () => ({}));
  mockInvoke("save_settings", () => undefined);
  mockInvoke("get_floater_status", () => ({ visible: false, style: "capsule" }));
  mockInvoke("set_floater_style", () => {});
  mockInvoke("set_floater_position", () => {});
  mockInvoke("show_floater", () => {});
  mockInvoke("destroy_floater", () => {});
  mockInvoke("toggle_floater", () => true);
}

// ========================================================================
// Rendering
// ========================================================================

describe("Settings.vue - Rendering", () => {
  beforeEach(() => {
    resetMocks();
    setupDefaults();
  });

  it("renders the settings title", () => {
    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    expect(wrapper.find("h1").text()).toBe("设置");
  });

  it("renders back button", () => {
    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    expect(wrapper.find(".back-btn").exists()).toBe(true);
  });

  it("emits back event on back button click", async () => {
    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    await wrapper.find(".back-btn").trigger("click");
    expect(wrapper.emitted("back")).toHaveLength(1);
  });

  it("renders a hotkey row per mode plus the cycle row", () => {
    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    const rows = wrapper.findAll(".hotkey-row");
    expect(rows).toHaveLength(5);
  });

  it("shows mode names in hotkey labels", () => {
    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    const labels = wrapper.findAll(".hotkey-label").map((el) => el.text().trim());
    expect(labels).toEqual([
      "智能模式",
      "省电模式",
      "性能模式",
      "极客模式",
      "循环切换模式",
    ]);
  });

  it("shows '未绑定' for unbound modes", () => {
    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    const inputs = wrapper.findAll(".hotkey-input");
    for (const input of inputs) {
      expect((input.element as HTMLInputElement).value).toBe("未绑定");
    }
  });

  it("shows bound shortcut value", async () => {
    mockInvoke("get_shortcuts", () => [
      { mode: "intelligent", shortcut: "Ctrl+Alt+1" },
    ]);
    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    await flushPromises();
    const inputs = wrapper.findAll(".hotkey-input");
    expect((inputs[0].element as HTMLInputElement).value).toBe("Ctrl+Alt+1");
  });

  it("renders clear-all button when shortcuts exist", async () => {
    mockInvoke("get_shortcuts", () => [
      { mode: "intelligent", shortcut: "Ctrl+Alt+1" },
    ]);
    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    await flushPromises();
    expect(wrapper.find(".clear-all-btn").exists()).toBe(true);
  });

  it("hides clear-all button when no shortcuts", () => {
    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    expect(wrapper.find(".clear-all-btn").exists()).toBe(false);
  });
});

// ========================================================================
// Floating window settings
// ========================================================================

describe("Settings.vue - Floating window settings", () => {
  beforeEach(() => {
    resetMocks();
    setupDefaults();
    // The style / corner controls are behind the enable switch. These tests
    // drive that state directly: `settings` is a module-level singleton, so
    // clicking the toggle would leak into every later test in this file.
    settings.value.floating.enabled = true;
  });

  afterEach(() => {
    settings.value.floating.enabled = false;
  });

  it("renders floating window section", () => {
    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    const sections = wrapper.findAll(".section-title");
    const titles = sections.map((el) => el.text());
    expect(titles).toContain("悬浮窗");
  });

  it("renders enable toggle for floating window", () => {
    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    const toggles = wrapper.findAll(".toggle");
    expect(toggles.length).toBeGreaterThanOrEqual(1);
  });

  it("labels the corner setting as the initial position", () => {
    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    const labels = wrapper.findAll(".setting-label").map((el) => el.text());
    expect(labels).toContain("悬浮窗初始位置");
  });

  it("explains that the corner is only the starting position", () => {
    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    const hint = wrapper.find(".setting-hint");
    expect(hint.exists()).toBe(true);
    // The wording must set the expectation that a drag is not overridden.
    expect(hint.text()).toContain("保留");
  });

  it("offers all four corners", () => {
    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    const labels = wrapper.findAll(".seg-position .seg-btn").map((el) => el.text());
    expect(labels).toEqual(["左上", "右上", "左下", "右下"]);
  });
});

// ========================================================================
// Autostart settings
// ========================================================================

describe("Settings.vue - Autostart settings", () => {
  beforeEach(() => {
    resetMocks();
    setupDefaults();
  });

  it("renders system section with autostart", () => {
    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    const labels = wrapper.findAll(".setting-label").map((el) => el.text());
    expect(labels).toContain("开机自启动");
  });
});

// ========================================================================
// Backend failures must be visible, not swallowed into console.error
// ========================================================================

describe("Settings.vue - Error reporting", () => {
  beforeEach(() => {
    resetMocks();
    setupDefaults();
  });

  it("shows the floater toggle failure instead of a false '已保存'", async () => {
    mockInvoke("show_floater", () => {
      throw new Error("创建悬浮窗失败: WebView2 不可用");
    });

    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    const toggle = wrapper.findAll(".toggle")[0];
    await toggle.trigger("click");
    await flushPromises();

    const err = wrapper.find(".error-indicator");
    expect(err.exists()).toBe(true);
    expect(err.text()).toContain("悬浮窗切换失败");
    expect(err.text()).toContain("WebView2 不可用");
    expect(wrapper.find(".save-indicator").exists()).toBe(false);
  });

  it("shows the autostart failure", async () => {
    const autostart = await import("@tauri-apps/plugin-autostart");
    (autostart.enable as any).mockRejectedValueOnce(new Error("拒绝访问注册表"));

    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    const toggles = wrapper.findAll(".toggle");
    await toggles[toggles.length - 1].trigger("click");
    await flushPromises();

    const err = wrapper.find(".error-indicator");
    expect(err.exists()).toBe(true);
    expect(err.text()).toContain("自启动切换失败");
    expect(err.text()).toContain("拒绝访问注册表");
  });

  it("clears a previous error before the next attempt", async () => {
    const autostart = await import("@tauri-apps/plugin-autostart");
    (autostart.enable as any).mockRejectedValueOnce(new Error("第一次失败"));

    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    const toggles = wrapper.findAll(".toggle");
    const autostartToggle = toggles[toggles.length - 1];

    await autostartToggle.trigger("click");
    await flushPromises();
    expect(wrapper.find(".error-indicator").exists()).toBe(true);

    (autostart.enable as any).mockResolvedValueOnce(undefined);
    await autostartToggle.trigger("click");
    await flushPromises();

    expect(wrapper.find(".error-indicator").exists()).toBe(false);
  });

  it("quick show/hide button calls toggle_floater", async () => {
    const toggleMock = vi.fn().mockResolvedValue(true);
    mockInvoke("toggle_floater", toggleMock);

    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    await wrapper.find(".mini-btn").trigger("click");
    await flushPromises();

    expect(toggleMock).toHaveBeenCalled();
    expect(wrapper.find(".mini-btn").text()).toBe("隐藏");
  });
});

// ========================================================================
// Unsupported modes
// ========================================================================

describe("Settings.vue - Unsupported modes", () => {
  beforeEach(() => {
    resetMocks();
    setupDefaults();
  });

  it("disables hotkey input for unsupported modes", () => {
    const unsupported = TEST_MODES.map((m) => ({
      ...m,
      supported: m.abi <= 3,
      greyed: m.abi === 4,
    }));
    const wrapper = mount(Settings, { props: { modes: unsupported } });
    const inputs = wrapper.findAll(".hotkey-input");
    // Index 3 = geek (unsupported)
    expect(inputs[3].attributes("disabled")).toBeDefined();
  });

  it("adds disabled class to unsupported hotkey rows", () => {
    const unsupported = TEST_MODES.map((m) => ({
      ...m,
      supported: false,
      greyed: false,
    }));
    const wrapper = mount(Settings, { props: { modes: unsupported } });
    const rows = wrapper.findAll(".hotkey-row");
    // The first four rows are per-mode; the last is the cycle binding, which is
    // not tied to any mode and therefore stays bindable.
    expect(rows).toHaveLength(5);
    for (const row of rows.slice(0, 4)) {
      expect(row.classes()).toContain("disabled");
    }
    expect(rows[4].classes()).not.toContain("disabled");
  });
});

// ========================================================================
// Keyboard recording
// ========================================================================

describe("Settings.vue - Keyboard recording", () => {
  beforeEach(() => {
    resetMocks();
    setupDefaults();
  });

  it("enters recording mode on input focus", async () => {
    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    const input = wrapper.findAll(".hotkey-input")[0];
    await input.trigger("focus");
    expect(input.classes()).toContain("recording");
  });

  it("captures Ctrl+key shortcut", async () => {
    const registerMock = vi.fn().mockResolvedValue("ok");
    mockInvoke("register_shortcut", registerMock);

    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    const input = wrapper.findAll(".hotkey-input")[0];
    await input.trigger("focus");

    await input.trigger("keydown", {
      key: "1",
      code: "Digit1",
      ctrlKey: true,
      altKey: false,
      shiftKey: false,
      metaKey: false,
    });

    expect(registerMock).toHaveBeenCalledWith({
      mode: "intelligent",
      shortcut: "Ctrl+1",
    });
  });

  it("cancels recording on Escape", async () => {
    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    const input = wrapper.findAll(".hotkey-input")[0];
    await input.trigger("focus");
    expect(input.classes()).toContain("recording");

    await input.trigger("keydown", {
      key: "Escape",
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
      metaKey: false,
    });

    expect(input.classes()).not.toContain("recording");
  });

  it("ignores bare modifier press (no key)", async () => {
    const registerMock = vi.fn();
    mockInvoke("register_shortcut", registerMock);

    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    const input = wrapper.findAll(".hotkey-input")[0];
    await input.trigger("focus");

    await input.trigger("keydown", {
      key: "Control",
      ctrlKey: true,
      altKey: false,
      shiftKey: false,
      metaKey: false,
    });

    expect(registerMock).not.toHaveBeenCalled();
  });

  it("ignores key press without modifier", async () => {
    const registerMock = vi.fn();
    mockInvoke("register_shortcut", registerMock);

    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    const input = wrapper.findAll(".hotkey-input")[0];
    await input.trigger("focus");

    await input.trigger("keydown", {
      key: "A",
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
      metaKey: false,
    });

    expect(registerMock).not.toHaveBeenCalled();
  });

  it("captures multi-modifier shortcut", async () => {
    const registerMock = vi.fn().mockResolvedValue("ok");
    mockInvoke("register_shortcut", registerMock);

    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    const input = wrapper.findAll(".hotkey-input")[0];
    await input.trigger("focus");

    await input.trigger("keydown", {
      key: "G",
      code: "KeyG",
      ctrlKey: true,
      altKey: true,
      shiftKey: true,
      metaKey: false,
    });

    expect(registerMock).toHaveBeenCalledWith({
      mode: "intelligent",
      shortcut: "Ctrl+Alt+Shift+G",
    });
  });
});

// ========================================================================
// Unbind and clear
// ========================================================================

describe("Settings.vue - Unbind and clear", () => {
  beforeEach(() => {
    resetMocks();
    setupDefaults();
  });

  it("shows unbind button for bound shortcuts", async () => {
    mockInvoke("get_shortcuts", () => [
      { mode: "intelligent", shortcut: "Ctrl+Alt+1" },
    ]);
    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    await flushPromises();
    expect(wrapper.find(".hotkey-clear").exists()).toBe(true);
  });

  it("calls unregister_shortcut on unbind click", async () => {
    const unregisterMock = vi.fn().mockResolvedValue("ok");
    mockInvoke("get_shortcuts", () => [
      { mode: "intelligent", shortcut: "Ctrl+Alt+1" },
    ]);
    mockInvoke("unregister_shortcut", unregisterMock);

    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    await flushPromises();
    await wrapper.find(".hotkey-clear").trigger("click");

    expect(unregisterMock).toHaveBeenCalledWith({ mode: "intelligent" });
  });

  it("calls clear_shortcuts on clear-all click", async () => {
    const clearMock = vi.fn().mockResolvedValue("ok");
    mockInvoke("get_shortcuts", () => [
      { mode: "intelligent", shortcut: "Ctrl+Alt+1" },
      { mode: "saving", shortcut: "Ctrl+Alt+2" },
    ]);
    mockInvoke("clear_shortcuts", clearMock);

    const wrapper = mount(Settings, { props: { modes: TEST_MODES } });
    await flushPromises();
    await wrapper.find(".clear-all-btn").trigger("click");

    expect(clearMock).toHaveBeenCalled();
  });
});
