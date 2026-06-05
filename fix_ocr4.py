import re

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'r', encoding='utf-8') as f:
    text = f.read()

# 1. Move hook instantiation lower
hook_instantiation = '''  const {
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

# Remove current hook instantiation
start_idx = text.find('const {')
if start_idx != -1:
    end_idx = text.find('getTextSourceBlocksForCurrentSelection,\n  });', start_idx)
    if end_idx != -1:
        text = text[:start_idx] + text[end_idx + 51:]

# Insert hook after getTextSourceBlocksForCurrentSelection
insert_pos = text.find('const getTextSourceBlocksForCurrentSelection = async (timeoutMs = 80) => {')
if insert_pos != -1:
    end_pos = text.find('};\n', insert_pos)
    text = text[:end_pos + 3] + '\n' + hook_instantiation + '\n' + text[end_pos + 3:]
else:
    print("Cannot find getTextSourceBlocksForCurrentSelection")

# 2. Fix redeclaration of translateResultPreviewBase64
# Wait, let me just remove it from where it might be in text.
text = re.sub(r'\s*const \[translateResultPreviewBase64, setTranslateResultPreviewBase64\] = useState<string \| null>\(null\);\n', '\n', text)

# 3. Fix draw function parameter
text = text.replace('translatedImg?: HTMLImageElement', 'translatedImg?: HTMLImageElement | HTMLCanvasElement | null')

# 4. Fix setIsTranslating/setIsOCRing directly called
text = text.replace('setIsTranslating(', 'isTranslatingRef.current = (')
text = text.replace('setIsOCRing(', 'isOCRingRef.current = (')

# 5. Fix OCR_WINDOW_SIZE import conflict
# If it's already imported, don't leave another const
text = re.sub(r'\s*const OCR_WINDOW_SIZE = \{ width: 500, height: 400 \};\n', '\n', text)

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'w', encoding='utf-8') as f:
    f.write(text)

with open('tauri-client/src/hooks/useScreenshotOcr.ts', 'r', encoding='utf-8') as f:
    hook = f.read()

# Fix string | undefined -> string
# line 280: previewBase64: translateResultPreviewBase64 || undefined,
hook = hook.replace('previewBase64: translateResultPreviewBase64 || undefined,', 'previewBase64: translateResultPreviewBase64 || "",')

# Export setters
hook = hook.replace('resetOcrState,', 'resetOcrState,\n    setTranslatedResult,\n    setTranslatePairs,')

with open('tauri-client/src/hooks/useScreenshotOcr.ts', 'w', encoding='utf-8') as f:
    f.write(hook)

print("Fixed")
