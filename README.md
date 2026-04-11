# Clipboard

中文 | [English](#english)

面向 `macOS Apple Silicon` 的本地剪贴板应用，使用 `Tauri 2 + React + TypeScript` 构建。  
应用常驻后台，监听系统剪贴板变化，记录历史并提供搜索、收藏、回写和悬浮面板能力。

## 特性

- 后台监听文本、图片和文件剪贴板历史
- 全局快捷键唤起悬浮面板
- 面板固定/自动隐藏
- 搜索、分页、收藏、重新复制和粘贴回前台应用
- 本地优先，不依赖云端服务

## 开发环境

- macOS 14+
- Apple Silicon
- Node.js 20+
- Rust 1.77+
- Xcode Command Line Tools

## 快速开始

```bash
npm install
npm run dev
```

常用命令：

```bash
npm run lint
npm run test
npm run test:e2e
npm run build:web
cd src-tauri && cargo test && cargo clippy -- -D warnings
```

## 项目结构

```text
src/                    React 前端
src/features/           UI feature 模块
src/hooks/              前端状态与副作用
src-tauri/src/          Rust 后端模块
tests/                  Vitest / Playwright 测试
docs/                   目标、架构与设计文档
```

## 文档

- [目标文档](docs/goal.md)
- [架构说明](docs/architecture.md)
- [Finder 文件剪贴板格式](docs/clipboard-pasteboard-format.md)
- [贡献指南](CONTRIBUTING.md)

## 许可证

本项目采用 [MIT](LICENSE) 许可证。

## English

Clipboard is a local clipboard manager for `macOS Apple Silicon`, built with `Tauri 2 + React + TypeScript`.

### Highlights

- Background clipboard history for text, images, and files
- Global shortcut to open the floating panel
- Pin / auto-hide window behavior
- Search, favorite, recopy, and paste-back workflow
- Local-first design with no cloud dependency

### Development

```bash
npm install
npm run dev
```

Useful commands:

```bash
npm run check
npm run test:e2e
cd src-tauri && cargo test && cargo clippy -- -D warnings
```

See [docs/architecture.md](docs/architecture.md) and [CONTRIBUTING.md](CONTRIBUTING.md) for implementation and collaboration details.
