/** 快捷键绑定配置，key 为模式 slug */
export type HotkeyMap = Record<string, string>;

export interface FloatingWindowSettings {
  enabled: boolean;
  style: 'capsule' | 'circle';
  position: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';
}

export interface AppSettings {
  hotkeys: HotkeyMap;
  floating: FloatingWindowSettings;
  autostart: boolean;
}

export const DEFAULT_SETTINGS: AppSettings = {
  hotkeys: {},
  floating: {
    enabled: false,
    style: 'capsule',
    position: 'top-right',
  },
  autostart: false,
};

/** 可切换的电源模式 slug 列表 */
export const MODE_SLUGS = ['intelligent', 'saving', 'performance', 'geek'] as const;
export type ModeSlug = (typeof MODE_SLUGS)[number];

export const MODE_LABELS: Record<ModeSlug, string> = {
  intelligent: '智能模式',
  saving: '省电模式',
  performance: '性能模式',
  geek: '极客模式',
};
