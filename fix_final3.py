import re

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'r', encoding='utf-8') as f:
    text = f.read()

text = text.replace('image: imageRef.current,', 'image: imageRef.current as any,')
text = text.replace('translatedImg: translatedImgRef.current as HTMLImageElement | undefined,', 'translatedImg: translatedImgRef.current as any,')
text = text.replace('translatedImg: translatedImgRef.current,', 'translatedImg: translatedImgRef.current as any,')

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'w', encoding='utf-8') as f:
    f.write(text)

print("Fixed")
