# coding=utf-8

dicts_path = 'tauri-client/src/i18n/dictionaries.ts'
with open(dicts_path, 'r', encoding='utf-8') as f:
    dict_content = f.read()

zh_insert = '"zh-CN": {\n    toolbar: {\n      rect: "矩形标注 1",\n      circle: "圆形标注 2",\n      arrow: "箭头标注 3",\n      brush: "画笔 4",\n      text: "文字标注 5 / T",\n      mosaic: "马赛克 6",\n      hintText: "点击选区添加文字；点击已有文字可编辑。",\n      hintShape: "矩形和圆形可拖动移动，边缘可调整大小。",\n      hintMosaic: "拖动需要打码的区域；可用撤销删除。",\n      hintDefault: "拖动绘制标注；可用撤销删除。",\n      move: "移动/调整选区",\n      viewResult: "查看翻译结果",\n      translate: "截图翻译并重绘 Ctrl+Q",\n      ocr: "OCR 识字 Ctrl+D",\n      scrollCapture: "滚动截图",\n      regionRecord: "区域录制",\n      windowRecord: "窗口录制",\n      displayRecord: "显示器录制",\n      recordRegion: "录制选区",\n      pin: "钉图",\n      undo: "撤销 Ctrl+Z",\n      redo: "恢复 Ctrl+Y / Ctrl+Shift+Z",\n      save: "保存 Ctrl+S",\n      cancel: "取消 Esc",\n      copy: "完成并复制 Ctrl+C",\n      size: "大小",\n      color: "颜色",\n    },\n'

en_insert = '"en-US": {\n    toolbar: {\n      rect: "Rectangle 1",\n      circle: "Circle 2",\n      arrow: "Arrow 3",\n      brush: "Brush 4",\n      text: "Text 5 / T",\n      mosaic: "Mosaic 6",\n      hintText: "Click selection to add text; click existing text to edit.",\n      hintShape: "Rectangles and circles can be moved and resized.",\n      hintMosaic: "Drag to blur region; use undo to remove.",\n      hintDefault: "Drag to draw; use undo to remove.",\n      move: "Move/Resize selection",\n      viewResult: "View translation result",\n      translate: "Translate and redraw Ctrl+Q",\n      ocr: "OCR Ctrl+D",\n      scrollCapture: "Scroll capture",\n      regionRecord: "Region recording",\n      windowRecord: "Window recording",\n      displayRecord: "Display recording",\n      recordRegion: "Record region",\n      pin: "Pin",\n      undo: "Undo Ctrl+Z",\n      redo: "Redo Ctrl+Y / Ctrl+Shift+Z",\n      save: "Save Ctrl+S",\n      cancel: "Cancel Esc",\n      copy: "Done and Copy Ctrl+C",\n      size: "Size",\n      color: "Color",\n    },\n'

if 'toolbar: {' not in dict_content:
    dict_content = dict_content.replace('"zh-CN": {\n', zh_insert)
    dict_content = dict_content.replace('"en-US": {\n', en_insert)
    with open(dicts_path, 'w', encoding='utf-8') as f:
        f.write(dict_content)

toolbar_path = 'tauri-client/src/components/screenshot/ScreenshotToolbar.tsx'
with open(toolbar_path, 'r', encoding='utf-8') as f:
    tb = f.read()

if 'import { useI18n } from' not in tb:
    tb = tb.replace('import React from "react";', 'import React from "react";\nimport { useI18n } from "../../i18n";')

tb = tb.replace('''const tools: ToolbarTool[] = [
  { key: "rect", tip: "矩形标注 1", icon: <BorderOutlined /> },
  { key: "circle", tip: "圆形标注 2", icon: <span style={{ fontSize: 22, lineHeight: 1 }}>○</span> },
  { key: "arrow", tip: "箭头标注 3", icon: <ArrowUpOutlined rotate={45} style={{ fontSize: 18 }} /> },
  { key: "brush", tip: "画笔 4", icon: <EditOutlined style={{ fontSize: 18 }} /> },
  { key: "text", tip: "文字标注 5 / T", icon: <span style={{ fontWeight: 800, fontSize: 19 }}>T</span> },
  { key: "mosaic", tip: "马赛克 6", icon: <span style={{ fontSize: 20, lineHeight: 1 }}>▦</span> },
];''', '')

tb = tb.replace('''const getToolHint = (annotationTool: AnnotationTool | null) => {
  if (annotationTool === "text") return "点击选区添加文字；点击已有文字可编辑。";
  if (annotationTool === "rect" || annotationTool === "circle") return "矩形和圆形可拖动移动，边缘可调整大小。";
  if (annotationTool === "mosaic") return "拖动需要打码的区域；可用撤销删除。";
  return "拖动绘制标注；可用撤销删除。";
};''', '')

new_inside_code = '''
  const { text } = useI18n();

  const tools: ToolbarTool[] = [
    { key: "rect", tip: text.toolbar.rect, icon: <BorderOutlined /> },
    { key: "circle", tip: text.toolbar.circle, icon: <span style={{ fontSize: 22, lineHeight: 1 }}>○</span> },
    { key: "arrow", tip: text.toolbar.arrow, icon: <ArrowUpOutlined rotate={45} style={{ fontSize: 18 }} /> },
    { key: "brush", tip: text.toolbar.brush, icon: <EditOutlined style={{ fontSize: 18 }} /> },
    { key: "text", tip: text.toolbar.text, icon: <span style={{ fontWeight: 800, fontSize: 19 }}>T</span> },
    { key: "mosaic", tip: text.toolbar.mosaic, icon: <span style={{ fontSize: 20, lineHeight: 1 }}>▦</span> },
  ];

  const getToolHint = (tool: AnnotationTool | null) => {
    if (tool === "text") return text.toolbar.hintText;
    if (tool === "rect" || tool === "circle") return text.toolbar.hintShape;
    if (tool === "mosaic") return text.toolbar.hintMosaic;
    return text.toolbar.hintDefault;
  };
'''

if 'const { text } = useI18n();' not in tb:
    tb = tb.replace('const normalizedGap = Math.max(0, Math.min(16, Number(buttonGap) || 0));', f'{new_inside_code}\n  const normalizedGap = Math.max(0, Math.min(16, Number(buttonGap) || 0));')

tb = tb.replace('title="移动/调整选区"', 'title={text.toolbar.move}')
tb = tb.replace('label: "查看翻译结果"', 'label: text.toolbar.viewResult')
tb = tb.replace('title="截图翻译并重绘 Ctrl+Q"', 'title={text.toolbar.translate}')
tb = tb.replace('A/文', 'A') 
tb = tb.replace('title="OCR 识字 Ctrl+D"', 'title={text.toolbar.ocr}')
tb = tb.replace('title="滚动截图"', 'title={text.toolbar.scrollCapture}')

tb = tb.replace('label: "区域录制"', 'label: text.toolbar.regionRecord')
tb = tb.replace('label: "窗口录制"', 'label: text.toolbar.windowRecord')
tb = tb.replace('label: "显示器录制"', 'label: text.toolbar.displayRecord')
tb = tb.replace('title="录制选区"', 'title={text.toolbar.recordRegion}')

tb = tb.replace('title="钉图"', 'title={text.toolbar.pin}')
tb = tb.replace('title="撤销 Ctrl+Z"', 'title={text.toolbar.undo}')
tb = tb.replace('title="恢复 Ctrl+Y / Ctrl+Shift+Z"', 'title={text.toolbar.redo}')
tb = tb.replace('title="保存 Ctrl+S"', 'title={text.toolbar.save}')
tb = tb.replace('title="取消 Esc"', 'title={text.toolbar.cancel}')
tb = tb.replace('title="完成并复制 Ctrl+C"', 'title={text.toolbar.copy}')

tb = tb.replace('<span>大小</span>', '<span>{text.toolbar.size}</span>')
tb = tb.replace('<span>颜色</span>', '<span>{text.toolbar.color}</span>')

with open(toolbar_path, 'w', encoding='utf-8') as f:
    f.write(tb)

print("Toolbar patched successfully!")
