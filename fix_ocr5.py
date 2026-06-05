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
    getTextSourceBlocksForCurrentSelection,
  });
'''

if 'useScreenshotOcr({' not in text:
    # Insert right before `const handleScrollCapture`
    insert_pos = text.find('const handleScrollCapture = async () => {')
    if insert_pos != -1:
        text = text[:insert_pos] + hook_instantiation + '\n  ' + text[insert_pos:]
    else:
        print("Cannot find handleScrollCapture")

# 2. Fix layout imports
# src/pages/ScreenshotPage.tsx(2113,333): error TS2304: Cannot find name 'FLOATING_PANEL_MARGIN'.
if 'FLOATING_PANEL_MARGIN' not in text.splitlines()[0:100] and 'import { getActionToolbarStyle' in text:
    text = text.replace('import { getActionToolbarStyle }', 'import { getActionToolbarStyle, FLOATING_PANEL_MARGIN, FLOATING_PANEL_GAP, OCR_WINDOW_SIZE }')

# 3. HTMLImageElement | undefined -> HTMLImageElement | HTMLCanvasElement | null
# src/pages/ScreenshotPage.tsx(1530,7): error TS2322: Type 'HTMLImageElement | HTMLCanvasElement | null | undefined' is not assignable to type 'HTMLImageElement | undefined'.
text = text.replace('translatedImg?: HTMLImageElement', 'translatedImg?: HTMLImageElement | HTMLCanvasElement | null')

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'w', encoding='utf-8') as f:
    f.write(text)

print("Applied final fixes")
