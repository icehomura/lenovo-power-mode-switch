/**
 * Shared test fixtures mirroring the Rust MODES constant.
 */
import type { ModeInfo } from "@/App.vue";

export const TEST_MODES: ModeInfo[] = [
  {
    id: "intelligent",
    name: "智能模式",
    desc: "按负载自动调节",
    icon: "⚖️",
    color: "#4A9EFF",
    abi: 1,
    supported: true,
    greyed: false,
  },
  {
    id: "saving",
    name: "省电模式",
    desc: "风扇更安静、功耗更低",
    icon: "🍃",
    color: "#34C759",
    abi: 2,
    supported: true,
    greyed: false,
  },
  {
    id: "performance",
    name: "性能模式",
    desc: "释放持续性能",
    icon: "⚡",
    color: "#FF9F0A",
    abi: 3,
    supported: true,
    greyed: false,
  },
  {
    id: "geek",
    name: "极客模式",
    desc: "最高性能，需硬件支持",
    icon: "🚀",
    color: "#FF3B30",
    abi: 4,
    supported: true,
    greyed: false,
  },
];

export const TEST_MODES_WITH_UNSUPPORTED: ModeInfo[] = TEST_MODES.map(
  (m) => ({
    ...m,
    supported: m.abi <= 3,
    // greyed means "supported but temporarily disabled" — only set when supported is true
    greyed: false,
  })
);

export const TEST_MODES_ALL_UNSUPPORTED: ModeInfo[] = TEST_MODES.map((m) => ({
  ...m,
  supported: false,
  greyed: false,
}));
