import re

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'r', encoding='utf-8') as f:
    text = f.read()

text = text.replace('overrideTranslatedImg: translatedImg,', 'overrideTranslatedImg: translatedImg as any,')

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'w', encoding='utf-8') as f:
    f.write(text)

print("Fixed")
