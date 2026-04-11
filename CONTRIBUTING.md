# Contributing

## 中文

### 开发约定

- 先运行 `npm install`
- 提交前至少执行：
  - `npm run check`
  - `npm run test:e2e`
  - `cd src-tauri && cargo test && cargo clippy -- -D warnings`
- 不要把 `dist/`、`src-tauri/target/`、截图样本等产物提交进仓库

### 提交风格

- 一个 commit 只表达一个清晰意图
- 优先使用 `feat:`, `fix:`, `refactor:`, `docs:`, `chore:` 前缀
- 如果改动影响 UI 或系统行为，请附上验证方式

### 代码原则

- 前端状态、副作用和展示组件分离
- Rust command 保持薄，系统能力放在独立模块
- 新逻辑优先补单测或组件测试

## English

### Before submitting

- Run `npm install`
- Validate with:
  - `npm run check`
  - `npm run test:e2e`
  - `cd src-tauri && cargo test && cargo clippy -- -D warnings`

### Commit style

- Keep each commit focused on one intention
- Prefer `feat:`, `fix:`, `refactor:`, `docs:`, and `chore:` prefixes
- Include validation notes for UI or system-behavior changes
