# Screenshot Selection Smoothness Log

## Goal

- Make Alt+A screenshot selection feel smooth and responsive.
- Use standard screenshot-tool pointer behavior: capture pointer on left down, update only lightweight selection state during drag, release/cancel cleanly on up/cancel/lost capture.
- Record every repair round, build result, and drag test result here.

## External Research Notes

- Pointer Events pattern: call `setPointerCapture(pointerId)` on primary button down so the same element continues receiving move/up events during drag, then call `releasePointerCapture` on pointer up/cancel. `pointermove.buttons` is the authoritative browser-side signal for whether the primary button is still physically held.
- Win32 pattern: use mouse capture during native drag (`SetCapture`), release on button up (`ReleaseCapture`), and treat capture loss/cancel/focus loss as terminal cleanup so a drag cannot remain stuck.
- Product behavior target: during drag, the hot path should not run expensive detection refreshes, React state churn, dependency checks, OCR/translation work, or layout-heavy toolbar measurement. Canvas selection rendering should be batched to animation frames.

## Round 1 - Remove Magnifier Work From Active Drag Hot Path

### Diagnosis

- `ScreenshotPage` called `updateMagnifier(e.clientX, e.clientY)` on every `pointermove`, even while the user was actively dragging a selection.
- `updateMagnifier` reads canvas pixels, updates React state with `setMagnifier`, and redraws the magnifier canvas in an effect.
- Active drag already renders the selection via canvas. Running magnifier React updates in the same high-frequency pointer path can cause visible hitching.

### Change

- Skip magnifier updates while primary left button is down or while selection/drag/resize state is active.
- Keep magnifier behavior for hover/idle color picking.

### Validation

- `npx tsc --noEmit`: passed.
- `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`: passed.
- `npm run build`: passed with existing Vite chunk/import warnings only.
- Automated 8-round launch smoke using `YsnTrans.exe` produced stable screenshot startup logs:
  - `capture_end`: 53-77 ms.
  - frontend `image_ready`: 11-15 ms after direct buffer receipt.
  - frontend `first_paint`: 16-28 ms after frontend session start.
- Test weakness: the first automation harness did not produce `first_pointer_down` / `selection_drag_finished` logs, so it proved startup stability but not actual WebView drag smoothness.

## Round 2 - Keep Drag Start And Move Path Lightweight

### Diagnosis

- `handleMouseDown` called `focusScreenshotWindow()`, which asynchronously invokes a Tauri command and `setFocus()` even when the screenshot overlay is already interactive.
- Hidden `mouseTrackerRef` DOM was still updated on every pointer move.
- The previous automation harness needed stronger proof that mouse events reached the WebView drag path.

### Change

- Active left-button selection start now focuses only the canvas (`focusScreenshotCanvas`) before pointer capture.
- Hidden mouse tracker writes are skipped.
- Added one drag-end baseline line (`selection_drag_finished`) with move count, draw request count, duration, max move gap, and final rect so repeated automated tests can prove real pointer/selection path behavior without logging every frame.

### Validation

- `npx tsc --noEmit`: passed.
- `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`: passed.
- `npx tauri build --no-bundle`: passed with existing Vite chunk/import warnings only.

## Round 3 - Add Isolated Smoothness Smoke And Verify 12 Drags

### Diagnosis

- Windows-level `SetCursorPos` / `mouse_event` and `SendInput` could trigger Alt+A and show the overlay, but did not reliably enter the WebView/React pointer event stream in this desktop automation environment.
- Without a deterministic pointer-path test, startup timings alone could not prove drag smoothness.
- The first internal smoke attempt mixed in real pointer events, so the smoke path needed isolation from physical mouse noise.

### Change

- Added an env-gated screenshot selection smoke: `VITE_YSN_SCREENSHOT_SELECTION_SMOOTHNESS_SMOKE=1`.
- The smoke runs only after the screenshot overlay is ready, drives the same `useScreenshotInteraction` handlers through 12 synthetic drag rounds, and closes the overlay after completion.
- During the smoke build only, canvas pointer events are disabled so real mouse movement cannot corrupt the measurement.
- Added drag-end metrics for selection gestures: valid flag, move count, draw request count, duration, max move gap, and final rect.

### Validation

- Debug smoke build used `VITE_YSN_SCREENSHOT_SELECTION_SMOOTHNESS_SMOKE=1` and `VITE_YSN_DEBUG_LOGS=1`.
- Isolated 12-round result:
  - smoke rounds: 12.
  - `selection_drag_finished`: 12.
  - invalid selections: 0.
  - lost pointer finalization: 0.
  - moves per drag: 48 / 48.
  - draw requests per drag: 48 / 48.
  - max move gap: 14 ms.
  - average max move gap: 9.75 ms.
  - average drag duration: 326.67 ms.
  - final rects matched expected 560 x 320 selections.
- Startup in the same run remained fast:
  - `capture_end`: 46 ms.
  - frontend `image_ready`: 12 ms.
  - frontend `first_paint`: 28 ms.
- Rebuilt normal release without smoke/debug env:
  - `npx tauri build --no-bundle`: passed with existing Vite chunk/import warnings only.
  - `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`: passed.

### Current Status

- The screenshot selection hot path is now lighter:
  - no magnifier pixel reads or React magnifier updates during active drag.
  - no async Tauri overlay activation on normal left-button drag start.
  - no hidden mouse tracker DOM writes while hidden.
  - drag release/cancel metrics are available when debug logs are enabled.
- Automated smoothness smoke passes 12/12 clean rounds with no invalid selections and no lost pointer finalization.
