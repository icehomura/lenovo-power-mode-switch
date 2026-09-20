# 应用图标

## 来源

**唯一源文件是 `../app-icon.svg`，不要手工编辑本目录下的任何文件。**
本目录所有产物都由 `pnpm tauri icon` 从该 SVG 生成，手工改动会在下次
生成时被覆盖。

## 重新生成

```bash
cd tauri-power-manager
pnpm tauri icon app-icon.svg
```

`tauri icon` 直接接受 SVG（内部光栅化到 1024×1024 再切各尺寸），因此
不需要先导出 PNG 中间文件。

## 产物

| 文件 | 尺寸 | 用途 |
|------|------|------|
| `32x32.png` | 32×32 | 任务栏 / 托盘 |
| `64x64.png` | 64×64 | 中等尺寸 |
| `128x128.png` | 128×128 | 应用列表 |
| `128x128@2x.png` | 256×256 | 高 DPI |
| `icon.png` | 512×512 | 通用 |
| `icon.ico` | 16/24/32/48/64/256 | Windows 打包与 exe 资源 |
| `icon.icns` | 多尺寸 | macOS 打包 |
| `Square*Logo.png` / `StoreLogo.png` | 各档 | Microsoft Store 资源 |

`tauri.conf.json` 的 `bundle.icon` 引用了其中的 32/128/128@2x/icns/ico，其余
文件是 CLI 的默认输出。

## 移动端图标已删除

生成命令默认还会产出 `android/`（mipmap 各密度）与 `ios/`（AppIcon 各尺寸）
共 35 个文件。本应用是 Windows 专用的（运行时依赖联想 Windows 原生 DLL），
这两个目录不会被使用，已删除。

注意：重新运行生成命令会把它们再生成出来。若不想保留，删掉即可——它们不被
`bundle.icon` 引用，也不会被打包。

## 设计要点

- 深色圆角底（`#2b2b52` → `#0d0d18`），与应用 UI 的 `#1a1a2e` 同色系。
- 闪电自上而下贯穿应用四种模式的主题色：
  智能蓝 `#4A9EFF` → 省电绿 `#34C759` → 性能琥珀 `#FF9F0A` → 极客红 `#FF3B30`。
- **刻意不加光晕 / 模糊**。早期版本用一份模糊副本做发光，但因该副本位于
  `scale(31)` 分组内，模糊半径被同比例放大，把整个底板糊成一片；而且
  光晕在 32px 下只是噪声，小图标靠的是对比度。
- 闪电轮廓额外加了同色描边（round join）。原始轮廓在 24 单位画布中仅约
  10 单位宽，缩到 32px 会过细；描边增粗约 20% 并磨圆尖角，这是它在小尺寸
  存活下来的关键。

## 移动端目录

`android/` 与 `ios/` 是 CLI 的默认输出。本应用是 Windows 专用的
（运行时依赖联想 Windows 原生 DLL），这两个目录不会被使用，可以删除；
删除后运行上面的生成命令即可恢复。

## 预览小尺寸

小尺寸是否可读要放大看，别直接盯 32px 位图：

```powershell
powershell -File ..\..\tools\icon-sheet.ps1
```

会在 `%TEMP%\icon-sheet.png` 生成 32/64/128 的 4 倍最近邻放大对照图。
