# Local OCR Deployment Plan for AI Implementation

This document is intended for future AI agents and developers. The goal is to move OCR from the N100 server to the Windows client, while keeping the N100 server responsible for authentication, translation, caching, admin configuration, quota control, logs, and cloud fallback.

## 1. Background and Goal

Current flow:

```text
Windows client EXE
  ↓ uploads selected image region
https://ocr.yousn.me
  ↓
N100: FastAPI + PaddleOCR + translation + image redraw
```

Observed bottleneck:

```text
Translate Step: 0.03 ms
Cache Hits: 5
OCR Step: 1054.81 ms
```

`pidstat` shows that during OCR, the N100 Python process can reach 82% to 87% CPU. On this 4 CPU system, Linux 100% is approximately one fully used core. The current bottleneck is PaddleOCR being limited by the N100 single-core or low-thread performance.

Target flow:

```text
Windows client EXE
  ├─ captures selected screen region
  ├─ runs local OCR
  ├─ redraws translated image locally
  ↓ sends only OCR text, boxes, and token
https://ocr.yousn.me
  ├─ authenticates client
  ├─ translates text
  ├─ caches translations
  ├─ manages user config, quota, and logs
  └─ provides cloud OCR fallback
```

Primary goals:

```text
Keep ocr_max_side = 1280 or higher
Reduce N100 OCR latency
Keep centralized authentication and translation channel management
Keep the client lightweight while idle
Start OCR only when needed
```

## 2. Design Principles

```text
Keep screen capture lightweight
Do not load OCR models inside the main UI process
Start the OCR process on demand
Shut down OCR after an idle timeout
Do not use N100 OCR as the default path
Keep /api/translate as a backward-compatible fallback endpoint
Add /api/translate_text as the main endpoint after local OCR
```

Recommended runtime mode:

```text
Default: local OCR enabled
Failure path: fallback to cloud /api/translate
Debug mode: allow serverUrl override
Production mode: hide serverUrl and use https://ocr.yousn.me
```

## 3. Option Comparison

### 3.1 Option A: PaddleOCR-json as a Local Child Process

This is the recommended first implementation.

Benefits:

```text
Simple Windows integration
Can be called through stdin/stdout, HTTP, or command-line arguments
No embedded Python inside the Tauri main process
Can be bundled as an external runtime inside the client resources directory
```

This fits the current repository because the settings page already contains:

```text
useLocalOcr
fallbackToRemoteOcr
localOcrExecutablePath
localOcrTimeoutMs
```

Missing implementation pieces:

```text
Tauri/Rust command: run_local_ocr
ScreenshotPage.tsx local OCR flow
N100 endpoint: /api/translate_text
Client-side image redraw
```

### 3.2 Option B: RapidOCR with ONNX Runtime

Benefits:

```text
Can use CPU, DirectML, CUDA, or OpenVINO backends
Model size and startup time may be better than a full Python PaddleOCR runtime
Medium engineering complexity
```

This is suitable for a later optimized version, not the first implementation.

### 3.3 Option C: Keep OCR on N100 and Lower ocr_max_side

This is not the main recommendation because the target is to keep 1280 or even increase image quality for small text.

### 3.4 Option D: GPU OCR Server

This can provide the best performance but has the highest cost and deployment complexity. It is not the first step.

## 4. Recommended Architecture

```text
Tauri frontend: ScreenshotPage.tsx
  ↓ capture_region
Tauri Rust: lib.rs
  ↓ save selected image region to temp file
LocalOcrManager
  ↓ start or reuse PaddleOCR-json.exe
  ↓ return OCR blocks: text + box + confidence
ScreenshotPage.tsx
  ↓ POST /api/translate_text
N100 FastAPI: app.py
  ↓ translation + cache
ScreenshotPage.tsx
  ↓ locally redraw translated text onto canvas/PNG
```

## 5. New Data Structures

### 5.1 OCR Block

```ts
interface OcrBlock {
  text: string;
  confidence: number;
  box: [number, number][];
}
```

### 5.2 Translation Request

```ts
interface TranslateTextRequest {
  blocks: OcrBlock[];
  source_lang?: string;
  target_lang?: string;
  render_mode?: "client";
}
```

### 5.3 Translation Response

```ts
interface TranslateTextResponse {
  status: "success" | "failed";
  translations: string[];
  cache_hits: number;
  channel: string;
  error?: string;
}
```

## 6. Server Changes

### 6.1 Add Endpoint

File: `server/app.py`

Add:

```text
POST /api/translate_text
```

Request header:

```text
x-api-key: client token
```

Request body:

```json
{
  "blocks": [
    {
      "text": "Hello world",
      "confidence": 0.98,
      "box": [[0,0],[100,0],[100,30],[0,30]]
    }
  ],
  "source_lang": "auto",
  "target_lang": "zh"
}
```

Response:

```json
{
  "status": "success",
  "translations": ["你好，世界"],
  "cache_hits": 1,
  "channel": "google"
}
```

### 6.2 Server Responsibilities

```text
Validate token
Read active_channel
Call translator.translate_batch
Return translated text array
Do not process image bytes
Do not run OCR
Do not redraw image
```

### 6.3 Keep Existing Endpoints

Keep:

```text
POST /api/translate
POST /api/ocr
```

Purpose:

```text
Backward compatibility with old clients
Fallback when local OCR fails
N100 OCR debugging
```

## 7. Tauri/Rust Client Changes

File: `tauri-client/src-tauri/src/lib.rs`

### 7.1 Add Command

```rust
#[tauri::command]
async fn run_local_ocr(image_base64: String, executable_path: Option<String>, timeout_ms: Option<u64>) -> Result<serde_json::Value, String>
```

### 7.2 Execution Flow

```text
Decode base64
Write app_data_dir/local_ocr_input.png
Find OCR executable:
  1. config.localOcrExecutablePath
  2. bundled resources/ocr/PaddleOCR-json.exe
  3. PATH
Start OCR child process
Pass image path
Apply timeout
Parse stdout JSON
Normalize output into OcrBlock[]
Return error on failure
```

### 7.3 Process Management

Phase 1: start the OCR process for every OCR request.

Phase 2: implement persistent process management:

```text
Start OCR child process on first use
Reuse process for consecutive requests
Auto-exit after 5 minutes of idleness
Kill OCR child process when client exits
```

Implement Phase 1 first to validate accuracy and the full request flow. Then optimize with a persistent process.

## 8. Frontend Changes

File: `tauri-client/src/pages/ScreenshotPage.tsx`

### 8.1 New Main Flow

```text
handleTranslate()
  ↓
Read config.useLocalOcr
  ↓
If true:
  captureRegionBase64()
  invoke("run_local_ocr")
  fetch(`${serverUrl}/api/translate_text`)
  drawTranslatedBlocks()
  setTranslatedResult()
  ↓
If failed and fallbackToRemoteOcr=true:
  invoke("api_translate")
```

### 8.2 Legacy Compatibility

```text
useLocalOcr=false
  → keep using api_translate
```

### 8.3 Local Redraw Function

Add:

```ts
function renderTranslatedBlocks(base64Image: string, blocks: OcrBlock[], translations: string[]): Promise<string>
```

Responsibilities:

```text
Create canvas
Draw selected image
Use box coordinates to sample background color
Cover original text area
Draw translated text
Export PNG base64
```

The first version can reuse the server-side `image_processor.py` redraw strategy:

```text
Sample background color
Compute font size from box height
Auto-wrap text
Choose black or white text based on background brightness
```

## 9. Settings Page Changes

File: `tauri-client/src/pages/Settings.tsx`

Keep and actually activate:

```text
useLocalOcr
fallbackToRemoteOcr
localOcrExecutablePath
localOcrTimeoutMs
```

Recommended defaults:

```json
{
  "useLocalOcr": true,
  "fallbackToRemoteOcr": true,
  "localOcrTimeoutMs": 8000
}
```

Production recommendation:

```text
Hide serverUrl
Use fixed https://ocr.yousn.me
Allow only token input
Show serverUrl only in Advanced settings
```

## 10. Packaging and Deployment

### 10.1 Suggested Directory Layout

```text
tauri-client/
  src-tauri/
    resources/
      ocr/
        PaddleOCR-json.exe
        models/
          det/
          rec/
          cls/
```

### 10.2 Tauri Config

File: `tauri-client/src-tauri/tauri.conf.json`

Add resource bundling so the OCR runtime is included in the installer.

### 10.3 Client Runtime Path Resolution

Priority:

```text
User-defined localOcrExecutablePath
Bundled resource directory
PATH environment variable
```

## 11. Performance Targets

Current N100:

```text
OCR Step: 100~1200ms
Translate Step after cache hit: 0.01~0.03ms
```

Expected Windows local CPU:

```text
R5 7500F CPU OCR: 80~300ms
RTX 4060 GPU OCR: 20~100ms
```

Acceptable metrics:

```text
First OCR: < 1500ms
OCR after warmup: < 300ms
Total latency after translation cache hit: < 500ms
Automatic fallback to cloud OCR if local OCR fails
```

## 12. Memory Strategy

Current client:

```text
40~80MB
```

After local OCR:

```text
Non-persistent OCR: idle 40~80MB, temporary 300MB~1GB during OCR
Persistent OCR: idle 300MB~1GB
```

Recommended behavior:

```text
Do not keep OCR always loaded by default
Keep OCR alive for 5 minutes during consecutive usage
Auto-exit when idle
```

## 13. Security and Privacy

Benefits:

```text
Images are not uploaded to N100 by default
N100 receives only OCR text and coordinates
Better privacy
Lower bandwidth usage
```

Important constraints:

```text
Do not hardcode token in source code
Hide serverUrl in production mode
Do not log full sensitive OCR text
Do not allow arbitrary executable paths without user confirmation
```

## 14. Test Checklist

### 14.1 Functional Tests

```text
Local OCR enabled: screenshot translation succeeds
Local OCR disabled: cloud translation succeeds
Invalid local OCR exe path: automatic cloud fallback works
Invalid token: returns 401 and shows friendly error
Server unreachable: shows network error
```

### 14.2 Performance Tests

```text
Translate the same region 5 times consecutively
Record OCR latency
Record translation latency
Record total latency
Record client memory usage
Check whether the OCR child process exits after idle timeout
```

### 14.3 Accuracy Tests

```text
English web page small text
Software menu text
Chinese UI text
Mixed Chinese and English
High DPI display
Multi-monitor setup
Dark background
Light background
```

## 15. Implementation Milestones

### Milestone 1: Server Text Translation Endpoint

```text
Add /api/translate_text
Reuse translator.translate_batch
Return translations/cache_hits/channel
```

### Milestone 2: Local OCR Tauri Command

```text
Add Rust command run_local_ocr
Call external OCR executable
Return normalized OcrBlock[]
```

### Milestone 3: Screenshot Page Integration

```text
handleTranslate supports useLocalOcr
On local OCR success, call /api/translate_text
On failure, fallback to /api/translate
```

### Milestone 4: Local Redraw

```text
Redraw translated text in client canvas using OCR boxes
Generate translatedResult base64
Keep copy/save logic unchanged
```

### Milestone 5: Packaging

```text
Bundle OCR runtime into Tauri resources
Client installation requires no manual OCR executable setup
```

## 16. Rollback Plan

If local OCR is unstable:

```text
useLocalOcr=false
fallbackToRemoteOcr=true
Restore legacy /api/translate flow
```

Do not remove N100 OCR until the local OCR path is stable in production.