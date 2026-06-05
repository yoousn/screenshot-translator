import re

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'r', encoding='utf-8') as f:
    text = f.read()

# 1. Add import
if 'useScreenshotAnnotation' not in text:
    text = text.replace(
        'import type { Annotation, AnnotationTool, EditingTextDraft } from "../types/screenshot";',
        'import type { Annotation, AnnotationTool, EditingTextDraft } from "../types/screenshot";\nimport { useScreenshotAnnotation, DEFAULT_ANNOTATION_COLOR, DEFAULT_ANNOTATION_TOOL, DEFAULT_ANNOTATION_SIZES } from "../hooks/useScreenshotAnnotation";'
    )

# 2. Remove constant definitions
text = text.replace('const DEFAULT_ANNOTATION_COLOR = "#ff4d4f";\n', '')
text = text.replace('const DEFAULT_ANNOTATION_TOOL: AnnotationTool = "rect";\n', '')
text = text.replace('const DEFAULT_ANNOTATION_SIZES: Record<AnnotationTool, number> = { rect: 4, circle: 4, mosaic: 16, arrow: 4, text: 4, brush: 4 };\n', '')

# 3. Replace state block
state_block_start = 'const [annotationTool, setAnnotationToolState] = useState<AnnotationTool | null>(null);'
state_block_end = 'const [draftAnnotation, setDraftAnnotation] = useState<Annotation | null>(null);\n'

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

if state_block_start in text:
    # Use regex to replace the whole block of state declarations + ref declarations
    # State block
    pattern_state = r'const \[annotationTool, setAnnotationToolState].*?const \[draftAnnotation, setDraftAnnotation] = useState<Annotation \| null>\(null\);\n'
    text = re.sub(pattern_state, hook_instantiation, text, flags=re.DOTALL)
    
    # Ref block
    pattern_refs = r'\s*const annotationToolRef = useRef.*?\n.*?const draftAnnotationRef = useRef.*?\n'
    text = re.sub(pattern_refs, '\n', text, flags=re.DOTALL)

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
    pattern = r'\s*const ' + func + r'\s*=\s*(?:\([^)]*\)|[^\s=]+)\s*=>\s*\{.*?\};\n'
    text = re.sub(pattern, '\n', text, flags=re.DOTALL)

# 5. Fix remaining resetScreenshotState references to these replaced refs/states
# Wait, let's just make sure resetScreenshotState uses resetAnnotations().
reset_pattern = r'setAnnotationToolState\(null\);\n\s*annotationSizesRef\.current = \{ \.\.\.DEFAULT_ANNOTATION_SIZES \};\n\s*setAnnotations\(\[\]\);\n\s*setAnnotationHistory\(\[\]\);\n\s*setRedoAnnotations\(\[\]\);\n\s*setAnnotationColor\(DEFAULT_ANNOTATION_COLOR\);\n\s*setAnnotationSizeState\(DEFAULT_ANNOTATION_SIZES\[DEFAULT_ANNOTATION_TOOL\]\);\n\s*annotationToolRef\.current = DEFAULT_ANNOTATION_TOOL;\n\s*annotationColorRef\.current = DEFAULT_ANNOTATION_COLOR;\n\s*annotationSizeRef\.current = DEFAULT_ANNOTATION_SIZES\[DEFAULT_ANNOTATION_TOOL\];\n\s*selectedAnnotationIndexRef\.current = null;\n\s*annotationsRef\.current = \[\];\n\s*annotationHistoryRef\.current = \[\];\n\s*redoAnnotationsRef\.current = \[\];\n\s*draftAnnotationRef\.current = null;\n\s*editingTextDraftRef\.current = null;\n\s*setSelectedAnnotationIndex\(null\);'

text = re.sub(reset_pattern, 'resetAnnotations();', text, flags=re.DOTALL)

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'w', encoding='utf-8') as f:
    f.write(text)

print("Patched ScreenshotPage.tsx")
