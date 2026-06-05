import re

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'r', encoding='utf-8') as f:
    text = f.read()

# Fix setAnnotationToolState -> setAnnotationTool
text = text.replace('setAnnotationToolState(', 'setAnnotationTool(')
text = text.replace('setDraftAnnotation(', 'setAnnotationDraft(')

# Fix remaining redeclared refs
refs_to_remove = [
    'annotationToolRef', 'annotationColorRef', 'annotationSizeRef', 'annotationSizesRef',
    'selectedAnnotationIndexRef', 'annotationsRef', 'annotationHistoryRef', 'redoAnnotationsRef',
    'draftAnnotationRef', 'editingTextDraftRef', 'cancelTextDraft'
]

for ref in refs_to_remove:
    # Remove lines like: const annotationToolRef = useRef(...);
    # Or const cancelTextDraft = ...;
    text = re.sub(r'^\s*const\s+' + ref + r'\s*=.*?\n', '', text, flags=re.MULTILINE)

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'w', encoding='utf-8') as f:
    f.write(text)


with open('tauri-client/src/hooks/useScreenshotAnnotation.ts', 'r', encoding='utf-8') as f:
    hook = f.read()

hook = hook.replace('import type { Annotation, AnnotationTool, EditingTextDraft } from "../types/screenshot";',
                    'import type { Annotation, AnnotationTool, EditingTextDraft } from "../types/screenshot";\nimport { makeTextAnnotation } from "../utils/annotationGeometry";')

hook = hook.replace('''commitAnnotation({
        type: "text",
        rect: { ...draft.rect },
        color: draft.color,
        size: draft.size,
        text: value,
      });''', '''commitAnnotation(makeTextAnnotation({ x: draft.x + 90, y: draft.y + 17 }, value, annotationColorRef.current, annotationSizeRef.current));''')

with open('tauri-client/src/hooks/useScreenshotAnnotation.ts', 'w', encoding='utf-8') as f:
    f.write(hook)

print('Done fixing')
