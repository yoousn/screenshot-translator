import os
import re

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'r', encoding='utf-8') as f:
    text = f.read()

# 1. Config Interface
config_def = '''export interface Config {
  serverUrl?: string;
  lanServerUrl?: string;
  preferLanServer?: boolean;
  clientToken?: string;
  useLocalOcr?: boolean;
  localOcrTimeoutMs?: number;
  channel?: string;
  targetLang?: string;
  rapidOcrWorkerEnabled?: boolean;
  toolbarButtonGap?: number;
}
'''
with open('tauri-client/src/types/config.ts', 'w', encoding='utf-8') as f:
    f.write(config_def)

text = re.sub(r'interface Config \{.*?\n\}\n', '', text, flags=re.DOTALL)
if 'import type { Config }' not in text:
    text = text.replace('import type { Annotation,', 'import type { Config } from "../types/config";\nimport type { Annotation,')

# 2. Layout Constants
layout_constants = '''
export const FLOATING_PANEL_MARGIN = 8;
export const FLOATING_PANEL_GAP = 8;
export const OCR_WINDOW_SIZE = { width: 500, height: 400 };
'''
with open('tauri-client/src/utils/screenshotLayout.ts', 'a', encoding='utf-8') as f:
    f.write(layout_constants)

text = re.sub(r'const FLOATING_PANEL_MARGIN = 8;\n', '', text)
text = re.sub(r'const FLOATING_PANEL_GAP = 8;\n', '', text)
text = re.sub(r'const OCR_WINDOW_SIZE = \{ width: 500, height: 400 \};\n', '', text)

text = text.replace('import { getActionToolbarStyle } from "../utils/screenshotLayout";', 'import { getActionToolbarStyle, FLOATING_PANEL_MARGIN, FLOATING_PANEL_GAP, OCR_WINDOW_SIZE } from "../utils/screenshotLayout";')

# 3. Hook Integration
# My previous patch inserted it? Let's check.
if 'useScreenshotOcr({' not in text:
    hook_instantiation = '''const {
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
  } = useScreenshotOcr({
    config: configRef.current,
    rectRef,
    captureRegionBase64,
    resetScreenshotState,
    draw,
    translatedImgRef,
    getTextSourceBlocksForCurrentSelection,
  });\n'''
    state_block_pattern = re.compile(
        r'\s*const \[isOCRing, setIsOCRing] = useState\(false\);\n'
        r'\s*const \[isTranslating, setIsTranslating] = useState\(false\);\n'
        r'\s*const \[translatePairs, setTranslatePairs] = useState<TranslatePair\[\] \| null>\(null\);\n'
        r'\s*const \[translatedResult, setTranslatedResult] = useState<string \| null>\(null\);\n'
        r'\s*const \[translateResultPreviewBase64, setTranslateResultPreviewBase64] = useState<string \| null>\(null\);\n'
    )
    text = state_block_pattern.sub('\n  ' + hook_instantiation, text)

# Remove old refs
text = re.sub(r'\s*const isOCRingRef = useRef\(false\);\n', '\n', text)
text = re.sub(r'\s*const isTranslatingRef = useRef\(false\);\n', '\n', text)
text = re.sub(r'\s*const ocrPrewarmPromiseRef = useRef<Promise<any> \| null>\(null\);\n', '\n', text)

# 4. Remove extracted functions
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

print("Fix applied")
