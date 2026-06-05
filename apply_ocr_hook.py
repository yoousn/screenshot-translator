import re
import os

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'r', encoding='utf-8') as f:
    text = f.read()

# 1. Config Interface Extraction
config_def = '''export interface Config {
  serverUrl?: string;
  lanServerUrl?: string;
  preferLanServer?: boolean;
  clientToken?: string;
  useLocalOcr?: boolean;
  fallbackToRemoteOcr?: boolean;
  localOcrTimeoutMs?: number;
  translationTimeoutMs?: number;
  rapidOcrWorkerEnabled?: boolean;
  targetLang?: string;
  channel?: string;
  enableUiControlDetection?: boolean;
  enableVisualDetection?: boolean;
  detectionBorderWidth?: number;
  toolbarButtonGap?: number;
  visualDetectionSensitivity?: number;
}
'''
with open('tauri-client/src/types/config.ts', 'w', encoding='utf-8') as f:
    f.write(config_def)

text = re.sub(r'interface Config \{.*?\n\}\n', '', text, flags=re.DOTALL)
if 'import type { Config }' not in text:
    text = text.replace('import type { Annotation,', 'import type { Config } from "../types/config";\nimport type { Annotation,')

# 2. Layout Constants Move
layout_constants = '''
export const FLOATING_PANEL_MARGIN = 8;
export const FLOATING_PANEL_GAP = 8;
export const OCR_WINDOW_SIZE = { width: 500, height: 400 };
'''
if not os.path.exists('tauri-client/src/utils/screenshotLayout.ts'):
    print("screenshotLayout.ts not found")
else:
    with open('tauri-client/src/utils/screenshotLayout.ts', 'r', encoding='utf-8') as f:
        layout_text = f.read()
    if 'FLOATING_PANEL_MARGIN' not in layout_text:
        with open('tauri-client/src/utils/screenshotLayout.ts', 'a', encoding='utf-8') as f:
            f.write(layout_constants)

text = re.sub(r'const FLOATING_PANEL_MARGIN = 8;\n', '', text)
text = re.sub(r'const FLOATING_PANEL_GAP = 8;\n', '', text)
text = re.sub(r'const OCR_WINDOW_SIZE = \{ width: 500, height: 400 \};\n', '', text)

if 'FLOATING_PANEL_MARGIN' not in text and 'import { getActionToolbarStyle' in text:
    text = text.replace('import { getActionToolbarStyle }', 'import { getActionToolbarStyle, FLOATING_PANEL_MARGIN, FLOATING_PANEL_GAP, OCR_WINDOW_SIZE }')

# 3. Add useScreenshotOcr import
if 'useScreenshotOcr' not in text:
    text = text.replace('import { useScreenshotAnnotation', 'import { useScreenshotOcr } from "../hooks/useScreenshotOcr";\nimport { useScreenshotAnnotation')

# 4. Remove OCR/Translate state variables
text = re.sub(r'\s*const \[isTranslating, setIsTranslating\] = useState\(false\);\n', '\n', text)
text = re.sub(r'\s*const \[isOCRing, setIsOCRing\] = useState\(false\);\n', '\n', text)
text = re.sub(r'\s*const \[translatePairs, setTranslatePairs\] = useState<TranslatePair\[\] \| null>\(null\);\n', '\n', text)
text = re.sub(r'\s*const \[translatedResult, setTranslatedResult\] = useState<string \| null>\(null\);\n', '\n', text)
text = re.sub(r'\s*const \[translateResultPreviewBase64, setTranslateResultPreviewBase64\] = useState<string \| null>\(null\);\n', '\n', text)

text = re.sub(r'\s*const isOCRingRef = useRef\(false\);\n', '\n', text)
text = re.sub(r'\s*const isTranslatingRef = useRef\(false\);\n', '\n', text)
text = re.sub(r'\s*const ocrPrewarmPromiseRef = useRef<Promise<any> \| null>\(null\);\n', '\n', text)

# 5. Insert Hook Instantiation
hook_instantiation = '''
  const {
    isOCRing,
    isTranslating,
    translatePairs,
    translatedResult,
    translateResultPreviewBase64,
    prewarmLocalOcrWorker,
    handleOCR,
    handleTranslate,
    handleShowTranslateResult,
    resetOcrState,
    isOCRingRef,
    isTranslatingRef,
    setTranslatedResult,
    setTranslatePairs,
  } = useScreenshotOcr({
    config: configRef.current,
    rectRef,
    captureRegionBase64,
    resetScreenshotState,
    draw,
    translatedImgRef,
    getTextSourceBlocksForCurrentSelection,
  });
'''
if 'const { isOCRing,' not in text:
    insert_pos = text.find('const getTextSourceBlocksForCurrentSelection = async (timeoutMs = 80) => {')
    if insert_pos != -1:
        end_pos = text.find('};\n', insert_pos)
        text = text[:end_pos + 3] + '\n' + hook_instantiation + '\n' + text[end_pos + 3:]

# 6. Fix function parameter in ScreenshotPage.tsx
text = text.replace('translatedImg?: HTMLImageElement', 'translatedImg?: HTMLImageElement | HTMLCanvasElement | null')

# 7. Replace old setIsTranslating directly called
text = text.replace('setIsTranslating(', 'isTranslatingRef.current = (')
text = text.replace('setIsOCRing(', 'isOCRingRef.current = (')

# 8. Replace OCR reset
reset_pattern = re.compile(
    r'\s*setIsOCRing\(false\);\n'
    r'\s*setIsTranslating\(false\);\n'
    r'\s*isOCRingRef\.current = false;\n'
    r'\s*isTranslatingRef\.current = false;\n'
    r'\s*setTranslatePairs\(null\);\n'
    r'\s*setTranslatedResult\(null\);\n'
    r'\s*setTranslateResultPreviewBase64\(null\);\n'
)
text = reset_pattern.sub('\n    resetOcrState();\n', text)

# 9. Remove extracted functions
funcs_to_remove = [
  'prewarmLocalOcrWorker',
  'normalizeScreenshotTranslateError',
  'handleOCR',
  'handleTranslate',
  'handleShowTranslateResult'
]

for func in funcs_to_remove:
    pattern = re.compile(r'\s*const\s+' + func + r'\s*=\s*(async\s+)?(\([^)]*\)|[a-zA-Z0-9_]+)\s*=>\s*\{')
    while True:
        match = pattern.search(text)
        if not match:
            break
        start = match.start()
        brace_count = 0
        in_string = False
        string_char = ''
        end = -1
        for i in range(match.end() - 1, len(text)):
            c = text[i]
            if not in_string:
                if c in ('"', "'", '`'):
                    in_string = True
                    string_char = c
                elif c == '{':
                    brace_count += 1
                elif c == '}':
                    brace_count -= 1
                    if brace_count == 0:
                        end = i + 1
                        break
            else:
                if c == string_char and text[i-1] != '\\':
                    in_string = False
        
        if end != -1:
            if end < len(text) and text[end] == ';':
                end += 1
            text = text[:start] + text[end:]
        else:
            break

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'w', encoding='utf-8') as f:
    f.write(text)

# 10. Fix useScreenshotOcr.ts
with open('tauri-client/src/hooks/useScreenshotOcr.ts', 'r', encoding='utf-8') as f:
    hook = f.read()

if 'isOCRingRef,' not in hook:
    hook = hook.replace('resetOcrState,', 'resetOcrState,\n    setTranslatedResult,\n    setTranslatePairs,\n    isOCRingRef,\n    isTranslatingRef,')
hook = hook.replace('previewBase64: translateResultPreviewBase64 || undefined,', 'previewBase64: translateResultPreviewBase64 || "",')

with open('tauri-client/src/hooks/useScreenshotOcr.ts', 'w', encoding='utf-8') as f:
    f.write(hook)

print("OCR Hook perfectly applied")
