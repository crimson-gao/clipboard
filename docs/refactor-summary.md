# Clipboard 前端整理与重构总结

## 1. 文档目的

本文档用于记录本轮前端整理的目标、已完成改动、修复的反模式，以及后续仍值得继续推进的重构方向。

本次工作重点不是引入新功能，而是：

- 简化代码结构
- 提升可读性
- 降低状态耦合
- 修复容易引发维护成本或竞态问题的实现方式
- 统一代码风格并补齐格式化

---

## 2. 本轮改动概览

本次改动主要集中在以下文件：

- `src/App.tsx`
- `src/hooks/useClipboardApp.ts`
- `src/settings.tsx`
- `src/lib/clipboard-api.ts`
- `src/features/clips/ClipList.tsx`
- `src/features/settings/SettingsPanel.tsx`
- `src/features/settings/ShortcutRecorder.tsx`

新增文件：

- `src/hooks/useDebouncedValue.ts`
- `src/features/clips/categories.tsx`

另外，`about.html`、`settings.html`、部分 CSS 和测试文件做了格式化整理，但没有引入新的业务逻辑。

---

## 3. 发现的主要问题

### 3.1 `App.tsx` 里混入了过多配置和动作包装

此前 `App.tsx` 同时承担了以下职责：

- 页面骨架渲染
- 分类图标与分类配置定义
- 多个“选中项执行动作”的包装函数
- 多个重复的内联点击处理逻辑

这会带来两个问题：

1. 页面主体被大量细节淹没，阅读成本高。
2. 逻辑复用差，后续新增按钮时容易继续复制粘贴。

### 3.2 `useClipboardApp` 过于臃肿

主 Hook 同时负责：

- 搜索输入和防抖
- 列表加载与分页
- 分类切换
- 选中态管理
- tab cache
- 键盘导航
- busy 状态
- 窗口状态同步

虽然功能上可用，但一个 Hook 里堆叠太多关注点，会让局部修改更容易产生副作用。

### 3.3 设置页自动保存存在竞态风险

原先 `settings.tsx` 使用 `setTimeout` 在 `useEffect` 中直接触发保存。这样虽然能做到“延迟提交”，但存在几个隐患：

- 快速连续修改时，旧请求可能晚于新请求返回
- 旧响应可能覆盖用户更晚的输入
- 保存完成后，草稿状态和服务端状态不一定完全对齐

这属于典型的“延迟副作用 + 无请求失效控制”的反模式。

### 3.4 异步事件订阅清理不严谨

`clipboard-api.ts` 中事件订阅的 `listen()` 是异步返回 unlisten 函数的。如果组件先卸载，再拿到 `dispose`，原实现可能出现：

- 订阅已逻辑卸载，但底层监听仍短暂存在
- 清理时机不够严格

这属于“异步资源初始化后清理不完整”的问题。

### 3.5 一些局部实现存在重复计算和样板代码

例如：

- `ClipList.tsx` 在循环内部重复做 `expandedClipIds.includes(...)`
- `SettingsPanel.tsx` 对多个字段分别写相同的 `onDraftChange` 样板
- `ShortcutRecorder.tsx` 有一些可提取的常量和格式问题

这些问题单独看都不大，但会持续拉低代码整洁度。

---

## 4. 已完成的重构

### 4.1 提取通用防抖 Hook

新增：

- `src/hooks/useDebouncedValue.ts`

作用：

- 统一封装“值延迟生效”的逻辑
- 让 `useClipboardApp` 的搜索防抖和 `settings.tsx` 的自动保存共享同一种实现方式

收益：

- 减少重复 `setTimeout + clearTimeout` 代码
- 提高行为一致性
- 让组件和 Hook 更关注业务语义，而不是计时器细节

### 4.2 将分类配置从 `App.tsx` 中拆出

新增：

- `src/features/clips/categories.tsx`

改动后：

- `App.tsx` 只负责消费 `buildCategories(counts)`
- 分类文案、计数、图标定义集中管理

收益：

- 页面结构更清晰
- 配置和渲染职责分离
- 后续新增分类或调整图标时更容易定位

### 4.3 清理 `App.tsx` 中重复动作包装

本次将若干内联动作改为具名函数，例如：

- 复制选中项
- 粘贴选中项
- 收藏/取消收藏选中项
- 打开设置窗口
- 固定窗口
- 复制文件路径
- 清空当前列表

收益：

- JSX 更短、更容易扫读
- 操作意图更直接
- 降低“临时包一层匿名函数”的噪声

### 4.4 收敛 `useClipboardApp` 中的局部复杂度

本次没有一次性把 `useClipboardApp` 拆成多个 Hook，但先完成了几步关键整理：

- 将搜索防抖迁移到 `useDebouncedValue`
- 提取列表相对选择逻辑
- 提取分类相对切换逻辑
- 保留原有行为不变的前提下，让状态流更直接

收益：

- 主 Hook 里少了一层手写防抖 effect
- 键盘导航逻辑更容易阅读
- 后续继续拆分时更容易下手

### 4.5 修复设置自动保存的竞态问题

在 `src/settings.tsx` 中：

- 使用 `debouncedDraft` 替代直接在 effect 里手写定时器
- 为保存请求增加 `isCurrent` 保护
- 保存成功后，用后端返回值回写 `windowState` 与 `draft`

收益：

- 避免旧请求覆盖新输入
- 减少状态漂移
- 让“界面草稿”和“已保存状态”重新对齐

这是本轮最重要的行为级修复之一。

### 4.6 修复异步订阅清理

在 `src/lib/clipboard-api.ts` 中：

- 为异步 `listen()` 增加 `isDisposed` 标记
- 如果调用方已经取消订阅，则在 `dispose` 返回后立即执行清理

收益：

- 订阅生命周期更严谨
- 减少组件快速挂载/卸载时的潜在泄漏风险

### 4.7 清理局部样板代码

完成的整理包括：

- `ClipList.tsx` 使用 `Set` 处理展开态查找，减少重复线性查询
- `SettingsPanel.tsx` 提取通用 `updateDraft` 更新器
- `ShortcutRecorder.tsx` 提取修饰键常量并统一格式

收益：

- 降低重复
- 代码更规整
- 局部逻辑更容易复用和扩展

---

## 5. 本轮修复的反模式

以下是本次明确识别并处理掉的反模式：

### 5.1 组件文件中混放大量静态配置

表现：

- `App.tsx` 中直接内嵌分类配置和 SVG 定义

处理：

- 提取到 `src/features/clips/categories.tsx`

### 5.2 effect 中手写计时器并直接触发保存

表现：

- `settings.tsx` 通过 `setTimeout` 防抖后立即发请求

处理：

- 改为通用防抖值 + 请求失效保护

### 5.3 异步资源初始化后缺乏稳妥清理

表现：

- 订阅先发起，清理函数后到达时没有处理“已卸载”状态

处理：

- 为订阅封装增加显式销毁标记

### 5.4 大量重复匿名函数和“只包一层 void”的调用方式

表现：

- JSX 中反复出现只做一层转发的点击处理逻辑

处理：

- 改为具名函数，减少噪声

### 5.5 可预先计算的数据在循环内重复计算

表现：

- 列表渲染中重复做展开态判断和预加载索引计算

处理：

- 提前计算 `Set` 和 `preloadIndex`

---

## 6. 当前效果

整理后的代码有几个明显变化：

- 页面组件阅读路径更清晰
- 业务逻辑和展示配置分离度更高
- 设置页自动保存更安全
- Tauri 事件订阅更稳妥
- 局部重复代码明显减少

本次改动以“低风险整理”为主，没有主动改变现有交互和产品行为。

---

## 7. 已完成验证

本轮改动完成后，已执行以下检查：

- `npm run format:write`
- `npm run lint`
- `npm run typecheck`
- `npm run test`

结果：

- 格式化通过
- ESLint 通过
- TypeScript 类型检查通过
- Vitest 测试通过

---

## 8. 建议的下一步重构方向

虽然本轮已经做了一次有效整理，但还有几个方向值得继续推进：

### 8.1 继续拆分 `useClipboardApp`

建议按职责拆成更小的 Hook，例如：

- `useClipQueryState`
- `useClipPagination`
- `useClipSelection`
- `useClipKeyboardNavigation`

这样可以进一步降低主 Hook 的心智负担。

### 8.2 统一前端 action 命名

当前存在：

- `handleXxx`
- `setXxx`
- `toggleXxx`
- `openXxx`

后续可以统一命名约定，减少“状态 setter”和“业务动作”混杂的感觉。

### 8.3 抽出更明确的视图模型层

当前 `ClipList` 内仍承担一部分“显示数据整理”职责，例如：

- `summary`
- `meta`
- 文件主路径
- 展开/收起展示判断

后续可考虑增加轻量 view-model 或 selector 层，进一步减轻组件模板中的判断逻辑。

### 8.4 增加针对竞态和边界条件的测试

建议补充：

- 设置项快速连续修改时的保存行为测试
- 订阅卸载时的清理行为测试
- 搜索输入组合输入法场景的测试

这会让后续继续整理时更安心。

---

## 9. 总结

本轮整理的核心价值不在于“代码变少了多少”，而在于把几个容易继续恶化的问题先按住了：

- 页面组件不再继续膨胀
- 防抖逻辑不再散落
- 设置保存的竞态风险被消除
- 事件订阅生命周期更完整
- 一批重复样板被收敛

这为下一轮更彻底的结构拆分打下了比较稳的基础。
