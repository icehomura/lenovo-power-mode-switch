import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppSettings } from "../types/settings";
import { DEFAULT_SETTINGS } from "../types/settings";

export const settings = ref<AppSettings>({ ...DEFAULT_SETTINGS });

export async function loadSettings() {
  try {
    const saved = await invoke<Partial<AppSettings>>("get_settings");
    settings.value = { ...DEFAULT_SETTINGS, ...saved };
  } catch {
    settings.value = { ...DEFAULT_SETTINGS };
  }
}

export async function saveSettings() {
  await invoke("save_settings", { settings: settings.value });
}

export function updateSettings(partial: Partial<AppSettings>) {
  Object.assign(settings.value, partial);
  saveSettings();
}

export function updateFloating(partial: Partial<AppSettings["floating"]>) {
  Object.assign(settings.value.floating, partial);
  saveSettings();
}

export function resetSettings() {
  settings.value = { ...DEFAULT_SETTINGS };
  saveSettings();
}
