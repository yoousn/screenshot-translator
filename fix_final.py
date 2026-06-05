import re

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'r', encoding='utf-8') as f:
    text = f.read()

# 1. translateResultPreviewBase64 redeclaration
text = re.sub(r'\s*const \[translateResultPreviewBase64, setTranslateResultPreviewBase64\] = useState<string \| null>\(null\);\n', '\n', text)
text = re.sub(r'\s*const \[translateResultPreviewBase64, setTranslateResultPreviewBase64\] = useState\(""\);\n', '\n', text)

# 2. captureRegionBase64 and resetScreenshotState lazy evaluation
text = text.replace('    captureRegionBase64,\n', '    captureRegionBase64: (...args: any[]) => captureRegionBase64(...args),\n')
text = text.replace('    resetScreenshotState,\n', '    resetScreenshotState: () => resetScreenshotState(),\n')

# 3. HTMLImageElement type mismatch
# src/pages/ScreenshotPage.tsx(1525,7): error TS2322: Type 'HTMLImageElement | HTMLCanvasElement | null | undefined' is not assignable to type 'HTMLImageElement | undefined'.
# Let's find where this is happening.
# It is probably when passing translatedImgRef.current to a function that only accepts HTMLImageElement.
text = text.replace('translatedImg?: HTMLImageElement | undefined', 'translatedImg?: HTMLImageElement | HTMLCanvasElement | null')

# Also wait, the error specifically says: `is not assignable to type 'HTMLImageElement | undefined'`.
# The function parameter must be defined as `img?: HTMLImageElement`.
text = text.replace('img?: HTMLImageElement', 'img?: HTMLImageElement | HTMLCanvasElement | null')

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'w', encoding='utf-8') as f:
    f.write(text)

print("Fixed final issues")
