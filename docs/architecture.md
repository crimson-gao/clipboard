# Clipboard 项目设计 / 架构文档

## 1. 文档目的

本文档用于说明 Clipboard 当前采用的实现路线、模块边界、关键数据流，以及后续演进时需要遵守的设计约束。

当前唯一桌面实现路线为：`Tauri 2 + React + TypeScript`。

---

## 2. 架构总览

应用分为两层：

- **Rust / Tauri 后端**：负责系统能力、剪贴板监听、持久化、窗口控制、托盘和全局快捷键。
- **React 前端**：负责 UI 渲染、交互状态、列表分页/筛选/搜索，以及调用 Tauri command。

核心原则：

1. **系统能力留在 Rust**：剪贴板、窗口、托盘、快捷键、Finder 集成不进入前端。
2. **页面状态留在 React**：筛选、分页、选中、展开、设置草稿等 UI 状态不下沉到 Rust。
3. **command 保持薄层**：`commands.rs` 只做参数接入和能力拼装，不承载复杂业务细节。
4. **单一路径优先**：不再维护 Electron 等第二套桌面外壳。

---

## 3. 当前代码结构

### 3.1 前端

- `src/App.tsx`
  - 页面编排
  - 顶部工具栏、分类切换、设置入口、固定窗口入口
- `src/hooks/useClipboardApp.ts`
  - 主界面状态中心
  - 数据加载、分页、搜索、列表选择、设置弹窗状态
  - 订阅 `clips:changed` / `open-settings` 事件
- `src/lib/clipboard-api.ts`
  - 前端唯一 Tauri 调用入口
  - 封装 `invoke` 与事件订阅
- `src/features/clips/`
  - `ClipList.tsx`：列表与条目交互
  - `ClipImagePreview.tsx`：图片预览
  - `clip-utils.ts`：分类、摘要、展示格式、分页常量
- `src/features/settings/`
  - `SettingsModal.tsx`：设置面板
  - `ShortcutRecorder.tsx`：快捷键录制
- `src/components/Tooltip.tsx`
  - 通用提示层
- `src/about.tsx`
  - About 窗口页面

### 3.2 后端

- `src-tauri/src/lib.rs`
  - Tauri 应用组装入口
  - 注册插件、托盘菜单、窗口事件、command handler
- `src-tauri/src/commands.rs`
  - 前端调用入口
  - 列表查询、收藏、复制、粘贴回前台、清空当前列表、窗口设置等
- `src-tauri/src/clipboard.rs`
  - 系统剪贴板监听
  - 文本 / HTML / 文件 / 图片读取与规范化
  - 定时清理过期记录
- `src-tauri/src/store.rs`
  - JSON 持久化
  - 数据模型写入、去重 upsert、过滤、搜索、计数、过期清理
- `src-tauri/src/shell.rs`
  - 托盘、全局快捷键、确认弹窗、写回剪贴板、粘贴回前台应用
- `src-tauri/src/window.rs`
  - macOS 悬浮窗口、自动隐藏、前后台应用切换、About 窗口
- `src-tauri/src/events.rs`
  - 前后端共享事件名与 tray menu id 常量
- `src-tauri/src/models.rs`
  - command 返回模型

---

## 4. 关键运行流程

### 4.1 应用启动

1. `lib.rs` 初始化 Tauri Builder。
2. 加载插件：
   - `tauri-plugin-clipboard-manager`
   - `tauri-plugin-global-shortcut`
   - `tauri-plugin-dialog`
3. 读取 `ClipboardState::load()`：
   - 从 app data dir 载入 `clipboard-store.json`
   - 执行迁移和过期数据清理
4. 配置主窗口 overlay 行为。
5. 启动剪贴板 watcher 与过期清理 scheduler。
6. 配置 tray 和全局快捷键。

### 4.2 剪贴板采集

1. `clipboard.rs` 监听系统剪贴板变化。
2. 读取优先级：
   - 文件列表
   - 纯文本
   - HTML 转纯文本
   - PNG 图片
3. 根据类型构造 `UpsertClipInput`。
4. `store.rs` 执行 `upsert_clip_item()`：
   - 按类型 + 内容 + 路径去重
   - 已存在则更新时间
   - 不存在则新增记录
5. 数据变化后触发 `clips:changed` 事件。

### 4.3 前端刷新

1. 前端在 `useClipboardApp()` 中订阅 `clips:changed`。
2. 事件触发后刷新 counts。
3. 当前 query/filter 对应 tab cache 若失效，则重新拉取分页列表。
4. 保留每个 tab 的：
   - 已加载 items
   - offset
   - hasMore
   - selectedClipId
   - expandedClipIds

### 4.4 打开主窗口

窗口可由两条路径打开：

- 全局快捷键
- tray 图标点击 / 菜单项

打开时如果需要“粘贴回前台应用”，会先记录当前前台 App 的 pid，然后再展示主窗口。

### 4.5 复制与粘贴回前台

#### 复制历史项

1. 前端调用 `copy_clip`。
2. Rust 将内容写回系统剪贴板。
3. 若窗口未固定，则自动隐藏。

#### 粘贴回前台应用

1. 前端调用 `paste_clip_and_hide`。
2. Rust 先执行 `copy_clip`。
3. 隐藏当前窗口。
4. 激活先前记录的前台应用。
5. 通过 macOS 原生事件发送 `Command + V`。

> 当前实现中，文件类条目复制回剪贴板时写回的是路径文本，而不是 Finder 原生 file promise / file-url 负载。

---

## 5. 数据设计

### 5.1 存储介质

当前持久化使用本地 JSON：

- 主文件：`clipboard-store.json`
- 图片文件：保存在 app data dir 下的 `blobs/`

选择 JSON 的原因：

- 实现简单
- 调试直接
- 便于快速验证产品行为

暂不引入 SQLite，除非出现以下问题：

- 历史量增长后列表查询明显变慢
- 搜索/分页成为瓶颈
- 需要更强一致性或索引能力

### 5.2 记录模型

每条剪贴板记录至少包含：

- `id`
- `kind`：`text | image | file`
- `content_text`
- `content_path`
- `file_paths`
- `is_favorite`
- `created_at`
- `updated_at`
- `image_width` / `image_height`

### 5.3 生命周期规则

- 默认保留最近 **7 天** 的非收藏记录
- 收藏记录不会因过期清理被删除
- 相同内容再次出现时只更新时间，不重复插入
- `data_version` 用于驱动前端缓存失效

---

## 6. 前后端接口边界

### 6.1 Tauri commands

当前前端只通过 `src/lib/clipboard-api.ts` 调用以下能力：

- `list_clips_page`
- `get_clip_image_preview`
- `get_clip_counts`
- `toggle_favorite`
- `copy_clip`
- `paste_clip_and_hide`
- `clear_current_clips`
- `show_clip_in_finder`
- `toggle_pin_window`
- `get_window_state`
- `update_window_settings`

约束：

- 前端不得在组件内部散落 `invoke()` 调用
- 新增 command 时，优先补到 `clipboard-api.ts`
- command 返回结构优先复用 `models.rs` / `src/types.ts`

### 6.2 事件

当前仅保留少量跨层事件：

- `clips:changed`
- `open-settings`

约束：

- 事件只负责“通知”，不负责传输复杂业务对象
- 复杂数据始终通过 command 拉取，避免双向状态漂移

---

## 7. UI 设计约束

主界面当前是一个以搜索 + 分类 + 历史列表 + 详情操作为核心的悬浮面板。

需要持续保持：

- **唤起快**：快捷键后尽快可见、可聚焦
- **动作少**：复制 / 收藏 / 回贴 / 定位文件都是单步操作
- **信息密度高**：文本、图片、文件在同一列表中稳定展示
- **窗口行为一致**：固定与非固定模式的隐藏/失焦行为必须可预期

不建议引入：

- 重型全局状态框架
- 大型 UI 组件库
- 需要多层同步的前后端双写状态

---

## 8. 当前已知设计取舍

### 8.1 已接受取舍

1. **JSON 优先于数据库**
   - 适合当前阶段
   - 代价是后续查询能力有限

2. **前端本地缓存分页结果**
   - 降低分类切换和搜索切换时的重复拉取
   - 需要依赖 `dataVersion` 做失效控制

3. **文件条目先以路径文本回写**
   - 先保证可见、可搜、可定位
   - 暂未实现 Finder 语义级回贴

4. **窗口行为偏 macOS 原生**
   - 使用 `NSWindow` 能力做悬浮和隐藏策略
   - 平台相关逻辑集中在 `window.rs`

### 8.2 需要继续控制的风险

- `commands.rs` 继续膨胀
- `useClipboardApp.ts` 承担过多状态编排
- command 命名与前端类型命名出现漂移
- 文档不随实现更新，导致后续误判边界

---

## 9. 后续演进建议

### 9.1 优先级高

1. 将 command 层继续保持为薄适配层
2. 在 store 层补足更多数据规则测试
3. 明确文件类条目的“复制路径”与“真实文件回贴”差异
4. 把设置、列表、详情交互边界继续收紧

### 9.2 视需求再做

- SQLite 持久化
- 更强的缩略图缓存策略
- 更完整的图片/文件元数据
- 多窗口或快捷操作面板

---

## 10. 设计决策摘要

- **桌面实现**：只保留 `Tauri 2`
- **状态边界**：系统状态在 Rust，页面状态在 React
- **持久化**：当前使用 JSON + blobs
- **窗口模型**：悬浮主窗口 + 可选固定 + About 子窗口
- **数据流**：watcher 写入 store，前端通过事件感知并拉取数据
- **演进原则**：先保证简单、可靠、可调试，再追求扩展性
