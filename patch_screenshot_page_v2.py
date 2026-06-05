import re

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'r', encoding='utf-8') as f:
    text = f.read()

# 1. Add import
if 'useScreenshotAnnotation' not in text:
    text = text.replace(
        'import type { Annotation, AnnotationTool, EditingTextDraft, OcrBlock, Point, Rect, TranslatePair } from "../types/screenshot";',
        'import type { Annotation, AnnotationTool, EditingTextDraft, OcrBlock, Point, Rect, TranslatePair } from "../types/screenshot";\nimport { useScreenshotAnnotation, DEFAULT_ANNOTATION_COLOR, DEFAULT_ANNOTATION_TOOL, DEFAULT_ANNOTATION_SIZES } from "../hooks/useScreenshotAnnotation";'
    )

# 2. Remove constant definitions
text = text.replace('const DEFAULT_ANNOTATION_COLOR = "#ff4d4f";\n', '')
text = text.replace('const DEFAULT_ANNOTATION_TOOL: AnnotationTool = "rect";\n', '')
text = text.replace('const DEFAULT_ANNOTATION_SIZES: Record<AnnotationTool, number> = { rect: 4, circle: 4, mosaic: 16, arrow: 4, text: 4, brush: 4 };\n', '')

# 3. Replace state block
hook_instantiation = '''const {
    annotationTool, setAnnotationTool,
    annotationColor, setAnnotationColor,
    annotationSize, setAnnotationSize: setAnnotationSizeState,
    selectedAnnotationIndex, setSelectedAnnotationIndex,
    editingTextDraft, setEditingTextDraft,
    annotations, setAnnotations,
    annotationHistory, setAnnotationHistory,
    redoAnnotations, setRedoAnnotations,
    draftAnnotation, setAnnotationDraft,

    annotationToolRef, annotationColorRef, annotationSizeRef, annotationSizesRef,
    selectedAnnotationIndexRef, annotationsRef, annotationHistoryRef, redoAnnotationsRef,
    draftAnnotationRef, editingTextDraftRef,

    pushAnnotationHistory, undoAnnotation, redoAnnotation, commitAnnotation,
    cancelTextDraft, commitTextDraft, deleteSelectedAnnotation, applyAnnotations,
    replaceAnnotations, resetAnnotations
  } = useScreenshotAnnotation(() => {
    renderNeededRef.current = true;
  });\n'''

# We know the exact text of the state block to remove:
state_block_pattern = re.compile(
    r'\s*const \[annotationTool, setAnnotationToolState] = useState<AnnotationTool \| null>\(null\);\n'
    r'\s*const \[annotationColor, setAnnotationColor] = useState\(DEFAULT_ANNOTATION_COLOR\);\n'
    r'\s*const \[annotationSize, setAnnotationSizeState] = useState\(DEFAULT_ANNOTATION_SIZES\[DEFAULT_ANNOTATION_TOOL\]\);\n'
    r'\s*const \[selectedAnnotationIndex, setSelectedAnnotationIndex] = useState<number \| null>\(null\);\n'
    r'\s*const \[editingTextDraft, setEditingTextDraft] = useState<EditingTextDraft>\(null\);\n'
    r'\s*const \[annotations, setAnnotations] = useState<Annotation\[\]>\(\[\]\);\n'
    r'\s*const \[annotationHistory, setAnnotationHistory] = useState<Annotation\[\]\[\]>\(\[\]\);\n'
    r'\s*const \[redoAnnotations, setRedoAnnotations] = useState<Annotation\[\]\[\]>\(\[\]\);\n'
    r'\s*const \[draftAnnotation, setDraftAnnotation] = useState<Annotation \| null>\(null\);\n'
)

text = state_block_pattern.sub('\n  ' + hook_instantiation, text)

# Refs to remove:
refs_pattern = re.compile(
    r'\s*const annotationToolRef = useRef<AnnotationTool>\(DEFAULT_ANNOTATION_TOOL\);\n'
    r'\s*const annotationColorRef = useRef\(DEFAULT_ANNOTATION_COLOR\);\n'
    r'\s*const annotationSizeRef = useRef\(DEFAULT_ANNOTATION_SIZES\[DEFAULT_ANNOTATION_TOOL\]\);\n'
    r'\s*const annotationSizesRef = useRef<Record<AnnotationTool, number>>\(\{ \.\.\.DEFAULT_ANNOTATION_SIZES \}\);\n'
    r'\s*const selectedAnnotationIndexRef = useRef<number \| null>\(null\);\n'
    r'\s*const annotationsRef = useRef<Annotation\[\]>\(\[\]\);\n'
    r'\s*const annotationHistoryRef = useRef<Annotation\[\]\[\]>\(\[\]\);\n'
    r'\s*const redoAnnotationsRef = useRef<Annotation\[\]\[\]>\(\[\]\);\n'
    r'\s*const draftAnnotationRef = useRef<Annotation \| null>\(null\);\n'
    r'\s*const editingTextDraftRef = useRef<EditingTextDraft>\(null\);\n'
)

text = refs_pattern.sub('\n', text)

# Sync refs block (if any existed, but it didn't in ScreenshotPage, it just did .current = ...)
sync_pattern = re.compile(
    r'\s*annotationSizesRef\.current = \{ \.\.\.DEFAULT_ANNOTATION_SIZES \};\n'
    r'\s*setAnnotationColor\(DEFAULT_ANNOTATION_COLOR\);\n'
    r'\s*setAnnotationSizeState\(DEFAULT_ANNOTATION_SIZES\[DEFAULT_ANNOTATION_TOOL\]\);\n'
    r'\s*annotationToolRef\.current = DEFAULT_ANNOTATION_TOOL;\n'
    r'\s*annotationColorRef\.current = DEFAULT_ANNOTATION_COLOR;\n'
    r'\s*annotationSizeRef\.current = DEFAULT_ANNOTATION_SIZES\[DEFAULT_ANNOTATION_TOOL\];\n'
)

# Replace all occurrences of these resets with just a blank since we are using resetAnnotations() where needed
text = sync_pattern.sub('', text)

# 4. Remove all extracted functions
funcs_to_remove = [
  'pushAnnotationHistory',
  'undoAnnotation',
  'redoAnnotation',
  'commitAnnotation',
  'setAnnotationDraft',
  'cancelTextDraft',
  'commitTextDraft',
  'deleteSelectedAnnotation',
  'applyAnnotations',
  'replaceAnnotations'
]

for func in funcs_to_remove:
    # use a robust parser for function blocks
    # We find "const funcName = (" or "const funcName = () =>" and remove until balanced braces
    pattern = re.compile(r'\s*const\s+' + func + r'\s*=\s*(\([^)]*\)|[a-zA-Z0-9_]+)\s*=>\s*\{')
    while True:
        match = pattern.search(text)
        if not match:
            break
        
        start = match.start()
        # Find closing brace
        brace_count = 0
        in_string = False
        string_char = ''
        end = -1
        for i in range(match.end() - 1, len(text)):
            c = text[i]
            if not in_string:
                if c in ('"', "'", '`'):
                    in_string = True
                    string_char = c
                elif c == '{':
                    brace_count += 1
                elif c == '}':
                    brace_count -= 1
                    if brace_count == 0:
                        end = i + 1
                        break
            else:
                if c == string_char and text[i-1] != '\\':
                    in_string = False
        
        if end != -1:
            # Check for trailing semicolon
            if end < len(text) and text[end] == ';':
                end += 1
            text = text[:start] + text[end:]
        else:
            break

# 5. Fix remaining resetScreenshotState references to these replaced refs/states
reset_pattern = re.compile(
    r'\s*setAnnotationToolState\(null\);\n'
    r'\s*setAnnotations\(\[\]\);\n'
    r'\s*setAnnotationHistory\(\[\]\);\n'
    r'\s*setRedoAnnotations\(\[\]\);\n'
    r'\s*selectedAnnotationIndexRef\.current = null;\n'
    r'\s*annotationsRef\.current = \[\];\n'
    r'\s*annotationHistoryRef\.current = \[\];\n'
    r'\s*redoAnnotationsRef\.current = \[\];\n'
    r'\s*draftAnnotationRef\.current = null;\n'
    r'\s*editingTextDraftRef\.current = null;\n'
    r'\s*setSelectedAnnotationIndex\(null\);'
)

text = reset_pattern.sub('\n    resetAnnotations();\n', text)

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'w', encoding='utf-8') as f:
    f.write(text)

print("Patched ScreenshotPage.tsx perfectly")
