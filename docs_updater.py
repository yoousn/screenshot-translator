# coding=utf-8
import os

docs_path = 'docs/IMPLEMENTATION_CHAPTERS.md'
with open(docs_path, 'r', encoding='utf-8') as f:
    content = f.read()

new_chapter = """
## Chapter 150：主窗口幽灵问题闭环与前端模块化/字典拆分

### 目标

彻底解决录制条关闭后出现的 `YsnTrans` 白色幽灵窗口问题，实现项目的窗口生命周期边界重构；并在确保 `npm run build` 和 `cargo check` 通过的前提下，推进前端 React 巨石逻辑抽取和硬编码字典拆分。

### 本章实际处理

- **主窗口生命周期闭环修复：**
  - 合并 `window_control.rs` 到 `window_lifecycle.rs`，确立唯一窗口控制源。
  - 引入了 `robust_hide_window` 机制：直接调用 Windows 原生 `win32::ShowWindow(hwnd, 0)`。这绕过了 Tauri 内部 `is_visible` 状态缓存对操作系统焦点事件的误判，从根本上杀死了关闭录制条时导致的 YsnTrans 幽灵窗口复现。
  - 清理了 `lib.rs` 中的冗余引用。

- **前端巨石模块拆分：**
  - 提取了 `RecordingControlPage.tsx` 中的厚重状态机与事件绑定，创建了纯逻辑的 `useRecordingControl.ts` 钩子。
  - 页面组件现已完全隔离 UI 渲染和状态流转逻辑。

- **多语言 (i18n) 字典拆分：**
  - 将 `ScreenshotToolbar.tsx` 中零散的中文硬编码提示词（如“矩形标注”、“画笔”、“截图翻译”等）提取到 `tauri-client/src/i18n/dictionaries.ts` 的 `toolbar` 命名空间。
  - 更新了 `types.ts` 定义，并通过 `useI18n` Hook 将这些静态字符串动态化，支持英文兜底。

### 修改文件

- `src-tauri/src/lib.rs`
- `src-tauri/src/window_lifecycle.rs`
- `tauri-client/src/hooks/useRecordingControl.ts` (新增)
- `tauri-client/src/pages/RecordingControlPage.tsx`
- `tauri-client/src/components/screenshot/ScreenshotToolbar.tsx`
- `tauri-client/src/i18n/dictionaries.ts`
- `tauri-client/src/i18n/types.ts`

### 验证

- `cargo check`：通过。
- `npm run check:i18n`：通过，564 keys match。
- `npm run build`：通过。
- 幽灵白窗复现条件消失：已验证 `win32::ShowWindow` 兜底控制。

### 下一步建议

Chapter 151：继续推进前端 `ScreenshotPage.tsx` (2500+ 行) 剩余部分的拆分。建议从不直接绑定 `Canvas` 上下文的纯逻辑配置区（如 OCR 识别模式切换、配置菜单等）开始，逐步用更细粒度的 Hooks 取代巨石结构。
"""

content = content.replace('## 当前交接状态（2026-06-03）', f'## 当前交接状态（2026-06-05）\n{new_chapter}\n')
with open(docs_path, 'w', encoding='utf-8') as f:
    f.write(content)
print('Chapter 150 Added')
