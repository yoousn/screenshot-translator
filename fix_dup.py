import re

with open('tauri-client/src/hooks/useScreenshotAnnotation.ts', 'r', encoding='utf-8') as f:
    lines = f.readlines()

lines = [line for line in lines if not line.startswith('import { makeTextAnnotation }')]
lines.insert(2, 'import { makeTextAnnotation } from "../utils/annotationGeometry";\n')

with open('tauri-client/src/hooks/useScreenshotAnnotation.ts', 'w', encoding='utf-8') as f:
    f.writelines(lines)
