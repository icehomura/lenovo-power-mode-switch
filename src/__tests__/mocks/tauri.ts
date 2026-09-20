/**
 * Tauri API mock utilities for frontend testing.
 *
 * Provides a controlled mock for `@tauri-apps/api/core` invoke(),
 * allowing tests to set per-command return values and error scenarios.
 */
import { vi } from "vitest";

// Store for command handlers registered by tests
const commandHandlers = new Map<string, (...args: unknown[]) => unknown>();

/**
 * Reset all mocked command handlers. Call in beforeEach/afterEach.
 */
export function resetMocks() {
  commandHandlers.clear();
}

/**
 * Register a mock handler for a Tauri command.
 *
 * @example
 * mockInvoke("get_modes", () => [{ id: "intelligent", name: "智能模式" }]);
 * mockInvoke("check_connection", () => "connected");
 */
export function mockInvoke(
  command: string,
  handler: (...args: unknown[]) => unknown
) {
  commandHandlers.set(command, handler);
}

/**
 * Mock the Tauri `invoke` function globally.
 * Must be called once per test file (in beforeAll or at module scope).
 */
export function setupTauriMock() {
  vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(async (command: string, args?: Record<string, unknown>) => {
      const handler = commandHandlers.get(command);
      if (!handler) {
        throw new Error(`[TauriMock] 未注册的命令: ${command}`);
      }
      return handler(args);
    }),
  }));
}
