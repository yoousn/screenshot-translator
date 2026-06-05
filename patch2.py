# coding=utf-8
import os

file_path = 'tauri-client/src/components/screenshot/ScreenshotToolbar.tsx'
with open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

if 'import { useI18n } from' not in content:
    content = content.replace('import React from "react";', 'import React from "react";\nimport { useI18n } from "../../i18n";')

# add const { t } = useI18n(); inside ScreenshotToolbar
if 'const { t } = useI18n();' not in content:
    content = content.replace('export default function ScreenshotToolbar({', 'export default function ScreenshotToolbar({\n')
    # find the block with normalizedGap
    content = content.replace('const normalizedGap =', 'const { t } = useI18n();\n  const normalizedGap =')

# Replace tools array logic
content = content.replace('''const tools: ToolbarTool[] = [
  { key: "rect", tip: "矩形标注 1", icon: <BorderOutlined /> },
  { key: "circle", tip: "圆形标注 2", icon: <span style={{ fontSize: 22, lineHeight: 1 }}>○</span> },
  { key: "arrow", tip: "箭头标注 3", icon: <ArrowUpOutlined rotate={45} style={{ fontSize: 18 }} /> },
  { key: "brush", tip: "画笔 4", icon: <EditOutlined style={{ fontSize: 18 }} /> },
  { key: "text", tip: "文字标注 5 / T", icon: <span style={{ fontWeight: 800, fontSize: 19 }}>T</span> },
  { key: "mosaic", tip: "马赛克 6", icon: <span style={{ fontSize: 20, lineHeight: 1 }}>▦</span> },
];''', '''// tools array is moved inside the component to use `t`''')

content = content.replace('''const getToolHint = (annotationTool: AnnotationTool | null) => {
  if (annotationTool === "text") return "点击选区添加文字；点击已有文字可编辑。";
  if (annotationTool === "rect" || annotationTool === "circle") return "矩形和圆形可拖动移动，边缘可调整大小。";
  if (annotationTool === "mosaic") return "拖动需要打码的区域；可用撤销删除。";
  return "拖动绘制标注；可用撤销删除。";
};''', '')

# Inside component, define tools and getToolHint
new_inside_code = '''
  const tools: ToolbarTool[] = [
    { key: "rect", tip: t("toolbar.rect"), icon: <BorderOutlined /> },
    { key: "circle", tip: t("toolbar.circle"), icon: <span style={{ fontSize: 22, lineHeight: 1 }}>○</span> },
    { key: "arrow", tip: t("toolbar.arrow"), icon: <ArrowUpOutlined rotate={45} style={{ fontSize: 18 }} /> },
    { key: "brush", tip: t("toolbar.brush"), icon: <EditOutlined style={{ fontSize: 18 }} /> },
    { key: "text", tip: t("toolbar.text"), icon: <span style={{ fontWeight: 800, fontSize: 19 }}>T</span> },
    { key: "mosaic", tip: t("toolbar.mosaic"), icon: <span style={{ fontSize: 20, lineHeight: 1 }}>▦</span> },
  ];

  const getToolHint = (tool: AnnotationTool | null) => {
    if (tool === "text") return t("toolbar.hintText");
    if (tool === "rect" || tool === "circle") return t("toolbar.hintShape");
    if (tool === "mosaic") return t("toolbar.hintMosaic");
    return t("toolbar.hintDefault");
  };
'''

content = content.replace('const { t } = useI18n();\n  const normalizedGap =', f'const {{ t }} = useI18n();\n{new_inside_code}\n  const normalizedGap =')

# Replace tooltips and labels
content = content.replace('title="移动/调整选区"', 'title={t("toolbar.move")}')
content = content.replace('label: "查看翻译结果"', 'label: t("toolbar.viewResult")')
content = content.replace('title="截图翻译并重绘 Ctrl+Q"', 'title={t("toolbar.translate")}')
content = content.replace('A/文', 'A') # Assuming A is neutral enough for both languages
content = content.replace('title="OCR 识字 Ctrl+D"', 'title={t("toolbar.ocr")}')
content = content.replace('title="滚动截图"', 'title={t("toolbar.scrollCapture")}')

content = content.replace('label: "区域录制"', 'label: t("toolbar.regionRecord")')
content = content.replace('label: "窗口录制"', 'label: t("toolbar.windowRecord")')
content = content.replace('label: "显示器录制"', 'label: t("toolbar.displayRecord")')
content = content.replace('title="录制选区"', 'title={t("toolbar.recordRegion")}')

content = content.replace('title="钉图"', 'title={t("toolbar.pin")}')
content = content.replace('title="撤销 Ctrl+Z"', 'title={t("toolbar.undo")}')
content = content.replace('title="恢复 Ctrl+Y / Ctrl+Shift+Z"', 'title={t("toolbar.redo")}')
content = content.replace('title="保存 Ctrl+S"', 'title={t("toolbar.save")}')
content = content.replace('title="取消 Esc"', 'title={t("toolbar.cancel")}')
content = content.replace('title="完成并复制 Ctrl+C"', 'title={t("toolbar.copy")}')

content = content.replace('<span>大小</span>', '<span>{t("toolbar.size")}</span>')
content = content.replace('<span>颜色</span>', '<span>{t("toolbar.color")}</span>')

with open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)

print("ScreenshotToolbar patched!")
