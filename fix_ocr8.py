import re

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'r', encoding='utf-8') as f:
    text = f.read()

# 1. Hook missing
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
    getTextSourceBlocksForCurrentSelection: (...args) => getTextSourceBlocksForCurrentSelection(...args),
  });
'''

if 'useScreenshotOcr({' not in text:
    # Insert right after `const configRef = useRef<Config>({});`
    insert_pos = text.find('const configRef = useRef<Config>({});')
    if insert_pos != -1:
        end_pos = text.find('\n', insert_pos)
        text = text[:end_pos] + '\n' + hook_instantiation + text[end_pos:]
    else:
        print("Could not find configRef")

# 2. Fix layout imports
text = re.sub(r'\s*const OCR_WINDOW_SIZE = \{ width: 500, height: 400 \};\n', '\n', text)

# 3. HTMLImageElement | undefined -> HTMLImageElement | HTMLCanvasElement | null
text = text.replace('translatedImg?: HTMLImageElement', 'translatedImg?: HTMLImageElement | HTMLCanvasElement | null')

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'w', encoding='utf-8') as f:
    f.write(text)

print("Applied final fixes v8")
