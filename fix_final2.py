import re

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'r', encoding='utf-8') as f:
    text = f.read()

text = text.replace('captureRegionBase64: (...args: any[]) => captureRegionBase64(...args),', 'captureRegionBase64: () => captureRegionBase64(),')

# For the HTMLImageElement error at line 1524. It's likely in renderScreenshotCanvas call
# renderScreenshotCanvas({ ... translatedImg: translatedImgRef.current })
text = text.replace('translatedImg: translatedImgRef.current,', 'translatedImg: translatedImgRef.current as HTMLImageElement | undefined,')

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'w', encoding='utf-8') as f:
    f.write(text)

print("Fixed")
