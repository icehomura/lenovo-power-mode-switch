# Lenovo Power Mode Switch

基于 Tauri v2 + Vue 3 + TypeScript 的 Lenovo 电源模式切换器，**仅支持 Windows x86-64**。

通过逆向工程的 DLL 桥接层直接与硬件通信，无需依赖 Lenovo Vantage 或联想电脑管家。

## 功能

- **四种电源模式**：智能、省电、性能、极客（按硬件能力动态检测可用性）
- **系统托盘**：托盘菜单直接切换模式（当前项带勾选），左键显示/隐藏窗口
- **悬浮窗**：运行时按需新建；胶囊型（图标 + 模式名）与圆形（仅图标）两种样式；单击循环切换模式，右键切换样式；可自由拖拽，拖动位置在隐藏/显示之间保留
- **全局快捷键**：为每种模式以及「循环切换」分别绑定全局快捷键，默认留空、由用户自行录制
- **开机自启**：可选开机自动启动
- **单实例保护**：重复启动会聚焦已有窗口，不会产生争抢硬件的第二实例
- **状态同步**：后端是硬件状态的唯一真相源，任何变更广播到所有窗口
- **持久化**：窗口状态、快捷键绑定与悬浮窗设置重启后自动恢复

详细功能说明见 [FEATURES.md](FEATURES.md)。

## 开发

```bash
pnpm install
pnpm tauri dev
```

## 构建

```bash
pnpm tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`。

## 运行要求

- **Windows x86-64**（唯一支持的平台，不提供 macOS / Linux 构建）
- `lpm_bridge.dll` 及其依赖的 addin DLL 文件位于可执行文件同级目录
- Lenovo 硬件（支持 IdeaNotebookAddin / PowerBattery 接口）

## 技术栈

- 前端: Vue 3 + TypeScript + Vite
- 后端: Rust + Tauri v2
- DLL 调用: libloading crate
- 插件: global-shortcut, autostart, store, window-state, shell, single-instance

## 项目结构

```
src-tauri/
├── src/
│   ├── lib.rs          # Tauri 入口、插件注册、托盘配置、模式广播
│   ├── main.rs         # 程序入口
│   ├── power.rs        # 硬件桥接层、模式管理与循环选择
│   ├── floater.rs      # 悬浮窗：运行时创建、样式、位置
│   └── shortcuts.rs    # 全局快捷键管理与执行
├── capabilities/
│   └── default.json    # Tauri v2 权限配置
├── resources/          # 运行时资源（DLL 等）
└── tauri.conf.json     # Tauri 配置（主窗口；悬浮窗运行时创建）
```

## 相关文档

- [功能说明](FEATURES.md)
- [更新说明](UPDATE.md)
