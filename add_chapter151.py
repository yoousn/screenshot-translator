# coding=utf-8
import os

docs_path = 'docs/IMPLEMENTATION_CHAPTERS.md'
with open(docs_path, 'r', encoding='utf-8') as f:
    content = f.read()

new_chapter = """
## Chapter 151：前端巨石代码拆解：截图批注 Hooks 抽取

### 目标

继续响应用户指令，拆分 `ScreenshotPage.tsx` 中的巨石逻辑。本轮聚焦于独立性最强的“标注管理模块”，将其剥离出主文件，进一步减轻 React UI 组件的心智负担，同时保障编译链完全不受损。

### 本章实际处理

- **抽取 `useScreenshotAnnotation` Hook**：
  - 将庞大的批注状态管理（包含 10 个以上 `useState` 与相关的 `useRef`，如 `annotationTool`, `annotationColor`, `annotations`, `annotationHistory`, `draftAnnotation` 等）和相关的核心操作函数（如 `pushAnnotationHistory`, `undoAnnotation`, `redoAnnotation`, `commitTextDraft`, `deleteSelectedAnnotation`）从 `ScreenshotPage.tsx` 提取到了纯逻辑文件 `tauri-client/src/hooks/useScreenshotAnnotation.ts`。
  - 保留了与 Canvas 重绘生命周期的交互，通过向 Hook 传入 `onRenderNeeded` 回调来无缝触发重绘。
  - 将与批注默认属性相关的硬编码常量（如 `DEFAULT_ANNOTATION_COLOR`, `DEFAULT_ANNOTATION_TOOL`）也一并集中封装在 Hook 文件顶部导出。

- **`ScreenshotPage.tsx` 安全更新**：
  - 成功移除了超过百行的状态定义、引用绑定以及增删改查实现函数，将其收敛至一行 Hook 的调用。
  - 通过 `replace_file_content` 和脚本自动化精确移除了旧版的所有硬编码操作，修复了因为抽取造成的 TS 类型断层，所有旧的 refs 已平滑迁移至 Hook 管理的闭包中。

### 修改文件

- `tauri-client/src/hooks/useScreenshotAnnotation.ts` (新增)
- `tauri-client/src/pages/ScreenshotPage.tsx`

### 验证

- `cargo check`：通过。
- `npm run check:i18n`：通过，564 keys match。
- `npm run build`：通过。无 TS 类型缺失与语法错误。

### 下一步建议

Chapter 152：`ScreenshotPage.tsx` 中仍然含有深耦合的画图逻辑与 OCR/翻译逻辑。下一步建议提取 `useScreenshotOcr.ts`（处理翻译与 OCR 发起及结果预览窗口状态）和 `useScrollCapture.ts`（滚动截图机制），以完成对主要纯业务状态流转的全面剥离，最终将 `ScreenshotPage.tsx` 还原为存粹的事件绑定与子组件容器。
"""

content = content.replace('## 当前交接状态（2026-06-05）\n', f'## 当前交接状态（2026-06-05）\n{new_chapter}\n')
with open(docs_path, 'w', encoding='utf-8') as f:
    f.write(content)
print('Chapter 151 Added')
