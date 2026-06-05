import re

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'r', encoding='utf-8') as f:
    text = f.read()

# Remove the state declarations
text = re.sub(r'\s*const \[isTranslating, setIsTranslating\] = useState\(false\);\n', '\n', text)
text = re.sub(r'\s*const \[isOCRing, setIsOCRing\] = useState\(false\);\n', '\n', text)
text = re.sub(r'\s*const \[translatePairs, setTranslatePairs\] = useState<TranslatePair\[\] \| null>\(null\);\n', '\n', text)
text = re.sub(r'\s*const \[translatedResult, setTranslatedResult\] = useState<string \| null>\(null\);\n', '\n', text)
text = re.sub(r'\s*const \[translateResultPreviewBase64, setTranslateResultPreviewBase64\] = useState<string \| null>\(null\);\n', '\n', text)

# Insert the hook at the top of the component (e.g. near `const rectRef = useRef<Rect>(...)`)
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

# Find a good place to insert it
if 'const { isOCRing,' not in text:
    text = text.replace('const rectRef = useRef<Rect>', hook_instantiation + '\n  const rectRef = useRef<Rect>')

# Also OCR_WINDOW_SIZE import conflict!
# src/pages/ScreenshotPage.tsx(16,76): error TS2440: Import declaration conflicts with local declaration of 'OCR_WINDOW_SIZE'.
# Wait, did I leave `const OCR_WINDOW_SIZE` inside `ScreenshotPage.tsx` somewhere?
text = re.sub(r'\s*const OCR_WINDOW_SIZE = \{ width: 500, height: 400 \};\n', '\n', text)

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'w', encoding='utf-8') as f:
    f.write(text)

print("Fixed variables and injected hook")
