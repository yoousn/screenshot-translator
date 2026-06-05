import re

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'r', encoding='utf-8') as f:
    text = f.read()

# Hook missing?
if 'useScreenshotOcr({' not in text:
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
    
    # We will move getTextSourceBlocksForCurrentSelection to the top of the component!
    pattern = re.compile(r'\s*const getTextSourceBlocksForCurrentSelection = async \(timeoutMs = 80\) => \{')
    match = pattern.search(text)
    if match:
        start = match.start()
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
            if end < len(text) and text[end] == ';':
                end += 1
            func_text = text[start:end]
            text = text[:start] + text[end:]
            
            # Now insert func_text and hook_instantiation at the top of ScreenshotPage
            insert_pos = text.find('const canvasRef = useRef<HTMLCanvasElement>(null);')
            if insert_pos != -1:
                text = text[:insert_pos] + func_text + '\n' + hook_instantiation + '\n  ' + text[insert_pos:]
            else:
                print("Could not find canvasRef")
    else:
        print("Could not find getTextSourceBlocksForCurrentSelection")

with open('tauri-client/src/pages/ScreenshotPage.tsx', 'w', encoding='utf-8') as f:
    f.write(text)

print("Applied final fixes v7")
