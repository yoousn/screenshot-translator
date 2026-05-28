# Local OCR Deployment Design (Tauri v2 & PaddleOCR-json)

This design document specifies the architecture, data structures, and lifecycle management for running local OCR on the Windows client while relying on the N100 server for text translation, token verification, and fallback Remote OCR.

---

## 1. Goal & Requirements

### 1.1 Objectives
* **Lower Server CPU Load**: Move high-CPU image processing and PaddleOCR inference from the N100 server to the Windows client.
* **Decrease Latency**: Achieve OCR speeds under 200ms on client PCs after warm-up (compared to 1s+ on N100 CPU).
* **Privacy & Bandwidth Optimization**: Send only extracted text blocks instead of raw images to the N100 server during standard operation.
* **Lightweight Footprint**: Ensure the PaddleOCR-json child process runs only on-demand and auto-terminates after a 5-minute idle timeout.
* **Seamless Fallback**: If local OCR fails or the executable is missing, seamlessly fall back to uploading the cropped image to N100's `/api/translate`.

---

## 2. Architecture & Data Flow

The architecture decouples image extraction, character recognition (OCR), text translation, and visual image rebuilding:

```text
+-------------------+           1. Capture Base64          +-------------------------+
|  ScreenshotPage   | -----------------------------------> |       Tauri Rust        |
|      (React)      | <----------------------------------- |     (run_local_ocr)     |
+-------------------+           4. OcrBlock[]              +-------------------------+
       |     ▲                                                          │
       │     │ 3. Text Array                                            │ 2. Writes ocr-<uuid>.png,
       │     │                                                          │    pipes path to stdin
       ▼     │                                                          ▼
+-------------------+                                      +-------------------------+
|    N100 Server    |                                      |   PaddleOCR-json.exe    |
| /api/translate_text|                                     |    (Resident Process)   |
+-------------------+                                      +-------------------------+
```

### 2.1 Component Responsibilities

1. **Tauri Rust Backend**:
   * Registers `run_local_ocr` command.
   * Manages `LocalOcrManager` singleton, wrapping the child process.
   * Performs base64 decoding and writes temporary cropped images with unique random names (`ocr-<uuid>.png`) to protect against concurrency race conditions.
   * Operates `PaddleOCR-json.exe` via stdin/stdout piping.
   * Runs an idle timer: kills the child process if no requests arrive within 5 minutes.
   * Kills child process forcefully on application exit.
2. **N100 FastAPI Server**:
   * Exposes new endpoint `POST /api/translate_text` to handle batch translation of extracted strings.
   * Validates client token, queries translation cache, contacts translators, and logs usage.
   * Maintains existing `/api/translate` and `/api/ocr` endpoints for old clients and fallback pathways.
3. **Tauri Frontend (ScreenshotPage & Canvas)**:
   * Handles local OCR triggering and coordinates server-side text translation.
   * Performs canvas-based background erasure and translated text overlay.
   * Triggers silent fallback to raw image upload `/api/translate` if local OCR encounters errors.

---

## 3. Data Structures

### 3.1 OCR Block
The standard representation returned by the local OCR command to the frontend.
PaddleOCR-json returns `score` field to indicate recognition confidence, which Rust backend explicitly maps to `confidence` in this structure:

```typescript
interface OcrBlock {
  text: string;
  confidence: number; // Mapped explicitly from the raw 'score' field returned by PaddleOCR-json
  box: [number, number][]; // 4 points: [[x1, y1], [x2, y2], [x3, y3], [x4, y4]]
}
```

### 3.2 Translate Text Request (POST /api/translate_text)

```json
{
  "blocks": [
    {
      "text": "Hello World",
      "confidence": 0.98,
      "box": [[10, 10], [100, 10], [100, 30], [10, 30]]
    }
  ],
  "source_lang": "auto",
  "target_lang": "zh"
}
```

### 3.3 Translate Text Response

```json
{
  "status": "success",
  "translations": ["你好，世界"],
  "cache_hits": 1,
  "channel": "google"
}
```

---

## 4. Resident OCR Process Management (Rust)

A dedicated structure, `LocalOcrManager`, will handle the state of the active subprocess:

```rust
pub struct LocalOcrState {
    child: Option<std::process::Child>,
    stdin: Option<std::process::ChildStdin>,
    reader: Option<std::io::BufReader<std::process::ChildStdout>>,
    last_used: std::time::Instant,
}
```

### 4.1 Process Communication Protocol
* **Spawn Parameters**:
  * Launched via `Command::new()`.
  * Set `cwd` strictly to the directory containing `PaddleOCR-json.exe` (this is critical to allow the binary to find its dependent DLLs).
  * Do NOT pass `-port` or `image_path` to use standard piping interaction mode.
  * Pass optional arguments such as `["-models_path=..."]` only if required.
  * Stdin & Stdout must be set to `Stdio::piped()`. Stderr should be set to `Stdio::null()` or `Stdio::inherit()`.
* **Initialization Synchronization**:
  * Upon launching the process, the manager must actively read the stdout stream until a line containing `OCR init completed.` is successfully received. No requests should be piped into stdin prior to receiving this signal.
* **Request Format**:
  Write to stdin: `{"image_path": "C:\\path\\to\\ocr-<uuid>.png"}\n`
* **Response Format**:
  Read a single line from stdout containing the JSON result. It will be parsed and mapped into the unified `OcrBlock[]` response structure.

### 4.2 Idle Timeout & Garbage Collection
A background tokio task polls the manager state:
* Every 30 seconds, it checks if `Instant::now() - last_used` exceeds 5 minutes.
* If yes, and `child` is `Some`, it sends a termination signal, kills the process, and drops the streams to free resources (approximately 300MB-1GB RAM/VRAM).

---

## 5. Front-End Canvas Redraw Strategy

When `useLocalOcr` is enabled and translation text is received, the frontend uses an HTML5 Canvas to replace the original text:

### 5.1 Step-by-Step Drawing Flow
1. **Background Cleansing**:
   * For each block box, read pixels at the four corners of the box in the original cropped image.
   * Average the RGB values of the corner pixels to determine the background fill color.
   * Apply `ctx.fillStyle = avgBgColor` and `ctx.fillRect()` to paint over the original text region.
2. **Text Contrast Matching**:
   * Calculate background relative luminance:
     $$Y = 0.299 \times R + 0.587 \times G + 0.114 \times B$$
   * Set text drawing fillStyle:
     * If $Y > 128$ (Light background), draw text using **Pure Black** (`#000000`).
     * If $Y \le 128$ (Dark background), draw text using **Pure White** (`#FFFFFF`).
3. **Typography & Layout**:
   * Font Family: `'Microsoft YaHei', -apple-system, sans-serif`.
   * Font Size: Automatically derived from the bounding box height (with a scaling threshold to prevent oversized or microscopic text).
   * Text Wrapping: Measure text metrics and auto-wrap sentences to fit the original bounding box width.

---

## 6. Packaging & Path Resolution (Tauri v2)

The OCR executables must be distributed with the client installer.

### 6.1 Directory Structure
We bundle binaries in `src-tauri/resources/ocr/`:

```text
tauri-client/src-tauri/resources/ocr/
  ├── PaddleOCR-json.exe
  └── models/
        ├── det/
        ├── rec/
        └── cls/
```

### 6.2 Tauri Configuration (`tauri.conf.json`)
Include the directory in the package bundle:

```json
{
  "bundle": {
    "active": true,
    "resources": [
      "resources/ocr/**/*"
    ]
  }
}
```

### 6.3 Executable Path Resolution Priority
In `run_local_ocr` command:
1. **User Custom Configuration**: Use `config.localOcrExecutablePath` if specified.
2. **Bundled Directory (Tauri v2 Standard API)**: Resolve using the official Tauri resource path resolution API:
   ```rust
   let resource_path = app.path().resolve("resources/ocr/PaddleOCR-json.exe", BaseDirectory::Resource)?;
   ```
3. **Environment fallback**: Attempt to locate `PaddleOCR-json.exe` on system `%PATH%`.

---

## 7. Security and Validation Measures

* **Input Sanitization**: Ensure temporary file paths passed to PaddleOCR-json are properly escaped and valid Windows absolute paths.
* **Process Separation**: Do not execute arbitrary terminal commands. Limit executions strictly to the verified PaddleOCR executable binary.
* **Silent Fallback**: In `ScreenshotPage.tsx`, wrap the local workflow in a try-catch blocks. If any exception throws, execute `api_translate` (sending the image raw bytes) to ensure zero friction for users.
