# coding=utf-8
import os

types_path = 'tauri-client/src/i18n/types.ts'
with open(types_path, 'r', encoding='utf-8') as f:
    content = f.read()

toolbar_interface = '''
  toolbar: {
    rect: string;
    circle: string;
    arrow: string;
    brush: string;
    text: string;
    mosaic: string;
    hintText: string;
    hintShape: string;
    hintMosaic: string;
    hintDefault: string;
    move: string;
    viewResult: string;
    translate: string;
    ocr: string;
    scrollCapture: string;
    regionRecord: string;
    windowRecord: string;
    displayRecord: string;
    recordRegion: string;
    pin: string;
    undo: string;
    redo: string;
    save: string;
    cancel: string;
    copy: string;
    size: string;
    color: string;
  };
'''

if 'toolbar: {' not in content:
    content = content.replace('export interface AppDictionary {', f'export interface AppDictionary {{\n{toolbar_interface}')
    with open(types_path, 'w', encoding='utf-8') as f:
        f.write(content)
    print('Patched types.ts')

dicts_path = 'tauri-client/src/i18n/dictionaries.ts'
with open(dicts_path, 'r', encoding='utf-8') as f:
    dict_content = f.read()

zh_toolbar = '''
    toolbar: {
      rect: "矩形标注 1",
      circle: "圆形标注 2",
      arrow: "箭头标注 3",
      brush: "画笔 4",
      text: "文字标注 5 / T",
      mosaic: "马赛克 6",
      hintText: "点击选区添加文字；点击已有文字可编辑。",
      hintShape: "矩形和圆形可拖动移动，边缘可调整大小。",
      hintMosaic: "拖动需要打码的区域；可用撤销删除。",
      hintDefault: "拖动绘制标注；可用撤销删除。",
      move: "移动/调整选区",
      viewResult: "查看翻译结果",
      translate: "截图翻译并重绘 Ctrl+Q",
      ocr: "OCR 识字 Ctrl+D",
      scrollCapture: "滚动截图",
      regionRecord: "区域录制",
      windowRecord: "窗口录制",
      displayRecord: "显示器录制",
      recordRegion: "录制选区",
      pin: "钉图",
      undo: "撤销 Ctrl+Z",
      redo: "恢复 Ctrl+Y / Ctrl+Shift+Z",
      save: "保存 Ctrl+S",
      cancel: "取消 Esc",
      copy: "完成并复制 Ctrl+C",
      size: "大小",
      color: "颜色",
    },
'''

en_toolbar = '''
    toolbar: {
      rect: "Rectangle 1",
      circle: "Circle 2",
      arrow: "Arrow 3",
      brush: "Brush 4",
      text: "Text 5 / T",
      mosaic: "Mosaic 6",
      hintText: "Click selection to add text; click existing text to edit.",
      hintShape: "Rectangles and circles can be moved and resized.",
      hintMosaic: "Drag to blur region; use undo to remove.",
      hintDefault: "Drag to draw; use undo to remove.",
      move: "Move/Resize selection",
      viewResult: "View translation result",
      translate: "Translate and redraw Ctrl+Q",
      ocr: "OCR Ctrl+D",
      scrollCapture: "Scroll capture",
      regionRecord: "Region recording",
      windowRecord: "Window recording",
      displayRecord: "Display recording",
      recordRegion: "Record region",
      pin: "Pin",
      undo: "Undo Ctrl+Z",
      redo: "Redo Ctrl+Y / Ctrl+Shift+Z",
      save: "Save Ctrl+S",
      cancel: "Cancel Esc",
      copy: "Done and Copy Ctrl+C",
      size: "Size",
      color: "Color",
    },
'''

if 'toolbar: {' not in dict_content:
    dict_content = dict_content.replace('"zh-CN": {', f'"zh-CN": {{\n{zh_toolbar}')
    dict_content = dict_content.replace('"en-US": {', f'"en-US": {{\n{en_toolbar}')
    with open(dicts_path, 'w', encoding='utf-8') as f:
        f.write(dict_content)
    print('Patched dictionaries.ts')
