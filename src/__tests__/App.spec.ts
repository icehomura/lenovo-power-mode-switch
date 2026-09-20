import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import App from "@/App.vue";
import { resetMocks, mockInvoke } from "./setup";
import {
  TEST_MODES,
  TEST_MODES_WITH_UNSUPPORTED,
  TEST_MODES_ALL_UNSUPPORTED,
} from "./fixtures/modes";

// Mock Settings child component — render a stub so App can mount
vi.mock("@/views/Settings.vue", () => ({
  default: {
    name: "Settings",
    props: ["modes"],
    emits: ["back"],
    template: '<div class="settings-stub"><button class="back-btn" @click="$emit(\'back\')">back</button></div>',
  },
}));

// Mock useSettings (App.vue imports loadSettings)
vi.mock("@/composables/useSettings", () => ({
  settings: {
    value: {
      autostart: false,
      floating: { enabled: false, style: "capsule", position: "top-right" },
      hotkeys: {},
    },
  },
  loadSettings: vi.fn().mockResolvedValue(undefined),
  updateSettings: vi.fn(),
  updateFloating: vi.fn(),
  resetSettings: vi.fn(),
  updateHotkey: vi.fn(),
  clearHotkey: vi.fn(),
  findHotkeyConflict: vi.fn().mockReturnValue(null),
}));

function setupHappyMocks() {
  mockInvoke("check_connection", () => "connected");
  mockInvoke("get_modes", () => TEST_MODES);
  mockInvoke("get_current_mode", () => "intelligent");
  mockInvoke("set_mode", (_args: any) => "intelligent");
  mockInvoke("get_floater_status", () => ({
    visible: false,
    style: "capsule",
  }));
  mockInvoke("toggle_floater", () => true);
}

// ========================================================================
// Rendering
// ========================================================================

describe("App.vue - Rendering", () => {
  beforeEach(() => {
    resetMocks();
    setupHappyMocks();
  });

  it("renders the title", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    expect(wrapper.find("h1").text()).toBe("Lenovo Power Mode Switch");
  });

  it("renders all four mode buttons", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    const buttons = wrapper.findAll(".mode-btn");
    expect(buttons).toHaveLength(4);
  });

  it("shows mode names correctly", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    const names = wrapper.findAll(".name").map((el) => el.text());
    expect(names).toEqual(["智能模式", "省电模式", "性能模式", "极客模式"]);
  });

  it("shows mode icons", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    const icons = wrapper.findAll(".icon").map((el) => el.text());
    expect(icons).toEqual(["⚖️", "🍃", "⚡", "🚀"]);
  });

  it("marks the current mode with active class", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    const activeBtn = wrapper.find(".mode-btn.active");
    expect(activeBtn.exists()).toBe(true);
    expect(activeBtn.find(".name").text()).toBe("智能模式");
  });

  it("shows checkmark on active mode", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    const activeBtn = wrapper.find(".mode-btn.active");
    expect(activeBtn.find(".check").text()).toBe("✓");
  });

  it("renders settings button", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    const btns = wrapper.findAll(".icon-btn");
    expect(btns.length).toBeGreaterThanOrEqual(2);
  });

  it("renders floater toggle button", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    const btns = wrapper.findAll(".icon-btn");
    expect(btns.length).toBeGreaterThanOrEqual(2);
  });
});

// ========================================================================
// Mode switching
// ========================================================================

describe("App.vue - Mode switching", () => {
  beforeEach(() => {
    resetMocks();
    setupHappyMocks();
  });

  it("calls set_mode when clicking a different mode", async () => {
    const setModeMock = vi.fn().mockResolvedValue("saving");
    mockInvoke("set_mode", setModeMock);
    // Don't override get_current_mode — keep default "intelligent"
    // so clicking "saving" (index 1) is a genuinely different mode

    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    await flushPromises();

    const buttons = wrapper.findAll(".mode-btn");
    await buttons[1].trigger("click");

    expect(setModeMock).toHaveBeenCalledWith({ mode: "saving" });
  });

  it("still calls set_mode when clicking the current mode (re-confirm)", async () => {
    const setModeMock = vi.fn().mockResolvedValue("intelligent");
    mockInvoke("set_mode", setModeMock);

    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    await flushPromises();

    const buttons = wrapper.findAll(".mode-btn");
    await buttons[0].trigger("click");

    // switchMode has no early return for same-mode; it re-applies
    expect(setModeMock).toHaveBeenCalledWith({ mode: "intelligent" });
  });

  it("shows error status on switch failure", async () => {
    mockInvoke("set_mode", () => {
      throw new Error("硬件拒绝");
    });

    const wrapper = mount(App);
    await vi.dynamicImportSettled();

    const buttons = wrapper.findAll(".mode-btn");
    await buttons[1].trigger("click");

    await vi.waitFor(() => {
      const footer = wrapper.find(".status-bar");
      expect(footer.text()).toContain("切换失败");
    });
  });
});

// ========================================================================
// Unsupported / greyed modes
// ========================================================================

describe("App.vue - Unsupported modes", () => {
  beforeEach(() => {
    resetMocks();
    mockInvoke("check_connection", () => "connected");
    mockInvoke("get_modes", () => TEST_MODES_WITH_UNSUPPORTED);
    mockInvoke("get_current_mode", () => "intelligent");
    mockInvoke("set_mode", () => "intelligent");
    mockInvoke("get_floater_status", () => ({
      visible: false,
      style: "capsule",
    }));
  });

  it("disables unsupported mode buttons", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();

    const buttons = wrapper.findAll(".mode-btn");
    // geek (index 3) is unsupported
    expect(buttons[3].attributes("disabled")).toBeDefined();
  });

  it("adds disabled class to unsupported modes", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();

    const buttons = wrapper.findAll(".mode-btn");
    expect(buttons[3].classes()).toContain("disabled");
  });

  it("shows unsupported hint text", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();

    const descs = wrapper.findAll(".desc").map((el) => el.text());
    expect(descs[3]).toBe("本机硬件不支持");
  });
});

describe("App.vue - All modes unsupported", () => {
  beforeEach(() => {
    resetMocks();
    mockInvoke("check_connection", () => "connected");
    mockInvoke("get_modes", () => TEST_MODES_ALL_UNSUPPORTED);
    mockInvoke("get_current_mode", () => "intelligent");
    mockInvoke("get_floater_status", () => ({
      visible: false,
      style: "capsule",
    }));
  });

  it("disables all mode buttons", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();

    const buttons = wrapper.findAll(".mode-btn");
    for (const btn of buttons) {
      expect(btn.attributes("disabled")).toBeDefined();
    }
  });
});

// ========================================================================
// Connection error
// ========================================================================

describe("App.vue - Connection error", () => {
  beforeEach(() => {
    resetMocks();
    mockInvoke("check_connection", () => "disconnected: DLL not found");
    mockInvoke("get_modes", () => []);
    mockInvoke("get_current_mode", () => "intelligent");
  });

  it("shows connection error on startup", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();

    const footer = wrapper.find(".status-bar");
    expect(footer.text()).toContain("桥接不可用");
    expect(footer.text()).toContain("DLL not found");
    expect(footer.classes()).toContain("error");
  });
});

// ========================================================================
// Navigation to Settings
// ========================================================================

describe("App.vue - Settings navigation", () => {
  beforeEach(() => {
    resetMocks();
    setupHappyMocks();
  });

  it("switches to settings view on gear button click", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();

    // Settings button is the first icon-btn
    const settingsBtn = wrapper.findAll(".icon-btn")[0];
    await settingsBtn.trigger("click");

    expect(wrapper.find(".settings-stub").exists()).toBe(true);
    expect(wrapper.find(".mode-list").exists()).toBe(false);
  });

  it("switches back to main view via Settings back event", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();

    const settingsBtn = wrapper.findAll(".icon-btn")[0];
    await settingsBtn.trigger("click");

    expect(wrapper.find(".settings-stub").exists()).toBe(true);

    const backBtn = wrapper.find(".settings-stub .back-btn");
    await backBtn.trigger("click");

    expect(wrapper.find(".mode-list").exists()).toBe(true);
  });
});

// ========================================================================
// Event listening
// ========================================================================

describe("App.vue - Event listening", () => {
  beforeEach(() => {
    resetMocks();
    setupHappyMocks();
  });

  it("registers mode-changed listener on mount", async () => {
    const { listen } = await import("@tauri-apps/api/event");
    mount(App);
    await vi.dynamicImportSettled();

    expect(listen).toHaveBeenCalledWith(
      "mode-changed",
      expect.any(Function)
    );
  });
});

// ========================================================================
// Frameless window controls
// ========================================================================

describe("App.vue - Window controls", () => {
  beforeEach(() => {
    resetMocks();
    setupHappyMocks();
  });

  function winBtns(wrapper: ReturnType<typeof mount>) {
    return wrapper.findAll(".win-btn");
  }

  it("renders the four window buttons", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();

    expect(winBtns(wrapper)).toHaveLength(4);
    const titles = winBtns(wrapper).map((b) => b.attributes("title"));
    expect(titles).toEqual(["置顶", "最小化", "最大化", "关闭到托盘"]);
  });

  it("minimizes the window", async () => {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();

    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    await winBtns(wrapper)[1].trigger("click");

    expect(win.minimize).toHaveBeenCalled();
  });

  it("maximizes when not maximized", async () => {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow() as any;
    win.isMaximized.mockResolvedValue(false);

    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    await winBtns(wrapper)[2].trigger("click");

    expect(win.maximize).toHaveBeenCalled();
    expect(win.unmaximize).not.toHaveBeenCalled();
  });

  it("unmaximizes when already maximized", async () => {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow() as any;
    win.isMaximized.mockResolvedValue(true);

    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    await winBtns(wrapper)[2].trigger("click");

    expect(win.unmaximize).toHaveBeenCalled();
  });

  it("closes (hides to tray) the window", async () => {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow();

    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    await winBtns(wrapper)[3].trigger("click");

    expect(win.close).toHaveBeenCalled();
  });

  it("toggles always-on-top on the pin button", async () => {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow() as any;

    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    await winBtns(wrapper)[0].trigger("click");

    expect(win.setAlwaysOnTop).toHaveBeenCalledWith(true);
    expect(winBtns(wrapper)[0].classes()).toContain("on");
  });

  it("drags the window from the title bar", async () => {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow() as any;

    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    await wrapper.find(".titlebar").trigger("mousedown", { button: 0 });

    expect(win.startDragging).toHaveBeenCalled();
  });

  it("does not drag when the press starts on a button", async () => {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow() as any;

    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    // mousedown originating on an action button must not move the window
    await wrapper.find(".titlebar .icon-btn").trigger("mousedown", { button: 0 });

    expect(win.startDragging).not.toHaveBeenCalled();
  });
});

// ========================================================================
// Floater toggle icon
// ========================================================================

describe("App.vue - Floater toggle icon", () => {
  beforeEach(() => {
    resetMocks();
    setupHappyMocks();
  });

  /** The floater button is the last app action before the window controls. */
  function floaterBtn(wrapper: ReturnType<typeof mount>) {
    const btns = wrapper.findAll(".titlebar-actions .icon-btn");
    return btns[btns.length - 1];
  }

  it("renders an svg icon, not a text glyph", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();

    const btn = floaterBtn(wrapper);
    expect(btn.find("svg").exists()).toBe(true);
    expect(btn.text()).toBe("");
  });

  it("offers to show the floater while it is hidden", async () => {
    mockInvoke("get_floater_status", () => ({ visible: false, style: "capsule" }));

    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    await flushPromises();

    expect(floaterBtn(wrapper).attributes("title")).toBe("显示悬浮窗");
    expect(floaterBtn(wrapper).classes()).not.toContain("active");
  });

  it("offers to hide the floater and marks it active while visible", async () => {
    mockInvoke("get_floater_status", () => ({ visible: true, style: "capsule" }));

    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    await flushPromises();

    expect(floaterBtn(wrapper).attributes("title")).toBe("隐藏悬浮窗");
    expect(floaterBtn(wrapper).classes()).toContain("active");
  });

  it("fills the inner pane of the icon while the floater is visible", async () => {
    mockInvoke("get_floater_status", () => ({ visible: true, style: "capsule" }));

    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    await flushPromises();

    const rects = floaterBtn(wrapper).findAll("rect");
    expect(rects).toHaveLength(2);
    expect(rects[1].attributes("fill")).toBe("currentColor");
  });

  it("leaves the inner pane hollow while the floater is hidden", async () => {
    mockInvoke("get_floater_status", () => ({ visible: false, style: "capsule" }));

    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    await flushPromises();

    const rects = floaterBtn(wrapper).findAll("rect");
    expect(rects[1].attributes("fill")).toBe("none");
  });

  it("toggles the floater through the backend on click", async () => {
    const toggleMock = vi.fn().mockResolvedValue(true);
    mockInvoke("toggle_floater", toggleMock);

    const wrapper = mount(App);
    await vi.dynamicImportSettled();
    await flushPromises();

    await floaterBtn(wrapper).trigger("click");
    await flushPromises();

    expect(toggleMock).toHaveBeenCalled();
  });
});

// ========================================================================
// Status bar
// ========================================================================

describe("App.vue - Status bar", () => {
  beforeEach(() => {
    resetMocks();
    setupHappyMocks();
  });

  it("shows current mode in status bar after load", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();

    const footer = wrapper.find(".status-bar");
    expect(footer.text()).toContain("当前模式");
    expect(footer.text()).toContain("智能模式");
  });

  it("does not have error class on successful load", async () => {
    const wrapper = mount(App);
    await vi.dynamicImportSettled();

    const footer = wrapper.find(".status-bar");
    expect(footer.classes()).not.toContain("error");
  });
});
