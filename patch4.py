import re

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'r', encoding='utf-8') as f:
    text = f.read()

# Verify constant locations
print('Has DEFAULT_ANNOTATION_COLOR?', 'const DEFAULT_ANNOTATION_COLOR = "#ff4d4f";' in text)
print('Has annotation state block?', 'const [annotationTool, setAnnotationToolState] = useState' in text)
