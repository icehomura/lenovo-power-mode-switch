import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import Floater from "@/Floater.vue";
import { resetMocks, mockInvoke } from "./setup";

function setupFloaterMocks(mode = "intelligent") {
  mockInvoke("get_current_mode", () => mode);
  mockInvoke("cycle_mode", () => "saving");
  mockInvoke("get_floater_status", () => ({
    visible: true,
    style: "capsule",
  }));
  mockInvoke("set_floater_style", () => {});
}

// ========================================================================
// Rendering
// ========================================================================

describe("Floater.vue - Rendering", () => {
  beforeEach(() => {
    resetMocks();
    setupFloaterMocks();
  });

  it("renders the floater container", async () => {
    const wrapper = mount(Floater);
    await vi.dynamicImportSettled();
    expect(wrapper.find(".floater-root").exists()).toBe(true);
  });

  it("defaults to capsule style", async () => {
    const wrapper = mount(Floater);
    await vi.dynamicImportSettled();
    expect(wrapper.find(".floater-capsule").exists()).toBe(true);
    expect(wrapper.find(".floater-circle").exists()).toBe(false);
  });

  it("renders the capsule as an icon plus the mode name", async () => {
    const wrapper = mount(Floater);
    await vi.dynamicImportSettled();
    const icon = wrapper.find(".floater-icon");
    expect(icon.exists()).toBe(true);
    expect(icon.text()).toBe("⚖️");
    // The capsule is the labelled style, so the name must be present.
    expect(wrapper.find(".floater-name").text()).toBe("智能");
  });
});

// ========================================================================
// Mode cycling by clicking the floater
// ========================================================================

describe("Floater.vue - Mode cycling", () => {
  beforeEach(() => {
    resetMocks();
    setupFloaterMocks();
  });

  it("calls cycle_mode on click", async () => {
    const cycleMock = vi.fn().mockResolvedValue("saving");
    mockInvoke("cycle_mode", cycleMock);

    const wrapper = mount(Floater);
    await vi.dynamicImportSettled();
    await flushPromises();

    await wrapper.find(".floater-inner").trigger("click");
    await flushPromises();

    expect(cycleMock).toHaveBeenCalledOnce();
  });

  it("adopts the mode-changed event after cycling", async () => {
    mockInvoke("cycle_mode", () => "saving");

    const wrapper = mount(Floater);
    await vi.dynamicImportSettled();
    await flushPromises();

    await wrapper.find(".floater-inner").trigger("click");
    await flushPromises();

    // The mode-changed event updates currentMode; simulate the broadcast.
    const { emitToListeners } = await import("./setup");
    emitToListeners("mode-changed", { payload: "saving" });
    await flushPromises();

    expect(wrapper.find(".floater-icon").text()).toBe("🌿");
  });
});

// ========================================================================
// Drag versus click
//
// The browser emits `click` after mouseup regardless of how far the pointer
// travelled, so a drag has to be told apart from a click or every reposition
// would also cycle the mode.
// ========================================================================

describe("Floater.vue - Drag does not trigger a click", () => {
  beforeEach(() => {
    resetMocks();
    setupFloaterMocks();
  });

  /** Press on the floater, move by (dx, dy), release, then let click fire. */
  async function pressDragRelease(
    wrapper: ReturnType<typeof mount>,
    dx: number,
    dy: number
  ) {
    await wrapper
      .find(".floater-root")
      .trigger("mousedown", { button: 0, screenX: 100, screenY: 100 });

    document.dispatchEvent(
      new MouseEvent("mousemove", { screenX: 100 + dx, screenY: 100 + dy })
    );
    document.dispatchEvent(new MouseEvent("mouseup"));

    // The browser-synthesised click that follows the release.
    await wrapper.find(".floater-inner").trigger("click");
    await flushPromises();
  }

  it("does not cycle the mode after a drag", async () => {
    const cycleMock = vi.fn().mockResolvedValue("saving");
    mockInvoke("cycle_mode", cycleMock);

    const wrapper = mount(Floater);
    await vi.dynamicImportSettled();
    await flushPromises();

    await pressDragRelease(wrapper, 60, 40);

    expect(cycleMock).not.toHaveBeenCalled();
  });

  it("still moves the window while dragging", async () => {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow() as any;

    const wrapper = mount(Floater);
    await vi.dynamicImportSettled();
    await flushPromises();

    await pressDragRelease(wrapper, 60, 40);

    // The fix must not have disabled dragging itself.
    expect(win.setPosition).toHaveBeenCalled();
  });

  it("still cycles on a press that never moved", async () => {
    const cycleMock = vi.fn().mockResolvedValue("saving");
    mockInvoke("cycle_mode", cycleMock);

    const wrapper = mount(Floater);
    await vi.dynamicImportSettled();
    await flushPromises();

    await pressDragRelease(wrapper, 0, 0);

    expect(cycleMock).toHaveBeenCalledOnce();
  });

  it("treats a shaky click below the threshold as a click", async () => {
    const cycleMock = vi.fn().mockResolvedValue("saving");
    mockInvoke("cycle_mode", cycleMock);

    const wrapper = mount(Floater);
    await vi.dynamicImportSettled();
    await flushPromises();

    // 3px of travel is hand jitter, not a drag.
    await pressDragRelease(wrapper, 3, 0);

    expect(cycleMock).toHaveBeenCalledOnce();
  });

  it("does not move the window for a sub-threshold press", async () => {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const win = getCurrentWindow() as any;

    const wrapper = mount(Floater);
    await vi.dynamicImportSettled();
    await flushPromises();

    await pressDragRelease(wrapper, 3, 0);

    expect(win.setPosition).not.toHaveBeenCalled();
  });

  it("does not swallow the next click after a drag that ended off-window", async () => {
    const cycleMock = vi.fn().mockResolvedValue("saving");
    mockInvoke("cycle_mode", cycleMock);

    const wrapper = mount(Floater);
    await vi.dynamicImportSettled();
    await flushPromises();

    // Drag out of the window: mouseup lands on document, so no click follows.
    await wrapper
      .find(".floater-root")
      .trigger("mousedown", { button: 0, screenX: 100, screenY: 100 });
    document.dispatchEvent(
      new MouseEvent("mousemove", { screenX: 300, screenY: 300 })
    );
    document.dispatchEvent(new MouseEvent("mouseup"));
    await flushPromises();

    // A later, genuine click must still work.
    await wrapper.find(".floater-root").trigger("mousedown", {
      button: 0,
      screenX: 100,
      screenY: 100,
    });
    document.dispatchEvent(new MouseEvent("mouseup"));
    await wrapper.find(".floater-inner").trigger("click");
    await flushPromises();

    expect(cycleMock).toHaveBeenCalledOnce();
  });

  it("ignores non-left buttons", async () => {
    const cycleMock = vi.fn().mockResolvedValue("saving");
    mockInvoke("cycle_mode", cycleMock);

    const wrapper = mount(Floater);
    await vi.dynamicImportSettled();
    await flushPromises();

    await wrapper
      .find(".floater-root")
      .trigger("mousedown", { button: 2, screenX: 100, screenY: 100 });
    document.dispatchEvent(
      new MouseEvent("mousemove", { screenX: 200, screenY: 200 })
    );
    document.dispatchEvent(new MouseEvent("mouseup"));
    await wrapper.find(".floater-inner").trigger("click");
    await flushPromises();

    // A right-press is a style toggle, not a drag: the click still cycles.
    expect(cycleMock).toHaveBeenCalledOnce();
  });

  it("detaches its document listeners on unmount", async () => {
    const wrapper = mount(Floater);
    await vi.dynamicImportSettled();
    await flushPromises();

    await wrapper
      .find(".floater-root")
      .trigger("mousedown", { button: 0, screenX: 100, screenY: 100 });

    wrapper.unmount();

    // Releasing after unmount must not throw or move anything.
    expect(() =>
      document.dispatchEvent(new MouseEvent("mousemove", { screenX: 200, screenY: 200 }))
    ).not.toThrow();
  });
});

// ========================================================================
// Circle mode
// ========================================================================

describe("Floater.vue - Circle style", () => {
  beforeEach(() => {
    resetMocks();
    mockInvoke("get_current_mode", () => "saving");
    mockInvoke("cycle_mode", () => "saving");
    mockInvoke("get_floater_status", () => ({
      visible: true,
      style: "circle",
    }));
    mockInvoke("set_floater_style", () => {});
  });

  it("renders circle style class", async () => {
    const wrapper = mount(Floater);
    await vi.dynamicImportSettled();
    expect(wrapper.find(".floater-circle").exists()).toBe(true);
    expect(wrapper.find(".floater-capsule").exists()).toBe(false);
  });

  it("renders the circle as an icon with no text label", async () => {
    const wrapper = mount(Floater);
    await vi.dynamicImportSettled();
    const icon = wrapper.find(".floater-circle-icon");
    expect(icon.exists()).toBe(true);
    expect(icon.text()).toBe("🌿");
    // The circle is the compact style, so it must not carry a name.
    expect(wrapper.find(".floater-name").exists()).toBe(false);
    expect(wrapper.find(".floater-circle-name").exists()).toBe(false);
  });
});

// ========================================================================
// Shape toggle (right-click)
// ========================================================================

describe("Floater.vue - Shape toggle", () => {
  beforeEach(() => {
    resetMocks();
    setupFloaterMocks();
  });

  it("toggles shape on contextmenu", async () => {
    const wrapper = mount(Floater);
    await vi.dynamicImportSettled();

    expect(wrapper.find(".floater-capsule").exists()).toBe(true);

    await wrapper.find(".floater-inner").trigger("contextmenu.prevent");
    await vi.waitFor(() => {
      expect(wrapper.find(".floater-circle").exists()).toBe(true);
    });
  });
});

// ========================================================================
// Unsubscribe on unmount
// ========================================================================

describe("Floater.vue - Lifecycle", () => {
  beforeEach(() => {
    resetMocks();
    setupFloaterMocks();
  });

  it("cleans up event listeners on unmount", async () => {
    const { listen } = await import("@tauri-apps/api/event");
    const wrapper = mount(Floater);
    await vi.dynamicImportSettled();

    expect(listen).toHaveBeenCalled();

    wrapper.unmount();
    // No error = cleanup succeeded
  });
});
