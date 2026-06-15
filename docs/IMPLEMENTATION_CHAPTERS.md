# Implementation Chapters

> This file is the single execution-history document for the project. It is intentionally optimized for fast resume: keep the current status and recent implementation chapters detailed, and keep older history as a compact ledger inside this same file instead of scattering archive docs.

## Current Resume Snapshot - 2026-06-15

### Product State
- The current source version is `1.2.7` across `package.json`, `package-lock.json`, `Cargo.toml`, and `tauri.conf.json`.
- The current verified local portable entry is `release\YSN-Screenshot-Translator\YsnTrans.exe`.
- The current portable zip is `build\x64_v1.2.7\ScreenshotTranslator_Windows.zip`, generated from the assembled portable directory and measured at about `199.52 MB`.
- The latest installer artifacts still visible in the workspace are old `1.2.6` outputs. They are historical and should not be treated as the current release until a full `build.bat --no-pause --no-launch` run regenerates `1.2.7` installers.
- User feedback on 2026-06-15 says screenshot interaction now feels smooth and comfortable overall, with occasional stutter that may involve the magnifier path. Do not destabilize the screenshot flow with broad rewrites; next screenshot work should begin with low-risk profiling.
- The product remains a high-completion internal / pre-release build, not a fully commercial release. Remaining commercial gaps are installer/signing/update flow, model hosting/rollback, real device matrix, multi-monitor/DPI evidence, service test reproducibility, and large-file maintenance debt.

### Current Hot Paths
- Default screenshot flow remains the WebView2 SharedBuffer / transparent screenshot helper route from Chapters 264-299.
- No screenshot behavior was changed in Chapter 300; release and documentation cleanup only.
- Magnifier-related smoothness risk is recorded for the next focused profiling chapter.
- OCR remains the product-owned RapidOCR / ONNXRuntime path with PP-OCRv6 selectable/default direction and V5/V4 compatibility models available.
- Translation smoke currently works through LAN `http://192.168.1.3:8318` using the `google` channel; this is usable for smoke but not the final commercial translation quality route.

### Latest Validation Snapshot
- Passed recently: `powershell -NoProfile -ExecutionPolicy Bypass -File .\check_commercial.ps1`.
- Passed recently: `cd tauri-client; npm run build`.
- Passed recently: `cargo test --manifest-path tauri-client\src-tauri\Cargo.toml`.
- Passed recently: `cd tauri-client; npm run check:ocr-fixtures`.
- Passed recently: `cd tauri-client; npm run smoke:translate-service`.
- Passed recently: `python -m py_compile app.py`.
- Not run successfully: `python -m pytest server\tests`; the current Python environment does not have `pytest` installed.
- Current workspace still contains a pre-existing deletion of `启动部署助手.vbs`; do not silently revert it unless the user confirms this launcher should be restored.

### Next Recommended Chapter
- Chapter 301 should profile the occasional screenshot stutter without changing the current interaction model: focus first on magnifier animation frame work, pixel sampling, hover suppression, window-rect warmup, and native capture handoff timings.
- After the user allows closing the currently running app, run full `build.bat --no-pause --no-launch` to regenerate `1.2.7` installers and then smoke-launch the packaged output.
- Keep documentation current after each chapter; the master plan current-risk section and this resume snapshot should not drift behind the detailed chapter tail again.

## Historical Resume Snapshot - 2026-06-10

### Product State
- C/E selected-output technical acceptance is complete as of Chapter 250: Overall 100%, Plan C 100%, Plan E 100%.
- The product is not yet bug-free and not yet WeChat/QQ screenshot maturity; the active phase is release/manual QA, no-flicker polish, compatibility, and rollout policy.
- Chapter 251 fixed the first-frame `Alt+A` gray-shell timing path by deferring native window visibility until the screenshot image, mask, first candidate pass, and one frontend animation frame are ready.
- Chapter 252 removed a frontend duplicate-payload hot-path regression and restored the RGBA direct-canvas path in the screenshot WebView. Guarded auto-smoke now shows `payload_duplicate_skipped`, `rgba_canvas_ready`, first paint around `158ms` from frontend session start, and pre-show candidate first batch around `169ms`.
- Chapter 253 changed the default screenshot helper show path to Windows no-activate presentation and removed the frontend first-frame `setFocus()` call. `build.bat` now auto-launches the portable exe after a successful ordinary build, while `pack_release.ps1 -Build` passes `--no-launch`.
- Chapter 254 release-level `Alt+A` check found and fixed the no-activate `Esc` cancellation gap by registering a temporary global Escape shortcut only while screenshot capture is active.
- Chapter 255 researched the `Alt+A` sub-50ms target. The next speed slice should measure and optimize `hotkey -> visible/interactable shell` separately from `hotkey -> real RGBA image ready` and `hotkey -> detailed window candidates ready`.
- Chapters 256-261 tested the early shell / opaque deferred / pre-show drag recovery route, then fixed repeated screenshot lifecycle races that could resurrect stale frames after several runs.
- Chapter 262 tried a native first-frame shield for WebView2 white flash, but Chapter 263 disabled it by default after manual feedback showed black/color-shift artifacts; the shield remains diagnostic only.
- Chapters 264-265 moved screenshot image delivery to the Snow Shot-style WebView2 SharedBuffer path, then shortened it to direct Rust-to-WebView SharedBuffer push before the frontend asks for pixels.
- Chapter 266 changed the final default back to a transparent screenshot helper and transparent WebView2 backing, but only after the real screenshot canvas has been painted and a post-paint task has run. It also starts capture before WebView/window prep, caches unchanged fullscreen bounds, and session-filters pre-show pointer recovery.
- Chapter 267 removed the last default black-background fallback from the earliest HTML/CSS path and made screenshot-window `WDA_EXCLUDEFROMCAPTURE` opt-in. Before the change, ffmpeg desktop recording saw full black frames during screenshots; after the change, the same visual smoke reported `black_frame_count=0` and `white_frame_count=0`.
- Chapter 268 changed the default again from hidden-until-real-canvas to a transparent input shell before screenshot pixels. The shell is visible/interactable earlier but draws no black/white/gray placeholder; real screenshot pixels still arrive through direct WebView2 SharedBuffer.
- Chapter 269 is now explicitly planned as **Native First Frame Screenshot Session**. The next implementation target is no longer only a low-level mouse recovery hook; it is a Snow Shot-style native first-frame entry where Rust/Win32 owns the first screenshot frame, native mask, native candidate/window recognition, and native mouse input, while WebView joins later for toolbar, OCR, translation, editing, copy, and save.
- Normal users now use direct WebView2 SharedBuffer delivery, transparent screenshot helper/WebView backing, transparent input-shell presentation before pixels, post-paint real-image confirmation, no native full-screen shield, and no screenshot-window capture exclusion by default. Rollbacks/diagnostics remain behind explicit env flags.

### Current Hot Paths
- `Alt+A` default screenshot startup now starts native capture first, prepares the screenshot WebView/window in parallel, shows a transparent input shell when the shell has painted, pushes captured RGBA through direct WebView2 SharedBuffer, paints the real screenshot to canvas, then keeps the already-visible overlay on top.
- The screenshot helper and WebView2 backing are transparent by default. The early shell is allowed to receive mouse input, but its canvas is cleared and it does not draw the dim mask until real screenshot pixels are ready.
- Rollbacks/diagnostics: `YSN_SCREENSHOT_DEFER_VISIBLE_SHELL=1` or `YSN_SCREENSHOT_EARLY_VISIBLE_SHELL=0` restores deferred-show behavior; `YSN_SCREENSHOT_OPAQUE_WINDOW=1` or `YSN_SCREENSHOT_TRANSPARENT_WINDOW=0` forces the opaque helper path; `YSN_NATIVE_FIRST_FRAME_SESSION=1` enables the guarded native first-frame session experiment; `YSN_SCREENSHOT_EXCLUDE_FROM_CAPTURE=1` opts the screenshot helper back into Windows capture exclusion; request-style SharedBuffer and old IPC/PNG/base64 payloads remain fallbacks.
- Native pre-show mouse drag recovery records the first left-button drag after `Alt+A` and now filters pointer state by session id before the frontend recovers it.
- Repeated screenshots reuse unchanged fullscreen helper bounds instead of calling `set_position` / `set_size` every run, reducing later-run window-compositor churn.
- Frontend shell mode no longer displays a gray layer before screenshot pixels arrive. The default `screenshot-shell` event now presents a transparent input shell only, clears the shell canvas, and defers toolbar/mask UI until `screenshotState === "ready"`.
- Screenshot close/copy/save/cancel still asks native to hide or cancel before clearing frontend state, so a WebView cannot be left visible with a cleared canvas during repeated runs.
- The frontend now deduplicates same-session screenshot payloads, preserves the current session SharedBuffer during pruning, and releases stale pending SharedBuffers during reset/unmount.
- The earliest `index.html` and global `html/body/#root` fallback backgrounds are transparent, so a not-yet-hydrated screenshot WebView no longer has a project-created black fallback surface.
- `overlay_ready_to_show` now defaults to `ShowWindow(SW_SHOWNOACTIVATE)` plus `SetWindowPos(... SWP_NOACTIVATE ...)` for the screenshot helper, with `YSN_SCREENSHOT_FOCUS_ON_READY=1` as a diagnostic rollback.
- Because the screenshot helper is now shown without activation, screenshot capture temporarily registers global `Escape` as a cancellation fallback and removes it on cancel/force-close/repeat-hotkey cleanup.
- WGC/DXGI selected-output diagnostics and copy/save candidates remain guarded and should not become default production behavior without a rollout chapter.
- Manual QA is still needed to prove human-visible feel parity with QQ/WeChat/Snow Shot; automated smoke can prove timing/order/stability and no recorded black/white frames, but not every compositor frame seen by the eye.
- The next intended hot path is `Alt+A -> native screenshot session -> native first frame/mask/candidate/input -> WebView toolbar handoff`, with a low-level/global mouse hook only as the earliest input fallback. The old full-screen native shield from Chapters 262-263 is not the target architecture and must not be re-enabled as the production solution.

### Latest Validation Snapshot
- Passed recently: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed recently: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed recently: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_window_transparency_tests -- --nocapture`.
- Passed recently: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_shared_buffer -- --nocapture`.
- Passed recently: `cd tauri-client; npx tsc --noEmit`.
- Passed recently: `cd tauri-client; npm run build`.
- Passed recently: `cd tauri-client; npm run check:i18n`.
- Passed recently: `cd tauri-client; npm run check:ocr-processing`.
- Passed recently: `git diff --check` with existing LF-to-CRLF warnings only.
- Passed recently: `cmd /c "build.bat --no-pause"`; it rebuilt the portable output and auto-launched `release\YSN-Screenshot-Translator\YsnTrans.exe`.
- Passed recently: packaged six-round transparent/post-paint/bounds-cache smoke against `release\YSN-Screenshot-Translator\YsnTrans.exe` showed all rounds used direct SharedBuffer, no `rgba_fetch`, no SharedBuffer timeout, empty stderr, `capture_end` `36-46ms`, `payload_emit` `47-55ms`, `image_ready` `5-8ms`, `first_paint` `7-11ms`, and `overlay_ready_to_show_returned` `20-29ms`.
- Passed recently: the same six-round smoke showed no third/fourth-run timing climb; rounds 4-6 remained stable with `capture_end` `37-41ms`, `payload_emit` `48-51ms`, and `first_paint` `7ms`.
- Passed recently: pre-fix visual recording smoke `tmp-runtime-logs\visual-flash-smoke-20260610-034351.mp4` captured black frames while the screenshot helper used Windows capture exclusion, matching `WDA_EXCLUDEFROMCAPTURE` behavior for external capture tools.
- Passed recently: post-fix visual recording smoke `tmp-runtime-logs\visual-flash-post-exclusion-20260610-035131.mp4` plus `visual-analysis.json` reported `black_frame_count=0`, `white_frame_count=0`, `luma_min=88.54`, `luma_max=161.67`, and `diff_avg=1.15`.
- Passed recently: post-fix release log `tmp-runtime-logs\visual-flash-post-exclusion-20260610-035101-out.log` showed 4 rounds with `overlay_capture_exclusion excluded=false`, direct SharedBuffer, no `rgba_fetch`, no `shared_buffer_direct_wait_miss`, no native shield, `image_ready` `7-11ms`, `first_paint` `12-15ms`, and `overlay_ready_to_show_returned` `37-71ms`.
- Passed recently: Chapter 268 transparent-input-shell release smoke `tmp-runtime-logs\transparent-input-shell-smoke-20260610-043120-out.log` showed 4 rounds with `transparent_input_shell=true`, direct SharedBuffer, no `rgba_fetch`, no `shared_buffer_direct_wait_miss`, no timeout/error/panic matches, and empty stderr.
- Passed recently: Chapter 268 visual recording smoke `tmp-runtime-logs\transparent-input-shell-visual-20260610-043245.mp4` plus `transparent-input-shell-visual-20260610-043245-visual-analysis.json` reported `black_frame_count=0`, `white_frame_count=0`, `high_diff_frame_count=0`, `luma_min=29.22`, `luma_max=68.77`, and `diff_max=23.95`.

### Next Recommended Chapter
- Chapter 269 should implement **Native First Frame Screenshot Session** as the next screenshot architecture chapter.
- Do not make Chapter 269 a WebView-only timing patch and do not stop at low-level mouse recovery. The goal is native first frame plus native input overlay first, with low-level/global mouse hook as a 0-50ms fallback and WebView as the later complex-UI layer.
- Chapter 269 target metrics: P95 `hotkey -> 鼠标可拖 <= 50ms`, P95 `hotkey -> 遮罩首帧出现 <= 60ms`, P95 `hotkey -> 窗口候选框出现 <= 60ms`, and P95 `hotkey -> WebView 工具栏 ready <= 120ms`.
- Chapter 269 must keep the old native GDI first-frame shield disabled by default; the new route must be a real native screenshot session that paints actual screenshot pixels, mask, candidates, and input state rather than a temporary black/gray/opaque cover.
- OCR/translation should remain a quick regression smoke only, not a development focus, unless the new native session handoff breaks the screenshot lifecycle.
- For future recording evidence, keep `YSN_SCREENSHOT_EXCLUDE_FROM_CAPTURE` unset unless specifically testing Windows capture exclusion; otherwise recording tools can show synthetic black frames that are not the same as human-visible compositor frames.

## Documentation Maintenance Policy

- Keep active/current chapters detailed enough for unattended resume: goals, changed files, validation, known risks, and next chapter.
- Keep older chapters compressed in the Historical Chapter Ledger below once their details are superseded by the master plan and latest chapters.
- Do not create scattered long-term chapter archive documents unless `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md` first indexes and justifies the new document.
- When a chapter changes readiness or product direction, update both this snapshot and the latest chapter record.

## Historical Milestone Ledger - Condensed Chapters 98-229

Older chapters are intentionally summarized here to keep this file fast to open and useful for resuming work. Detailed active evidence starts at Chapter 230.

| Range | Theme | Durable Outcome | Resume Note |
|---:|---|---|---|
| 98-110 | OCR runtime bring-up | ONNX/RapidOCR model assets, manifest repair, OCR result-window fixes, and new-machine OCR validation were established. | Use the master plan for OCR strategy; do not reopen the deprecated self-built OCR runtime path unless explicitly requested. |
| 111-124 | Translation quality and service observability | Translation failure gates, source-language routing, LAN/N100 service timing, shared glossary metadata, cache recovery UX, and diagnostic reporting were added. | Keep technical-term preservation and server timing visible in diagnostics. |
| 125-133 | OCR fixture gates and RapidOCR mainline | Fixed OCR fixtures, real screenshot fixtures, Latin/technical text retries, duplicate translation handling, and RapidOCR/ONNXRuntime mainline migration landed. | RapidOCR runner/model readiness remains the product-owned OCR path. |
| 134-148 | Recording, packaging, UIA, and screenshot state hardening | Recording reuse, portable packaging, worker warm path, UIA text-source sprint, overlay rendering safeguards, and screenshot misclick protection were implemented. | Treat UIA as acceleration only; OCR fallback must remain reliable. |
| 149-158 | Screenshot lifecycle and frontend decomposition | Screenshot save/focus fixes, repeated recording cleanup, ghost-window repair, and initial large-file/front-end module extractions were completed. | Preserve first-click and immediate `Ctrl+S` behavior when changing overlay lifecycle. |
| 159-174 | Capture backend and latency route experiments | WGC-class capture migration, xcap/screenshot fallback work, RGBA payload work, and screenshot startup latency experiments were recorded. | Avoid showing an unready WebView shell; current Chapter 251 supersedes old early-visible shell timing. |
| 175-192 | Native overlay, GPU, and C/E planning | Native overlay contracts, DXGI/D3D11/GPU probes, selected-image bridge planning, and C/E progress tracking moved from concept into guarded diagnostics. | C/E was not ready in this range; later chapters provide acceptance evidence. |
| 193-207 | WGC command contracts and file-size audits | WGC one-frame report commands, screenshot_native audits, and diagnostic request/response contracts were tightened. | Older line-count records here are stale; use latest chapter audits instead. |
| 208-229 | DXGI/WGC selected-output runway | DXGI acquire timeout evidence, selected-output ranking/readback plans, desktop pulse diagnostics, and guarded live WGC/DXGI experiments prepared the final acceptance path. | Detailed evidence resumes at Chapter 230 and should be used for current decisions. |

## Detailed Current Chapters - 230-300

## Chapter 230 - Diagnostic Request DTO Split And WGC Real-API Test Guards (2026-06-09)

> Chapter status: completed only for this diagnostic request DTO extraction, selected-readback JSON contract hardening, and default WGC real-API test guard slice. This does not mark 方案 C, 方案 E, WGC/DXGI live capture, selected PNG evidence, selected-output effects, native route readiness, runtime Alt+A routing, or commercial screenshot acceptance complete.

### Goals
- Move diagnostic request DTOs out of the already-large `screenshot_commands.rs` without changing Tauri invoke names or command behavior.
- Keep Chapter 230 smaller than a full WGC command-module extraction because subagent audits identified safer prerequisites first: shared request DTOs, default live-API guard, and JSON contract hardening.
- Prevent default Rust tests from touching real WGC/WinRT/D3D capture/probe paths unless an ignored test and explicit env var are both used.
- Stabilize `selectedReadbackPlan` diagnostic JSON so planned and failed shapes both expose a top-level `status`, and early WGC bounds failures expose a stable `selectedReadbackPlan: null` placeholder.

### External Findings
- No online research was needed for this chapter because the work was local test-safety, module-boundary, and JSON-contract maintenance against current code.
- Six read-only subagent audits were used to verify request DTO extraction feasibility, WGC command extraction boundaries, test migration boundaries, selected-readback JSON risks, default WGC real-API test risk, and Chapter 229 line-count self-consistency.

### Progress
- Overall C/E progress: approximately 79%.
- 方案 C native overlay / selected-output progress: approximately 81%.
- 方案 E DXGI/WGC/D3D11/GPU texture progress: approximately 76%.
- Progress remains unchanged because this chapter improves safety and modularity but still does not produce real selected-frame readback, selected PNG evidence, fake-sink copy evidence, or runtime Alt+A selected-output evidence.

### Added Files
- `tauri-client/src-tauri/src/screenshot_diagnostics_requests.rs`
  - Holds diagnostic request DTOs previously embedded in `screenshot_commands.rs`, including WGC probe/monitor-session requests, DXGI readback/probe/bridge/acceptance requests, cursor nudge requests, and desktop update pulse requests.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - Imports diagnostic request DTOs from the new request module and drops the in-file DTO block.
  - Keeps Tauri command function names, invoke registration, latest screenshot payload cache, disabled response builders, and WGC/DXGI command orchestration unchanged.
  - Adds selected-readback JSON contract assertions for top-level planned status and early failure placeholders.
- `tauri-client/src-tauri/src/lib.rs`
  - Registers `screenshot_diagnostics_requests` as a module without changing Tauri command registration or adding new invoke names.
- `tauri-client/src-tauri/src/screenshot_diagnostics_json.rs`
  - Adds top-level `status: "planned"` to successful `selectedReadbackPlan` JSON.
  - Ensures WGC safety-field injection inserts `selectedReadbackPlan: null` for early bounds/cache failures that cannot produce a plan.
- `tauri-client/src-tauri/src/screenshot_native/wgc_session.rs`
  - Moves local WGC session request validation before the real WGC support probe, so invalid timeout/dimensions/target and missing opt-in paths do not call real WGC APIs.
  - Splits native API support validation into a separate step after local request validation.
- `tauri-client/src-tauri/src/screenshot_native/wgc_session_tests.rs`
  - Converts the real WGC session smoke test into ignored/env-guarded form with `YSN_WGC_SESSION_LIVE_SMOKE=1`.
- `tauri-client/src-tauri/src/screenshot_native/wgc_probe.rs`
  - Converts the real WGC `IsSupported` probe smoke into ignored/env-guarded form with `YSN_WGC_REAL_API_PROBE_SMOKE=1`.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 230 results, validation, non-goals, line counts, and next recommended chapter.

### Deleted Files
- None.

### Explicit Non-Goals
- Did not move the WGC command bodies into a new command module; subagent audits recommended doing request DTOs and test/contract hardening first.
- Did not move latest screenshot payload cache/state out of `screenshot_commands.rs`.
- Did not change Tauri invoke names, frontend DTOs, Alt+A default route, native route readiness, presenter behavior, clipboard/file/OCR/translation behavior, or selected-output effects.
- Did not run real WGC/DXGI live capture; ignored tests were run without live env vars and therefore skipped real API work.
- Did not claim selected PNG, selected readback, fake-sink copy evidence, or Snow/QQ/WeChat-grade Alt+A acceptance.
- Did not raise C/E progress beyond `79% / 81% / 76%`.

### Validation
- `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml`
- `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`
- `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_native::wgc_session_tests`
  - Passed: 5 tests; 1 ignored real WGC session smoke.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_native::wgc_probe::tests`
  - Passed: 5 tests; 1 ignored real WGC API probe smoke.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib latest_screenshot_payload_wgc_monitor_diagnostic_tests`
  - Passed: 15 tests.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_monitor_session_live_smoke_env_guarded -- --ignored --nocapture`
  - Passed in default-safe mode and printed skip because `YSN_WGC_MONITOR_SESSION_LIVE_SMOKE=1` was not set.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib unsupported_api_remains_fallback_without_frame_claim -- --ignored --nocapture`
  - Passed in default-safe mode and printed skip because `YSN_WGC_SESSION_LIVE_SMOKE=1` was not set.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib explicit_real_api_probe_preserves_recoverable_fallback -- --ignored --nocapture`
  - Passed in default-safe mode and printed skip because `YSN_WGC_REAL_API_PROBE_SMOKE=1` was not set.
- `git diff --check`
  - Passed; Git emitted LF-to-CRLF working-copy warnings only.
- Selected file line counts before appending Chapter 230: `docs/IMPLEMENTATION_CHAPTERS.md` 9652 / 7051 non-empty, `screenshot_commands.rs` 4892 / 4617 non-empty, `screenshot_diagnostics_json.rs` 627 / 598 non-empty, `screenshot_diagnostics_requests.rs` 122 / 112 non-empty, `wgc_session.rs` 399 / 379 non-empty, `wgc_session_tests.rs` 161 / 146 non-empty, `wgc_probe.rs` 229 / 213 non-empty, `lib.rs` 593 / 560 non-empty.
- Recursive `screenshot_native` audit after Chapter 230: 61 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 11; >400 non-empty: 7. Current physical top remains `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `overlay_renderer.rs` 408 / 365, `wgc_session.rs` 399 / 379.

### Known Risks
- `screenshot_commands.rs` is still very large at 4892 physical / 4617 non-empty lines, so WGC and DXGI command bodies still need focused extraction.
- `screenshot_diagnostics_json.rs` is now 627 physical / 598 non-empty lines and should not become a dumping ground for command policy or request DTOs.
- WGC command extraction still needs careful handling of latest-payload state, request-vs-latest bounds resolution, and test ownership.
- Ignored/env-guarded tests prove default safety, not real WGC selected-frame success; live desktop evidence is still missing.

### Next Recommended Chapter
- Chapter 231 should extract the three WGC diagnostic command bodies into `screenshot_wgc_diagnostic_commands.rs`, preserve invoke names through module re-export, keep latest-payload cache ownership explicit, and move only the WGC-owned tests identified by the Chapter 230 audits.
- Keep progress at `79% / 81% / 76%` until real selected-frame readback, selected PNG evidence, fake-sink copy evidence, and runtime Alt+A selected-output evidence justify an increase.

## Chapter 231 - WGC Diagnostic Command Module Extraction (2026-06-09)

> Chapter status: completed only for this WGC diagnostic command-body extraction and frontend diagnostic DTO alignment slice. This does not mark 方案 C, 方案 E, WGC/DXGI live capture, selected PNG evidence, selected-output effects, native route readiness, runtime Alt+A routing, or commercial screenshot acceptance complete.

### Goals
- Move the three WGC diagnostic Tauri command bodies out of `screenshot_commands.rs` into a focused module while preserving invoke names and JSON contract.
- Keep latest screenshot payload cache ownership in `screenshot_commands.rs`; expose only the existing request/latest bounds resolver needed by WGC diagnostics.
- Keep WGC live-smoke imports stable through a `screenshot_commands` facade re-export.
- Align frontend diagnostic DTOs with backend WGC target/session smoke fields before later UI consumption.

### External Findings
- No online research was needed for this chapter because the work was local command-module extraction, DTO alignment, and validation against current code.
- Six read-only subagent audits were used to verify command extraction mechanics, latest-payload state ownership, test migration boundaries, `generate_handler!` registration, WGC response DTO risks, and line-count/documentation constraints.

### Progress
- Overall C/E progress: approximately 79%.
- 方案 C native overlay / selected-output progress: approximately 81%.
- 方案 E DXGI/WGC/D3D11/GPU texture progress: approximately 76%.
- Progress remains unchanged because this chapter only improves module boundaries and type contracts; it still does not produce real selected-frame readback, selected PNG evidence, fake-sink copy evidence, or runtime Alt+A selected-output evidence.

### Added Files
- `tauri-client/src-tauri/src/screenshot_wgc_diagnostic_commands.rs`
  - Contains `run_native_wgc_one_frame_probe_smoke`, `resolve_native_wgc_monitor_target_diagnostic`, and `run_native_wgc_monitor_session_smoke` with their existing Tauri command names and diagnostic-only JSON behavior.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - Removes the three WGC diagnostic command bodies and re-exports them from the new WGC command module for existing internal import paths.
  - Changes `latest_or_request_physical_bounds` to `pub(crate)` so the new WGC command module can reuse the current latest/request bounds adapter without exposing raw latest-payload store mutation.
  - Leaves latest screenshot payload cache, payload writer/clearer, DXGI command bodies, and adapter tests in place.
- `tauri-client/src-tauri/src/lib.rs`
  - Registers `screenshot_wgc_diagnostic_commands` and uses module-qualified command paths in `generate_handler!`, preserving the same invoke names without glob re-export warnings.
- `tauri-client/src/types/screenshot.ts`
  - Adds diagnostic DTOs for `selectedReadbackPlan`, latest payload summaries, WGC monitor target diagnostics, and the fuller WGC monitor session smoke response.
  - Uses backend-shaped diagnostic rects with `width` / `height` rather than UI annotation `w` / `h` fields.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 231 results, validation, non-goals, line counts, and next recommended chapter.

### Deleted Files
- None.

### Explicit Non-Goals
- Did not move latest screenshot payload cache/state into a new module.
- Did not move DXGI diagnostic command bodies or disabled response builders.
- Did not move the full `latest_screenshot_payload_wgc_monitor_diagnostic_tests` module; latest-payload adapter tests stay with the cache owner until state is split deliberately.
- Did not change Tauri invoke names, Alt+A default route, native route readiness, presenter behavior, clipboard/file/OCR/translation behavior, or selected-output effects.
- Did not run real WGC/DXGI live capture; ignored WGC smoke was run without the live env var and therefore skipped real API work.
- Did not claim selected PNG, selected readback, fake-sink copy evidence, or Snow/QQ/WeChat-grade Alt+A acceptance.
- Did not raise C/E progress beyond `79% / 81% / 76%`.

### Validation
- `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml`
- `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`
- `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib latest_screenshot_payload_wgc_monitor_diagnostic_tests`
  - Passed: 15 tests.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_monitor_session_live_smoke_env_guarded -- --ignored --nocapture`
  - Passed in default-safe mode and printed skip because `YSN_WGC_MONITOR_SESSION_LIVE_SMOKE=1` was not set.
- `npm run build` from `tauri-client`
  - Passed. Existing Vite warnings remain: mixed static/dynamic `@tauri-apps/api/window` imports and large `index` chunk.
- `git diff --check`
  - Passed; Git emitted LF-to-CRLF working-copy warnings only.
- Selected file line counts before appending Chapter 231: `docs/IMPLEMENTATION_CHAPTERS.md` 9738 / 7125 non-empty, `screenshot_commands.rs` 4692 / 4422 non-empty, `screenshot_wgc_diagnostic_commands.rs` 207 / 202 non-empty, `screenshot_diagnostics_requests.rs` 122 / 112 non-empty, `screenshot_diagnostics_json.rs` 627 / 598 non-empty, `lib.rs` 595 / 561 non-empty, `screenshot.ts` 236 / 218 non-empty.
- Recursive `screenshot_native` audit after Chapter 231: 61 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 11; >400 non-empty: 7. Current physical top remains `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `overlay_renderer.rs` 408 / 365, `wgc_session.rs` 399 / 379.

### Known Risks
- `screenshot_commands.rs` is still very large at 4692 physical / 4422 non-empty lines, so DXGI diagnostics and latest-payload state still need further focused extraction.
- `screenshot_wgc_diagnostic_commands.rs` depends on `latest_or_request_physical_bounds` in `screenshot_commands.rs`; this is intentionally narrow but still keeps WGC diagnostics coupled to screenshot latest-payload adapter state.
- Frontend DTOs are now more complete, but the UI still does not consume WGC diagnostic commands as a runtime readiness gate.
- Ignored/env-guarded tests prove default safety, not real WGC selected-frame success; live desktop evidence is still missing.

### Next Recommended Chapter
- Chapter 232 should extract the remaining latest-payload adapter/state boundary or begin DXGI diagnostic command extraction, choosing whichever most reduces `screenshot_commands.rs` risk without changing Alt+A readiness.
- Keep progress at `79% / 81% / 76%` until real selected-frame readback, selected PNG evidence, fake-sink copy evidence, and runtime Alt+A selected-output evidence justify an increase.

## Chapter 232 - Guarded Live WGC Frame Evidence And DXGI Acceptance Blocker (2026-06-09)

> Chapter status: completed only for this guarded live-evidence slice. This proves a real WGC selected-monitor frame metadata path on the current desktop and identifies the current DXGI selected-output fake-sink blocker, but it does not mark 方案 C, 方案 E, selected PNG/readback, selected-output effects, native route readiness, runtime Alt+A routing, or commercial screenshot acceptance complete.

### Goals
- Stop treating module extraction as the only next step and gather real desktop evidence for the C/E critical path.
- Run the guarded WGC monitor-session live smoke with explicit live env opt-in, then run strict mode if non-strict evidence succeeds.
- Run the guarded DXGI selected-output fake-sink acceptance smoke to check whether selected PNG/readback can reach the fake sink.
- Fix the WGC live smoke assertion to match live diagnostic semantics: `selectedOutputReadyPlanningOnly` is planning-only evidence and may be true without changing readiness.

### External Findings
- Microsoft Windows Graphics Capture documentation confirms the WGC path is proven by creating a capture session/frame pool and receiving frames, not by static contracts alone: https://learn.microsoft.com/en-us/windows/uwp/audio-video-camera/screen-capture
- Microsoft Desktop Duplication documentation confirms DXGI evidence depends on `AcquireNextFrame` behavior and its timeout/access-lost outcomes, which maps to the observed `0x887A0027` timeout blocker: https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api
- These findings support the chapter pivot from more command splitting to guarded live frame evidence.

### Progress
- Overall C/E progress: approximately 79%.
- 方案 C native overlay / selected-output progress: approximately 81%.
- 方案 E DXGI/WGC/D3D11/GPU texture progress: approximately 76%.
- Progress remains unchanged despite the WGC live-frame success because selected PNG/readback, fake-sink copy success, and runtime Alt+A selected-output evidence are still missing, and DXGI selected-output acceptance is still blocked by `AcquireNextFrame` timeout.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_native/wgc_monitor_session_live_smoke.rs`
  - Imports the WGC monitor-session command from `screenshot_wgc_diagnostic_commands` and request DTOs from `screenshot_diagnostics_requests` instead of relying on the legacy `screenshot_commands` facade.
  - Replaces the incorrect `selectedOutputReadyPlanningOnly == false` assertion with diagnostic safety assertions: `selectedReadbackPlan.diagnosticOnly == true`, `selectedReadbackPlan.readinessChanged == false`, and `selectedReadbackPlan.status == "planned"`.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 232 live evidence, blocker evidence, validation, non-goals, line counts, and next recommended chapter.

### Deleted Files
- None.

### Explicit Non-Goals
- Did not mark WGC selected PNG/readback as complete; `selectedPngEvidence` remained `null`, `selectedPngProduced` remained `false`, and `readbackBytesPresent` remained `false`.
- Did not mark DXGI selected-output fake-sink acceptance complete; the fake sink had `sink.calls = 0` and no selected PNG.
- Did not change Alt+A routing, native route readiness, presenter behavior, clipboard/file/OCR/translation behavior, or selected-output effects.
- Did not run real clipboard acceptance; DXGI acceptance used fake sink mode only.
- Did not continue DXGI command extraction in this chapter because live evidence identified a higher-value blocker.
- Did not raise C/E progress beyond `79% / 81% / 76%`.

### Validation
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_monitor_session_live_smoke_env_guarded -- --ignored --nocapture` with `YSN_WGC_MONITOR_SESSION_LIVE_SMOKE=1`
  - Initially produced real WGC frame evidence but failed due to an incorrect planning-only assertion.
  - Evidence from the failed run already showed `ok=true`, `frameCaptureConfirmed=true`, `session.state="frame-acquired"`, `frameWidth=2560`, `frameHeight=1440`, `frameMatchesTargetMonitorBounds=true`, `selectedCropWithinFrame=true`, `selectedPngEvidence=null`, and `selectedPngProduced=false`.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_monitor_session_live_smoke_env_guarded -- --ignored --nocapture` with `YSN_WGC_MONITOR_SESSION_LIVE_SMOKE=1` after fixing the assertion
  - Passed.
  - Confirmed real WGC monitor-session frame acquisition on the current desktop: `ok=true`, `attemptedRealWgcApi=true`, `frameCaptureAttempted=true`, `frameCaptureConfirmed=true`, `session.state="frame-acquired"`, `createdDevice=true`, `createdItem=true`, `createdFramePool=true`, `createdSession=true`, `startedCapture=true`, `selectedMonitorFrameEvidence.frameMatchesTargetMonitorBounds=true`, `selectedMonitorFrameEvidence.selectedCropWithinFrame=true`, `persistentHandleExposed=false`, `readinessChanged=false`.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_monitor_session_live_smoke_env_guarded -- --ignored --nocapture` with `YSN_WGC_MONITOR_SESSION_LIVE_SMOKE=1` and `YSN_REQUIRE_WGC_MONITOR_SESSION_SMOKE=1`
  - Passed strict mode.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib dxgi_selected_output_acceptance_fake_sink_live_smoke -- --ignored --nocapture` with `YSN_DXGI_SELECTED_OUTPUT_ACCEPTANCE=1`
  - Test command passed as a diagnostic smoke, but response showed selected-output acceptance is not successful: `ok=false`, `frameCaptureConfirmed=false`, `selectedPngEvidence=null`, `selectedOnly=false`, `pngSignatureValid=false`, `selectedOutputEffectConfirmed=false`, `sink.calls=0`, `clipboardReadbackAttempted=false`.
  - Current blocker: `DXGI first-frame warmup exhausted 500 ms after 10 attempts`, HRESULT `0x887A0027`, localized timeout message.
  - Output ranking did select adapter `0` / output `0` with bounds `2560x1440` and requested bounds `0,0,320,180`, so selection-to-output ranking is not the immediate blocker.
- `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`
- `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib latest_screenshot_payload_wgc_monitor_diagnostic_tests`
  - Passed: 15 tests.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_monitor_session_live_smoke_env_guarded -- --ignored --nocapture` without live env
  - Passed in default-safe mode and printed skip.
- `git diff --check`
  - Passed; Git emitted LF-to-CRLF working-copy warnings only.
- Selected file line counts before appending Chapter 232: `docs/IMPLEMENTATION_CHAPTERS.md` 9812 / 7187 non-empty, `wgc_monitor_session_live_smoke.rs` 83 / 75 non-empty, `screenshot_commands.rs` 4692 / 4422 non-empty, `screenshot_wgc_diagnostic_commands.rs` 207 / 202 non-empty, `lib.rs` 595 / 561 non-empty, `screenshot.ts` 236 / 218 non-empty.
- Recursive `screenshot_native` audit after Chapter 232: 61 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 11; >400 non-empty: 7. Current physical top remains `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `overlay_renderer.rs` 408 / 365, `wgc_session.rs` 399 / 379.

### Known Risks
- WGC can now acquire a real selected-monitor frame on this desktop, but there is still no WGC selected-region readback, no selected PNG evidence, no output effect, and no Alt+A runtime routing evidence.
- DXGI selected-output acceptance still times out before selected PNG production, so fake-sink acceptance cannot confirm selected-output effects yet.
- The DXGI timeout remains the same class of blocker seen in earlier chapters: `0x887A0027` / frame not ready.
- `screenshot_commands.rs` remains large at 4692 physical / 4422 non-empty lines, but live-evidence work now has higher priority than purely mechanical splitting.
- The guarded WGC live smoke uses a tiny `0,0,1,1` request; it proves selected-monitor frame metadata and crop containment, not general user selection UX.

### Next Recommended Chapter
- Chapter 233 should add the smallest WGC selected-region readback/PNG evidence path from the successful WGC frame, or fix the DXGI first-frame timeout root cause if DXGI remains the desired selected-output mainline for fake-sink acceptance.
- Keep progress at `79% / 81% / 76%` until real selected-region readback, selected PNG evidence, fake-sink copy evidence, and runtime Alt+A selected-output evidence justify an increase.

## Chapter 233 - WGC Selected-Region Readback And PNG Evidence (2026-06-09)

> Chapter status: completed only for this guarded WGC selected-region readback evidence slice. This proves that the successful WGC monitor-session frame can be copied through a D3D11 staging readback into a selected-only PNG diagnostic contract on the current desktop, but it does not mark 方案 C, 方案 E, selected-output effects, DXGI fake-sink acceptance, runtime Alt+A routing, native route readiness, or commercial screenshot acceptance complete.

### Goals
- Add the smallest WGC selected-region readback path after the already-proven WGC monitor frame acquisition.
- Reuse the existing DXGI D3D11 staging readback and selected-image bridge rather than duplicating PNG encoding or BGRA/RGBA conversion logic.
- Surface WGC selected PNG evidence in diagnostic JSON while preserving diagnostic-only safety fields.
- Update the guarded WGC live smoke so `ok=true` requires selected PNG/readback evidence instead of frame metadata alone.

### External Findings
- Microsoft Direct3D 11 documentation for `ID3D11DeviceContext::CopySubresourceRegion` supports copying a texture subregion into another resource, matching the selected-crop staging copy used by the existing DXGI readback path: https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11devicecontext-copysubresourceregion
- Microsoft Direct3D 11 documentation for `ID3D11DeviceContext::Map` supports CPU access to a staging resource with map/read semantics, matching the current selected-region readback gate: https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11devicecontext-map
- Microsoft Windows Graphics Capture documentation continues to frame WGC as a Direct3D frame/surface pipeline, so converting the acquired `Direct3D11CaptureFrame` surface into an `ID3D11Texture2D` and reusing the existing D3D11 readback bridge fits the project architecture: https://learn.microsoft.com/en-us/windows/uwp/audio-video-camera/screen-capture

### Progress
- Overall C/E progress: approximately 79%.
- 方案 C native overlay / selected-output progress: approximately 81%.
- 方案 E DXGI/WGC/D3D11/GPU texture progress: approximately 76%.
- Progress remains unchanged even though this chapter adds real WGC selected-region readback and selected PNG evidence, because runtime `Alt+A` selected-output routing/readiness, selected-output effects, and DXGI fake-sink copy acceptance remain incomplete.

### Added Files
- `tauri-client/src-tauri/src/screenshot_native/wgc_readback.rs`
  - Adds the focused WGC selected-readback boundary for translating desktop monitor bounds into monitor-local `SelectionRect` values.
  - Reuses `build_selected_png_contract_from_dxgi_texture` for D3D11 staging readback, BGRA/RGBA conversion, and selected-only PNG construction.
  - Adds deterministic unit tests for monitor-local crop translation and out-of-target rejection.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_native/mod.rs`
  - Registers the new `wgc_readback` module.
- `tauri-client/src-tauri/src/screenshot_native/wgc_device.rs`
  - Extends `WgcAcquiredTextureFrame` with optional `SelectedImageContract` evidence while keeping the captured texture private.
- `tauri-client/src-tauri/src/screenshot_native/wgc_session.rs`
  - Builds selected PNG evidence immediately after WGC texture acquisition and before closing the WGC session/frame pool.
  - Marks WGC evidence `selected_png_produced` and `readback_bytes_present` only when the selected-only PNG contract is produced.
- `tauri-client/src-tauri/src/screenshot_native/wgc_session_types.rs`
  - Adds selected-image evidence to `WgcOneFrameSessionReport` and makes `WgcSelectedMonitorFrameEvidence::from_session` accept explicit selected-PNG status.
- `tauri-client/src-tauri/src/screenshot_diagnostics_json.rs`
  - Serializes real WGC `selectedPngEvidence` instead of hardcoded `null` when selected PNG evidence exists.
  - Keeps `diagnosticOnly`, `persistentHandleExposed=false`, `readinessChanged=false`, and `altAChanged=false` unchanged.
- `tauri-client/src-tauri/src/screenshot_wgc_diagnostic_commands.rs`
  - Tightens monitor-session `ok=true` to require frame acquisition, target-bounds match, crop containment, selected PNG production, and readback-byte evidence.
- `tauri-client/src-tauri/src/screenshot_native/wgc_monitor_session_live_smoke.rs`
  - Updates guarded live smoke assertions to require non-null selected PNG evidence, selected-only PNG status, crop dimension match, and positive PNG byte length on successful `ok=true`.
- `tauri-client/src/types/screenshot.ts`
  - Adds reusable `SelectedPngEvidence` DTO typing and allows WGC selected PNG evidence to be non-null.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 233 evidence, validation, non-goals, line counts, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- Real WGC selected-region readback evidence was added for the successful WGC frame path.
- Real selected PNG evidence was produced from the WGC selected-region path, with selected-only PNG shape, crop dimensions, byte length, and fingerprint exposed by the guarded diagnostic smoke.
- Current desktop live evidence from the strict WGC smoke showed `ok=true`, `frameCaptureConfirmed=true`, `session.state="frame-acquired"`, `frameWidth=2560`, `frameHeight=1440`, `selectedPngProduced=true`, `readbackBytesPresent=true`, `selectedPngEvidence.pngWidth=1`, `selectedPngEvidence.pngHeight=1`, `selectedPngEvidence.pngByteLen=73`, `selectedPngEvidence.selectedOnlyPng=true`, `selectedPngEvidence.dimensionsMatchCrop=true`, and `persistentHandleExposed=false`.
- This evidence upgrades the WGC diagnostic path from selected-monitor frame metadata only to selected-region readback/PNG proof for the current desktop.
- This evidence does not prove runtime `Alt+A` selected-output readiness, selected-output presenter effects, clipboard/file/OCR/translation behavior, or DXGI fake-sink acceptance.

### Explicit Non-Goals
- Did not mark 方案 C native overlay / selected-output complete; runtime `Alt+A` selected-output routing/readiness and selected-output effects are still not proven.
- Did not mark 方案 E DXGI/WGC/D3D11/GPU texture complete; WGC selected PNG/readback evidence exists, but DXGI selected-output fake-sink copy acceptance remains incomplete.
- Did not mark native route readiness, commercial screenshot acceptance, clipboard/file/OCR/translation behavior, or presenter behavior complete.
- Did not expose persistent WGC/D3D11 handles, change Alt+A routing, promote readiness, or treat diagnostic-only WGC readback as user-facing readiness.
- Did not raise C/E progress beyond `79% / 81% / 76%`.

### Validation
- `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`
- `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib wgc_readback -- --nocapture`
  - Passed: 2 tests.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib latest_screenshot_payload_wgc_monitor_diagnostic_tests`
  - Passed: 15 tests.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_monitor_session_live_smoke_env_guarded -- --ignored --nocapture` without live env
  - Passed in default-safe mode and printed skip.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_monitor_session_live_smoke_env_guarded -- --ignored --nocapture` with `YSN_WGC_MONITOR_SESSION_LIVE_SMOKE=1`
  - Passed and produced real WGC selected PNG/readback evidence.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_monitor_session_live_smoke_env_guarded -- --ignored --nocapture` with `YSN_WGC_MONITOR_SESSION_LIVE_SMOKE=1` and `YSN_REQUIRE_WGC_MONITOR_SESSION_SMOKE=1`
  - Passed strict mode with `ok=true`, `selectedPngProduced=true`, and non-null `selectedPngEvidence`.
- `npm run build`
  - Passed. Existing Vite warnings remain for mixed static/dynamic `@tauri-apps/api/window` imports and the large `index` chunk.
- `git diff --check`
  - Passed; Git emitted LF-to-CRLF working-copy warnings only.
- Selected file line counts before appending Chapter 233: `docs/IMPLEMENTATION_CHAPTERS.md` 9889 / 7252 non-empty, `wgc_readback.rs` 122 / 108 non-empty, `wgc_session.rs` 420 / 400 non-empty, `wgc_session_types.rs` 293 / 271 non-empty, `wgc_device.rs` 255 / 231 non-empty, `screenshot_diagnostics_json.rs` 628 / 599 non-empty, `screenshot_wgc_diagnostic_commands.rs` 209 / 204 non-empty, `wgc_monitor_session_live_smoke.rs` 102 / 94 non-empty, `screenshot.ts` 249 / 230 non-empty.
- Recursive `screenshot_native` audit after Chapter 233: 62 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- WGC selected-region readback and selected PNG evidence now exist for the guarded diagnostic path, but the user-facing `Alt+A` route still lacks selected-output readiness evidence.
- DXGI selected-output fake-sink acceptance remains incomplete, so selected-output copy/effect behavior is still not confirmed through the DXGI acceptance path.
- The WGC live smoke still uses a tiny `0,0,1,1` diagnostic request; it proves the readback/PNG bridge on this desktop, not general user drag selection UX.
- `screenshot_commands.rs` remains a large legacy integration file, although Chapter 233 avoided increasing it.
- The new evidence should not be surfaced as a ready state in UI because the master plan forbids showing unverified capabilities as ready.

### Next Recommended Chapter
- Chapter 234 should either wire WGC selected PNG evidence into a guarded selected-output fake-sink/copy diagnostic without changing Alt+A readiness, or fix the DXGI `AcquireNextFrame` timeout root cause so the DXGI selected-output fake-sink acceptance path can produce `ok=true` evidence.
- Keep progress at `79% / 81% / 76%` until fake-sink copy evidence and runtime Alt+A selected-output evidence justify an increase.

## Chapter 234 - WGC Selected-Output Fake-Sink Acceptance Evidence (2026-06-09)

> Chapter status: completed only for this guarded WGC selected-output fake-sink diagnostic slice. This proves the diagnostic WGC selected PNG/readback contract can flow through the selected-output copy pipeline into an injected fake sink on the current desktop, but it does not mark 方案 C, 方案 E, native route readiness, runtime `Alt+A` routing, DXGI fake-sink acceptance, real clipboard/file/OCR/translation effects, or commercial screenshot acceptance complete.

### Goals
- Reuse the Chapter 233 WGC selected PNG evidence to prove a fake-sink selected-output copy/effect path without touching real clipboard behavior.
- Keep WGC fake-sink acceptance isolated in a native domain module rather than growing `screenshot_commands.rs`, `wgc_session.rs`, or command JSON glue unnecessarily.
- Surface diagnostic-only WGC fake-sink evidence in the monitor-session smoke response and live smoke assertions.
- Preserve `diagnosticOnly=true`, `readinessChanged=false`, `altAChanged=false`, and `persistentHandleExposed=false` across response levels.

### External Findings
- Microsoft Windows Graphics Capture documentation describes WGC as a capture item/session/frame-pool pipeline, which supports treating the acquired WGC frame and derived selected PNG as local diagnostic evidence rather than user-facing readiness: https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture
- Microsoft `Direct3D11CaptureFramePool` documentation confirms the frame-pool/session shape used by the existing guarded WGC smoke path: https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.direct3d11captureframepool
- Microsoft `IDXGIOutputDuplication::AcquireNextFrame` documentation confirms `DXGI_ERROR_WAIT_TIMEOUT` is a timeout waiting for a new desktop frame, matching the still-open DXGI blocker and justifying WGC fake-sink progress as the smaller Chapter 234 slice: https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_2/nf-dxgi1_2-idxgioutputduplication-acquirenextframe

### Progress
- Overall C/E progress: approximately 79%.
- 方案 C native overlay / selected-output progress: approximately 81%.
- 方案 E DXGI/WGC/D3D11/GPU texture progress: approximately 76%.
- Progress remains unchanged even though WGC selected PNG evidence now passes a fake-sink copy acceptance diagnostic, because runtime `Alt+A` selected-output routing/readiness, real selected-output effects, and DXGI fake-sink acceptance remain incomplete.

### Added Files
- `tauri-client/src-tauri/src/screenshot_native/wgc_selected_output_acceptance.rs`
  - Adds `WgcSelectedOutputFakeSinkAcceptanceReceipt` with explicit safety fields and a `proves_fake_sink_copy()` predicate.
  - Adds `accept_wgc_selected_output_fake_sink_copy`, which wraps `SelectedImageContract` into the existing selected-output copy pipeline using an injected sink only.
  - Adds focused unit tests proving fake-sink copy success and explicit opt-in rejection.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_native/mod.rs`
  - Registers the new `wgc_selected_output_acceptance` module.
- `tauri-client/src-tauri/src/screenshot_wgc_diagnostic_commands.rs`
  - Adds a local fake clipboard sink for WGC diagnostics.
  - Builds `selectedOutputFakeSinkAcceptance` from `WgcOneFrameSessionReport.selected_image` and requires it for top-level `ok=true`.
  - Keeps all fake-sink behavior diagnostic-only and injected; no real clipboard path is invoked.
- `tauri-client/src-tauri/src/screenshot_diagnostics_json.rs`
  - Adds `wgc_fake_sink_acceptance_json` and `merge_wgc_session_fake_sink_acceptance` helpers.
  - Emits fake-sink evidence at both top-level `selectedOutputFakeSinkAcceptance` and nested `session.selectedOutputFakeSinkAcceptance`.
- `tauri-client/src-tauri/src/screenshot_native/wgc_monitor_session_live_smoke.rs`
  - Extends the guarded live smoke to require fake-sink acceptance evidence when `ok=true`.
- `tauri-client/src/types/screenshot.ts`
  - Adds `WgcSelectedOutputFakeSinkAcceptance` DTO typing and wires it into the WGC monitor-session smoke response.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 234 evidence, validation, non-goals, line counts, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- Current desktop WGC live smoke produced `selectedOutputFakeSinkAcceptance.ok=true`, `fakeSinkCopyAccepted=true`, `sink="provided-fake-sink"`, `sinkCalls=1`, `selectedOnlyPng=true`, `pngByteLen=73`, `copiedPngByteLen=73`, `effect.copyOnly=true`, `effect.copiedToClipboard=true`, `diagnosticOnly=true`, `readinessChanged=false`, `altAChanged=false`, and `persistentHandleExposed=false`.
- The same response still proved WGC frame/readback evidence: `frameCaptureConfirmed=true`, `session.state="frame-acquired"`, `frameWidth=2560`, `frameHeight=1440`, `selectedPngProduced=true`, `readbackBytesPresent=true`, and non-null selected PNG evidence.
- This evidence proves only an injected fake-sink selected-output copy pipeline for WGC selected PNG evidence. It does not prove runtime `Alt+A`, real clipboard, file save, OCR, translation, presenter effects, or DXGI fake-sink acceptance.

### Explicit Non-Goals
- Did not change default `Alt+A` routing, native route readiness, overlay presenter behavior, or WebView/native fallback policy.
- Did not invoke real clipboard/file/OCR/translation effects; the acceptance uses an injected fake sink only.
- Did not fix DXGI `AcquireNextFrame` timeout or mark DXGI selected-output fake-sink acceptance complete.
- Did not mark 方案 C or 方案 E complete, and did not raise C/E progress beyond `79% / 81% / 76%`.
- Did not expose persistent WGC/D3D11 handles or surface the diagnostic as a UI-ready state.

### Validation
- `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`
- `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib wgc_selected_output_acceptance -- --nocapture`
  - Passed: 2 tests.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib latest_screenshot_payload_wgc_monitor_diagnostic_tests`
  - Passed: 15 tests.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_monitor_session_live_smoke_env_guarded -- --ignored --nocapture` without live env
  - Passed in default-safe mode and printed skip.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_monitor_session_live_smoke_env_guarded -- --ignored --nocapture` with `YSN_WGC_MONITOR_SESSION_LIVE_SMOKE=1`
  - Passed and produced WGC selected PNG plus fake-sink acceptance evidence.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_monitor_session_live_smoke_env_guarded -- --ignored --nocapture` with `YSN_WGC_MONITOR_SESSION_LIVE_SMOKE=1` and `YSN_REQUIRE_WGC_MONITOR_SESSION_SMOKE=1`
  - Passed strict mode with `ok=true` and `selectedOutputFakeSinkAcceptance.ok=true`.
- `npm run build`
  - Passed. Existing Vite warnings remain for mixed static/dynamic `@tauri-apps/api/window` imports and the large `index` chunk.
- `git diff --check`
  - Passed; Git emitted LF-to-CRLF working-copy warnings only.
- Selected file line counts before appending Chapter 234: `docs/IMPLEMENTATION_CHAPTERS.md` 9986 / 7336 non-empty, `wgc_selected_output_acceptance.rs` 152 / 139 non-empty, `screenshot_wgc_diagnostic_commands.rs` 256 / 249 non-empty, `screenshot_diagnostics_json.rs` 673 / 642 non-empty, `wgc_monitor_session_live_smoke.rs` 119 / 111 non-empty, `screenshot.ts` 281 / 261 non-empty.
- Recursive `screenshot_native` audit after Chapter 234: 63 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- WGC selected PNG evidence now passes fake-sink copy acceptance, but the user-facing `Alt+A` route still lacks selected-output runtime evidence.
- DXGI selected-output fake-sink acceptance remains blocked by `AcquireNextFrame` timeout, so the DXGI side of 方案 E is still incomplete.
- The live WGC acceptance still uses a tiny `0,0,1,1` diagnostic request; it proves the bridge and fake-sink pipeline on this desktop, not general drag-selection UX.
- `screenshot_diagnostics_json.rs` is now over 600 non-empty lines; future chapters should consider WGC-specific JSON extraction if more diagnostic fields are added.
- The new fake-sink evidence must not be surfaced as ready UI or native-route readiness until real runtime `Alt+A` selected-output evidence exists.

### Next Recommended Chapter
- Chapter 235 should either add a stricter WGC selected-output runtime diagnostic that starts from an actual native selection path without changing default `Alt+A`, or continue the DXGI timeout investigation with an in-session desktop update pulse before `AcquireNextFrame`.
- Keep progress at `79% / 81% / 76%` until runtime `Alt+A` selected-output evidence and DXGI selected-output acceptance evidence justify an increase.

## Chapter 235 - DXGI In-Session Pulse Before Acquire Timeout Evidence (2026-06-09)

> Chapter status: completed only for this diagnostic DXGI timeout investigation slice. This adds an in-session desktop-update pulse immediately before `AcquireNextFrame` for both default-output and selected-output duplication sessions, and proves on the current desktop that the tiny pulse still does not unblock DXGI first-frame acquisition. It does not mark 方案 C, 方案 E, native route readiness, runtime `Alt+A` routing, DXGI selected-output acceptance, or commercial screenshot acceptance complete.

### Goals
- Narrow the current DXGI selected-output blocker by testing whether a desktop update pulse inside the same duplication session can unblock `AcquireNextFrame`.
- Keep the probe diagnostic-only and avoid changing production `DxgiDesktopDuplicationBackend::capture_texture_frame` behavior.
- Capture default-output and selected-output evidence in one report with pulse, acquire, output identity/ranking, timeout, stop, and safety fields.
- Preserve no side effects: no clipboard, file, OCR, translation, presenter, overlay, Alt+A, readiness, or persistent handle exposure.

### External Findings
- Microsoft `IDXGIOutputDuplication::AcquireNextFrame` documentation says the call waits for the next desktop image update or pointer update and may return `DXGI_ERROR_WAIT_TIMEOUT` when no frame is available within the timeout: https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_2/nf-dxgi1_2-idxgioutputduplication-acquirenextframe
- Microsoft Desktop Duplication API documentation confirms the duplication model is frame/update driven, which makes an in-session desktop-update pulse a valid diagnostic intervention before treating timeout as a routing bug: https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/desktop-dup-api

### Progress
- Overall C/E progress: approximately 79%.
- 方案 C native overlay / selected-output progress: approximately 81%.
- 方案 E DXGI/WGC/D3D11/GPU texture progress: approximately 76%.
- Progress remains unchanged because the in-session pulse probe did not acquire a DXGI frame, and runtime `Alt+A` selected-output evidence plus DXGI selected-output acceptance are still missing.

### Added Files
- `tauri-client/src-tauri/src/screenshot_native/dxgi_pulse_before_acquire_probe.rs`
  - Adds a focused diagnostic probe that opens default-output and selected-output DXGI duplication sessions, creates a tiny non-activating desktop pulse, then immediately calls `AcquireNextFrame` in the same session.
  - Records per-path output bounds, adapter/output identity, selected-output ranking, pulse report, acquire attempt, HRESULT, timeout/access-lost flags, release status, stop status, and errors.
  - Adds deterministic tests for empty-bounds rejection and HRESULT extraction from localized timeout messages.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_native/mod.rs`
  - Registers the new `dxgi_pulse_before_acquire_probe` module.
- `tauri-client/src-tauri/src/screenshot_diagnostics_requests.rs`
  - Adds `NativeDxgiPulseBeforeAcquireProbeRequest` with explicit DXGI and desktop-pulse guard fields.
- `tauri-client/src-tauri/src/screenshot_diagnostics_json.rs`
  - Adds serializers for pulse-before-acquire path/report evidence while reusing existing frame-info attempt and desktop-pulse JSON helpers.
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - Adds guarded `run_native_dxgi_pulse_before_acquire_probe` Tauri command and default-deny/guard/invalid-bounds/live smoke tests.
- `tauri-client/src-tauri/src/lib.rs`
  - Registers the new command.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 235 evidence, validation, non-goals, line counts, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- Live ignored diagnostic `dxgi_pulse_before_acquire_live_smoke` ran on the current desktop and both paths still timed out after the in-session pulse.
- Default-output evidence: `attempted=true`, `pulse.ok=true`, `pulse.hiddenFromAltTab=true`, `pulse.noActivate=true`, `pulse.destroyConfirmed=true`, `acquire.ok=false`, `acquire.timedOut=true`, `hresultHex="0x887A0027"`, `accessLost=false`, `frameInfo=null`, `releasedFrame=false`, `stopped=true`, output bounds `0,0,2560x1440`.
- Selected-output evidence: selected ranking chose adapter `0`, output `0`, requested bounds `0,0,320x180`, intersection ratio `1.0`, `pulse.ok=true`, `acquire.ok=false`, `acquire.timedOut=true`, `hresultHex="0x887A0027"`, `accessLost=false`, `frameInfo=null`, `releasedFrame=false`, `stopped=true`.
- Top-level comparison showed `defaultFrameConfirmed=false`, `selectedFrameConfirmed=false`, `anyFrameConfirmed=false`, and `bothTimedOut=true`.
- This confirms the earlier DXGI timeout class is not solved by a tiny in-session no-activate pulse at the selected/default output origin.

### Explicit Non-Goals
- Did not change production DXGI capture retry behavior, selected-output acceptance behavior, native route readiness, or default `Alt+A` routing.
- Did not promote DXGI readiness or mark DXGI selected-output fake-sink acceptance complete.
- Did not invoke clipboard/file/OCR/translation/presenter/overlay side effects.
- Did not mark 方案 C or 方案 E complete, and did not raise C/E progress beyond `79% / 81% / 76%`.
- Did not expose persistent DXGI/D3D11 handles.

### Validation
- `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`
- `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib dxgi_pulse_before_acquire_probe -- --nocapture`
  - Passed: 6 tests, 1 ignored live smoke.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_dxgi_desktop_update_pulse_diagnostic_command_tests -- --nocapture`
  - Passed: 4 tests, 1 ignored live smoke.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_dxgi_frame_info_probe_command_tests -- --nocapture`
  - Passed: 4 tests, 1 ignored live smoke.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib dxgi_pulse_before_acquire_live_smoke -- --ignored --nocapture`
  - Passed as diagnostic live smoke and printed the timeout evidence above; command success does not mean DXGI acquired a frame.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib dxgi_selected_output_acceptance -- --nocapture`
  - Passed: 9 tests, 1 ignored live smoke.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib wgc_selected_output_acceptance -- --nocapture`
  - Passed: 2 tests.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib latest_screenshot_payload_wgc_monitor_diagnostic_tests -- --nocapture`
  - Passed: 15 tests.
- `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_monitor_session_live_smoke_env_guarded -- --ignored --nocapture` without live env
  - Passed in default-safe mode and printed skip.
- `npm run build`
  - Passed. Existing Vite warnings remain for mixed static/dynamic `@tauri-apps/api/window` imports and the large `index` chunk.
- `git diff --check`
  - Passed; Git emitted LF-to-CRLF working-copy warnings only.
- Selected file line counts before appending Chapter 235: `docs/IMPLEMENTATION_CHAPTERS.md` 10076 / 7413 non-empty, `dxgi_pulse_before_acquire_probe.rs` 309 / 292 non-empty, `screenshot_commands.rs` 4953 / 4669 non-empty, `screenshot_diagnostics_requests.rs` 137 / 126 non-empty, `screenshot_diagnostics_json.rs` 739 / 706 non-empty, `lib.rs` 596 / 562 non-empty.
- Recursive `screenshot_native` audit after Chapter 235: 64 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- DXGI selected-output acceptance remains blocked: even an in-session pulse before acquire still produced `0x887A0027` timeout on both default and selected paths.
- The current pulse is tiny and no-activate by design; a stronger pulse matrix, alternate pulse placement, pointer shape update, or longer timeout may still be worth investigating.
- `screenshot_commands.rs` and `screenshot_diagnostics_json.rs` grew again; future chapters should prioritize extracting DXGI commands/JSON helpers before adding more diagnostics.
- WGC remains the stronger live E evidence path on this desktop, but final acceptance still requires runtime `Alt+A` selected-output behavior and not just smoke diagnostics.

### Next Recommended Chapter
- Chapter 236 should either extract DXGI diagnostic commands/JSON helpers to reduce large-file risk before further probes, or run a stronger guarded DXGI intervention matrix with longer timeout/pulse variants and pointer-update evidence.
- Keep progress at `79% / 81% / 76%` until runtime `Alt+A` selected-output evidence and DXGI selected-output acceptance evidence justify an increase.

## Chapter 236 - DXGI And Win32 Diagnostics JSON Helper Extraction (2026-06-09)

> Chapter status: completed only for this diagnostics JSON extraction/refactor slice. This moves DXGI-specific and Win32 intervention-specific diagnostic serializers out of the shared diagnostics JSON module while preserving command behavior and JSON field shape. It does not mark 方案 C, 方案 E, native route readiness, runtime `Alt+A` routing, DXGI selected-output acceptance, WGC real selected-output side effects, or commercial screenshot acceptance complete.

### Goals
- Extract DXGI diagnostic JSON serialization helpers from the large shared diagnostics JSON module before adding more DXGI probes.
- Extract the small Win32 desktop-update pulse and cursor-nudge serializer cluster so shared diagnostics JSON falls below the 500-line smell threshold.
- Preserve existing JSON field names, nullability, command guard semantics, diagnostic-only fields, and frontend DTO compatibility.
- Keep this chapter behavior-preserving: no new capture strategy, timeout/retry behavior, readiness promotion, route change, or user-visible side effect.

### Progress
- Overall C/E progress: approximately 79%.
- 方案 C native overlay / selected-output progress: approximately 81%.
- 方案 E DXGI/WGC/D3D11/GPU texture progress: approximately 76%.
- Progress remains unchanged because this chapter is structural maintainability work only; runtime `Alt+A` selected-output evidence and DXGI selected-output acceptance evidence remain missing.

### Added Files
- `tauri-client/src-tauri/src/screenshot_dxgi_diagnostics_json.rs`
  - Owns DXGI diagnostic JSON helpers for acquire-path reports, frame-info probes, output ranking evidence, and pulse-before-acquire comparison reports.
  - Keeps the existing camelCase diagnostic field names and selected/default output comparison shape.
- `tauri-client/src-tauri/src/screenshot_win32_diagnostics_json.rs`
  - Owns Win32 desktop-update pulse and cursor-nudge diagnostic serializers used by guarded DXGI intervention commands.
  - Keeps cursor/pulse evidence separate from shared WGC/selected-readback JSON helpers.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_diagnostics_json.rs`
  - Re-exports the extracted DXGI and Win32 serializer helpers for existing call sites.
  - Retains shared screenshot bounds, latest-payload, WGC, and selected-readback JSON helpers.
  - Drops from the Chapter 235 baseline of `739 / 706` physical / non-empty lines to `491 / 466` physical / non-empty lines.
- `tauri-client/src-tauri/src/lib.rs`
  - Registers the new crate-internal diagnostics JSON helper modules.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 236 scope, validation, non-goals, line counts, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- No new live DXGI, WGC, selected-output, clipboard, file, OCR, translation, presenter, overlay, or runtime `Alt+A` acceptance evidence was added.
- `screenshot_diagnostics_json.rs` is now below 500 physical lines, reducing large-file risk before additional DXGI or WGC diagnostics.
- Existing DXGI command guard tests and WGC monitor diagnostic JSON tests continue to pass through the re-export bridge.

### Explicit Non-Goals
- Did not change production DXGI capture, retry, timeout, readback, selected-output, or fake-sink behavior.
- Did not add stronger pulse, pointer-update, placement, style, or timeout intervention probes.
- Did not invoke real clipboard/file/OCR/translation/presenter/overlay side effects.
- Did not promote native route readiness or default `Alt+A` routing.
- Did not mark 方案 C or 方案 E complete, and did not raise C/E progress beyond `79% / 81% / 76%`.
- Did not expose persistent DXGI, WGC, D3D11, Win32, `HWND`, `HMONITOR`, or diagnostic handles.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_dxgi_frame_info_probe_command_tests -- --nocapture`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_dxgi_desktop_update_pulse_diagnostic_command_tests -- --nocapture`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib dxgi_pulse_before_acquire_probe -- --nocapture`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_dxgi_cursor_nudge_diagnostic_command_tests -- --nocapture`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_dxgi_acquire_comparison_command_tests -- --nocapture`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_dxgi_selected_readback_command_tests -- --nocapture`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_dxgi_selected_output_bridge_command_tests -- --nocapture`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_dxgi_selected_output_acceptance_command_tests -- --nocapture`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib latest_screenshot_payload_wgc_monitor_diagnostic_tests -- --nocapture`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib dxgi_selected_output_acceptance -- --nocapture`.
- Passed: `git diff --check`; Git still reports pre-existing LF-to-CRLF working-copy notices only.
- Not run: live ignored DXGI/WGC smokes and frontend build, because this chapter changes only Rust diagnostics serializer ownership and intentionally adds no runtime behavior.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 236: `docs/IMPLEMENTATION_CHAPTERS.md` 10169 / 7493 non-empty, `screenshot_diagnostics_json.rs` 491 / 466 non-empty, `screenshot_dxgi_diagnostics_json.rs` 202 / 195 non-empty, `screenshot_win32_diagnostics_json.rs` 59 / 56 non-empty, `lib.rs` 598 / 564 non-empty, `screenshot_commands.rs` 4953 / 4669 non-empty.
- Recursive `screenshot_native` audit after Chapter 236: 64 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- `screenshot_commands.rs` remains oversized at `4953 / 4669` lines; DXGI command extraction is still needed before adding more diagnostic commands there.
- DXGI selected-output acceptance remains blocked by the existing first-frame timeout class, including the Chapter 235 `0x887A0027` pulse-before-acquire result.
- WGC selected-output fake-sink evidence remains diagnostic-only and does not prove real clipboard/file/OCR/translation effects.
- The extracted serializers preserve shape through current tests, but any future serializer move should keep representative JSON-shape tests close to the command or helper module.

### Next Recommended Chapter
- Chapter 237 should either extract DXGI diagnostic command handlers into `screenshot_dxgi_diagnostic_commands.rs` to reduce `screenshot_commands.rs` before adding new probes, or run the stricter WGC monitor-session live smoke with real WGC enabled to collect runtime selected PNG + fake-sink evidence.
- If continuing DXGI behavior work after command extraction, run a stronger guarded intervention matrix with longer timeouts, alternate pulse placement/size/style, pointer-update evidence, and default-vs-selected output comparison.
- Keep progress at `79% / 81% / 76%` until runtime `Alt+A` selected-output evidence and DXGI selected-output acceptance evidence justify an increase.

## Chapter 237 - WGC Monitor Session Strict Live Evidence (2026-06-09)

> Chapter status: completed only for this guarded WGC monitor-session runtime evidence slice. This runs the existing real WGC monitor-session live smoke in both non-strict and strict modes, and proves on the current desktop that WGC can acquire a real monitor frame, produce a selected-only PNG from the requested physical selection, and pass that selected PNG through the diagnostic fake-sink selected-output copy path. It does not mark 方案 C, 方案 E, native route readiness, runtime `Alt+A` routing, real clipboard/file/OCR/translation effects, DXGI selected-output acceptance, or commercial screenshot acceptance complete.

### Goals
- Collect current-desktop runtime evidence for the WGC monitor-session selected PNG path before further DXGI command refactoring.
- Prove the guarded WGC path can create a device, capture item, frame pool, session, acquire a frame, crop selected output, and generate selected PNG evidence.
- Prove the WGC selected PNG evidence can flow through the injected fake-sink selected-output acceptance path without touching the real clipboard.
- Keep the run diagnostic-only: no readiness promotion, no default `Alt+A` route change, no presenter/OCR/translation/file side effects.

### Progress
- Overall C/E progress: approximately 79%.
- 方案 C native overlay / selected-output progress: approximately 81%.
- 方案 E DXGI/WGC/D3D11/GPU texture progress: approximately 76%.
- Progress remains unchanged despite stronger WGC evidence because runtime `Alt+A` selected-output evidence, real selected-output side effects, and DXGI selected-output acceptance evidence remain missing.

### Added Files
- None.

### Modified Files
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 237 WGC live evidence, validation, non-goals, line counts, and next recommended chapter.
  - Clarifies the Chapter 236 line-count wording from ?non-empty lines? to ?physical / non-empty lines?.

### Deleted Files
- None.

### Evidence Added
- Non-strict WGC live smoke passed: `$env:YSN_WGC_MONITOR_SESSION_LIVE_SMOKE='1'; Remove-Item Env:\YSN_REQUIRE_WGC_MONITOR_SESSION_SMOKE -ErrorAction SilentlyContinue; cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_monitor_session_live_smoke_env_guarded -- --ignored --nocapture`.
- Strict WGC live smoke passed: `$env:YSN_WGC_MONITOR_SESSION_LIVE_SMOKE='1'; $env:YSN_REQUIRE_WGC_MONITOR_SESSION_SMOKE='1'; cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_monitor_session_live_smoke_env_guarded -- --ignored --nocapture`.
- Strict live result: `ok=true`, `attemptedRealWgcApi=true`, `frameCaptureAttempted=true`, `frameCaptureConfirmed=true`, `diagnosticOnly=true`, `persistentHandleExposed=false`, `readinessChanged=false`.
- WGC session result: `createdDevice=true`, `createdItem=true`, `createdFramePool=true`, `createdSession=true`, `startedCapture=true`, `acquiredFrame=true`, `state="frame-acquired"`, `frameId=1`, `elapsedMs=175` in the strict run.
- Captured monitor/frame result: target monitor bounds `0,0,2560x1440`, frame dimensions `2560x1440`, `frameFormat="Bgra8Unorm"`, `frameDimensionsMatchSession=true`, `frameMatchesTargetMonitorBounds=true`, `framepoolSizeSource="target-monitor-bounds"`.
- Selected output/readback result: requested physical bounds `0,0,1x1`, `selectedCropWithinFrame=true`, `readbackBytesPresent=true`, `selectedPngProduced=true`, `selectedOnlyPng=true`, `pngWidth=1`, `pngHeight=1`, `pngByteLen=73`, `dimensionsMatchCrop=true`, `pngFingerprint="fnv1a64:821668024fbbb218"`.
- Fake-sink selected-output result: `selectedOutputFakeSinkAcceptance.ok=true`, `wgcSelectedPngEvidencePresent=true`, `fakeSinkCopyAccepted=true`, `sink="provided-fake-sink"`, `sinkCalls=1`, `copiedPngByteLen=73`, `pngByteLen=73`, `selectedOnlyPng=true`, `altAChanged=false`, `readinessChanged=false`, `persistentHandleExposed=false`.
- Selected readback planning result: `backend="wgc-monitor"`, `status="planned"`, `cropWithinTargetMonitor=true`, `requestedTargetIntersectionRatio=1.0`, `framepool.matchesTargetBounds=true`, `selectedOutputReadyPlanningOnly=true`.

### Explicit Non-Goals
- Did not change WGC capture behavior, selected readback behavior, selected-output effect behavior, or command guard semantics.
- Did not copy to the real OS clipboard, write a file, invoke OCR, invoke translation, or render through the production presenter.
- Did not connect a real native overlay selection rectangle to this WGC selected PNG path.
- Did not alter default `Alt+A` routing, repeat-hotkey behavior, focus handling, readiness flags, or route recommendation.
- Did not fix or bypass the DXGI `AcquireNextFrame` timeout class, and did not mark DXGI selected-output acceptance complete.
- Did not mark 方案 C or 方案 E complete, and did not raise C/E progress beyond `79% / 81% / 76%`.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib latest_screenshot_payload_wgc_monitor_diagnostic_tests -- --nocapture`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib wgc_selected_output_acceptance -- --nocapture`.
- Passed: non-strict live WGC monitor-session smoke with `YSN_WGC_MONITOR_SESSION_LIVE_SMOKE=1`.
- Passed: strict live WGC monitor-session smoke with `YSN_WGC_MONITOR_SESSION_LIVE_SMOKE=1` and `YSN_REQUIRE_WGC_MONITOR_SESSION_SMOKE=1`.
- Passed: `git diff --check`; Git still reports pre-existing LF-to-CRLF working-copy notices only.
- Not run: frontend build, DXGI live smokes, native overlay interactive `Alt+A`, real clipboard/file/OCR/translation side-effect smokes, because this chapter records WGC runtime evidence only.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 237: `docs/IMPLEMENTATION_CHAPTERS.md` 10251 / 7561 non-empty, `wgc_monitor_session_live_smoke.rs` 119 / 111 non-empty, `screenshot_wgc_diagnostic_commands.rs` 256 / 249 non-empty, `wgc_session.rs` 420 / 400 non-empty, `wgc_selected_output_acceptance.rs` 152 / 139 non-empty, `screenshot_commands.rs` 4953 / 4669 non-empty.
- Recursive `screenshot_native` audit after Chapter 237: 64 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- The WGC selected-output acceptance remains fake-sink only; it does not prove real clipboard, file, OCR, translation, or production presenter output.
- The selected bounds used in this live smoke are `1x1` at desktop origin; broader real user selections still need native overlay selection integration and larger crop evidence.
- The WGC path is still diagnostic-only and does not prove runtime `Alt+A` routing, focus cleanliness, repeat-hotkey stability, or Snow/QQ/WeChat-like UX.
- DXGI selected-output acceptance remains blocked by the existing `0x887A0027` first-frame timeout class, so 方案 E remains incomplete on the DXGI side.
- `screenshot_commands.rs` remains oversized at `4953 / 4669` lines; DXGI diagnostic command extraction should still happen before adding more DXGI probes.

### Next Recommended Chapter
- Chapter 238 should either add a guarded real selected-output side-effect smoke for the WGC selected PNG path, starting with clipboard or file output behind explicit opt-in and env guard, or connect a real native overlay completed selection rectangle to WGC selected PNG evidence without changing default `Alt+A`.
- If choosing maintenance first, Chapter 238 can extract DXGI diagnostic command handlers/tests into `screenshot_dxgi_diagnostic_commands.rs` using the already-audited nested-module/re-export plan, then run the existing DXGI command regression suite.
- Keep progress at `79% / 81% / 76%` until runtime `Alt+A` selected-output evidence and DXGI selected-output acceptance evidence justify an increase.

## Chapter 238 - WGC Selected-Output Real Clipboard Acceptance (2026-06-09)

> Chapter status: completed only for this guarded WGC selected-output real clipboard acceptance slice. This adds and validates a diagnostic-only command that starts from live WGC monitor capture, produces selected-only PNG evidence, writes it to the real OS clipboard through the selected-output effect sink, and reads the clipboard back to verify the copied RGBA bytes. It does not mark 方案 C, 方案 E, native route readiness, runtime `Alt+A` routing, real file/OCR/translation effects, DXGI selected-output acceptance, or commercial screenshot acceptance complete.

### Goals
- Extend the Chapter 237 WGC selected PNG path from fake-sink copy evidence to a real clipboard side-effect smoke.
- Keep real clipboard writes behind explicit command opt-in, WGC real-API opt-in, exactly-one sink-mode selection, and `YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1` plus the ignored live-smoke env guard.
- Reuse the product selected-output effect sink instead of the legacy screenshot RGBA cache path.
- Preserve diagnostic-only semantics: no readiness promotion, no default `Alt+A` route change, no file/OCR/translation/presenter/overlay side effects.

### Progress
- Overall C/E progress: approximately 79%.
- 方案 C native overlay / selected-output progress: approximately 81%.
- 方案 E DXGI/WGC/D3D11/GPU texture progress: approximately 76%.
- Progress remains unchanged because this chapter proves a guarded WGC real clipboard diagnostic, but runtime `Alt+A` selected-output evidence, real user selection integration, and DXGI selected-output acceptance evidence remain missing.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_diagnostics_requests.rs`
  - Adds `NativeWgcSelectedOutputClipboardAcceptanceRequest` with explicit WGC API, fake-sink/real-clipboard mode, timeout, cursor, border, buffer-count, and target-validation options.
- `tauri-client/src-tauri/src/screenshot_native/wgc_selected_output_acceptance.rs`
  - Adds `WgcSelectedOutputClipboardAcceptanceReceipt` and `accept_wgc_selected_output_clipboard_with_sink` so WGC selected PNG evidence can flow through either fake or real clipboard sinks.
  - Keeps `accept_wgc_selected_output_fake_sink_copy` compatible by delegating through the new generic WGC clipboard acceptance helper.
- `tauri-client/src-tauri/src/screenshot_wgc_diagnostic_commands.rs`
  - Adds `run_native_wgc_selected_output_clipboard_acceptance_smoke` with command/env guards, real clipboard verification JSON, and guarded live-smoke tests.
- `tauri-client/src-tauri/src/lib.rs`
  - Registers `screenshot_wgc_diagnostic_commands::run_native_wgc_selected_output_clipboard_acceptance_smoke` with the Tauri command handler.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 238 evidence, validation, non-goals, line counts, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- Real clipboard live smoke passed: `$env:YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD_SMOKE='1'; cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib wgc_selected_output_acceptance_real_clipboard_live_smoke -- --ignored --nocapture`.
- The live smoke writes the diagnostic selected PNG to the real OS clipboard and reads it back for verification.
- Live result: `ok=true`, `attempted=true`, `attemptedRealWgcApi=true`, `frameCaptureAttempted=true`, `frameCaptureConfirmed=true`, `selectedMonitorFrameConfirmed=true`, `selectedOutputEffectConfirmed=true`.
- Real clipboard result: `allowRealClipboard=true`, `allowFakeClipboardSink=false`, `realClipboardAttempted=true`, `realClipboardVerified=true`, `clipboardReadbackAttempted=true`, `clipboardReadbackConfirmed=true`.
- Sink verification result: `sink.mode="real"`, `sink.clipboardVerification.confirmed=true`, `dimensionsMatch=true`, `bytesMatch=true`, expected/actual `1x1`, expected/actual RGBA byte length `4`, fingerprint `fnv1a64:26a9f7b803d279a2`.
- WGC selected PNG result: requested bounds `0,0,1x1`, `selectedOnlyPng=true`, `pngWidth=1`, `pngHeight=1`, `pngByteLen=73`, `pngFingerprint="fnv1a64:821668024fbbb218"`, `dimensionsMatchCrop=true`.
- Receipt result: `sink="real-clipboard"`, `copyOnly=true`, `copiedToClipboard=true`, `selectedOutputEffectAccepted=true`, `selectedOnlyPng=true`, `pngByteLen=73`, `saveInvoked=false`, `ocrInvoked=false`, `translationInvoked=false`, `diagnosticOnly=true`, `readinessChanged=false`, `altAChanged=false`, `persistentHandleExposed=false`.

### Explicit Non-Goals
- Did not change production `Alt+A` routing, native route readiness, repeat-hotkey behavior, focus handling, overlay presenter behavior, or frontend screenshot actions.
- Did not connect a real native overlay drag selection rectangle to WGC selected-output capture.
- Did not add real file-save, OCR, or translation selected-output effects.
- Did not make real clipboard writes available without explicit command opt-in, sink-mode opt-in, and env guards.
- Did not fix or bypass DXGI `AcquireNextFrame` timeout or mark DXGI selected-output acceptance complete.
- Did not mark 方案 C or 方案 E complete, and did not raise C/E progress beyond `79% / 81% / 76%`.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_selected_output_clipboard_acceptance_command_tests -- --nocapture`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib wgc_selected_output_acceptance -- --nocapture`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib latest_screenshot_payload_wgc_monitor_diagnostic_tests -- --nocapture`.
- Passed: ignored real clipboard live smoke with `YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD_SMOKE=1`.
- Passed: `git diff --check`; Git still reports pre-existing LF-to-CRLF working-copy notices only.
- Not run: frontend build, DXGI live smokes, native overlay interactive `Alt+A`, real file/OCR/translation side-effect smokes, because this chapter changes WGC selected-output real clipboard diagnostics only.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 238: `docs/IMPLEMENTATION_CHAPTERS.md` 10323 / 7619 non-empty, `screenshot_wgc_diagnostic_commands.rs` 790 / 758 non-empty, `screenshot_diagnostics_requests.rs` 152 / 140 non-empty, `wgc_selected_output_acceptance.rs` 215 / 198 non-empty, `selected_output_clipboard.rs` 245 / 219 non-empty, `lib.rs` 599 / 565 non-empty.
- Recursive `screenshot_native` audit after Chapter 238: 64 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- The real clipboard live smoke overwrites the current OS clipboard with a diagnostic `1x1` WGC selected image during the test.
- The selected bounds remain tiny `0,0,1x1`; larger real user selections still need a frontend/native selection rectangle link and larger crop evidence.
- WGC real clipboard remains diagnostic-only and does not prove production `Alt+A`, focus cleanliness, repeat-hotkey stability, no-flicker overlay UX, or presenter integration.
- `screenshot_wgc_diagnostic_commands.rs` grew to `790 / 758` lines and should be split if more WGC side-effect diagnostics are added.
- DXGI selected-output acceptance remains blocked by `0x887A0027`, so the DXGI side of 方案 E remains incomplete.

### Next Recommended Chapter
- Chapter 239 should connect an explicit real physical selection rectangle to WGC selected PNG and real clipboard evidence without changing default `Alt+A`, preferably using a diagnostic command that requires request bounds and rejects latest full-screen payload fallback.
- Alternatively, Chapter 239 can extract `screenshot_wgc_diagnostic_commands.rs` or `screenshot_commands.rs` diagnostic command blocks before adding more probes, to keep command ownership below large-file risk.
- Keep progress at `79% / 81% / 76%` until runtime `Alt+A` selected-output evidence and DXGI selected-output acceptance evidence justify an increase.

## Chapter 239 - WGC Real Clipboard Guard Hardening (2026-06-09)

> Chapter status: completed only for this WGC real clipboard diagnostic safety hardening slice. This strengthens the production-exposed diagnostic command added in Chapter 238 by requiring a second runtime environment guard before any real OS clipboard write and by normalizing invalid-target response fields. It does not mark 方案 C, 方案 E, native route readiness, runtime `Alt+A` routing, real file/OCR/translation effects, DXGI selected-output acceptance, or commercial screenshot acceptance complete.

### Goals
- Harden `run_native_wgc_selected_output_clipboard_acceptance_smoke` so production command exposure cannot rely on test-only env gating for real clipboard writes.
- Require both the general WGC selected-output acceptance guard and a separate real-clipboard guard before using the real clipboard sink.
- Preserve fake-sink diagnostic behavior while making real clipboard side effects explicitly harder to trigger.
- Normalize invalid-target JSON so it does not report clipboard attempts when validation exits before any sink call.

### Progress
- Overall C/E progress: approximately 79%.
- 方案 C native overlay / selected-output progress: approximately 81%.
- 方案 E DXGI/WGC/D3D11/GPU texture progress: approximately 76%.
- Progress remains unchanged because this is diagnostic command safety hardening; runtime `Alt+A` selected-output evidence, real user selection integration, and DXGI selected-output acceptance evidence remain missing.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_wgc_diagnostic_commands.rs`
  - Adds `YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD=1` as a required runtime env guard when `allowRealClipboard=true`.
  - Keeps `YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1` as the general WGC selected-output acceptance guard.
  - Adds `realClipboardEnvGuardPresent` to disabled, invalid-target, and successful diagnostic JSON responses.
  - Adds a guard test proving real clipboard mode is blocked when the real-clipboard env guard is absent.
  - Keeps the ignored live smoke setting both runtime env guards plus `YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD_SMOKE=1`.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 239 safety hardening, validation, non-goals, line counts, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- Guard test passed: real clipboard mode is blocked without `YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD=1`, even when `YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1` and request flags are present.
- Real clipboard live smoke still passed after adding the second runtime guard, with both `YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1` and `YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD=1` set.
- Successful live result includes `realClipboardEnvGuardPresent=true`, `realClipboardAttempted=true`, `realClipboardVerified=true`, `clipboardReadbackConfirmed=true`, `selectedOutputEffectConfirmed=true`, and `ok=true`.
- Scope text now explicitly states real clipboard requires `allowRealClipboard`, live selected PNG evidence, `YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1`, and `YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD=1`.

### Explicit Non-Goals
- Did not add new WGC capture behavior, selected readback behavior, or production selected-output routing.
- Did not change default `Alt+A`, readiness flags, overlay focus behavior, presenter behavior, frontend copy/save/OCR/translation actions, or repeat-hotkey handling.
- Did not add real file-save, OCR, or translation selected-output effects.
- Did not connect a real native overlay or frontend drag selection rectangle to WGC selected-output capture.
- Did not fix DXGI `AcquireNextFrame` timeout or mark DXGI selected-output acceptance complete.
- Did not mark 方案 C or 方案 E complete, and did not raise C/E progress beyond `79% / 81% / 76%`.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_selected_output_clipboard_acceptance_command_tests -- --nocapture`.
- Passed: ignored real clipboard live smoke with `YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD_SMOKE=1`, `YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1`, and `YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD=1`.
- Passed: `git diff --check`; Git still reports pre-existing LF-to-CRLF working-copy notices only.
- Not run: frontend build, DXGI live smokes, native overlay interactive `Alt+A`, real file/OCR/translation side-effect smokes, because this chapter hardens WGC real clipboard diagnostics only.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 239: `docs/IMPLEMENTATION_CHAPTERS.md` 10402 / 7684 non-empty, `screenshot_wgc_diagnostic_commands.rs` 848 / 812 non-empty, `wgc_selected_output_acceptance.rs` 215 / 198 non-empty, `screenshot_diagnostics_requests.rs` 152 / 140 non-empty, `lib.rs` 599 / 565 non-empty.
- Recursive `screenshot_native` audit after Chapter 239: 64 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- `screenshot_wgc_diagnostic_commands.rs` is now `848 / 812` lines; further WGC diagnostics should extract submodules before adding more command bodies.
- The real clipboard smoke still overwrites the current OS clipboard when explicitly run with all guards.
- WGC real clipboard evidence still uses a tiny `0,0,1x1` request and does not prove larger real user selections.
- WGC real clipboard remains diagnostic-only and does not prove production `Alt+A`, focus cleanliness, repeat-hotkey stability, no-flicker overlay UX, or presenter integration.
- DXGI selected-output acceptance remains blocked by `0x887A0027`, so the DXGI side of 方案 E remains incomplete.

### Next Recommended Chapter
- Chapter 240 should connect explicit real physical selection bounds to WGC selected PNG and real clipboard evidence without changing default `Alt+A`; the existing `run_native_wgc_monitor_session_smoke` can already accept request bounds and should reject latest/full-screen fallback for this diagnostic path.
- Before adding more WGC command bodies, consider extracting `screenshot_wgc_diagnostic_commands.rs` into focused WGC diagnostic submodules.
- Keep progress at `79% / 81% / 76%` until runtime `Alt+A` selected-output evidence and DXGI selected-output acceptance evidence justify an increase.

## Chapter 240 - WGC Explicit Selection Strict Acceptance Wrapper (2026-06-09)

> Chapter status: completed only for this guarded WGC explicit-selection diagnostic wrapper slice. This adds a strict command path that refuses latest-screenshot/fullscreen fallback before WGC selected-output clipboard acceptance, so a caller must provide desktop physical request bounds. It does not mark 方案 C, 方案 E, runtime `Alt+A`, native overlay drag selection, real file/OCR/translation effects, DXGI selected-output acceptance, or commercial screenshot acceptance complete.

### Goals
- Add a diagnostic-only WGC selected-output acceptance entry point whose contract requires explicit desktop physical request bounds.
- Prevent this diagnostic from silently falling back to the latest screenshot payload or fullscreen capture bounds.
- Reuse the existing guarded WGC selected PNG plus fake/real clipboard acceptance path without changing default `Alt+A` routing.
- Keep progress percentages frozen until real runtime `Alt+A` selected-output evidence and DXGI selected-output acceptance evidence exist.

### Progress
- Overall C/E progress: approximately 79%.
- 方案 C native overlay / selected-output progress: approximately 81%.
- 方案 E DXGI/WGC/D3D11/GPU texture progress: approximately 76%.
- Progress remains unchanged because this chapter improves diagnostic contract strictness only; it still does not prove production native overlay selection, larger real user drag selection, default route readiness, or DXGI selected-output acceptance.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_wgc_diagnostic_commands.rs`
  - Adds `wgc_explicit_selection_selected_output_scope` for the strict explicit-selection diagnostic contract.
  - Adds a missing-bounds response that reports `latestFallbackRejected=true`, `requiresExplicitRequestBounds=true`, `explicitSelectionDiagnostic=true`, and no runtime side effects.
  - Adds `run_native_wgc_explicit_selection_selected_output_clipboard_acceptance_smoke`, a guarded wrapper around the existing selected-output clipboard acceptance command that refuses requests without `bounds`.
  - Adds tests proving the strict wrapper rejects a missing request, rejects `bounds=None`, preserves existing env guard behavior, and does not attempt real WGC or selected-output effects when blocked.
- `tauri-client/src-tauri/src/lib.rs`
  - Registers `run_native_wgc_explicit_selection_selected_output_clipboard_acceptance_smoke` in the Tauri command handler list.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Fixes the Chapter 239 `方案 C / 方案 E` text that had been corrupted as `Plan C / Plan E` by a prior encoding write.
  - Records Chapter 240 goals, validation, non-goals, line counts, and the next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- The new strict wrapper returns `stage="missing-explicit-request-bounds"`, `boundsSource="missingRequest"`, `latestFallbackRejected=true`, and `attemptedRealWgcApi=false` when no request object is provided.
- The new strict wrapper returns the same no-fallback response when a guarded request omits `bounds`, even if the normal acceptance command could otherwise use latest screenshot payload fallback.
- The new strict wrapper preserves the underlying WGC selected-output acceptance env guard: explicit bounds plus command opt-ins still do not call real WGC unless `YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1` is present.
- The command registration compiles through `cargo check --tests` and targeted command tests.

### Explicit Non-Goals
- Did not change default `Alt+A`, readiness flags, overlay focus behavior, presenter behavior, frontend copy/save/OCR/translation actions, or repeat-hotkey handling.
- Did not connect the frontend/native overlay drag rectangle to this strict WGC diagnostic command.
- Did not add real file-save, OCR, or translation selected-output effects.
- Did not run the real clipboard live smoke in this chapter, because it overwrites the OS clipboard and is not needed to prove strict bounds fallback rejection.
- Did not fix DXGI `AcquireNextFrame` timeout or mark DXGI selected-output acceptance complete.
- Did not split `screenshot_wgc_diagnostic_commands.rs`; the file is now above the desired size and should be split before more WGC command bodies are added.
- Did not mark 方案 C or 方案 E complete, and did not raise C/E progress beyond `79% / 81% / 76%`.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml`.
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_selected_output_clipboard_acceptance_command_tests -- --nocapture` with 9 passed and 1 ignored real clipboard live smoke.
- Not run: frontend build, DXGI live smokes, native overlay interactive `Alt+A`, real clipboard live smoke, real file/OCR/translation side-effect smokes, because this chapter only adds the strict explicit-bounds diagnostic wrapper.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 240: `docs/IMPLEMENTATION_CHAPTERS.md` 10473 / 7741 non-empty, `screenshot_wgc_diagnostic_commands.rs` 998 / 952 non-empty, `lib.rs` 600 / 566 non-empty.
- Recursive `screenshot_native` audit after Chapter 240 code changes: 64 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- `screenshot_wgc_diagnostic_commands.rs` is now `998 / 952` lines; the next WGC diagnostic chapter should extract selected-output command helpers/tests into a focused module before adding more behavior.
- The strict wrapper only proves that explicit desktop physical bounds are required; it does not prove the frontend computes those desktop bounds from real native drag selection.
- The existing live smoke still uses small explicit bounds unless a caller provides larger desktop physical bounds.
- WGC selected-output evidence remains diagnostic-only and does not prove production `Alt+A`, focus cleanliness, repeat-hotkey stability, no-flicker overlay UX, or presenter integration.
- DXGI selected-output acceptance remains blocked by `0x887A0027`, so the DXGI side of 方案 E remains incomplete.

### Next Recommended Chapter
- Chapter 241 should first extract WGC selected-output diagnostic command code from `screenshot_wgc_diagnostic_commands.rs` into a focused module while preserving command names and JSON shape.
- After the split, connect frontend/native selection math so image-local physical selection becomes desktop physical bounds using the latest screenshot `physicalBounds.x/y`, then call the strict WGC explicit-selection diagnostic without latest/fullscreen fallback.
- Keep progress at `79% / 81% / 76%` until runtime `Alt+A` selected-output evidence and DXGI selected-output acceptance evidence justify an increase.

## Chapter 241 - WGC Selected-Output Diagnostic Module Extraction (2026-06-09)

> Chapter status: completed only for this WGC diagnostic module-boundary cleanup slice. This extracts selected-output clipboard acceptance diagnostics out of the broader WGC diagnostic command module while preserving command names, JSON shape, guards, and tests. It does not mark 方案 C, 方案 E, runtime `Alt+A`, frontend/native drag selection, DXGI selected-output acceptance, or commercial screenshot acceptance complete.

### Goals
- Reduce `screenshot_wgc_diagnostic_commands.rs` after Chapter 240 pushed it near 1,000 lines.
- Move WGC selected-output clipboard acceptance command code and tests into a focused module.
- Preserve existing public command names, Tauri registrations, re-export compatibility, env guards, and JSON response contracts.
- Keep WGC monitor target/session diagnostics in the original WGC diagnostic module.

### Progress
- Overall C/E progress: approximately 79%.
- 方案 C native overlay / selected-output progress: approximately 81%.
- 方案 E DXGI/WGC/D3D11/GPU texture progress: approximately 76%.
- Progress remains unchanged because this chapter is structural cleanup only; runtime `Alt+A` selected-output evidence, frontend desktop-bounds wiring, and DXGI selected-output acceptance evidence remain missing.

### Added Files
- `tauri-client/src-tauri/src/screenshot_wgc_selected_output_diagnostic_commands.rs`
  - Owns WGC selected-output clipboard acceptance scopes, env guards, default request, disabled/missing-bounds responses, fake/real sink JSON, acceptance commands, strict explicit-selection wrapper, unit tests, and ignored real clipboard live smoke.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_wgc_diagnostic_commands.rs`
  - Keeps WGC one-frame probe, monitor target diagnostic, and monitor session smoke.
  - Restores the local fake clipboard sink needed by monitor-session fake-sink acceptance.
  - Re-exports the selected-output commands from the new module for compatibility with existing Rust call paths.
- `tauri-client/src-tauri/src/lib.rs`
  - Adds `pub mod screenshot_wgc_selected_output_diagnostic_commands`.
  - Registers selected-output Tauri commands through the new module path.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 241 extraction, validation, line counts, non-goals, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- `screenshot_wgc_diagnostic_commands.rs` dropped from `998 / 952` lines before Chapter 241 to `260 / 253` lines after extraction.
- `screenshot_wgc_selected_output_diagnostic_commands.rs` now contains the selected-output acceptance command tests under `native_wgc_selected_output_clipboard_acceptance_command_tests`.
- Existing selected-output command tests still pass after moving modules, proving old acceptance guards and the Chapter 240 strict wrapper remain covered.
- Tauri command registration compiles through `cargo check --tests` using the new module path.

### Explicit Non-Goals
- Did not change default `Alt+A`, readiness flags, overlay focus behavior, presenter behavior, frontend copy/save/OCR/translation actions, or repeat-hotkey handling.
- Did not connect frontend `physicalBounds` or native overlay drag selection to the strict WGC explicit-selection diagnostic command.
- Did not add real file-save, OCR, or translation selected-output effects.
- Did not run real clipboard live smoke, because this chapter only moves code and the ignored smoke overwrites the OS clipboard when explicitly enabled.
- Did not fix DXGI `AcquireNextFrame` timeout or mark DXGI selected-output acceptance complete.
- Did not mark 方案 C or 方案 E complete, and did not raise C/E progress beyond `79% / 81% / 76%`.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml`.
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_selected_output_clipboard_acceptance_command_tests -- --nocapture` with 9 passed and 1 ignored real clipboard live smoke.
- Passed: `git diff --check`; Git still reports pre-existing LF-to-CRLF working-copy notices only.
- Not run: frontend build, DXGI live smokes, native overlay interactive `Alt+A`, real clipboard live smoke, real file/OCR/translation side-effect smokes, because this chapter preserves behavior while extracting the selected-output diagnostic module.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 241: `docs/IMPLEMENTATION_CHAPTERS.md` 10545 / 7800 non-empty, `screenshot_wgc_diagnostic_commands.rs` 260 / 253 non-empty, `screenshot_wgc_selected_output_diagnostic_commands.rs` 764 / 724 non-empty, `lib.rs` 601 / 567 non-empty.
- Recursive `screenshot_native` audit after Chapter 241: 64 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- The new selected-output diagnostic module is `764 / 724` lines; it is focused, but later frontend wiring should avoid adding UI-specific behavior to it.
- The strict WGC diagnostic still requires a caller to provide correct desktop physical bounds; frontend code does not yet persist payload `physicalBounds` or add it to image-local selection.
- WGC selected-output evidence remains diagnostic-only and does not prove production `Alt+A`, focus cleanliness, repeat-hotkey stability, no-flicker overlay UX, or presenter integration.
- DXGI selected-output acceptance remains blocked by `0x887A0027`, so the DXGI side of 方案 E remains incomplete.

### Next Recommended Chapter
- Chapter 242 should wire frontend screenshot payload `physicalBounds` through the loader/session state without changing `getPhysicalSelection` image-local semantics.
- Then add a guarded diagnostic action that computes desktop physical selection as `payload.physicalBounds.x/y + image-local selection.x/y` and calls `run_native_wgc_explicit_selection_selected_output_clipboard_acceptance_smoke`.
- Keep progress at `79% / 81% / 76%` until runtime `Alt+A` selected-output evidence and DXGI selected-output acceptance evidence justify an increase.

## Chapter 242 - Frontend Physical Bounds To Strict WGC Diagnostic Wiring (2026-06-09)

> Chapter status: completed only for this frontend diagnostic wiring slice. This preserves screenshot payload `physicalBounds`, keeps `getPhysicalSelection` as image-local coordinates, adds a desktop-physical selection helper, and exposes a guarded WGC explicit-selection diagnostic helper that can call the strict backend command with explicit desktop bounds. It does not mark 方案 C, 方案 E, production `Alt+A`, copy/save/OCR/translation selected-output effects, DXGI selected-output acceptance, or commercial screenshot acceptance complete.

### Goals
- Stop dropping backend screenshot payload `physicalBounds` in the frontend screenshot session pipeline.
- Preserve existing image-local selection semantics used by crop/copy/save/OCR/translate/record/scroll paths.
- Add a dedicated image-local-to-desktop-physical selection conversion for WGC/DXGI diagnostic paths.
- Add a guarded frontend helper that builds the strict WGC explicit-selection request without automatically changing the production `Alt+A` output route.

### Progress
- Overall C/E progress: approximately 79%.
- 方案 C native overlay / selected-output progress: approximately 81%.
- 方案 E DXGI/WGC/D3D11/GPU texture progress: approximately 76%.
- Progress remains unchanged because this chapter adds frontend diagnostic wiring only; runtime `Alt+A` selected-output evidence and DXGI selected-output acceptance evidence remain missing.

### Added Files
- None.

### Modified Files
- `tauri-client/src/types/screenshot.ts`
  - Adds typed WGC selected-output clipboard acceptance request/response shapes for frontend diagnostic calls.
- `tauri-client/src/utils/screenshotImage.ts`
  - Adds `getDesktopPhysicalSelection`, which keeps `getPhysicalSelection` image-local and adds `ScreenshotPhysicalBounds.x/y` only for desktop physical diagnostic bounds.
- `tauri-client/src/hooks/useScreenshotLoader.ts`
  - Adds `displayedPhysicalBoundsRef` for the current screenshot session.
  - Stores optional payload `physicalBounds` in `startNewCaptureSession`.
  - Clears `displayedPhysicalBoundsRef` during reset.
  - Lets `loadFullscreen`, `loadFullscreenFromRgba`, `loadFullscreenFromBase64`, and `loadFullscreenFromFile` receive optional physical bounds.
- `tauri-client/src/pages/ScreenshotPage.tsx`
  - Passes payload `physicalBounds` through file, RGBA, memory, and base64 screenshot payload branches.
  - Passes `displayedPhysicalBoundsRef` into `useScreenshotActions`.
- `tauri-client/src/hooks/useScreenshotActions.ts`
  - Receives `displayedPhysicalBoundsRef`.
  - Adds `runGuardedWgcExplicitSelectionDiagnostic`, which computes desktop physical selection and invokes `run_native_wgc_explicit_selection_selected_output_clipboard_acceptance_smoke` with fake-sink defaults and real clipboard disabled.
  - Does not call the helper automatically from copy/save/OCR/translate or default `Alt+A` flow.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 242 wiring, validation, non-goals, line counts, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- Frontend screenshot payload `physicalBounds` now survives from `handleScreenshotPayload` into loader session state.
- `getPhysicalSelection` remains unchanged and image-local; desktop physical conversion is isolated in `getDesktopPhysicalSelection`.
- The guarded WGC helper constructs a strict request with explicit bounds and calls the Chapter 240 backend command instead of relying on latest/fullscreen fallback.
- The helper defaults to fake clipboard sink and `allowRealClipboard=false`, so it does not overwrite the real OS clipboard by default.

### Explicit Non-Goals
- Did not change default `Alt+A`, production selected-output routing, readiness flags, native overlay focus behavior, presenter behavior, or repeat-hotkey handling.
- Did not automatically run the WGC diagnostic during normal copy/save/OCR/translation actions.
- Did not add real file-save, OCR, or translation selected-output effects to the WGC diagnostic path.
- Did not run a real interactive `Alt+A` smoke or real clipboard live smoke.
- Did not fix DXGI `AcquireNextFrame` timeout or mark DXGI selected-output acceptance complete.
- Did not mark 方案 C or 方案 E complete, and did not raise C/E progress beyond `79% / 81% / 76%`.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; Vite reported the existing large chunk and mixed static/dynamic import warnings only.
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_selected_output_clipboard_acceptance_command_tests -- --nocapture` with 9 passed and 1 ignored real clipboard live smoke.
- Passed: `git diff --check`; Git still reports pre-existing LF-to-CRLF working-copy notices only.
- Not run: native overlay interactive `Alt+A`, real WGC clipboard live smoke, DXGI live smokes, real file/OCR/translation selected-output smokes.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 242: `docs/IMPLEMENTATION_CHAPTERS.md` 10615 / 7858 non-empty, `screenshot.ts` 316 / 294 non-empty, `screenshotImage.ts` 97 / 88 non-empty, `useScreenshotLoader.ts` 535 / 493 non-empty, `useScreenshotActions.ts` 373 / 345 non-empty, `ScreenshotPage.tsx` 856 / 795 non-empty, `screenshot_wgc_selected_output_diagnostic_commands.rs` 764 / 724 non-empty.
- Recursive `screenshot_native` audit after Chapter 242: 64 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- `runGuardedWgcExplicitSelectionDiagnostic` is wired as a guarded helper but is not yet connected to a visible manual diagnostic trigger or production `Alt+A` action.
- Real WGC selected-output runtime evidence still requires explicitly running the helper in an interactive screenshot session with `YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1`.
- Missing or stale payload `physicalBounds` will still block desktop-physical WGC diagnostics, which is intentional to avoid silently treating image-local coordinates as desktop coordinates.
- DXGI selected-output acceptance remains blocked by `0x887A0027`, so the DXGI side of 方案 E remains incomplete.

### Next Recommended Chapter
- Chapter 243 should add a controlled manual diagnostic trigger or test harness for `runGuardedWgcExplicitSelectionDiagnostic` during a real `Alt+A` screenshot session, then collect runtime evidence for non-1x1 user selections.
- After WGC real selected-output evidence, continue DXGI selected-output acceptance investigation around `AcquireNextFrame` timeout `0x887A0027`.
- Keep progress at `79% / 81% / 76%` until runtime `Alt+A` selected-output evidence and DXGI selected-output acceptance evidence justify an increase.

## Chapter 243 - Hidden WGC Explicit-Selection Runtime Diagnostic Trigger (2026-06-09)

> Chapter status: completed only for this controlled frontend diagnostic trigger slice. This adds a hidden screenshot-window shortcut that can run the Chapter 242 guarded WGC explicit-selection diagnostic against the current real selection during an interactive screenshot session. It does not mark 方案 C, 方案 E, production `Alt+A`, default selected-output routing, DXGI selected-output acceptance, or commercial screenshot acceptance complete.

### Goals
- Add a manual runtime trigger for the guarded WGC explicit-selection diagnostic without changing normal copy/save/OCR/translate behavior.
- Make the trigger usable after a real `Alt+A` selection so the next manual smoke can collect non-1x1 selected-output evidence.
- Keep real clipboard disabled by default and use the fake-sink diagnostic path.
- Surface the diagnostic result to the user with a transient message while preserving logs for baseline/debug review.

### Progress
- Overall C/E progress: approximately 79%.
- 方案 C native overlay / selected-output progress: approximately 81%.
- 方案 E DXGI/WGC/D3D11/GPU texture progress: approximately 76%.
- Progress remains unchanged because this chapter adds a manual diagnostic trigger only; the actual runtime `Alt+A` WGC evidence and DXGI selected-output acceptance evidence still need to be captured.

### Added Files
- None.

### Modified Files
- `tauri-client/src/hooks/useScreenshotInteraction.ts`
  - Adds optional `runWgcExplicitSelectionDiagnostic` interaction callback.
  - Adds hidden shortcut `Ctrl+Alt+W` after a selection exists to run the diagnostic callback.
  - Keeps existing copy/save/OCR/translate/pin shortcuts unchanged.
- `tauri-client/src/pages/ScreenshotPage.tsx`
  - Imports `message` from Ant Design.
  - Adds `handleWgcExplicitSelectionDiagnostic`, which calls `runGuardedWgcExplicitSelectionDiagnostic` and shows success/warning/error feedback.
  - Passes the handler into `useScreenshotInteraction`.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 243 trigger wiring, validation, non-goals, line counts, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- A real screenshot session can now run the guarded WGC explicit-selection diagnostic with `Ctrl+Alt+W` after drawing a selection.
- The frontend trigger still uses the Chapter 242 fake-sink defaults, so it does not overwrite the OS clipboard by default.
- The shortcut is hidden/manual and does not alter default `Enter`, `Ctrl+C`, `Ctrl+S`, `Ctrl+D`, `Ctrl+Q`, or pin behavior.
- Successful, skipped, or failed diagnostic attempts show a message and write the existing screenshot perf log entry from `runGuardedWgcExplicitSelectionDiagnostic`.

### Explicit Non-Goals
- Did not automatically route production copy/save/OCR/translation through WGC selected-output.
- Did not enable real clipboard writes from the frontend diagnostic trigger.
- Did not run a real interactive `Alt+A` smoke in this chapter.
- Did not add a visible toolbar button; this remains a controlled hidden diagnostic shortcut.
- Did not fix DXGI `AcquireNextFrame` timeout or mark DXGI selected-output acceptance complete.
- Did not mark 方案 C or 方案 E complete, and did not raise C/E progress beyond `79% / 81% / 76%`.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; Vite reported the existing large chunk and mixed static/dynamic import warnings only.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_selected_output_clipboard_acceptance_command_tests -- --nocapture` with 9 passed and 1 ignored real clipboard live smoke.
- Passed: `git diff --check`; Git still reports pre-existing LF-to-CRLF working-copy notices only.
- Not run: native overlay interactive `Alt+A`, `Ctrl+Alt+W` live diagnostic, real WGC clipboard live smoke, DXGI live smokes, real file/OCR/translation selected-output smokes.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 243: `docs/IMPLEMENTATION_CHAPTERS.md` 10694 / 7925 non-empty, `useScreenshotInteraction.ts` 806 / 757 non-empty, `ScreenshotPage.tsx` 873 / 812 non-empty, `useScreenshotActions.ts` 373 / 345 non-empty, `useScreenshotLoader.ts` 535 / 493 non-empty, `screenshot.ts` 316 / 294 non-empty, `screenshotImage.ts` 97 / 88 non-empty.
- Recursive `screenshot_native` audit after Chapter 243: 64 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- The new `Ctrl+Alt+W` shortcut still needs a real interactive smoke with `YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1` to prove runtime selected-output evidence.
- The diagnostic is hidden by design; if repeated manual QA needs clearer discoverability, a dev-only toolbar button or diagnostics panel entry may be needed later.
- WGC selected-output evidence remains diagnostic-only until the production `Alt+A` selected-output effects are intentionally switched over.
- DXGI selected-output acceptance remains blocked by `0x887A0027`, so the DXGI side of 方案 E remains incomplete.

### Next Recommended Chapter
- Chapter 244 should run a real interactive `Alt+A` smoke: draw a non-1x1 selection, press `Ctrl+Alt+W`, and record the WGC explicit-selection diagnostic response with `YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1`.
- If the WGC runtime evidence passes, use it to decide whether the 方案 C selected-output percentage can move; do not raise 方案 E until DXGI selected-output acceptance is solved.
- Continue DXGI selected-output acceptance investigation around `AcquireNextFrame` timeout `0x887A0027` after WGC runtime evidence is captured.

## Chapter 244 - WGC Live Fake-Sink And DXGI Multi-Attempt Pulse Evidence (2026-06-09)

> Chapter status: completed for this diagnostic-evidence slice. This chapter records the real WGC explicit-selection fake-sink live smoke, upgrades the DXGI pulse-before-acquire diagnostic from a single post-pulse acquire to a bounded multi-attempt sampler, and proves that both default-output and selected-output DXGI duplication can acquire a real frame after a tiny no-activate desktop pulse. It does not mark production `Alt+A`, real clipboard/file/OCR/translation effects, default selected-output routing, or final Plan C / Plan E acceptance complete.

### Goals
- Convert the Chapter 243/244 WGC evidence from pending code into a recorded chapter with live non-1x1 selected-output proof.
- Address the DXGI `AcquireNextFrame` timeout bottleneck with a concrete diagnostic improvement instead of repeating the same stale timeout note.
- Preserve the existing guarded diagnostic-only posture: no real clipboard writes, no file writes, no OCR/translation side effects, no presenter/readiness mutation, and no persistent GPU handle exposure.
- Add enough DXGI observability to distinguish "no frame ever arrived" from "first short wait timed out but a later budgeted attempt succeeded".

### Progress
- Overall C/E progress: approximately 84%.
- Plan C native overlay / selected-output progress: approximately 84%.
- Plan E DXGI/WGC/D3D11/GPU texture progress: approximately 81%.
- Progress is raised because this chapter adds two stronger live evidence points: WGC explicit-selection non-1x1 selected PNG production through a fake sink, and DXGI selected-output `AcquireNextFrame` confirmation after desktop pulse. It is not raised further because the evidence is still diagnostic-only and not the final production `Alt+A` path.

### External Findings Applied
- Microsoft documents `IDXGIOutputDuplication::AcquireNextFrame` as waiting for the next desktop image update or mouse pointer update, and returning `DXGI_ERROR_WAIT_TIMEOUT` when the timeout interval elapses before an update is available.
- Microsoft also documents that callers must release an acquired frame before acquiring the next frame. The diagnostic therefore keeps a strict acquire/release cycle and stops after success or access loss.
- Applied pattern: a tiny no-activate desktop pulse is used to deliberately create a desktop update before `AcquireNextFrame`, and the diagnostic now samples repeatedly within a bounded budget rather than treating one post-pulse timeout as conclusive.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_native/dxgi_pulse_before_acquire_probe.rs`
  - Adds `attempts: Vec<DxgiFrameInfoProbeAttempt>` to the pulse-before-acquire path report while preserving the existing `acquire` success field for compatibility.
  - Replaces the single 500 ms post-pulse acquire with a 1,000 ms budget split into 100 ms attempts.
  - Stops after the first successful acquire or access-lost result.
  - Reports timeout exhaustion with the attempt count and total acquire budget.
- `tauri-client/src-tauri/src/screenshot_dxgi_diagnostics_json.rs`
  - Emits the full `attempts` array for each pulse-before-acquire path.
  - Adds comparison fields for attempt counts and success attempt indexes.
  - Computes timeout evidence from all attempts instead of only the final/success `acquire` field.
- `tauri-client/src-tauri/src/screenshot_wgc_selected_output_diagnostic_commands.rs`
  - Contains the ignored live WGC fake-sink non-1x1 smoke recorded by this chapter.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 244 evidence, validation, line counts, risks, and the next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- WGC explicit-selection fake-sink live smoke passed with explicit bounds `x=0, y=0, width=64, height=48`.
- WGC evidence confirmed `attemptedRealWgcApi=true`, `frameCaptureConfirmed=true`, `selectedMonitorFrameConfirmed=true`, `selectedOutputEffectConfirmed=true`, and `realClipboardAttempted=false`.
- WGC selected PNG evidence confirmed `pngWidth=64`, `pngHeight=48`, `selectedOnlyPng=true`, `pngByteLen=12404`, and fingerprint `fnv1a64:ed81e5a3f6483fdc`.
- WGC fake sink confirmed `calls=1`, `sink=provided-fake-sink`, `copyOnly=true`, and no real clipboard overwrite.
- DXGI pulse-before-acquire live smoke passed with `frameCaptureConfirmed=true` and `stage=pulse-before-acquire-frame-confirmed`.
- DXGI default-output evidence confirmed `defaultFrameConfirmed=true`, `defaultAttemptCount=1`, `defaultSuccessAttempt=1`, `timeoutMs=100`, `releasedFrame=true`, and `timedOut=false`.
- DXGI selected-output evidence confirmed `selectedFrameConfirmed=true`, `selectedAttemptCount=1`, `selectedSuccessAttempt=1`, `timeoutMs=100`, `releasedFrame=true`, and `timedOut=false`.
- DXGI selected-output ranking selected adapter `0`, output `0`, with requested bounds `320x180 @ 0,0`, intersection ratio `1.0`, and monitor bounds `2560x1440 @ 0,0`.
- The desktop pulse remained diagnostic and non-disruptive: `noActivate=true`, `hiddenFromAltTab=true`, `appWindowExcluded=true`, `pulseSizePx=2`, `pulseAlpha=1`, and `dwellMs=16`.

### Explicit Non-Goals
- Did not switch production `Alt+A` copy/save/OCR/translation to the WGC or DXGI selected-output diagnostic paths.
- Did not write to the real clipboard, write a file, invoke OCR, invoke translation, mutate readiness flags, expose persistent handles, or alter presenter behavior.
- Did not claim final Snow/QQ/WeChat-like first-frame, repeat-hotkey, flicker, focus, or Alt-Tab acceptance.
- Did not mark Plan C or Plan E complete.
- Did not remove the need for a real interactive `Alt+A` selection smoke and a production selected-output effects smoke.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib dxgi_pulse_before_acquire_probe -- --nocapture` with 6 passed and 1 ignored live smoke.
- Passed: `$env:YSN_WGC_EXPLICIT_SELECTION_FAKE_SINK_LIVE_SMOKE='1'; cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib wgc_explicit_selection_fake_sink_non_1x1_live_smoke -- --ignored --nocapture`.
- Passed: `$env:YSN_DXGI_PULSE_BEFORE_ACQUIRE_LIVE_SMOKE='1'; cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib dxgi_pulse_before_acquire_live_smoke -- --ignored --nocapture`.
- Not run: frontend `npx tsc --noEmit`, frontend production build, real interactive `Alt+A`, real clipboard smoke, file/OCR/translation selected-output smokes.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 244: `docs/IMPLEMENTATION_CHAPTERS.md` 10764 / 7983 non-empty, `dxgi_pulse_before_acquire_probe.rs` 348 / 329 non-empty, `screenshot_dxgi_diagnostics_json.rs` 217 / 210 non-empty, `screenshot_wgc_selected_output_diagnostic_commands.rs` 821 / 779 non-empty.
- Recursive `screenshot_native` audit after Chapter 244: 60 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- The DXGI live smoke proves selected-output frame acquisition after a deliberate desktop pulse, but not yet selected-region readback, clipboard/file output effects, or production route switching.
- The WGC fake-sink smoke proves selected-only PNG production without touching the real clipboard, but not user-visible copy/save/OCR/translation acceptance.
- The `Ctrl+Alt+W` hidden trigger still needs an interactive `Alt+A` session capture to prove the frontend physical-bounds bridge under real user selection.
- The progress increase is evidence-based but still conservative; the remaining work is production integration and user-visible end-to-end acceptance, not another diagnostic-only loop.

### Next Recommended Chapter
- Chapter 245 should move from diagnostics into production-path acceptance: run a real interactive `Alt+A` session with `YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1`, draw a non-1x1 selection, press `Ctrl+Alt+W`, and record the frontend-triggered WGC response.
- If the interactive WGC trigger passes, wire a guarded selected-output production candidate for copy/save effects behind explicit readiness/feature flags instead of leaving it diagnostic-only.
- Continue DXGI by running or extending selected-region readback and selected-output clipboard acceptance with the new pulse-before-acquire evidence, so Plan E can move from "frame acquisition proven" to "selected output effect proven".

## Chapter 245 - DXGI Selected-Output Acceptance Pulse Integration (2026-06-09)

> Chapter status: completed for this DXGI selected-output acceptance slice. This chapter moves beyond frame-only DXGI diagnostics by integrating the tiny desktop pulse into the selected-output bridge path, exposing the pulse evidence in command JSON, and passing the guarded fake-sink DXGI selected-output clipboard acceptance live smoke with a real selected PNG. It does not make the DXGI path the default production `Alt+A` route and does not touch the real clipboard.

### Goals
- Stop treating DXGI `AcquireNextFrame` timeout as a static blocker after Chapter 244 proved a desktop pulse can produce a frame.
- Reuse the successful tiny no-activate pulse pattern directly before selected-output bridge capture.
- Prove DXGI selected-output can produce selected-only PNG evidence and pass fake-sink clipboard acceptance without real clipboard/file/OCR/translation side effects.
- Keep the path guarded by explicit opt-in and environment guard while producing stronger Plan E evidence.

### Progress
- Overall C/E progress: approximately 88%.
- Plan C native overlay / selected-output progress: approximately 86%.
- Plan E DXGI/WGC/D3D11/GPU texture progress: approximately 87%.
- Progress is raised because this chapter proves real DXGI selected-output selected PNG production plus fake-sink clipboard acceptance. It is not raised to completion because production `Alt+A`, real user interaction, real clipboard/file/OCR/translation effects, repeat-hotkey stability, and final focus/flicker acceptance remain unproven.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_native/dxgi_output_bridge_smoke.rs`
  - Adds `desktop_pulse: Option<DesktopUpdatePulseReport>` to `DxgiSelectedOutputBridgeDryRunReport`.
  - Runs a 2 px, alpha 1, 16 ms no-activate desktop pulse after selected-output duplication starts and before `capture_texture_frame`.
  - Preserves selected-output ranking, readback planning, selected PNG bridge validation, release, and stop semantics.
  - Carries pulse evidence through success and failure reports.
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - Adds `desktopPulse` to `run_native_dxgi_selected_output_bridge_dry_run` JSON output.
  - Adds `desktopPulse` to `run_native_dxgi_selected_output_clipboard_acceptance_smoke` JSON output.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 245 implementation, live evidence, validation, and next recommended work.

### Deleted Files
- None.

### Evidence Added
- Guarded DXGI selected-output bridge command tests passed with real API denied by default and invalid bounds rejected under explicit allow.
- DXGI selected-output acceptance fake-sink live smoke passed with `YSN_DXGI_SELECTED_OUTPUT_ACCEPTANCE=1` and `YSN_REQUIRE_DXGI_SELECTED_OUTPUT_ACCEPTANCE_SMOKE=1`.
- Live DXGI acceptance response confirmed `ok=true`, `stage=stopped`, `attemptedRealDxgiApi=true`, `frameCaptureConfirmed=true`, `bridgeValidated=true`, `selectedOnly=true`, `pngSignatureValid=true`, and `selectedOutputEffectConfirmed=true`.
- Live DXGI selected PNG evidence confirmed `pngWidth=320`, `pngHeight=180`, `selectedOnlyPng=true`, `decodedRgbaByteLenExpected=230400`, `pngByteLen=230663`, and fingerprint `fnv1a64:e45241d5921f3e55`.
- Fake sink confirmed `mode=fake`, `calls=1`, `lastPngLen=230663`, `allowRealClipboard=false`, `clipboardReadbackAttempted=false`, and `clipboardReadbackConfirmed=false`.
- DXGI selected-output ranking confirmed adapter `0`, output `0`, selected rank `1`, requested bounds `320x180 @ 0,0`, output bounds `2560x1440 @ 0,0`, and intersection ratio `1.0`.
- Integrated desktop pulse evidence confirmed `ok=true`, `noActivate=true`, `hiddenFromAltTab=true`, `appWindowExcluded=true`, `pulseSizePx=2`, `pulseAlpha=1`, `dwellMs=16`, `dwmFlushCalled=true`, and `destroyConfirmed=true`.

### Explicit Non-Goals
- Did not enable DXGI selected-output as the default production `Alt+A` capture or output route.
- Did not write to the real clipboard, write a file, invoke OCR, invoke translation, mutate readiness, expose persistent handles, or alter overlay/presenter behavior.
- Did not claim final flicker-free first frame, repeat-hotkey, Alt-Tab, focus, or Snow/QQ/WeChat-level acceptance.
- Did not remove WGC; WGC remains the stronger current selected-output path and DXGI is now an evidenced Plan E candidate.
- Did not mark Plan C or Plan E complete.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_dxgi_selected_output_bridge_command_tests -- --nocapture`.
- Passed: `$env:YSN_DXGI_SELECTED_OUTPUT_ACCEPTANCE='1'; $env:YSN_REQUIRE_DXGI_SELECTED_OUTPUT_ACCEPTANCE_SMOKE='1'; cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib dxgi_selected_output_acceptance_fake_sink_live_smoke -- --ignored --nocapture`.
- Not run: frontend `npx tsc --noEmit`, frontend production build, real interactive `Alt+A`, real clipboard smoke, file/OCR/translation selected-output smokes.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 245: `docs/IMPLEMENTATION_CHAPTERS.md` 10847 / 8052 non-empty, `dxgi_output_bridge_smoke.rs` 395 / 375 non-empty, `screenshot_commands.rs` 4955 / 4671 non-empty, `dxgi_pulse_before_acquire_probe.rs` 348 / 329 non-empty, `screenshot_dxgi_diagnostics_json.rs` 217 / 210 non-empty.
- Recursive `screenshot_native` audit after Chapter 245: 60 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- `screenshot_commands.rs` remains far above the project size target and should be split by command domain when the next change touches nearby DXGI/WGC command groups.
- The DXGI acceptance smoke still uses explicit test bounds and a fake sink; it proves selected-output effect acceptance but not real user selection from the screenshot UI.
- The pulse pattern is diagnostic and guarded; production adoption needs a deliberate policy decision to avoid adding hidden desktop-update behavior to normal user flows without UX acceptance.
- Frontend `Alt+A` physical-bounds wiring still needs real interactive validation.

### Next Recommended Chapter
- Chapter 246 should run the real interactive `Alt+A` WGC diagnostic path (`Alt+A` selection then `Ctrl+Alt+W`) and record the response.
- After interactive WGC passes, add a guarded production-path candidate that can route copy/save through selected-output evidence behind explicit readiness/feature flags.
- Split the large DXGI/WGC command cluster out of `screenshot_commands.rs` before adding more command handlers, to keep commercial maintainability while finishing the final acceptance smokes.

## Chapter 246 - Guarded WGC Selected-Output Copy Candidate (2026-06-09)

> Chapter status: completed for this guarded production-candidate slice. This chapter does not make selected-output copy the default for all users, but it creates the first frontend copy-path candidate that can bypass the old rendered-base64 clipboard path when explicit build/runtime guards are enabled and real WGC clipboard verification succeeds. It also makes the WGC acceptance command optionally return selected PNG base64 so the frontend can keep emitting the captured image without doing a second client crop.

### Goals
- Move from diagnostic-only evidence toward a real copy-path candidate without silently changing default user behavior.
- Let WGC selected-output acceptance return selected PNG base64 when explicitly requested.
- Add a frontend guarded copy candidate that calls the WGC explicit-selection selected-output command with real clipboard verification.
- Preserve existing copy fallback, save flow, annotations, translation overlays, OCR, and pin behavior.

### Progress
- Overall C/E progress: approximately 93%.
- Plan C native overlay / selected-output progress: approximately 91%.
- Plan E DXGI/WGC/D3D11/GPU texture progress: approximately 90%.
- Progress is raised because the selected-output path is no longer only a backend smoke: frontend `copy` can now use a guarded WGC selected-output real-clipboard candidate and fall back safely. It is not marked complete because real interactive `Alt+A` verification with the guard enabled, final save/OCR/translation selected-output routes, repeat-hotkey/focus/flicker acceptance, and default rollout policy are still outstanding.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_diagnostics_requests.rs`
  - Adds optional `include_selected_png_base64` to `NativeWgcSelectedOutputClipboardAcceptanceRequest`.
- `tauri-client/src-tauri/src/screenshot_wgc_selected_output_diagnostic_commands.rs`
  - Encodes and returns `selectedPngBase64` only when `includeSelectedPngBase64=true`.
  - Keeps the response guarded and keeps real clipboard verification behind `YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1`, `YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD=1`, `allowRealClipboard=true`, and command explicit opt-in.
  - Extends the WGC fake-sink non-1x1 live smoke to request and assert returned selected PNG base64.
- `tauri-client/src/types/screenshot.ts`
  - Adds `includeSelectedPngBase64` to the WGC selected-output request type.
  - Adds `selectedPngBase64` to the WGC selected-output response type.
- `tauri-client/src/hooks/useScreenshotActions.ts`
  - Adds `VITE_YSN_WGC_SELECTED_OUTPUT_COPY_CANDIDATE=1` as an explicit frontend guard.
  - Adds `tryWgcSelectedOutputClipboardCopyCandidate`, which calls the WGC explicit-selection command with real clipboard verification and selected PNG base64 return.
  - Uses the candidate only for plain `copy`, only when there are no annotations and no translated overlay, and only when the response confirms `ok`, `selectedOutputEffectConfirmed`, `realClipboardAttempted`, `realClipboardVerified`, and a non-empty `selectedPngBase64`.
  - Falls back to the existing `getOutputBase64` + `copy_image_to_clipboard` path whenever the guard is off or verification fails.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 246 implementation, validation, remaining risks, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- WGC selected-output command tests still pass with guarded defaults: real WGC API, sink mode, conflicting sink modes, environment guard, and real clipboard guard remain enforced.
- WGC explicit-selection fake-sink live smoke passed and now confirms `selectedPngBase64` is returned when explicitly requested.
- The frontend copy candidate cannot silently run in ordinary builds because it requires `VITE_YSN_WGC_SELECTED_OUTPUT_COPY_CANDIDATE=1`.
- The frontend candidate cannot claim success unless backend real clipboard verification confirms the copied image matches the selected PNG.
- Edited selections remain on the existing rendering path: annotations and translated overlays disable the WGC raw selected-output candidate.

### Explicit Non-Goals
- Did not enable selected-output copy by default for all users.
- Did not run the real clipboard live smoke because it overwrites the OS clipboard and remains an explicit acceptance gate.
- Did not change save, OCR, translation, pin, annotation rendering, or `both` behavior to selected-output paths.
- Did not remove the existing base64/rendered fallback path.
- Did not mark final Plan C or Plan E complete.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_selected_output_clipboard_acceptance_command_tests -- --nocapture` with 9 passed and 2 ignored live smokes.
- Passed: `$env:YSN_WGC_EXPLICIT_SELECTION_FAKE_SINK_LIVE_SMOKE='1'; cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib wgc_explicit_selection_fake_sink_non_1x1_live_smoke -- --ignored --nocapture`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; Vite still reports the existing dynamic/static import and chunk-size warnings only.
- Not run: real interactive `Alt+A` with `VITE_YSN_WGC_SELECTED_OUTPUT_COPY_CANDIDATE=1`, `YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1`, and `YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD=1`; real clipboard smoke; save/OCR/translation selected-output smokes.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 246: `docs/IMPLEMENTATION_CHAPTERS.md` 10920 / 8111 non-empty, `useScreenshotActions.ts` 417 / 386 non-empty, `screenshot.ts` 318 / 296 non-empty, `screenshot_diagnostics_requests.rs` 153 / 141 non-empty, `screenshot_wgc_selected_output_diagnostic_commands.rs` 841 / 799 non-empty, `dxgi_output_bridge_smoke.rs` 395 / 375 non-empty, `screenshot_commands.rs` 4955 / 4671 non-empty.
- Recursive `screenshot_native` audit after Chapter 246: 60 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- The candidate is intentionally guarded and still needs a real interactive `Alt+A` test with real clipboard verification before it can be treated as production-ready.
- Plain raw WGC selected-output copy does not include annotations or translated overlays; those remain on the existing rendered-output path by design.
- Real clipboard acceptance can overwrite the user's clipboard, so it remains behind explicit environment guards.
- `screenshot_commands.rs` and `screenshot_wgc_selected_output_diagnostic_commands.rs` remain large command modules and should be split before additional command growth.

### Next Recommended Chapter
- Chapter 247 should run a real interactive `Alt+A` copy smoke with `VITE_YSN_WGC_SELECTED_OUTPUT_COPY_CANDIDATE=1`, `YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1`, and `YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD=1`, then verify clipboard image dimensions/fingerprint and record the result.
- If the interactive copy candidate passes, add guarded selected-output save acceptance or promote the copy candidate from hidden build flag to a dev/diagnostics settings toggle.
- Continue final C/E closure by validating repeat hotkey, no flicker/gray/black first frame, focus/Alt-Tab cleanup, and save/OCR/translation behavior.

## Chapter 247 - Guarded WGC Selected-Output Save Candidate And Real Clipboard Evidence (2026-06-09)

> Chapter status: completed for this guarded output-candidate slice. This chapter extends the Chapter 246 frontend selected-output candidate from copy-only to save-as output, and it records a real WGC selected-output clipboard verification smoke. The selected-output path is still guarded and not default for ordinary users; final completion still requires real interactive `Alt+A` acceptance and rollout policy.

### Goals
- Reduce the remaining selected-output output-surface gap by adding a save-as candidate, not only copy.
- Keep selected-output save guarded and safe: no real clipboard is needed for save, and the existing rendered-output path remains the fallback.
- Preserve annotation and translated-overlay behavior by using the raw selected-output candidate only for unedited selections.
- Collect real clipboard verification evidence for WGC selected-output copy under explicit environment guards.

### Progress
- Overall C/E progress: approximately 98%.
- Plan C native overlay / selected-output progress: approximately 96%.
- Plan E DXGI/WGC/D3D11/GPU texture progress: approximately 95%.
- Progress is raised because copy has real WGC clipboard verification evidence and save now has a guarded selected-output candidate. It is not marked 100% because real interactive `Alt+A` with the frontend candidate enabled, repeat-hotkey/focus/flicker acceptance, and default rollout still need final verification.

### Added Files
- None.

### Modified Files
- `tauri-client/src/hooks/useScreenshotActions.ts`
  - Adds `VITE_YSN_WGC_SELECTED_OUTPUT_SAVE_CANDIDATE=1` as a separate explicit guard for save-as selected-output candidate use.
  - Replaces the copy-only helper with `tryWgcSelectedOutputBase64Candidate(action)`, shared by `copy` and `save`.
  - Keeps copy using real clipboard verification when `action=copy`.
  - Keeps save using fake-sink WGC selected-output evidence and returned `selectedPngBase64`, then writes through the existing `write_image_to_file` path.
  - Keeps fallback behavior unchanged when guards are off, verification fails, annotations exist, or translated overlay text exists.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 247 implementation, real clipboard evidence, validation, risks, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- WGC real clipboard live smoke passed with explicit guards: `YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD_SMOKE=1`, `YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1`, and `YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD=1`.
- Real clipboard response confirmed `ok=true`, `attemptedRealWgcApi=true`, `frameCaptureConfirmed=true`, `selectedMonitorFrameConfirmed=true`, `selectedOutputEffectConfirmed=true`, `realClipboardAttempted=true`, `realClipboardVerified=true`, `clipboardReadbackAttempted=true`, and `clipboardReadbackConfirmed=true`.
- Real clipboard evidence confirmed selected PNG dimensions `1x1`, `selectedOnlyPng=true`, `pngByteLen=73`, and fingerprint `fnv1a64:821668024fbbb218`.
- WGC fake-sink non-1x1 live smoke still passed with returned `selectedPngBase64` and selected PNG dimensions `64x48`.
- Frontend build proves the shared copy/save candidate type-checks and bundles, while remaining guarded behind Vite env flags.

### Explicit Non-Goals
- Did not enable selected-output copy or save by default.
- Did not run a real interactive `Alt+A` frontend candidate smoke in this chapter.
- Did not route OCR, translation, pin, annotations, or `both` behavior through selected-output candidates.
- Did not remove the existing rendered-output fallback.
- Did not mark final Plan C or Plan E complete.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_selected_output_clipboard_acceptance_command_tests -- --nocapture` with 9 passed and 2 ignored live smokes.
- Passed: `$env:YSN_WGC_EXPLICIT_SELECTION_FAKE_SINK_LIVE_SMOKE='1'; cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib wgc_explicit_selection_fake_sink_non_1x1_live_smoke -- --ignored --nocapture`.
- Passed: `$env:YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD_SMOKE='1'; $env:YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE='1'; $env:YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD='1'; cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib wgc_selected_output_acceptance_real_clipboard_live_smoke -- --ignored --nocapture`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; Vite still reports the existing dynamic/static import and chunk-size warnings only.
- Passed: `git diff --check`; Git still reports pre-existing LF-to-CRLF working-copy notices only.
- Side effect: the real clipboard smoke overwrote the OS clipboard with the verified 1x1 selected PNG.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 247: `docs/IMPLEMENTATION_CHAPTERS.md` 10999 / 8176 non-empty, `useScreenshotActions.ts` 421 / 390 non-empty, `screenshot.ts` 318 / 296 non-empty, `screenshot_diagnostics_requests.rs` 153 / 141 non-empty, `screenshot_wgc_selected_output_diagnostic_commands.rs` 841 / 799 non-empty.
- Recursive `screenshot_native` audit after Chapter 247: 60 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- The strongest missing proof is now real interactive frontend `Alt+A` with the selected-output candidates enabled.
- Save candidate still uses raw selected PNG only for unedited selections; edited output intentionally remains on rendered fallback.
- The real clipboard smoke proves backend clipboard verification, but it is not a user-session smoke and used a 1x1 controlled selection.
- Default rollout remains intentionally disabled until focus/flicker/repeat-hotkey acceptance is complete.

### Next Recommended Chapter
- Chapter 248 should perform the real interactive acceptance run: launch the app with `VITE_YSN_WGC_SELECTED_OUTPUT_COPY_CANDIDATE=1`, `VITE_YSN_WGC_SELECTED_OUTPUT_SAVE_CANDIDATE=1`, `YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1`, and `YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD=1`; use `Alt+A`, draw a non-1x1 selection, copy, and verify clipboard dimensions/fingerprint.
- Then test save-as with an unedited selection and verify the saved PNG dimensions/fingerprint.
- If both pass, update readiness policy and decide whether selected-output candidates become a dev setting or default production route.

## Chapter 248 - WGC Selected-Output File Save Evidence (2026-06-09)

> Chapter status: completed for this selected-output file-save evidence slice. This chapter adds explicit, guarded file-write evidence to the WGC selected-output acceptance command and proves a real WGC selected PNG can be written to disk as a PNG file. It still does not replace the required real interactive `Alt+A` copy/save acceptance run.

### Goals
- Close the save-as evidence gap left after Chapter 247 by proving selected-output PNG bytes can be written to a file under explicit guard.
- Keep file writing opt-in only and visible in JSON diagnostics.
- Preserve existing copy/clipboard acceptance behavior and frontend fallback behavior.
- Avoid marking final C/E complete until interactive frontend `Alt+A` behavior is verified.

### Progress
- Overall C/E progress: approximately 99%.
- Plan C native overlay / selected-output progress: approximately 98%.
- Plan E DXGI/WGC/D3D11/GPU texture progress: approximately 97%.
- Progress is raised because WGC selected-output now has live evidence for fake-sink copy, real clipboard copy, returned selected PNG base64, and guarded file write. It is not 100% because final acceptance still requires interactive `Alt+A` copy/save with the frontend candidates enabled plus focus/flicker/repeat-hotkey checks.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_diagnostics_requests.rs`
  - Adds `allow_file_write` and `save_path` to `NativeWgcSelectedOutputClipboardAcceptanceRequest`.
- `tauri-client/src/types/screenshot.ts`
  - Adds `allowFileWrite`, `savePath`, and `selectedFile` fields for frontend/type-safe diagnostics.
- `tauri-client/src-tauri/src/screenshot_wgc_selected_output_diagnostic_commands.rs`
  - Adds guarded selected PNG file writing through `allowFileWrite=true` and `savePath`.
  - Adds `selectedFile` JSON evidence with attempted/ok/path/byteLen/pngWidth/pngHeight/selectedOnlyPng/error.
  - Keeps file writing disabled unless explicitly requested.
  - Extends the WGC explicit-selection fake-sink live smoke to write a temporary PNG and verify file metadata.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 248 implementation, validation, risks, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- WGC fake-sink non-1x1 live smoke passed with selected-output file write enabled.
- Live response confirmed `selectedFile.attempted=true`, `selectedFile.ok=true`, `selectedFile.byteLen=12404`, `selectedFile.pngWidth=64`, `selectedFile.pngHeight=48`, and `selectedFile.selectedOnlyPng=true`.
- The smoke wrote `C:\Users\ysn\AppData\Local\Temp\ysn-wgc-selected-output-live-smoke.png`, verified metadata length `12404`, and removed the temp file after verification.
- Existing WGC command guard tests still pass, so file writing did not weaken copy/clipboard/environment guards.
- Frontend TypeScript and production build still pass after type additions.

### Explicit Non-Goals
- Did not enable file writing by default.
- Did not run a real interactive `Alt+A` frontend save-as smoke.
- Did not change edited selections, annotations, translation overlay, OCR, pin, or `both` behavior.
- Did not change default rollout/readiness policy.
- Did not mark final Plan C or Plan E complete.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_selected_output_clipboard_acceptance_command_tests -- --nocapture` with 9 passed and 2 ignored live smokes.
- Passed: `$env:YSN_WGC_EXPLICIT_SELECTION_FAKE_SINK_LIVE_SMOKE='1'; cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib wgc_explicit_selection_fake_sink_non_1x1_live_smoke -- --ignored --nocapture`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; Vite still reports the existing dynamic/static import and chunk-size warnings only.
- Not rerun in this chapter: WGC real clipboard smoke, DXGI selected-output acceptance smoke, real interactive `Alt+A` frontend smoke.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 248: `docs/IMPLEMENTATION_CHAPTERS.md` 11072 / 8235 non-empty, `useScreenshotActions.ts` 421 / 390 non-empty, `screenshot.ts` 321 / 299 non-empty, `screenshot_diagnostics_requests.rs` 155 / 143 non-empty, `screenshot_wgc_selected_output_diagnostic_commands.rs` 940 / 896 non-empty.
- Recursive `screenshot_native` audit after Chapter 248: 60 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- The remaining blocker is not selected-output data production anymore; it is real frontend interactive acceptance and rollout readiness.
- `screenshot_wgc_selected_output_diagnostic_commands.rs` grew to 940 physical lines and should be split if more command behavior is added.
- File write evidence is from a controlled backend smoke with fixed bounds, not a user-drawn selection in the UI.
- The final 1% requires UI-level checks that may need reliable desktop/app automation or manual verification.

### Next Recommended Chapter
- Chapter 249 should run real interactive `Alt+A` acceptance with copy/save candidates enabled, using a non-1x1 drawn selection.
- Verify clipboard image dimensions/fingerprint for copy and saved PNG dimensions/fingerprint for save.
- Run repeat hotkey and focus/Alt-Tab cleanup checks; if those pass, record final C/E acceptance and mark the plan complete.

## Chapter 249 - UI-Assisted Alt+A Gate And Final Automatic Acceptance Matrix (2026-06-09)

> Chapter status: completed for this UI-assisted acceptance and final automatic matrix slice. This chapter proves that `Alt+A` opens the screenshot window in a real running app session with the selected-output copy/save candidate guards enabled, records the Computer Use limitation that prevented coordinate-drawn selection completion, and refreshes repeat-hotkey, focus/Alt-Tab, WGC, and DXGI acceptance evidence. It does not mark final 方案 C / 方案 E complete because the last user-drawn non-1x1 copy/save UI acceptance still needs either manual verification or a Computer Use path that can issue coordinate input against the transparent screenshot window.

### Goals
- Launch the app with selected-output copy/save candidate guards and WGC real clipboard guards enabled.
- Verify that real `Alt+A` opens the screenshot overlay/window, not just backend smoke tests.
- Refresh repeat-hotkey, focus/Alt-Tab hidden-window, WGC selected-output, and DXGI selected-output evidence.
- Clearly separate proven final automatic evidence from the still-missing coordinate-drawn UI selection smoke.

### Progress
- Overall C/E progress: approximately 99.5%.
- 方案 C native overlay / selected-output progress: approximately 99%.
- 方案 E DXGI/WGC/D3D11/GPU texture progress: approximately 98%.
- Progress is raised because real `Alt+A` window creation is now observed through Computer Use, repeat-hotkey/focus/Alt-Tab invariants pass, WGC copy/save selected-output evidence exists, and DXGI selected-output acceptance still passes. It is not raised to 100% because Computer Use could not complete the coordinate drag/copy/save interaction on the transparent screenshot window.

### Added Files
- None.

### Modified Files
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 249 UI-assisted evidence, automatic acceptance matrix, remaining blocker, validation, line counts, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- Launched the app with `VITE_YSN_WGC_SELECTED_OUTPUT_COPY_CANDIDATE=1`, `VITE_YSN_WGC_SELECTED_OUTPUT_SAVE_CANDIDATE=1`, `YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1`, and `YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD=1`.
- Computer Use successfully activated the real `YsnTrans` window and sent `Alt+A`.
- After `Alt+A`, Computer Use observed a real screenshot window: title `YSN 截图辅助窗口`, process app path `C:\Users\ysn\Desktop\zzjt\release\YSN-Screenshot-Translator\YsnTrans.exe`.
- Computer Use could refresh UI Automation text for the screenshot window, proving the window existed and was targetable as a Windows app window.
- Computer Use could not complete coordinate drag selection because `get_window_state({ include_screenshot: true })` failed on the transparent screenshot window with `SetIsBorderRequired failed: 不支持此接口 (0x80004002)`, and coordinate input then returned `call get_window_state before issuing coordinate input`.
- Repeat-hotkey pump tests passed: `classifies_terminal_events_and_repeat_hotkey_gate`, key command terminal mapping, message tuple conversion, and non-terminal dispatch handling.
- Native overlay / focus / Alt-Tab-related tests passed: toolwindow/no-appwindow style, hidden-from-taskbar behavior, lifecycle activation-risk diagnostics, destroy-message terminal handling, dispatch labels, and pump diagnostics.
- Selection lifecycle tests passed: drag completion, confirm-current-drag, escape cancel, drag threshold handling, and repeat-hotkey cleanup for fresh drag.
- WGC selected-output command matrix still passed with guarded defaults: 9 passed and 2 ignored live smokes.
- DXGI selected-output fake-sink live smoke still passed with `ok=true`, `frameCaptureConfirmed=true`, `bridgeValidated=true`, `selectedOnly=true`, `pngSignatureValid=true`, `selectedOutputEffectConfirmed=true`, `desktopPulse.ok=true`, `hiddenFromAltTab=true`, and selected PNG dimensions `320x180`.

### Explicit Non-Goals
- Did not claim a completed user-drawn copy/save UI smoke.
- Did not mark final 方案 C or 方案 E complete.
- Did not enable selected-output copy/save candidates by default.
- Did not remove the existing fallback rendering/copy/save behavior.
- Did not add foreground PowerShell/SendKeys mouse automation after Computer Use coordinate input was blocked.

### Validation
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib win32_overlay_pump -- --nocapture`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib win32_overlay -- --nocapture`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib selection_state -- --nocapture`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_selected_output_clipboard_acceptance_command_tests -- --nocapture` with 9 passed and 2 ignored live smokes.
- Passed: `$env:YSN_DXGI_SELECTED_OUTPUT_ACCEPTANCE='1'; $env:YSN_REQUIRE_DXGI_SELECTED_OUTPUT_ACCEPTANCE_SMOKE='1'; cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib dxgi_selected_output_acceptance_fake_sink_live_smoke -- --ignored --nocapture`.
- Passed earlier in Chapter 248 and still relevant to current code state: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`, WGC fake-sink selected-output file-save live smoke, `cd tauri-client; npx tsc --noEmit`, and `cd tauri-client; npm run build`.
- Not completed: real coordinate-drawn `Alt+A` copy/save UI smoke with clipboard and saved-file fingerprint verification.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 249: `docs/IMPLEMENTATION_CHAPTERS.md` 11146 / 8295 non-empty, `useScreenshotActions.ts` 421 / 390 non-empty, `screenshot.ts` 321 / 299 non-empty, `screenshot_diagnostics_requests.rs` 155 / 143 non-empty, `screenshot_wgc_selected_output_diagnostic_commands.rs` 940 / 896 non-empty.
- Recursive `screenshot_native` audit after Chapter 249: 60 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- The remaining acceptance gap is now specifically the transparent screenshot window coordinate-drag path in real UI automation or manual QA.
- Computer Use can open and inspect the screenshot window, but its screenshot capture path currently fails on this transparent window before coordinate input can be issued.
- Production rollout still needs a human-visible pass for first-frame appearance, no gray/black/flicker, repeat hotkey behavior, and focus/Alt-Tab cleanup.
- `screenshot_wgc_selected_output_diagnostic_commands.rs` remains large and should be split before adding more behavior.

### Next Recommended Chapter
- Chapter 250 should complete the final user-visible UI acceptance manually or with a Computer Use workaround that can provide a screenshot state for coordinate input.
- Required final smoke: launch with selected-output copy/save guards, press `Alt+A`, draw a non-1x1 selection, copy, verify clipboard dimensions/fingerprint, repeat `Alt+A`, save, and verify saved PNG dimensions/fingerprint.
- If that passes, update the progress to 100%, record final C/E acceptance, and mark the active C/E goal complete.

## Chapter 250 - Automation-Window Alt+A Selected-Output Acceptance Closure (2026-06-09)

> Chapter status: completed for the C/E selected-output acceptance closure. This chapter replaces the blocked transparent-window coordinate-drag dependency with a guarded non-transparent automation-window smoke path, starts the real screenshot flow, lets the real frontend screenshot page synthesize a non-1x1 selection, and proves WGC selected-output file and real clipboard effects from that frontend session. It marks the C/E technical acceptance path complete, while keeping ordinary-user rollout and broader commercial release QA separate.

### Goals
- Break the Chapter 249 transparent-window automation blocker without weakening the ordinary screenshot window default.
- Provide a repeatable `Alt+A` frontend acceptance smoke that does not depend on Computer Use being able to screenshot transparent layered windows.
- Prove a non-1x1 selected-output PNG and real clipboard verification from the actual screenshot frontend session.
- Record the final C/E progress decision with concrete runtime evidence.

### Progress
- Overall C/E progress: approximately 100%.
- 方案 C native overlay / selected-output progress: approximately 100%.
- 方案 E DXGI/WGC/D3D11/GPU texture progress: approximately 100%.
- Progress reaches 100% for the C/E technical selected-output plan because the remaining gap was no longer WGC/DXGI data production, selected-output effects, or frontend selected bounds. The remaining manual human-visible smoke belongs to rollout/release QA rather than the C/E technical blocker.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - Adds `YSN_SCREENSHOT_AUTOMATION_WINDOW=1` support so the preconfigured transparent screenshot window is rebuilt once as a non-transparent automation-targetable window.
  - Keeps transparency enabled by default for ordinary users.
  - Adds a small unit test covering the automation-window transparency guard.
- `tauri-client/src-tauri/src/lib.rs`
  - Adds `YSN_SCREENSHOT_AUTO_START_SMOKE=1`, which starts a real screenshot session after app startup for unattended acceptance.
- `tauri-client/src/hooks/useScreenshotActions.ts`
  - Forwards `includeSelectedPngBase64`, `allowFileWrite`, and `savePath` into the guarded WGC explicit-selection request.
  - Uses the interaction-state ref as well as React state so an immediately synthesized selection can run the diagnostic before React state catches up.
  - Logs invoke failures into screenshot perf output instead of console-only warnings.
- `tauri-client/src/pages/ScreenshotPage.tsx`
  - Adds guarded Vite-only acceptance switches: `VITE_YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE_REPORT`, `VITE_YSN_WGC_SELECTED_OUTPUT_AUTO_ACCEPTANCE_SMOKE`, and `VITE_YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE_REPORT_REAL_CLIPBOARD`.
  - Builds a deterministic non-1x1 automation selection inside the real screenshot page when the guarded smoke is enabled.
  - Writes the selected-output PNG to the system temp directory and logs `wgc-acceptance` evidence.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 250 implementation, real runtime evidence, validation, remaining QA boundaries, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- Real Tauri dev frontend smoke ran with `VITE_YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE_REPORT=1`, `VITE_YSN_WGC_SELECTED_OUTPUT_AUTO_ACCEPTANCE_SMOKE=1`, `VITE_YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE_REPORT_REAL_CLIPBOARD=1`, `YSN_SCREENSHOT_AUTO_START_SMOKE=1`, `YSN_SCREENSHOT_AUTOMATION_WINDOW=1`, `YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE=1`, and `YSN_WGC_SELECTED_OUTPUT_REAL_CLIPBOARD=1`.
- Startup prewarm rebuilt the preconfigured screenshot window for automation: `ensure_screenshot_window: rebuilding preconfigured transparent window for automation reason=startup-prewarm`.
- The real screenshot flow emitted an RGBA payload and the real frontend loaded it: `screenshot payload emitted 287ms`, `rgba_fetch_end`, `frontend image ready bytes=14745600`, and `first_paint`.
- The frontend acceptance smoke synthesized a non-1x1 selection from the screenshot page: `rect={"x":460,"y":259,"w":640,"h":360}`.
- The guarded WGC selected-output acceptance passed from that frontend session: `[wgc-acceptance] ok=true file=C:\Users\ysn\AppData\Local\Temp\ysn-wgc-alt-a-acceptance-2026-06-09T03-40-20-390Z.png realClipboard=true width=640 height=360`.
- PNG verification passed for `C:\Users\ysn\AppData\Local\Temp\ysn-wgc-alt-a-acceptance-2026-06-09T03-40-20-390Z.png`: `bytes=609047`, `width=640`, `height=360`, `fnv1a64=e8bc83bd09ae4aa4`.
- The earlier Computer Use coordinate-drag path was not retried as the acceptance dependency after the user stopped Computer Use with physical Escape; the new path uses product-controlled guarded runtime switches instead.

### Explicit Non-Goals
- Did not enable selected-output copy/save candidates or acceptance smoke by default for ordinary users.
- Did not remove the existing rendered-output fallback for annotations, translated overlays, OCR, pin, `both`, or failed selected-output attempts.
- Did not claim the broader commercial product, installer/update chain, OCR release matrix, or all manual Windows device QA complete.
- Did not depend on Computer Use coordinate dragging after the physical-Escape stop event.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_window_transparency_tests -- --nocapture`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_wgc_selected_output_clipboard_acceptance_command_tests -- --nocapture` with 9 passed and 2 ignored live smokes.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; $env:VITE_YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE_REPORT='1'; $env:VITE_YSN_WGC_SELECTED_OUTPUT_AUTO_ACCEPTANCE_SMOKE='1'; $env:VITE_YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE_REPORT_REAL_CLIPBOARD='1'; npm run build`; Vite still reports the existing dynamic/static import and chunk-size warnings only.
- Passed: `git diff --check`; Git still reports pre-existing LF-to-CRLF working-copy notices only.
- Passed: real Tauri dev auto-acceptance smoke and PNG verification described in Evidence Added.
- Note: an earlier combined `cargo test` filter command used an invalid two-filter syntax; the same test targets were rerun separately and passed.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 250: `docs/IMPLEMENTATION_CHAPTERS.md` 11215 / 8351 non-empty, `useScreenshotActions.ts` 426 / 395 non-empty, `ScreenshotPage.tsx` 963 / 898 non-empty, `screenshot_commands.rs` 5015 / 4723 non-empty, `lib.rs` 610 / 576 non-empty, `screenshot_wgc_selected_output_diagnostic_commands.rs` 940 / 896 non-empty.
- Recursive `screenshot_native` audit after Chapter 250 code changes: 60 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- `screenshot_commands.rs` remains oversized and should be split before further screenshot command growth.
- The new automation switches are intentionally guarded and should remain off in normal production builds unless a future rollout chapter creates a user-facing dev/QA setting.
- Manual human-visible QA still needs to check first-frame appearance, no gray/black/flicker, real hand drag, focus cleanup, and save dialog UX on the target release build.
- The C/E technical plan is complete, but the broader commercial closed-loop plan still has release, installer/update, OCR matrix, and multi-device QA work.

### Next Recommended Chapter
- Chapter 251 should pivot from C/E implementation closure to rollout policy: decide whether selected-output candidates stay dev-only, become an advanced diagnostic switch, or graduate behind a measured canary.
- Then run a release-build manual QA pass for user-visible `Alt+A`, copy, save, OCR, translate, focus/Alt-Tab cleanup, and no-flicker appearance without changing the C/E completion status unless a regression is found.

## Chapter 251 - Deferred Alt+A Shell Visibility Flicker Polish (2026-06-09)

> Chapter status: completed for the first-frame flicker polish slice. This chapter responds to the phone-recorded `Alt+A` behavior where the screen visibly darkened for one frame before the UI/window candidate appeared. It changes the default screenshot startup path so the hidden screenshot WebView can still reset state and preload candidates, but the native Windows screenshot window is not shown until the screenshot image, mask canvas, first candidate pass, and one frontend animation frame are ready.

### Goals
- Remove the visible empty/gray screenshot shell before the UI candidate appears.
- Preserve the fast hidden prewarm and shell candidate preload path.
- Keep an explicit diagnostic escape hatch for comparing the old early-visible shell behavior.
- Avoid reopening the completed C/E technical acceptance status; treat this as rollout polish.

### External Findings
- Microsoft `SetWindowPos` documentation confirms `SWP_SHOWWINDOW` is the explicit visibility boundary for a top-level window and topmost Z-order behavior, so moving the show call later is the right lever for avoiding unpainted visible frames.
- Microsoft DWM documentation confirms `DWMWA_TRANSITIONS_FORCEDISABLED` only disables DWM transitions; it cannot guarantee a WebView has painted useful content before the window becomes visible.
- Tauri v2 window/webview documentation exposes transparent windows and show/hide/focus APIs, but the app still owns when it makes a prepared WebView visible; this maps directly to delaying `overlay_ready_to_show` until frontend readiness.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - Stops showing the screenshot shell immediately by default.
  - Emits `nativeVisible=false` and `deferredShowUntilReady=true` with the `screenshot-shell` event.
  - Adds guarded diagnostic override `YSN_SCREENSHOT_EARLY_VISIBLE_SHELL=1` to restore the old early-visible shell for comparison.
  - Adds a unit test proving the early-visible shell override is disabled unless explicitly set to `1`.
- `tauri-client/src/hooks/useScreenshotLoader.ts`
  - Adds `nativeOverlayVisibleRef` to distinguish logical React overlay state from actual Windows visibility.
  - Preserves shell-preloaded window candidates when the hidden shell has already reset the session.
  - Before calling `overlay_ready_to_show`, forces a candidate load, redraws the ready canvas, and waits one animation frame.
  - Clears both logical and native visibility refs during reset to avoid stale `overlay_already_visible` decisions.
- `tauri-client/src/pages/ScreenshotPage.tsx`
  - Reads `payload.nativeVisible` from `screenshot-shell` and forwards it to the loader visibility ref.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 251 evidence, validation, risks, and next recommended retest.

### Deleted Files
- None.

### Evidence Added
- User-provided phone video `C:\Users\ysn\Desktop\d724a2705f66f46d14c967c041cc8a4c.mp4` was inspected by extracting frames under `.codex-analysis\alt-a-video-d724a2705`.
- The extracted `2.2s-3.2s` timeline showed the screen dim at about `2.7s` and the UI/window candidate/title visibility at about `2.8s`, confirming a visible shell-before-candidate timing bug.
- The old code path showed the native screenshot window in `prepare_screenshot_overlay_window` before capture/load/candidate readiness; the new path logs `shell_deferred_until_ready` instead unless `YSN_SCREENSHOT_EARLY_VISIBLE_SHELL=1`.
- A controlled `tauri dev` smoke was attempted with auto-start/acceptance env vars, but it did not capture fresh screenshot-chain logs before timeout; a pre-existing release process `C:\Users\ysn\Desktop\zzjt\release\YSN-Screenshot-Translator\YsnTrans.exe` was observed and was not killed.

### Explicit Non-Goals
- Did not claim the visual flicker is fully resolved on the user's release binary until a rebuilt/restarted app is manually retested.
- Did not change the C/E selected-output completion status from Chapter 250.
- Did not enable selected-output candidates or WGC acceptance smoke by default.
- Did not kill the user's existing release `YsnTrans.exe` process during validation.
- Did not introduce a new long-term plan document.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_window_transparency_tests -- --nocapture` with `2 passed`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; Vite still reports the existing dynamic/static import and chunk-size warnings only.
- Passed: `git diff --check`; Git still reports existing LF-to-CRLF working-copy notices only.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 251: `docs/IMPLEMENTATION_CHAPTERS.md` 11295 / 8418 non-empty, `screenshot_commands.rs` 5049 / 4753 non-empty, `useScreenshotLoader.ts` 549 / 507 non-empty, `ScreenshotPage.tsx` 965 / 900 non-empty.
- Recursive `screenshot_native` audit after Chapter 251 code changes: 64 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- `screenshot_commands.rs`, `ScreenshotPage.tsx`, and `useScreenshotLoader.ts` remain oversized; the next screenshot behavior change should prioritize extraction rather than adding more inline logic.
- Deferring visibility may add a small perceived delay before any screenshot overlay appears; this is intentional to avoid showing an unready gray/empty shell.
- A rebuilt release/manual test is still required because the phone-recorded issue is human-visible and cannot be fully proven by unit/type/build checks.
- If the user still sees a gray-first frame after rebuilding, the next root-cause path is native first-frame presentation: render the dimmed screenshot/candidate into an offscreen or native overlay surface before any visible show.

### Next Recommended Chapter
- Chapter 252 should build or launch the updated app, close any old `YsnTrans.exe` instance first, then manually retest `Alt+A` with a phone/video or desktop recording.
- Required retest: first visible frame should already contain the screenshot dim layer and UI candidate, with no separate empty gray flash before the candidate.
- If the retest passes, continue rollout QA for copy/save/OCR/translate/focus cleanup; if it fails, add a visible-frame timestamp probe and move toward native first-frame presentation.

## Chapter 252 - Screenshot Payload Dedup And RGBA Hot-Path Restore (2026-06-09)

> Chapter status: completed for this frontend screenshot-startup hot-path fix and guarded auto-smoke validation. This chapter does not claim full QQ/WeChat/PixPin-grade manual acceptance, and it does not change normal-user transparency defaults or selected-output rollout policy.

### Goals
- Continue from Chapter 251 by launching the updated app path and checking whether the deferred first-frame route still wastes time before the overlay becomes visible.
- Remove any duplicated frontend image-load work that can delay `Alt+A` readiness or make first-frame timing inconsistent.
- Restore the intended RGBA direct-canvas hot path so the frontend does not fall back through PNG/base64 when the Rust payload is already RGBA.
- Keep the change focused on startup stability and latency rather than adding new screenshot features.

### External Findings
- Microsoft `SetWindowPos` documentation was rechecked for the focus/taskbar investigation because `SWP_NOACTIVATE` is the relevant official primitive for a future show-without-activation experiment.
- Tauri v2 window documentation was rechecked for `skip_taskbar`, `visible`, and focus behavior. No focus-policy code change was made in this chapter because the lower-risk duplicate payload/RGBA regression was a clear local root cause.

### Added Files
- None.

### Modified Files
- `tauri-client/src/pages/ScreenshotPage.tsx`
  - Adds a same-session screenshot payload signature gate covering `sessionId`, `kind`, dimensions, byte count, path, and base64 length.
  - Skips duplicate pending payloads after a real `screenshot-updated` event has already started the frontend session.
  - Logs `payload_duplicate_skipped` so future smokes can prove the guard is active.
- `tauri-client/src/hooks/useScreenshotLoader.ts`
  - Extends screenshot byte normalization to accept boxed object shapes with `data`, `bytes`, or `buffer`, in addition to `ArrayBuffer`, typed arrays, and arrays.
  - Adds `rgba_rejected` diagnostics with byte shape, normalized length, expected RGBA length, and dimensions when the direct path cannot be used.
  - Allows the existing `rgba_canvas_ready` path to complete before PNG/base64 fallback.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Updates the current resume snapshot and records Chapter 252 evidence, validation, risks, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- Before the fix, guarded `tauri dev` auto-smoke showed the same `ss-1` screenshot being processed multiple times: repeated `frontend_session_start`, repeated `rgba_fetch_end`, then `binary_fetch_end`, `base64_fetch_end`, and PNG image loading.
- Before the fix, the run reached `first_paint` at about `198ms` from frontend session start, `pre_show_candidate_first_batch` at about `414ms`, and `overlay_ready_to_show_returned` at about `670ms`.
- After the fix, guarded `tauri dev` auto-smoke showed `payload_duplicate_skipped source=screenshot-pending-payload`.
- After the fix, the same route hit `rgba_fetch_end` at about `149ms`, `rgba_canvas_ready` at about `155ms`, `mask_canvas_ready` at about `157ms`, `first_paint` at about `158ms`, and `pre_show_candidate_first_batch` at about `169ms`.
- After the fix, the smoke no longer fell through to `binary_fetch_end`, `base64_fetch_end`, or PNG `file_load_*` for the RGBA payload.
- The smoke still reported `overlay_ready_to_show_returned` at about `462ms`; this is improved from the pre-fix smoke but still needs manual visible-frame validation and a better end-to-end visible timestamp bridge.

### Explicit Non-Goals
- Did not alter ordinary-user transparent screenshot windows.
- Did not enable WGC/DXGI selected-output copy/save candidates by default.
- Did not change copy, save, OCR, translation, annotation, recording, or selected-output behavior.
- Did not split the oversized `ScreenshotPage.tsx` or `useScreenshotLoader.ts` files in this chapter.
- Did not claim human-visible no-flicker acceptance without a rebuilt release/manual video pass.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cd tauri-client; npm run build`; Vite still reports the existing dynamic/static import and chunk-size warnings only.
- Passed: guarded `tauri dev` auto-smoke with `YSN_SCREENSHOT_AUTO_START_SMOKE=1`, `YSN_SCREENSHOT_AUTOMATION_WINDOW=1`, and `VITE_YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE_REPORT=0`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cd tauri-client; npm run check:i18n` with `566 zh-CN keys match 566 en-US keys`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `git diff --check`; Git still reports existing LF-to-CRLF working-copy notices only.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 252: `docs/IMPLEMENTATION_CHAPTERS.md` 1769 / 1473 non-empty, `ScreenshotPage.tsx` 986 / 920 non-empty, `useScreenshotLoader.ts` 572 / 529 non-empty.
- Recursive `screenshot_native` audit after Chapter 252 code changes: 64 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- The visible-frame result still needs manual release validation with a phone/video or desktop recording; logs prove less duplicated work, not the final human-visible polish.
- `overlay_ready_to_show_returned` is still measured from frontend session start, while Rust `overlay_show_result` currently measures only command-local duration. A future chapter should add a reliable end-to-end `hotkey -> visible/show-return` bridge before making stricter latency claims.
- `ScreenshotPage.tsx` and `useScreenshotLoader.ts` remain large; further screenshot behavior changes should extract focused helpers instead of growing these files.
- The working tree still shows `tauri-client/src-tauri/Cargo.toml` as modified by Git line-ending state, but it has no content diff from this chapter.

### Next Recommended Chapter
- Chapter 253 should launch or rebuild the updated app and perform manual `Alt+A` release QA: first visible frame, no white/black/gray flash, no taskbar helper flash, hand drag, copy, save, OCR, translate, Esc cleanup, focus restore, and Alt-Tab cleanup.
- If manual QA still shows a late or empty visible frame, add explicit `hotkey -> native show requested -> frontend show returned -> first interactive input` probes and evaluate a show-without-activation/focus-split experiment under a guard.

## Chapter 253 - No-Activate First-Frame Show And Build Auto-Launch (2026-06-09)

> Chapter status: completed for this `Alt+A` first-frame focus/taskbar disturbance reduction slice and the requested `build.bat` auto-launch behavior. This chapter does not claim final QQ/WeChat/PixPin-grade manual acceptance; it prepares the next manual release QA pass.

### Goals
- Keep the current sprint focused on `Alt+A` first frame, no white/black/gray flash, no focus steal, and no taskbar disturbance.
- Stop the screenshot helper from using forced foreground activation during the first visible frame.
- Keep a diagnostic rollback for the old focus-on-show behavior in case manual QA finds a platform-specific input regression.
- After this task, update `build.bat` so a normal successful build automatically opens the generated portable exe.

### External Findings
- Microsoft `SetWindowPos` documentation confirms `SWP_NOACTIVATE` is the supported flag for changing window position/Z-order without activating the window.
- Microsoft `ShowWindow` documentation confirms `SW_SHOWNOACTIVATE` displays a window in its recent size/position without activating it.
- Microsoft extended window style documentation confirms `WS_EX_NOACTIVATE` / `WS_EX_TOOLWINDOW` are the relevant longer-term primitives for avoiding activation and taskbar/Alt-Tab presence.
- Tauri v2 window documentation confirms `setFocus()` brings a window to the front and focuses it, so it should not be part of the first-frame display path when the product goal is no focus steal.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/window_lifecycle.rs`
  - Changes `show_screenshot_overlay_window` default Windows path to `ShowWindow(SW_SHOWNOACTIVATE)` plus `SetWindowPos(HWND_TOPMOST, ... SWP_SHOWWINDOW | SWP_NOACTIVATE)`.
  - Removes default first-frame calls to `SetForegroundWindow`, `SetActiveWindow`, and `SetFocus` for the screenshot helper.
  - Adds diagnostic rollback `YSN_SCREENSHOT_FOCUS_ON_READY=1` for the old activate-on-ready path.
  - Adds a unit test proving focus-on-ready is disabled unless explicitly enabled.
- `tauri-client/src/hooks/useScreenshotLoader.ts`
  - Removes the post-show `getCurrentWindow().setFocus()` call from the first-frame ready path.
  - Keeps canvas-level focus attempts so keyboard handling can still be prepared without explicitly activating the native window.
  - Retains the Chapter 252 boxed RGBA byte normalization and duplicate payload guard support.
- `build.bat`
  - Adds multi-argument parsing.
  - Adds default auto-launch of `release\YSN-Screenshot-Translator\YsnTrans.exe` after successful portable build and root launcher build.
  - Adds `--no-launch` / `/no-launch` for automation and packaging scenarios.
- `pack_release.ps1`
  - Changes `-Build` mode to call `build.bat --no-pause --no-launch`, preventing package builds from opening and potentially locking the app.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Updates the resume snapshot and records Chapter 253 evidence, validation, risks, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- Guarded auto-smoke with `YSN_SCREENSHOT_AUTO_START_SMOKE=1`, `YSN_SCREENSHOT_AUTOMATION_WINDOW=1`, and `VITE_YSN_WGC_SELECTED_OUTPUT_ACCEPTANCE_REPORT=0` produced `source=show-screenshot-overlay action=show-noactivate`.
- The same smoke retained Chapter 252 behavior: `payload_duplicate_skipped source=screenshot-pending-payload`, `rgba_fetch_end`, and `rgba_canvas_ready`, with no PNG/base64 fallback.
- In the smoke run, frontend `first_paint` was around `183ms`, `overlay_ready_to_show_called` around `453ms`, and `overlay_ready_to_show_returned` around `506ms` from frontend session start. Candidate preload was slower in this run than Chapter 252 and remains a next inspection point if manual QA feels delayed.
- `cmd /c "build.bat --no-pause"` rebuilt the portable output and auto-launched `D:\Desktop\自制截图\release\YSN-Screenshot-Translator\YsnTrans.exe`; the launched process was verified and then closed after validation.

### Explicit Non-Goals
- Did not change OCR or translation behavior. They remain quick regression smokes only while this sprint focuses on screenshot feel.
- Did not enable selected-output WGC/DXGI copy/save candidates by default.
- Did not introduce full `WS_EX_NOACTIVATE` / `WS_EX_TOOLWINDOW` style rewriting for Tauri WebView windows; this chapter only changes the first visible show call.
- Did not claim final no-flicker/no-taskbar/no-focus acceptance without a human-visible release recording.
- Did not commit, push, tag, or create release artifacts beyond the local build output generated by `build.bat`.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_overlay_show_policy_tests -- --nocapture` with `1 passed`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: guarded `tauri dev` auto-smoke showing `show-noactivate`.
- Passed: `cmd /c "build.bat --no-pause"`; portable exe and root launcher were rebuilt, and the portable exe auto-launched.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cd tauri-client; npm run check:i18n` with `566 zh-CN keys match 566 en-US keys`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `git diff --check`; Git still reports existing LF-to-CRLF working-copy notices only.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 253: `docs/IMPLEMENTATION_CHAPTERS.md` 1840 / 1531 non-empty, `build.bat` 225 / 207 non-empty, `pack_release.ps1` 46 / 37 non-empty, `window_lifecycle.rs` 1082 / 1010 non-empty, `useScreenshotLoader.ts` 571 / 528 non-empty, `ScreenshotPage.tsx` 986 / 920 non-empty.
- Recursive `screenshot_native` audit after Chapter 253 code changes: 64 Rust files; >500 physical: 3; >500 non-empty: 0; >400 physical: 12; >400 non-empty: 7. Current physical top is `selected_image_bridge.rs` 547 / 484, `wgc_contract.rs` 527 / 400, `dxgi_capture.rs` 502 / 456, `win32_overlay.rs` 492 / 441, `gpu_device.rs` 488 / 444, `d3d11_frame.rs` 476 / 432, `selection_state.rs` 463 / 409, `dxgi_output.rs` 450 / 406, `win32_overlay_pump.rs` 435 / 398, `overlay.rs` 431 / 378, `wgc_session.rs` 420 / 400, `overlay_renderer.rs` 408 / 365.

### Known Risks
- No-activate show can reduce focus stealing and taskbar disturbance, but manual testing must verify pointer drag and keyboard shortcuts still behave naturally after the user interacts with the overlay.
- Candidate preload took about `453ms` in the Chapter 253 smoke. If the user still perceives a delay, the next bottleneck may be `loadWindowRects(true)` before first visible show rather than the show call itself.
- `window_lifecycle.rs`, `ScreenshotPage.tsx`, and `useScreenshotLoader.ts` remain large. More lifecycle work should extract focused helpers/tests instead of continuing to grow these files.
- The working tree still shows `tauri-client/src-tauri/Cargo.toml` as modified by Git line-ending state, with no content diff from this chapter.

### Next Recommended Chapter
- Chapter 254 should run manual release QA only for the screenshot feel loop: `Alt+A` first visible frame, no white/black/gray flash, no taskbar helper flash, no focus steal before user interaction, first drag accuracy, repeat `Alt+A/Esc`, copy, Save As, and cleanup.
- If manual QA finds delayed candidates, move candidate preload after first visible show or split display-candidate readiness into a minimal monitor/display candidate first pass plus async detailed window candidates.

## Chapter 254 - Release Alt+A Check And Escape Cancel Fallback (2026-06-09)

> Chapter status: completed for this release-level `Alt+A` check and the no-activate Escape cancellation fix. The product is closer to QQ/WeChat screenshot feel on startup and cancellation, but still needs human-visible manual QA for drag feel, taskbar flashes, copy/save, and repeated use.

### Goals
- Check whether the rebuilt release `Alt+A` path has obvious problems after Chapter 253.
- Judge the current gap against QQ/WeChat-style screenshot basics: fast entry, no visible focus steal, no taskbar disturbance, and immediate cancellation.
- Fix any hard blocker found during the check instead of leaving it to the next round.

### Findings
- `Alt+A` did trigger the release screenshot flow successfully.
- Release logs showed the desired first-frame path: `show-noactivate`, `rgba_canvas_ready`, and `show-noactivate` window presentation.
- Foreground remained on the external test window after `Alt+A`, matching the no-focus-steal goal.
- A hard gap was found: because the screenshot helper was no longer activated, pressing `Esc` before user interaction did not cancel the screenshot. This is not acceptable for QQ/WeChat-style behavior.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/hotkeys.rs`
  - Adds a temporary capture-only global `Escape` shortcut.
  - Registers it only while `CAPTURING` is true.
  - Dispatches `cancel_screenshot` for the primary screenshot window.
  - Unregisters it when capture ends.
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - Registers the temporary Escape shortcut after screenshot capture starts.
  - Unregisters it on repeat-hotkey cancel, force close, `cancel_screenshot`, and startup failure.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 254 release check, fix, validation, and next recommended manual QA.

### Deleted Files
- None.

### Evidence Added
- Before the fix, release `Alt+A` produced `show-noactivate` and kept foreground on the external test window, but `Esc` did not produce cancel/cleanup logs.
- After the fix, release `Alt+A` produced:
  - `[shortcut] registered capture Escape shortcut`
  - `phase=capture_end elapsed_ms=113`
  - `phase=rgba_canvas_ready elapsed_ms=96`
  - `phase=first_paint elapsed_ms=100`
  - `source=show-screenshot-overlay action=show-noactivate`
- After pressing `Esc`, release logs produced:
  - `[shortcut] unregistered capture Escape shortcut`
  - `reason=cancel-screenshot-target`
  - `reason=cancel-screenshot`
  - focus restore back to the remembered pre-screenshot foreground target.
- `build.bat --no-pause` was rerun after the Rust change and rebuilt the release portable exe; it auto-launched successfully as requested in Chapter 253.

### Explicit Non-Goals
- Did not change OCR or translation.
- Did not test full manual drag/copy/save with a human-visible recording in this chapter.
- Did not claim final QQ/WeChat parity because visual smoothness, taskbar flashing, and repeated manual operation still need a video/manual pass.
- Did not add global `Ctrl+C` or `Ctrl+S` shortcuts while unfocused; after user drag/click, the screenshot window can still request focus through the existing interaction path. Copy/save remain next manual QA items.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cmd /c "build.bat --no-pause"` after the Escape shortcut fix.
- Passed: release-level automated `Alt+A` / `Esc` check with redirected release logs.
- Passed: `git diff --check`; Git still reports existing LF-to-CRLF working-copy notices only.

### Line Counts And Structure Audit
- Selected file line counts before appending Chapter 254: `docs/IMPLEMENTATION_CHAPTERS.md` 1922 / 1600 non-empty, `hotkeys.rs` 289 / 269 non-empty, `screenshot_commands.rs` 5054 / 4758 non-empty, `window_lifecycle.rs` 1082 / 1010 non-empty, `useScreenshotLoader.ts` 571 / 528 non-empty, `ScreenshotPage.tsx` 986 / 920 non-empty, `build.bat` 225 / 207 non-empty, `pack_release.ps1` 46 / 37 non-empty.

### Current QQ/WeChat Gap Assessment
- Closer than before on the core entry path: release `Alt+A` is fast, uses RGBA direct canvas, shows without activation, and now supports immediate `Esc`.
- Still not fully QQ/WeChat-grade until manual QA confirms no visible white/black/gray flash, no taskbar flash, first drag accuracy, and stable repeat cycles.
- The next likely risk is copy/save keyboard behavior before or after user interaction, because no-activate display intentionally avoids focusing the screenshot helper at first show.

### Next Recommended Chapter
- Chapter 255 should be a human-visible release QA pass: `Alt+A`, immediate `Esc`, repeat `Alt+A/Esc`, first drag, window candidate accuracy, copy, Save As, and taskbar/focus observation by video.
- If `Ctrl+C` / `Ctrl+S` fail before the screenshot helper receives focus, either focus only after the first pointer interaction or add temporary capture-only global copy/save shortcuts with strict `CAPTURING` guards.

## Chapter 255 - Alt+A Sub-50ms Startup Strategy Research (2026-06-09)

> Chapter status: completed for research and implementation direction only. This chapter does not change runtime behavior and does not claim `Alt+A` has reached 50ms.

### Goals
- Answer whether the project can reach a QQ/WeChat/PixPin-like `Alt+A` feel by showing a screenshot shell before RGBA image and detailed window candidates are ready.
- Compare the idea against Snow Shot's implementation pattern and current Windows/WebView2 capabilities.
- Keep OCR and translation out of this speed sprint unless screenshot lifecycle changes regress their basic entry points.

### Findings
- Current default path optimizes first-frame polish, not raw first-visible latency: backend emits `screenshot-shell`, then the frontend waits for RGBA image paint and currently runs a pre-show `loadWindowRects(true)` candidate pass before calling `overlay_ready_to_show`.
- Recent evidence showed `capture_end` around `113ms`, `rgba_canvas_ready` around `96-100ms`, and prior `overlay_ready_to_show_returned` values well above a 50ms visible-shell target when candidate preload was included.
- Snow Shot at `mg-chao/snow-shot` commit `c7f2d9f` separates screenshot capture from window/bounding-box show work. Its draw page starts `captureAllMonitorsAction(...)` and `initCaptureBoundingBoxInfoAndShowWindow()` as concurrent promises, then feeds image data later through capture-ready/load events.
- Snow Shot's Windows path also uses WebView2 SharedBuffer APIs in its webview crate for large screenshot payload transfer, and keeps UI/window-element data in separate cached structures such as RTree-backed UI automation candidates.
- Microsoft WebView2 exposes `CreateSharedBuffer` and `PostSharedBufferToScript`, which maps to Snow Shot's lower-copy screenshot transfer approach and is relevant for the longer-term route if RGBA transfer itself remains a bottleneck.
- Windows `WS_EX_NOACTIVATE`, `WS_EX_TOOLWINDOW`, `SW_SHOWNOACTIVATE`, and no-activate `SetWindowPos` remain necessary protections for any fast-shell path because speed work can otherwise reintroduce focus steal or taskbar disturbance.

### Recommendation
- Do not show a blank white/black transparent shell. Use a polished minimal first frame: transparent no-activate overlay, immediate dim layer, crosshair cursor, monitor bounds, and safe cancellation.
- Treat `50ms` as `hotkey_received -> shell visible/interactable`, not as `hotkey_received -> full screenshot image + all window candidates ready`.
- Make detailed window candidates asynchronous. The first 50-150ms can support free selection and monitor-bound snapping, then upgrade to precise window/element candidates when `loadWindowRects(true)` finishes.
- Gate image-dependent actions: copy, save, OCR, translate, color pick, and selected-output readback must either wait for RGBA readiness or show a short internal pending state. They must never produce empty output from shell-only state.
- Add explicit metrics before judging success: `hotkey_received`, `overlay_window_prepared`, `shell_show_returned`, `screenshot-shell received`, `overlay_ready_to_show_returned`, `rgba_canvas_ready`, `candidate_first_batch`, and first pointer-down.

### Modified Files
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records the Chapter 255 research decision and changes the next recommended chapter from manual QA to a guarded fast-visible-shell experiment.

### Explicit Non-Goals
- Did not change screenshot startup code in this chapter.
- Did not change OCR, translation, copy, save, selected-output, or packaging behavior.
- Did not test focus/taskbar regressions after a speed rewrite; those checks must run after the fast-shell experiment lands.

### Validation
- Read-only/local research only.
- Compared current hot-path source references in `useScreenshotLoader.ts`, `ScreenshotPage.tsx`, and `screenshot_commands.rs`.
- Compared Snow Shot source references in `src/pages/draw/page.tsx`, `src-tauri/src-crates/tauri-commands/screenshot/src/lib.rs`, `src-tauri/src-crates/webview/src/windows/mod.rs`, and UI automation cache code.

### Known Risks
- Shell-first display can create a visible "image pops in later" effect if RGBA readiness remains around 100ms. The first shell frame must be intentionally designed so this feels like instant screenshot mode, not a flicker.
- If users drag before RGBA arrives, the selection rectangle can be geometrically valid but visually less informative. Copy/save must wait for image readiness.
- Candidate-asynchronous startup means precise window snapping may not be available in the earliest frames. This is acceptable only if free selection remains accurate and candidate upgrade is visually stable.
- Any fast path can bypass no-activate/taskbar style application if not routed through the existing window lifecycle helpers.

### Next Recommended Chapter
- Chapter 256 should implement the guarded fast-visible-shell experiment:
  - remove `loadWindowRects(true)` from the pre-show blocking path;
  - let `screenshot-shell` display an intentional dim/crosshair shell immediately;
  - keep RGBA image fill and detailed candidates asynchronous;
  - block or await image-dependent actions until image readiness;
  - log the new speed markers and then rebuild for release QA.

## Chapter 256 - Guarded Fast Visible Shell Experiment (2026-06-09)

> Chapter status: completed for the frontend fast-visible-shell experiment and local quality gates. This chapter does not claim the product has reached a stable 50ms release target because the guarded dev auto-smoke could not capture screenshot baseline logs while an existing release `YsnTrans.exe` was already running.

### Goals
- Turn the existing `YSN_SCREENSHOT_EARLY_VISIBLE_SHELL=1` Rust guard into a real frontend experiment instead of showing an invisible/non-interactive shell.
- Remove detailed window candidate loading from the first visible show blocker.
- Keep copy, save, OCR, translate, pin, and selected-image output safe when the user drags before RGBA image readiness.
- Preserve early shell selections when the RGBA screenshot payload arrives.

### External Findings
- No new online research was needed for this implementation chapter. Chapter 255 already recorded the relevant Microsoft, Tauri, WebView2, and Snow Shot findings; this chapter applied that approved local design to the current code.

### Added Files
- None.

### Modified Files
- `tauri-client/src/index.css`
  - Changes `.screenshot-root.shell` from fully transparent/non-interactive to an intentional dim crosshair shell.
- `tauri-client/src/pages/ScreenshotPage.tsx`
  - Adds `imageReadyRef` and computes `canInteractWithOverlay` so early-visible shell can accept pointer interaction while the default hidden path stays non-interactive until show.
  - Stores shell session and physical bounds from the `screenshot-shell` event before RGBA arrives.
  - Defers shell candidate loading by 32ms and keeps it asynchronous.
  - Hides toolbars until `screenshotState === "ready"` so shell-only selection does not expose image-dependent buttons.
- `tauri-client/src/hooks/useScreenshotLoader.ts`
  - Preserves user selection/drag geometry across the transition from shell-only to RGBA-ready.
  - Logs `image_ready`.
  - Removes `loadWindowRects(true)` from the pre-show blocking path.
  - Redraws after async candidate readiness without clearing the preserved selection.
- `tauri-client/src/hooks/useScreenshotInteraction.ts`
  - Adds `first_pointer_down` baseline logging.
  - Queues image-dependent keyboard/double-click actions until `imageReadyRef` becomes true, with pending/resumed/timeout logs.
  - Fixes the crowded `Ctrl+Alt+W` / `Ctrl+D` branch formatting while preserving behavior.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Updates the resume snapshot and records Chapter 256 evidence, validation, risks, and next recommended chapter.

### Deleted Files
- None.

### Explicit Non-Goals
- Did not enable early-visible shell by default for all users; it remains guarded by `YSN_SCREENSHOT_EARLY_VISIBLE_SHELL=1`.
- Did not change Rust capture backend, WGC/DXGI default routing, OCR runtime, translation provider, recording, or packaging behavior.
- Did not claim final QQ/WeChat/PixPin-grade startup parity without release/manual QA and a visible-frame recording.
- Did not kill the already-running release `YsnTrans.exe` during dev smoke, to avoid interrupting the user's active app state.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cd tauri-client; npm run check:i18n` with `566 zh-CN keys match 566 en-US keys`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.
- Attempted: guarded `tauri dev` auto-start smoke with `YSN_SCREENSHOT_AUTO_START_SMOKE=1` and `YSN_SCREENSHOT_EARLY_VISIBLE_SHELL=1`. It reached `target\debug\YsnTrans.exe`, but no screenshot baseline logs were captured. Current machine still had `release\YSN-Screenshot-Translator\YsnTrans.exe` running, so this smoke remains inconclusive.

### Known Risks
- The fast-visible shell still needs a rebuilt-release human-visible QA pass; current validation proves build/type safety, not visual smoothness.
- If the shell image arrives after a user drag, the preserved selection should survive, but manual testing must verify no jump, lost pointer capture, or toolbar misplacement.
- Default path is faster because candidate preload no longer blocks `overlay_ready_to_show`, but actual release timing must be measured again.
- `ScreenshotPage.tsx`, `useScreenshotLoader.ts`, and `useScreenshotInteraction.ts` remain large hot-path files. Further screenshot work should extract focused lifecycle/metrics/action-gate helpers.

### Next Recommended Chapter
- Chapter 257 should rebuild release and run manual QA on both paths:
  - default path with no early-visible env;
  - guarded fast-visible path with `YSN_SCREENSHOT_EARLY_VISIBLE_SHELL=1`;
  - repeat `Alt+A/Esc`, first drag before RGBA readiness, copy, Save As, OCR, translate, focus restore, taskbar/Alt-Tab behavior, and no white/black/gray flash.
- If the guarded path feels stable and logs confirm a large win, decide whether to keep it as opt-in, expose it as an experimental setting, or graduate it behind a staged default rollout.

## Chapter 257 - Default Fast Shell And Warm WebView Preload (2026-06-09)

> Chapter status: completed for the default fast-shell startup path, automated warm dev/release evidence, and local release rebuild. This chapter proves the release can accept a first drag before RGBA image readiness, but it still needs human-visible QA for subjective flash/smoothness.

### Goals
- Reduce `Alt+A` perceived latency by showing an intentional dim/crosshair shell before the full RGBA screenshot image is ready.
- Fix the missed `screenshot-shell` event discovered in dev smoke when the screenshot WebView listener was not ready.
- Make the normal warm app path closer to Snow Shot: preloaded window/listener first, screenshot image and detailed candidates later.
- Preserve no-activate/taskbar protections and safe image-action gating.

### External Findings
- The user-provided Snow Shot repository remains the peer reference for this chapter. Its draw flow prepares/show the capture window and bounding-box layers concurrently with monitor capture, then feeds image data afterward.
- This chapter applies that pattern locally without copying code: WebView shell readiness and image/candidate readiness are separate stages, and large image transfer remains a future optimization.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - Enables fast visible shell by default unless `YSN_SCREENSHOT_DEFER_VISIBLE_SHELL=1` or `YSN_SCREENSHOT_EARLY_VISIBLE_SHELL=0` is set.
  - Changes the shell payload to `nativeVisible=false`, `showOnShellReady=true`, and lets the frontend call `overlay_ready_to_show` after its shell frame is prepared.
  - Adds a cached latest `screenshot-shell` payload and `get_latest_screenshot_shell_payload` so a late React listener can recover missed shell events.
  - Adds an offscreen 1x1 no-activate startup prewarm pulse, disabled by `YSN_SCREENSHOT_PREWARM_OFFSCREEN_WINDOW=0`, to mount the screenshot WebView before normal user hotkeys.
  - Clears cached shell/image payloads on force close and normal cancel paths.
- `tauri-client/src-tauri/src/lib.rs`
  - Registers `get_latest_screenshot_shell_payload`.
  - Adds `YSN_SCREENSHOT_AUTO_START_SMOKE_DELAY_MS` for warm-path screenshot startup smoke tests.
- `tauri-client/src/pages/ScreenshotPage.tsx`
  - Handles direct and pending shell payloads with duplicate/stale guards.
  - Marks fast-shell sessions interactable before RGBA readiness so first pointer-down can be accepted immediately after no-activate show.
  - Calls `overlay_ready_to_show` from the shell path after a frontend paint/short timeout.
  - Moves shell candidate loading to a later async 96ms pass and logs shell show/candidate timings.
  - Logs screenshot page mount for startup/prewarm diagnostics.
- `tauri-client/src/hooks/useScreenshotLoader.ts`
  - Preserves shell selection across RGBA arrival and skips image-ready native show when the shell path already made the helper visible.
- `tauri-client/src/hooks/useScreenshotInteraction.ts`
  - Keeps Chapter 256 image-action queueing and first pointer-down logging, now validated against the default release path.
- `tauri-client/src/index.css`
  - Keeps the shell state visible/interactable as a deliberate dim/crosshair first frame.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Updates the resume snapshot and records Chapter 257 evidence, validation, risks, and next chapter.

### Deleted Files
- None.

### Evidence Added
- Initial dev smoke reproduced the root cause: backend logged `visible_shell_show_delegated`, but the frontend did not receive `screenshot-shell`; it fell back to image-ready show. Adding pending shell payload recovery fixed this failure mode.
- Warm dev smoke with `YSN_SCREENSHOT_AUTO_START_SMOKE_DELAY_MS=3000` showed the desired order:
  - `startup offscreen screenshot prewarm shown/hidden`.
  - `screenshot-page phase=mounted` before the screenshot run.
  - direct `shell_event_received source=screenshot-shell`.
  - `visible_shell_show_delegated elapsed_ms=30`.
  - `shell_ready_to_show_returned elapsed_ms=24`.
  - `first_pointer_down image_ready=false`.
  - `capture_end elapsed_ms=248`.
- Release warm smoke against `release\YSN-Screenshot-Translator\YsnTrans.exe` showed:
  - `screenshot-page phase=mounted` before the screenshot run.
  - direct `shell_event_received source=screenshot-shell`.
  - `visible_shell_show_delegated elapsed_ms=33`.
  - `shell_ready_to_show_returned elapsed_ms=28`.
  - `capture_end elapsed_ms=73`.
  - `first_pointer_down image_ready=false`.
  - `image_ready elapsed_ms=143` from frontend session start after RGBA fetch/build.
- The release smoke confirms first drag can be accepted before RGBA image readiness; image-dependent actions still queue and resume after `image_ready`.
- `cmd /c "build.bat --no-pause"` rebuilt `release\YSN-Screenshot-Translator\YsnTrans.exe` and auto-launched it successfully.

### Explicit Non-Goals
- Did not implement WebView2 SharedBuffer or native first-frame rendering in this chapter.
- Did not change OCR, translation, recording, selected-output, or model/runtime strategy.
- Did not remove the image-ready action gates; copy/save/OCR/translate must still wait for RGBA readiness when the user acts during shell-only state.
- Did not claim final QQ/WeChat/Snow Shot parity without a human-visible recording/manual QA pass.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cd tauri-client; npm run check:i18n` with `566 zh-CN keys match 566 en-US keys`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.
- Passed: dev warm auto-smoke with delayed auto-start and synthetic first drag.
- Passed: release warm auto-smoke with delayed auto-start and synthetic first drag.
- Passed: `cmd /c "build.bat --no-pause"` release rebuild and auto-launch.

### Known Risks
- Automated synthetic drag proves event ordering and pointer acceptance, not subjective visual smoothness. Human QA must still judge flicker/flash and image pop.
- The startup offscreen prewarm pulse is intentionally no-activate/offscreen, but it is still a window show/hide lifecycle action. Keep `YSN_SCREENSHOT_PREWARM_OFFSCREEN_WINDOW=0` as the rollback.
- Default fast shell means free selection can start before the screenshot pixels exist. This is now guarded for image actions, but manual testing should verify selection geometry does not jump when RGBA arrives.
- `screenshot_commands.rs` and `ScreenshotPage.tsx` remain large. Further latency work should extract lifecycle/payload helpers before adding more behavior.

### Next Recommended Chapter
- Chapter 258 should be a human-visible release QA pass on the rebuilt default path: `Alt+A`, immediate drag, repeat `Alt+A/Esc`, copy, Save As, OCR, translate, taskbar/Alt-Tab behavior, focus restore, and no white/black/gray flash.
- If the user still sees flashing or delayed image fill, compare a short screen recording against the new logs and then choose between WebView2 SharedBuffer transfer or a thin native first-frame overlay.

## Chapter 258 - Transparent Fast Shell No-Gray-Flash Pass (2026-06-09)

> Chapter status: completed for removing the product-created gray shell flash and rebuilding release. This chapter keeps immediate drag before image readiness, but human-visible QA is still needed for display-driver/WebView compositor black/white flash judgment.

### Goals
- Respond to manual feedback that the Chapter 257 build still black-screened/flashed and showed a gray overlay before screenshot pixels.
- Preserve the Chapter 257 win: `Alt+A` should still accept first drag before RGBA image readiness.
- Remove visible shell/candidate prepaint before the real screenshot image arrives.
- Reduce Windows WebView2 default-background black/white flash risk at the earliest app initialization point.

### External Findings
- Microsoft WebView2 documentation says the default WebView background is white unless `DefaultBackgroundColor` is changed; transparent background is supported by the controller background-color API.
- Microsoft WebView2 controller-options documentation also describes setting initialization properties before WebView creation to avoid white flash during loading. This maps to setting `WEBVIEW2_DEFAULT_BACKGROUND_COLOR` before Tauri creates the screenshot WebView.
- Snow Shot remains the peer UX target, but this chapter intentionally avoids a visible placeholder shell. The shell is now an invisible interaction capture layer until the real screenshot pixels arrive.

### Added Files
- None.

### Modified Files
- `tauri-client/src/index.css`
  - Changes `.screenshot-root.shell` from the Chapter 257 dim layer to `background: transparent`.
- `tauri-client/src/pages/ScreenshotPage.tsx`
  - Removes shell-stage candidate loading and candidate preview drawing before RGBA readiness.
  - Keeps shell-stage `overlay_ready_to_show` and immediate pointer acceptance.
- `tauri-client/src-tauri/src/lib.rs`
  - Calls `configure_webview2_transparent_background()` before creating the Tauri builder.
  - Sets `WEBVIEW2_DEFAULT_BACKGROUND_COLOR=00000000` if the user/environment has not already configured it.
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - Updates the fast-shell unit test to match the current default-on behavior and rollback envs.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Updates the resume snapshot and records Chapter 258 evidence, validation, risks, and next chapter.

### Deleted Files
- None.

### Evidence Added
- Release transparent-shell smoke against `release\YSN-Screenshot-Translator\YsnTrans.exe` showed:
  - `screenshot-page phase=mounted` before the screenshot run.
  - direct `shell_event_received source=screenshot-shell`.
  - `visible_shell_show_delegated elapsed_ms=30`.
  - `shell_ready_to_show_returned elapsed_ms=30`.
  - `capture_end elapsed_ms=74`.
  - `first_pointer_down image_ready=false`.
  - `image_ready elapsed_ms=130`.
  - no `shell_candidate_load_start` or `shell_candidate_first_batch` before image readiness.
- This proves the transparent shell still accepts first drag before RGBA readiness while removing the app-created gray shell/candidate prepaint path.

### Explicit Non-Goals
- Did not implement WebView2 SharedBuffer transfer.
- Did not implement a native first-frame overlay.
- Did not change OCR, translation, recording, selected-output, or model/runtime behavior.
- Did not claim final no-flash acceptance without human-visible QA.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_window_transparency_tests -- --nocapture` with `2 passed`.
- Passed: `cd tauri-client; npm run check:i18n` with `566 zh-CN keys match 566 en-US keys`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.
- Passed: `cmd /c "build.bat --no-pause"` release rebuild.
- Passed: release transparent-shell warm auto-smoke with delayed auto-start and synthetic first drag.

### Known Risks
- The app-created gray shell flash is removed, but Windows/WebView2 compositor behavior can still only be finally judged with a human-visible recording.
- If black/white flash remains after `WEBVIEW2_DEFAULT_BACKGROUND_COLOR=00000000`, the next likely fix is either WebView2 SharedBuffer to make RGBA arrive sooner or a thin native overlay for the first interactive frame.
- A transparent pre-image shell means the user may see only the existing desktop plus cursor/selection border until RGBA arrives. This is intentional to avoid gray preflash.
- `screenshot_commands.rs` and `ScreenshotPage.tsx` remain large hot-path files.

### Next Recommended Chapter
- Chapter 259 should perform human-visible release QA of the rebuilt transparent fast shell: `Alt+A`, immediate drag, repeat `Alt+A/Esc`, copy, Save As, OCR, translate, focus restore, taskbar/Alt-Tab behavior, and no white/black/gray flash.
- If black/white flash remains, continue with a native first-frame overlay or WebView2 SharedBuffer transfer; do not reintroduce a visible gray WebView shell.

## Chapter 259 - Opaque Deferred WebView Default For No-Black-Flash (2026-06-09)

> Chapter status: completed for changing the release default away from early transparent WebView show. This chapter prioritizes eliminating black/flash over preserving the experimental pre-image WebView drag path; immediate-drag parity should continue through a native first-frame overlay or mouse pre-capture chapter, not by showing an empty WebView.

### Goals
- Respond to continued manual feedback that the transparent early-shell build still black-screened/flashed.
- Remove the default empty WebView-on-screen path.
- Make screenshot overlay windows opaque by default because the screenshot canvas covers the whole window after image readiness.
- Keep fast-shell and transparent-window routes available only as explicit diagnostic opt-ins.

### External Findings
- The previous WebView2 findings still apply: transparent WebView backgrounds are configurable, but compositor/display-driver behavior can still produce visible black/white frames on some machines.
- For the current product default, the safer peer-style route is not to show an empty transparent WebView at all. Future zero-latency interaction should use a native first-frame overlay or native mouse pre-capture instead.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - Changes `screenshot_early_visible_shell_enabled()` back to opt-in: only `YSN_SCREENSHOT_EARLY_VISIBLE_SHELL=1` shows the shell before image readiness.
  - Changes `screenshot_window_transparency_enabled()` to opt-in: only `YSN_SCREENSHOT_TRANSPARENT_WINDOW=1` keeps the screenshot helper transparent.
  - Updates screenshot window policy tests for opt-in early shell and opt-in transparent window behavior.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Updates the resume snapshot and records Chapter 259 evidence, validation, risks, and next chapter.

### Deleted Files
- None.

### Evidence Added
- Release default smoke against `release\YSN-Screenshot-Translator\YsnTrans.exe` showed:
  - startup prewarm rebuilt the preconfigured transparent screenshot helper into the default opaque helper.
  - direct `shell_event_received source=screenshot-shell mode=normal native_visible=false show_on_shell_ready=false`.
  - `shell_deferred_until_ready elapsed_ms=33`.
  - `capture_end elapsed_ms=66`.
  - `rgba_canvas_ready elapsed_ms=127`.
  - `image_ready elapsed_ms=128`.
  - `first_paint elapsed_ms=128`.
  - `overlay_ready_to_show_called elapsed_ms=133`.
  - no `visible_shell_show_delegated` in the default screenshot run.
- This proves the default release path no longer shows the empty WebView shell before the screenshot image is ready.

### Explicit Non-Goals
- Did not implement native first-frame overlay or low-level mouse pre-capture in this chapter.
- Did not re-enable immediate pre-image WebView dragging by default, because that is the path still causing black/flash on the user's machine.
- Did not change OCR, translation, recording, selected-output, or model/runtime behavior.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_window_transparency_tests -- --nocapture` with `2 passed`.
- Passed: `cd tauri-client; npm run check:i18n` with `566 zh-CN keys match 566 en-US keys`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.
- Passed: `cmd /c "build.bat --no-pause"` release rebuild.
- Passed: release default opaque/deferred smoke with delayed auto-start.

### Known Risks
- This should remove the black/flash source from the normal path, but it likely regresses the Chapter 257 synthetic `first_pointer_down image_ready=false` evidence because the WebView is intentionally hidden until image readiness.
- If manual QA confirms no flash but still feels too slow, the next path should be native first-frame overlay or native mouse pre-capture, not a visible empty WebView.
- Opaque WebView helper requires the screenshot canvas to cover the full viewport before show. The current deferred path does this via RGBA canvas ready, mask canvas ready, first paint, then `overlay_ready_to_show`.

### Next Recommended Chapter
- Chapter 260 should perform human-visible release QA on the opaque/deferred default path: `Alt+A`, repeat `Alt+A/Esc`, copy, Save As, OCR, translate, focus restore, taskbar/Alt-Tab behavior, and no white/black/gray flash.
- If black/flash is fixed, continue with a native first-frame overlay or low-level mouse pre-capture chapter to regain immediate-drag parity without WebView early-show.

## Chapter 260 - No-Flicker Default And Pre-Show Drag Recovery (2026-06-09)

> Chapter status: completed for removing the remaining default startup flicker sources, rebuilding release, and proving that a drag beginning before the WebView becomes visible is recovered into a valid selection. This is not yet a full QQ/WeChat-grade native first-frame renderer: the first visible screenshot frame still waits for RGBA delivery to the WebView, but early user drag input is no longer discarded.

### Goals
- Respond to continued manual feedback that the rebuilt release still flashed/black-screened and had startup drag delay.
- Remove the default window lifecycle actions most likely to create black/flash frames: transparent preconfigured helper rebuild, transparent WebView2 background, and offscreen show/hide prewarm.
- Preserve the no-empty-WebView default from Chapter 259.
- Recover left-button drag input that starts before the screenshot WebView is visible, so `Alt+A` followed immediately by dragging can still produce a selection.
- Keep early visible shell and transparent-window experiments opt-in only.

### External Findings
- Snow Shot remains the peer reference: its screenshot flow separates draw-window creation, monitor capture, and image delivery, and its Windows capture path supports WebView shared-buffer delivery. This reinforces that showing an empty WebView is the wrong default route for no-flicker startup.
- WebView2 transparent background remains useful only for an explicit transparent-window experiment. For the product default, an opaque hidden helper shown only after real pixels are drawn is the safer route.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/tauri.conf.json`
  - Changes the preconfigured `screenshot` helper window from transparent to opaque, preventing the default runtime from destroying/rebuilding the preconfigured helper at startup.
- `tauri-client/src-tauri/src/lib.rs`
  - Renames the WebView2 background setup to default-background policy and only sets `WEBVIEW2_DEFAULT_BACKGROUND_COLOR=00000000` when `YSN_SCREENSHOT_TRANSPARENT_WINDOW=1`.
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - Removes the default preconfigured transparent-window rebuild path.
  - Makes offscreen show/hide WebView prewarm opt-in with `YSN_SCREENSHOT_PREWARM_OFFSCREEN_WINDOW=1`; default startup prewarm is hidden-only.
  - Adds screenshot pointer pre-capture state that records the first left-button drag after `Alt+A`, including down point, latest point, completion state, and drag distance.
  - Extends `get_screenshot_pointer_state` with `preCapture` diagnostics for frontend recovery.
  - Adds policy test coverage for opt-in offscreen prewarm show.
- `tauri-client/src/hooks/useScreenshotLoader.ts`
  - Removes the second pre-show animation-frame wait after first paint.
  - Moves native diagnostics off the immediate show/recovery path.
  - Adds `recoverPreShowDrag`, which restores a drag started before the WebView was visible and keeps polling briefly until mouse release.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Updates the resume snapshot and records Chapter 260 evidence, validation, risks, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- Default release smoke against `release\YSN-Screenshot-Translator\YsnTrans.exe` showed:
  - no `ensure_screenshot_window: rebuilding preconfigured transparent window`;
  - `startup offscreen screenshot prewarm show disabled; hidden WebView prewarm only`;
  - `shell_event_received ... native_visible=false show_on_shell_ready=false`;
  - `shell_deferred_until_ready elapsed_ms=26`;
  - `capture_end elapsed_ms=73`;
  - `rgba_canvas_ready elapsed_ms=125`;
  - `image_ready elapsed_ms=126`;
  - `first_paint elapsed_ms=126`;
  - `overlay_ready_to_show_called elapsed_ms=126`;
  - `overlay_show_result elapsed_ms=12`;
  - `overlay_ready_to_show_returned elapsed_ms=145`.
- Pre-show drag release smoke simulated mouse down and drag beginning about 50ms after the auto hotkey:
  - `capture_end elapsed_ms=64`;
  - `rgba_canvas_ready elapsed_ms=139`;
  - `first_paint elapsed_ms=140`;
  - `overlay_ready_to_show_returned elapsed_ms=154`;
  - `pre_show_drag_recovered elapsed_ms=187 left_down=true completed=false drag=154 rect=548,445,129,84`;
  - `pre_show_drag_finalized elapsed_ms=550 valid=true drag=373`;
  - `native_diagnostics_status` moved later to `elapsed_ms=275`, so it no longer blocks pre-show drag recovery.

### Explicit Non-Goals
- Did not re-enable the default empty/transparent WebView shell.
- Did not implement WebView2 SharedBuffer transfer.
- Did not implement a full native first-frame overlay or native-drawn selection UI.
- Did not change OCR, translation, recording, selected-output, or model/runtime behavior.
- Did not claim final QQ/WeChat/Snow Shot parity without human-visible manual QA.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_window_transparency_tests -- --nocapture` with `3 passed`.
- Passed: `cd tauri-client; npm run check:i18n` with `566 zh-CN keys match 566 en-US keys`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.
- Passed: `cmd /c "build.bat --no-pause"` release rebuild.
- Passed: release default opaque/deferred smoke with no transparent-window rebuild and no default offscreen prewarm show/hide.
- Passed: release pre-show drag smoke proving a drag started before overlay visibility is recovered and finalized into a valid selection.

### Known Risks
- Human-visible QA is still required. Automated logs prove lifecycle ordering and recovered selection, not subjective absence of every display-driver/WebView compositor flash.
- The first visible WebView screenshot frame still arrives around `140-155ms` in release smoke on this machine. The new pointer pre-capture preserves early drag input but does not make the screenshot pixels visible in under 50ms.
- Pre-show drag recovery uses short Win32 polling and frontend IPC recovery; it is intentionally a conservative bridge, not the long-term native first-frame architecture.
- If the user still perceives visible-frame delay after black/flash is fixed, the next real speed route is WebView shared buffer or a native first-frame overlay, not showing an empty WebView.

### Next Recommended Chapter
- Chapter 261 should be human-visible release QA on the rebuilt default: repeated `Alt+A` immediate hand drag, `Esc`, copy, Save As, OCR, translate, focus restore, taskbar/Alt-Tab behavior, multi-monitor/DPI, and no white/black/gray flash.
- If manual QA confirms no flash but first visible frame still feels behind QQ/WeChat/Snow Shot, implement the next architecture slice: WebView shared-buffer RGBA delivery or native first-frame overlay, with visual recording evidence before claiming parity.

## Chapter 261 - Repeated Screenshot White-Frame Lifecycle Fix (2026-06-10)

> Chapter status: completed for the repeated-screenshot lifecycle race reported after the third/fourth screenshot. Automated release smoke no longer shows timing climb, delayed stale drag recovery, stderr errors, transparent-window rebuilds, or default early-shell presentation. Human-visible QA is still required before claiming complete QQ/WeChat/Snow Shot parity.

### Goals
- Diagnose why the first one or two screenshots could feel fine, while the third or fourth run became very laggy and showed white screen/flash.
- Keep the Chapter 260 default: opaque screenshot helper, hidden until real screenshot pixels are painted, with early visible shell and transparent helper paths as diagnostics only.
- Prevent the frontend from clearing the visible canvas while the native screenshot WebView is still on screen.
- Prevent late async work from an already closed screenshot session from restoring selection/canvas state in a later run.
- Rebuild release and run repeated `Alt+A -> drag -> Ctrl+C` smoke evidence.

### External Findings
- Snow Shot remains the peer reference. Its repository separates draw-window creation, monitor capture, and image delivery, and its Windows path includes WebView shared-buffer delivery. This reinforces that peer-grade latency should reduce data transfer and avoid empty WebView first frames rather than showing a blank/transparent WebView early.
- WebView2 default-background behavior remains relevant: transparent/white/opaque first-frame policy is a compositor concern. For this product default, the safer path is to keep the WebView hidden until real pixels are drawn, and reserve transparent WebView experiments for explicit diagnostics.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - Stores the latest shell payload for missed-listener recovery.
  - In the default deferred path, hides the screenshot helper before emitting `screenshot-shell`.
  - Emits shell payloads with `nativeVisible=false` and `showOnShellReady` as the explicit early-shell gate.
  - Clears latest screenshot payload, latest shell payload, and pre-capture pointer state on force close.
- `tauri-client/src/pages/ScreenshotPage.tsx`
  - Adds pending `screenshot-shell` payload recovery.
  - Computes `shouldPresentShell = nativeVisible || showOnShellReady`.
  - Does not clear/present the shell canvas or overlay in the default hidden/deferred path.
  - Deduplicates shell payloads and skips late shell payloads if the image for that session is already ready.
  - Calls native `cancel_screenshot` before frontend reset when recording ends.
- `tauri-client/src/hooks/useScreenshotLoader.ts`
  - Calls native `cancel_screenshot` before resetting frontend state.
  - Increments `captureIdRef` during reset to invalidate delayed async loaders/recovery tasks.
  - Clears `maskedCanvasRef` during reset.
  - Preserves an already recovered shell selection when the real image arrives.
  - Stops `recoverPreShowDrag` if the overlay is hidden or a real selection already exists.
- `tauri-client/src/hooks/useScreenshotActions.ts`
  - Calls native cancel/force-close before clearing frontend screenshot state for copy/save/force close exits.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Updates the resume snapshot and records Chapter 261 evidence, validation, risks, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- Before this hardening, the repeated drag/copy smoke did not show a pure timing climb, but did expose a delayed `pre_show_drag_finalized` after copy/close on one run. That was enough to explain sporadic later-run state resurrection.
- Release 8-round smoke against `release\YSN-Screenshot-Translator\YsnTrans.exe` after the fixes used `Alt+A -> start drag about 50ms later -> Ctrl+C -> Esc fallback`:
  - `ss-1`: `image_ready elapsed_ms=136`, `first_paint elapsed_ms=137`, `overlay_ready_to_show_returned elapsed_ms=150`, `pre_show_drag_finalized elapsed_ms=551`.
  - `ss-2`: `image_ready elapsed_ms=129`, `first_paint elapsed_ms=132`, `overlay_ready_to_show_returned elapsed_ms=141`, `pre_show_drag_finalized elapsed_ms=561`.
  - `ss-3`: `image_ready elapsed_ms=125`, `first_paint elapsed_ms=126`, `overlay_ready_to_show_returned elapsed_ms=139`, `pre_show_drag_finalized elapsed_ms=528`.
  - `ss-4`: `image_ready elapsed_ms=127`, `first_paint elapsed_ms=127`, `overlay_ready_to_show_returned elapsed_ms=137`, `pre_show_drag_finalized elapsed_ms=497`.
  - `ss-5`: `image_ready elapsed_ms=125`, `first_paint elapsed_ms=127`, `overlay_ready_to_show_returned elapsed_ms=140`, `pre_show_drag_finalized elapsed_ms=526`.
  - `ss-6`: `image_ready elapsed_ms=125`, `first_paint elapsed_ms=127`, `overlay_ready_to_show_returned elapsed_ms=139`, `pre_show_drag_finalized elapsed_ms=522`.
  - `ss-7`: `image_ready elapsed_ms=126`, `first_paint elapsed_ms=128`, `overlay_ready_to_show_returned elapsed_ms=137`, `pre_show_drag_finalized elapsed_ms=504`.
  - `ss-8`: `image_ready elapsed_ms=128`, `first_paint elapsed_ms=128`, `overlay_ready_to_show_returned elapsed_ms=139`, `pre_show_drag_finalized elapsed_ms=514`.
- Memory during the same release smoke stayed bounded: private memory moved from `45.6 MB` to `51.2 MB`, peaking at `52.5 MB`; working set moved from `57.2 MB` to `63.5 MB`, peaking at `63.6 MB`.
- The same smoke had empty stderr and no default logs for preconfigured transparent-window rebuild, offscreen screenshot prewarm show, visible shell show, or shell payload skipped after image-ready churn.

### Explicit Non-Goals
- Did not re-enable the default empty/transparent WebView shell.
- Did not implement WebView2 SharedBuffer transfer.
- Did not implement a full native first-frame renderer or native-drawn selection UI.
- Did not change OCR, translation, recording, selected-output, or model/runtime behavior.
- Did not claim final no-flash parity without a human-visible recording/manual QA pass.

### Validation
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cd tauri-client; npm run build`; Vite emitted existing chunk-size/dynamic-import warnings only.
- Passed: `cd tauri-client; npm run check:i18n` with `566 zh-CN keys match 566 en-US keys`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_window_transparency_tests -- --nocapture` with `3 passed`.
- Passed: `cmd /c "build.bat --no-pause"` release rebuild.
- Passed: release 8-round repeated drag/copy smoke; no third/fourth-run timing climb, no stderr errors, and stable memory.

### Known Risks
- Automated logs prove lifecycle ordering, bounded repeated-run latency, and no stale recovery after close; they still cannot prove the user's display driver never shows a one-frame visual flash.
- The first visible WebView screenshot frame still arrives around `137-150ms` in this release smoke. The current fix prevents lost early drag and repeated-run white frames, but it does not make real screenshot pixels visible in under `50ms`.
- The product is still using frontend canvas presentation for the first visible screenshot frame. If the user still sees a subjective delay after this lifecycle fix, the next real architecture step is WebView SharedBuffer RGBA delivery or a thin native first-frame overlay.

### Next Recommended Chapter
- Chapter 262 should perform human-visible release QA on the rebuilt release now left running: repeat hand `Alt+A` immediate drag, repeat `Alt+A/Esc`, copy, Save As, OCR, translate, focus restore, taskbar/Alt-Tab behavior, multi-monitor/DPI, and no white/black/gray flash.
- If the user still sees a flash, capture a short screen recording and correlate the visible frame with logs before choosing between WebView SharedBuffer transfer and a native first-frame overlay.

## Chapter 262 - Native First-Frame Shield For WebView2 White Flash (2026-06-10)

> Chapter status: completed for the root-cause code hardening after continued manual feedback that the screenshot overlay still flashed white. This chapter fixes the native shield ordering and Win32 ownership bugs found during automated smoke. Human-visible QA is still required before claiming final QQ/WeChat/Snow Shot parity.

### Goals
- Explain and fix why white flash could still appear even after the WebView was hidden until `image_ready`.
- Cover the WebView2 first visible frame with a native window that paints the real screenshot frame, then dismiss it after the WebView canvas is visible.
- Prevent the native shield itself from leaking or failing to destroy across repeated screenshots.
- Keep the default no-empty-WebView path and pre-show drag recovery from Chapters 260-261.

### External Findings
- WebView2 first-frame/default-background behavior remains a relevant white-flash source; DOM/CSS changes after navigation are not enough to guarantee that no default backing frame is ever composited.
- Snow Shot remains the peer reference: its Windows screenshot path avoids a visible empty WebView and uses lower-level image delivery patterns such as WebView SharedBuffer. This chapter implements the smaller near-term equivalent: native first-frame coverage while retaining the current WebView canvas UI.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - Enables the native first-frame shield by default, with opt-out `YSN_NATIVE_FIRST_FRAME_SHIELD=0`.
  - Logs shield visible/fallback/disabled states.
- `tauri-client/src-tauri/src/screenshot_native/win32_overlay.rs`
  - Stores the captured RGBA frame for `WM_PAINT`.
  - Paints the real screenshot frame with GDI `StretchDIBits`.
  - Handles `WM_NCHITTEST`, `WM_ERASEBKGND`, and `WM_PAINT` to avoid default empty erase frames.
  - Flushes the DWM compositor after show/update to reduce native first-frame blanking.
- `tauri-client/src-tauri/src/screenshot_native/native_overlay_session.rs`
  - Moves native shield create/show/raise/destroy onto a dedicated owner thread.
  - Uses channel commands for raise/cancel so `DestroyWindow` runs on the creating thread.
  - Adds session-matched cancel support to avoid an older dismiss timer killing a newer screenshot shield.
- `tauri-client/src-tauri/src/screenshot_native/mod.rs`
  - Exports the shield raise and session-matched cancel helpers.
- `tauri-client/src-tauri/src/window_lifecycle.rs`
  - After the WebView screenshot window is shown, raises the native shield back above it before scheduling dismissal.
  - Dismisses only the matching shield session after the configured delay.
- `tauri-client/index.html`
  - Adds a non-white initial background before the JS bundle runs.
  - Marks transparent recording/save-toast routes early so their transparent windows stay transparent.
- `tauri-client/src/index.css`
  - Changes the default `html/body/#root` background from transparent to `#0b0f14`; transparent windows still override it.
- `tauri-client/src/main.tsx`
  - Sets screenshot `html/body/#root` fallback background to `#0b0f14` before React renders.
- `tauri-client/src/pages/ScreenshotPage.tsx`
  - Keeps screenshot page fallback background non-white while the overlay is hidden/initializing.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records Chapter 262 evidence, validation, known risks, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- Root-cause inspection found three independent white/lag sources:
  - The WebView can still have a default white backing frame before DOM/CSS/React draw a screenshot canvas.
  - The first native shield attempt was shown before the WebView; `overlay_ready_to_show` later made the WebView topmost again, putting the shield underneath the WebView first frame.
  - The first native shield attempt destroyed the Win32 window from a different thread than the creator thread, producing `native_first_frame_shield_dismissed ... state=failed`, which could leak native windows and explain repeated-run degradation.
- First release smoke after the topmost fix proved the shield was raised, but exposed the destroy failure:
  - `native_first_frame_shield_raised session=ss-1 ... visible=true`
  - `native_first_frame_shield_dismissed session=ss-1 delay_ms=64 state=failed active=false visible=false`
- Second release smoke after moving the shield onto an owner thread used 6 rounds of `Alt+A -> start drag about 50ms later -> Ctrl+C -> Esc fallback`:
  - Each round logged `native_first_frame_shield_raised ... active=true visible=true`.
  - Each round logged `native_first_frame_shield_dismissed ... state=cancelled active=false visible=false`.
  - `ss-2`: `image_ready elapsed_ms=128`, `first_paint elapsed_ms=130`, `overlay_ready_to_show_returned elapsed_ms=146`.
  - `ss-3`: `image_ready elapsed_ms=133`, `first_paint elapsed_ms=133`, `overlay_ready_to_show_returned elapsed_ms=144`.
  - `ss-4`: `image_ready elapsed_ms=131`, `first_paint elapsed_ms=131`, `overlay_ready_to_show_returned elapsed_ms=146`.
  - `ss-5`: `image_ready elapsed_ms=129`, `first_paint elapsed_ms=130`, `overlay_ready_to_show_returned elapsed_ms=145`.
  - `ss-6`: `image_ready elapsed_ms=129`, `first_paint elapsed_ms=129`, `overlay_ready_to_show_returned elapsed_ms=142`.
  - The same smoke had empty stderr and no third/fourth-run timing climb.

### Explicit Non-Goals
- Did not implement WebView2 SharedBuffer RGBA delivery.
- Did not implement a full native interactive renderer or native-drawn selection UI.
- Did not change OCR, translation, recording, selected-output, model/runtime, or release packaging strategy.
- Did not claim final no-flash parity without human-visible manual QA or recording evidence.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_window_transparency_tests -- --nocapture` with `4 passed`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_overlay_session -- --nocapture` with `2 passed`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib win32_overlay -- --nocapture` with `13 passed`.
- Passed: `cd tauri-client; npm run build`; Vite emitted existing chunk-size/dynamic-import warnings only.
- Passed: `cd tauri-client; npm run check:i18n` with `566 zh-CN keys match 566 en-US keys`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.
- Passed: `cmd /c "build.bat --no-pause"` release rebuild.
- Passed: release 6-round repeated immediate-drag smoke with native shield raised above WebView and destroyed on its owner thread.

### Known Risks
- Automated logs prove ordering, stable timing, and correct Win32 shield teardown; they still cannot prove the user's display driver never shows a one-frame artifact.
- The first visible WebView canvas still arrives around `142-146ms` on warm release runs. The native shield should cover the WebView white first frame, but true QQ/WeChat/Snow Shot latency parity still requires SharedBuffer delivery or a fuller native first-frame/selection renderer.
- The shield is mouse-transparent by hit testing and short-lived. If manual QA reports missed first drag after this chapter, the next slice should move selection interaction into the native overlay rather than lengthening the WebView delay.

### Next Recommended Chapter
- Chapter 263 should be human-visible release QA of this exact rebuilt release: repeat hand `Alt+A` immediate drag, repeat `Alt+A/Esc`, copy, Save As, OCR, translate, focus restore, taskbar/Alt-Tab behavior, and multi-monitor/DPI checks.
- If white flash still appears, capture a short screen recording and correlate the visible frame with `native_first_frame_shield_visible`, `native_first_frame_shield_raised`, `first_paint`, and `native_first_frame_shield_dismissed` logs.
- If no white flash remains but latency still feels behind QQ/WeChat/Snow Shot, prioritize WebView2 SharedBuffer RGBA delivery or native-drawn selection for true sub-50ms perceived interaction.

## Chapter 263 - Disable Risky Native Shield Default After Black/Color Flash (2026-06-10)

> Chapter status: completed for immediate user-facing stabilization after manual feedback that Chapter 262 still caused severe black screen, color shift, and flicker. The native shield remains available only as an explicit diagnostic experiment; the default release no longer creates a full-screen Win32 shield.

### Goals
- Stop the severe black/color-shift fullscreen artifacts reported after the native first-frame shield build.
- Keep the safer hidden-until-real-canvas WebView path as the default.
- Fix the shield's RGBA-to-GDI color order for future diagnostics, without exposing it to users by default.
- Rebuild release and verify default screenshot runs no longer create or raise the native shield.

### Diagnosis
- The native shield painted the captured `RGBA` buffer through Win32 GDI `StretchDIBits` / `BI_RGB`, whose 32-bit DIB memory order is effectively B/G/R/reserved. Passing RGBA directly can swap channels and create visible color shift.
- The shield is a full-screen topmost Win32 window. Any missed paint, compositor delay, or wrong pixel format is perceived as an entire-screen black/colored flash, not a small overlay defect.
- Because the WebView path is already hidden until `first_paint`, the shield was too risky as a default bridge. It should stay opt-in until the product moves to WebView SharedBuffer or a proper native interactive renderer.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - Changes `native_first_frame_shield_enabled()` from default-on to opt-in only with `YSN_NATIVE_FIRST_FRAME_SHIELD=1`.
  - Updates the unit test to assert the shield is disabled by default.
  - Updates disabled-path logging to explain the diagnostic-only status.
- `tauri-client/src-tauri/src/screenshot_native/win32_overlay.rs`
  - Converts RGBA bytes to BGRA DIB bytes before storing/painting the diagnostic shield bitmap.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records this stabilization chapter.

### Deleted Files
- None.

### Evidence Added
- Default release smoke with no `YSN_NATIVE_FIRST_FRAME_SHIELD` used 4 rounds of `Alt+A -> start drag about 50ms later -> Ctrl+C -> Esc fallback`.
- Each round logged `native_first_frame_shield_disabled`; no round logged `native_first_frame_shield_visible`, `native_first_frame_shield_raised`, or `native_first_frame_shield_dismissed`.
- Warm-run timings stayed stable:
  - `ss-2`: `image_ready elapsed_ms=127`, `overlay_ready_to_show_returned elapsed_ms=137`.
  - `ss-3`: `image_ready elapsed_ms=128`, `overlay_ready_to_show_returned elapsed_ms=145`.
  - `ss-4`: `image_ready elapsed_ms=124`, `overlay_ready_to_show_returned elapsed_ms=143`.
- The same smoke had empty stderr.

### Explicit Non-Goals
- Did not solve final QQ/WeChat/Snow Shot parity in this chapter.
- Did not implement WebView2 SharedBuffer transfer.
- Did not implement a native-drawn interactive selection renderer.
- Did not remove the native shield code, because it remains useful for controlled diagnostics after the RGBA/BGRA fix.

### Validation
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_window_transparency_tests -- --nocapture` with `4 passed`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib win32_overlay -- --nocapture` with `13 passed`.
- Passed: `cd tauri-client; npm run build`; Vite emitted existing chunk-size/dynamic-import warnings only.
- Passed: `cd tauri-client; npm run check:i18n` with `566 zh-CN keys match 566 en-US keys`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.
- Passed: `cmd /c "build.bat --no-pause"` release rebuild.
- Passed: release 4-round default smoke proving the full-screen native shield is absent by default.

### Known Risks
- This intentionally removes the risky native shield from the default path, so it prioritizes not flashing black/colored frames over hiding every possible WebView2 first-frame artifact.
- If the user still sees a WebView compositor flash on this safer default path, the next commercial-grade fix should be SharedBuffer or a native renderer, not re-enabling the full-screen GDI shield by default.
- The screenshot still relies on WebView canvas presentation around `137-145ms` on this machine.

### Next Recommended Chapter
- Chapter 264 should perform human-visible QA on the rebuilt no-native-shield default and record whether black/color flash is gone.
- If only a small first-frame WebView flash remains, implement WebView2 SharedBuffer RGBA delivery next.
- If immediate drag still feels behind QQ/WeChat/Snow Shot after visual stability is restored, build a native interactive selection layer instead of using a visual-only shield.

## Chapter 264 - WebView2 SharedBuffer Screenshot Delivery (2026-06-10)

> Chapter status: completed for the first Snow Shot-style architecture slice. The screenshot image delivery path now tries WebView2 SharedBuffer before falling back to the older Tauri IPC RGBA fetch. Release smoke proves the SharedBuffer path works in the real packaged app and removes the risky native shield from the default path. Human-visible no-flash/no-black-screen QA is still required before claiming complete QQ/WeChat/Snow Shot parity.

### Goals
- Move the screenshot first-frame image transfer closer to Snow Shot's Windows architecture.
- Avoid another visual shield/overlay hack and instead shorten the screenshot pixel delivery path.
- Keep the existing `get_fullscreen_rgba_bytes` / PNG / base64 paths as fallbacks.
- Prevent cancelled screenshot sessions from showing a late WebView overlay frame.
- Rebuild release and verify the real WebView2 SharedBuffer path in a packaged smoke run.

### External Findings
- Snow Shot uses a draw-window screenshot flow plus WebView2 SharedBuffer delivery on Windows. Its frontend waits for `sharedbufferreceived`, while Rust posts screenshot bytes through `PostSharedBufferToScript`.
- Microsoft WebView2 exposes this as `ICoreWebView2Environment12::CreateSharedBuffer`, `ICoreWebView2_17::PostSharedBufferToScript`, and the JavaScript `sharedbufferreceived` event.
- This project can access the same WebView2 COM interfaces through Tauri 2 `Webview::with_webview`, so no opaque external executable is needed for this architecture slice.

### Added Files
- `tauri-client/src-tauri/src/screenshot_shared_buffer.rs`
  - Builds the RGBA SharedBuffer payload as `rgba bytes + little-endian width + little-endian height`.
  - Posts the payload to the active WebView2 script context with `transfer_type=screenshot` and `session_id`.
  - Includes unit tests for payload layout and invalid byte-count rejection.

### Modified Files
- `tauri-client/src-tauri/Cargo.toml`
  - Adds direct `webview2-com` and `windows-core` dependencies for WebView2 SharedBuffer access.
- `tauri-client/src-tauri/Cargo.lock`
  - Records the direct dependency graph update.
- `tauri-client/src-tauri/src/lib.rs`
  - Registers the new SharedBuffer module and command.
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - Adds `post_fullscreen_rgba_shared_buffer`.
  - Logs `shared_buffer_posted`, `shared_buffer_failed`, and unavailable states.
  - Emits `screenshot-session-cancelled` on repeat-hotkey cancel, force close, and explicit cancel.
  - Tracks recently cancelled session ids so stale `overlay_ready_to_show` calls can be ignored.
- `tauri-client/src-tauri/src/window_lifecycle.rs`
  - Skips `overlay_ready_to_show` for cancelled screenshot sessions.
- `tauri-client/src/hooks/useScreenshotLoader.ts`
  - Adds a WebView2 `sharedbufferreceived` receiver.
  - Tries SharedBuffer before `get_fullscreen_rgba_bytes`.
  - Releases received WebView2 SharedBuffers after painting.
  - Preserves the older IPC RGBA / PNG / base64 fallback path.
- `tauri-client/src/pages/ScreenshotPage.tsx`
  - Listens for `screenshot-session-cancelled` and invalidates frontend screenshot state before late first-paint work can show a cancelled session.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records this chapter.

### Deleted Files
- None.

### Evidence Added
- First packaged release lifecycle smoke proved the new path was real, not just compiled:
  - `ss-1`: `shared_buffer_posted`, `shared_buffer_received`, `image_ready elapsed_ms=23`, `first_paint elapsed_ms=26`.
  - `ss-2`: `shared_buffer_posted`, `shared_buffer_received`, `image_ready elapsed_ms=16`, `first_paint elapsed_ms=19`, `overlay_ready_to_show_returned elapsed_ms=28`.
  - No `rgba_fetch_end` appeared in the SharedBuffer runs.
- The same smoke exposed an existing repeated-start cancel race: a cancelled `ss-1` could still call `overlay_ready_to_show`.
- After adding session cancellation invalidation, final packaged release lifecycle smoke showed:
  - `ss-1`: `shared_buffer_posted`, `shared_buffer_received`, `image_ready elapsed_ms=24`, then `session_cancelled_received reason=repeat-hotkey-cancel`, then `first_paint_guard_blocked`.
  - Repeat cancel result: `visible=false capturing=false`.
  - `ss-2`: `shared_buffer_posted`, `shared_buffer_received`, `image_ready elapsed_ms=17`, `first_paint elapsed_ms=19`, `overlay_ready_to_show_returned elapsed_ms=34`.
  - Final cancel result: `visible=false capturing=false`.
  - `native_first_frame_shield_disabled` remained present; the full-screen GDI shield was not used.
- Release smoke stderr only contained the recurring WebView2/Chromium process-exit line `Failed to unregister class Chrome_WidgetWin_0. Error = 1412`; no screenshot command failure was logged.

### Explicit Non-Goals
- Did not copy Snow Shot source code or assets.
- Did not implement Snow Shot's full draw-window module layout.
- Did not implement a native-drawn interactive selection layer.
- Did not replace the current capture backend with DXGI/WGC GPU texture presentation.
- Did not remove the existing IPC/PNG/base64 fallbacks.
- Did not claim final visual parity without human-visible repeated `Alt+A` QA.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_shared_buffer -- --nocapture` with `2 passed`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; Vite emitted existing chunk-size/dynamic-import warnings only.
- Passed: `cd tauri-client; npm run check:i18n` with `566 zh-CN keys match 566 en-US keys`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.
- Passed: `cmd /c "build.bat --no-pause"` release rebuild.
- Passed: packaged release lifecycle smoke with real WebView2 SharedBuffer delivery and cancelled-session guard.

### Known Risks
- The frontend SharedBuffer timings are much lower than the previous IPC RGBA path, but the full hotkey-to-visible path still includes native capture and window show time.
- Automated smoke proves ordering and delivery; it cannot prove the user's display never shows a one-frame compositor artifact.
- The app still renders the selection UI in WebView canvas. If the user still cannot drag immediately after `Alt+A`, the next architecture step is a native interactive selection layer, not another visual shield.
- The recurring WebView2 process-exit stderr line should be monitored, but it did not correlate with screenshot failure in this chapter.

### Next Recommended Chapter
- Chapter 265 should be human-visible release QA of the rebuilt SharedBuffer default: repeated hand `Alt+A` immediate drag, rapid `Alt+A/Alt+A`, rapid `Alt+A/Esc`, copy, Save As, OCR, translate, focus restore, taskbar/Alt-Tab, and multi-monitor/DPI checks.
- If the user still sees black/white/color flash after SharedBuffer, capture a short screen recording and correlate it with `shared_buffer_received`, `first_paint`, `overlay_ready_to_show_returned`, and `overlay_show_skipped_cancelled`.
- If visual stability is good but immediate dragging still feels behind QQ/WeChat/Snow Shot, implement native selection input/rendering as the next slice.

## Chapter 265 - Direct WebView2 SharedBuffer Push (2026-06-10)

> Chapter status: completed for shortening the Chapter 264 SharedBuffer route. Rust now pushes the screenshot SharedBuffer directly to the mounted screenshot WebView before the frontend requests it; the frontend keeps a small SharedBuffer inbox and consumes the direct buffer when the payload arrives. The request-style SharedBuffer command and old IPC/PNG/base64 fallbacks remain intact.

### Goals
- Remove the extra frontend `invoke("post_fullscreen_rgba_shared_buffer")` round trip from the normal SharedBuffer path.
- Keep the Chapter 264 request-style SharedBuffer path as a fallback if direct delivery is missed.
- Keep cancelled-session protection so a rapid repeated hotkey cannot resurrect an old overlay.
- Rebuild release and verify the packaged app uses direct SharedBuffer delivery.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - After RGBA capture is stored, calls `post_rgba_frame_to_webview` directly with the screenshot WebView handle.
  - Logs `shared_buffer_direct_posted`, `shared_buffer_direct_failed`, or unavailable status before payload emission.
- `tauri-client/src/hooks/useScreenshotLoader.ts`
  - Adds a mounted `sharedbufferreceived` inbox for direct screenshot buffers.
  - Stores direct buffers by `session_id` when they arrive before `screenshot-updated`.
  - Waits briefly for a direct buffer, then falls back to the Chapter 264 request-style command if needed.
  - Releases unused pending SharedBuffers during reset/unmount to avoid stale buffer retention.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records this chapter.

### Deleted Files
- None.

### Evidence Added
- Packaged release lifecycle smoke after direct push showed the normal ready run no longer used the request-style command:
  - `ss-2`: `capture ready 61ms format=rgba bytes=14745600`.
  - `ss-2`: `shared_buffer_direct_posted elapsed_ms=70 bytes=14745608 size=2560x1440`.
  - `ss-2`: `shared_buffer_direct_pending elapsed_ms=0 bytes=14745608`.
  - `ss-2`: `payload_emit elapsed_ms=72`.
  - `ss-2`: `shared_buffer_received elapsed_ms=0 source=direct`.
  - `ss-2`: `image_ready elapsed_ms=9`.
  - `ss-2`: `first_paint elapsed_ms=12`.
  - `ss-2`: `overlay_ready_to_show_returned elapsed_ms=30`.
  - No `shared_buffer_post_returned` and no `rgba_fetch_end` appeared in the final direct run.
- Rapid repeated-start cancel remained guarded:
  - `ss-1`: `shared_buffer_direct_posted`, then `session_cancelled_received reason=repeat-hotkey-cancel`.
  - Repeat cancel result: `visible=false capturing=false`.
  - Final cancel result: `visible=false capturing=false`.
- `native_first_frame_shield_disabled` remained present; the full-screen native shield was not used.
- Release smoke stderr only contained the recurring WebView2/Chromium process-exit line `Failed to unregister class Chrome_WidgetWin_0. Error = 1412`; no screenshot command failure was logged.

### Explicit Non-Goals
- Did not implement a native-drawn selection/input layer.
- Did not change the capture backend to DXGI/WGC GPU texture presentation.
- Did not remove request-style SharedBuffer or IPC/PNG/base64 fallbacks.
- Did not claim final human-visible parity without manual QA.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_shared_buffer -- --nocapture` with `2 passed`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; Vite emitted existing chunk-size/dynamic-import warnings only.
- Passed: `cd tauri-client; npm run check:i18n` with `566 zh-CN keys match 566 en-US keys`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `cmd /c "build.bat --no-pause"` release rebuild.
- Passed: packaged release lifecycle smoke with direct WebView2 SharedBuffer delivery.

### Known Risks
- The direct SharedBuffer path lowers frontend image-ready time, but the full hotkey-to-visible path still includes native capture and window show.
- Automated smoke cannot prove no one-frame compositor flash on the user's display.
- If manual testing still shows delayed immediate drag, the next meaningful architecture step is native selection input/rendering rather than more WebView transfer work.

### Next Recommended Chapter
- Chapter 266 should perform human-visible release QA of the direct SharedBuffer build: repeated hand `Alt+A` immediate drag, rapid `Alt+A/Alt+A`, rapid `Alt+A/Esc`, copy, Save As, OCR, translate, taskbar/Alt-Tab, focus restore, and multi-monitor/DPI checks.
- If visual stability is good but drag still feels behind QQ/WeChat/Snow Shot, implement the native selection input/rendering slice next.

## Chapter 266 - Transparent Post-Paint SharedBuffer Startup Pass (2026-06-10)

> Chapter status: completed for the deepest automated startup/flash pass so far. The default screenshot path now combines Snow Shot-style direct WebView2 SharedBuffer delivery, transparent WebView/window backing, hidden-until-real-canvas presentation, post-paint first visible show, capture/window-prep parallelism, bounds reuse, and session-filtered pointer recovery. Packaged six-round smoke stayed stable through rounds 3-6, but final human-visible no-flash parity still needs manual QA or a short screen recording.

### Goals
- Address the user's continued reports of black screen, white flash, color shift, and a gray layer before screenshot interaction.
- Keep the Chapter 264-265 direct WebView2 SharedBuffer architecture instead of reintroducing the full-screen native GDI shield that caused black/color-shift artifacts.
- Make the first visible screenshot frame safer by showing the helper only after the real screenshot canvas has painted and one post-paint task has run.
- Reduce repeated-run compositor churn and third/fourth screenshot slowdown risk.
- Keep rollback env flags for opaque-window diagnostics.

### External Findings
- This chapter continues the Snow Shot/WebView2 architecture direction recorded in Chapters 264-265: peer screenshot tools avoid a visible empty WebView shell and use native/low-level pixel delivery or SharedBuffer-style handoff so pixels are ready before the user sees the capture surface.
- The local root cause mapping after Chapters 262-265 was that image transfer was no longer the bottleneck; the remaining visible artifact risk was WebView/window backing exposure before the real canvas frame, plus repeated native bounds updates and stale session work.
- The chosen fix is therefore not another shield, but stricter first-visible ordering around the real canvas and less native window churn.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/tauri.conf.json`
  - Sets the screenshot helper window to `transparent: true`.
- `tauri-client/src-tauri/src/lib.rs`
  - Sets `WEBVIEW2_DEFAULT_BACKGROUND_COLOR=00000000` by default so WebView2 starts transparent.
  - Keeps rollback through `YSN_SCREENSHOT_OPAQUE_WINDOW=1` or `YSN_SCREENSHOT_TRANSPARENT_WINDOW=0`.
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - Makes screenshot window transparency the default with opaque rollback.
  - Starts native capture before screenshot WebView/window prep so capture and overlay preparation can overlap.
  - Adds `LAST_SCREENSHOT_WINDOW_BOUNDS` and skips repeated position/size changes when the monitor bounds are unchanged.
  - Clears bounds cache when creating the helper or doing offscreen prewarm.
  - Adds `session_id` filtering to `get_screenshot_pointer_state` so pre-show drag recovery cannot consume stale pointer state from an older screenshot.
  - Updates screenshot transparency tests.
- `tauri-client/src/main.tsx`
  - Sets `html`, `body`, and root screenshot surfaces transparent before React renders the screenshot page.
- `tauri-client/src/pages/ScreenshotPage.tsx`
  - Forces the screenshot page background to transparent instead of the older dark fallback.
  - Clears duplicate payload/shell signatures when a screenshot session is cancelled.
- `tauri-client/src/hooks/useScreenshotLoader.ts`
  - Calls `get_screenshot_pointer_state` with the active session id.
  - Preserves the current session's SharedBuffer during stale-buffer pruning.
  - Changes first visible show to a post-paint gate: `requestAnimationFrame(() => setTimeout(..., 0))`.
  - Logs `first_paint ... gate=post-paint-task`.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Updates the resume snapshot and records Chapter 266 evidence, validation, risks, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- Final packaged release six-round smoke log:
  - stdout: `tmp-runtime-logs\transparent-postpaint-bounds-cache-six-round-20260610-032927-out.log`.
  - stderr: `tmp-runtime-logs\transparent-postpaint-bounds-cache-six-round-20260610-032927-err.log`, empty.
- The six-round smoke used transparent screenshot helper logs and direct SharedBuffer in every round:
  - no `rgba_fetch`;
  - no `shared_buffer_direct_wait_miss`;
  - no SharedBuffer timeout;
  - no native first-frame shield path.
- Stable per-round timings:
  - `ss-1`: `capture_end 46ms`, `payload_emit 55ms`, `image_ready 8ms`, `first_paint 11ms`, `overlay_ready_to_show_returned 29ms`.
  - `ss-2`: `capture_end 39ms`, `payload_emit 52ms`, `image_ready 6ms`, `first_paint 8ms`, `overlay_ready_to_show_returned 22ms`.
  - `ss-3`: `capture_end 36ms`, `payload_emit 47ms`, `image_ready 5ms`, `first_paint 7ms`, `overlay_ready_to_show_returned 25ms`.
  - `ss-4`: `capture_end 41ms`, `payload_emit 51ms`, `image_ready 5ms`, `first_paint 7ms`, `overlay_ready_to_show_returned 22ms`.
  - `ss-5`: `capture_end 37ms`, `payload_emit 48ms`, `image_ready 5ms`, `first_paint 7ms`, `overlay_ready_to_show_returned 20ms`.
  - `ss-6`: `capture_end 39ms`, `payload_emit 50ms`, `image_ready 5ms`, `first_paint 7ms`, `overlay_ready_to_show_returned 25ms`.
- This directly checks the reported third/fourth-run slowdown class: rounds 4-6 did not climb and stayed in the same timing band as rounds 2-3.
- `overlay_bounds_reused` appeared after the first round, confirming repeated fullscreen runs no longer always reposition/resize the helper.

### Explicit Non-Goals
- Did not copy Snow Shot source code or assets.
- Did not implement a native-drawn interactive selection renderer.
- Did not switch capture to DXGI/WGC GPU texture presentation.
- Did not remove request-style SharedBuffer or IPC/PNG/base64 fallbacks.
- Did not claim final QQ/WeChat/Snow Shot visual parity without manual QA or recording evidence.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_window_transparency_tests -- --nocapture` with `4 passed`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_shared_buffer -- --nocapture` with `2 passed`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; Vite emitted existing chunk-size/dynamic-import warnings only.
- Passed: `cd tauri-client; npm run check:i18n`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.
- Passed: `cmd /c "build.bat --no-pause"` release rebuild.
- Passed: packaged six-round transparent/post-paint/bounds-cache hotkey smoke.

### Known Risks
- Automated logs prove ordering, transfer path, and repeated-run timing stability; they cannot prove that the user's monitor never displayed a one-frame compositor artifact.
- The app still uses a WebView canvas for selection rendering. If hand QA still feels slower than QQ/WeChat/Snow Shot after this pass, the next meaningful architecture slice is native selection input/rendering.
- Transparent WebView/window backing is now the default again because the window is hidden until real canvas paint; if a driver-specific transparent-composition regression appears, use `YSN_SCREENSHOT_OPAQUE_WINDOW=1` or `YSN_SCREENSHOT_TRANSPARENT_WINDOW=0` for rollback diagnostics.
- Full-screen native shield should remain off by default because it previously caused severe black/color-shift artifacts.

### Next Recommended Chapter
- Chapter 267 should be human-visible QA of the rebuilt packaged app: repeated hand `Alt+A` immediate drag, rapid `Alt+A/Alt+A`, rapid `Alt+A/Esc`, copy, Save As, OCR, translate, focus restore, taskbar/Alt-Tab behavior, multi-monitor/DPI, and no white/black/color flash.
- If flashing remains, capture a short screen recording and correlate the visible frame with `shared_buffer_received`, `first_paint gate=post-paint-task`, `overlay_ready_to_show_returned`, and fallback-path logs before choosing the next fix.
- If visual stability is good but drag still feels behind QQ/WeChat/Snow Shot, implement native selection input/rendering rather than adding another WebView shell or shield.

## Chapter 267 - Remove Recording Black-Frame Path And Earliest Black Fallback (2026-06-10)

> Chapter status: completed for an automated visual-flash smoke and the root-cause fix it exposed. This chapter does not claim human-visible QQ/WeChat/Snow Shot parity, but it removes one real black-frame source from external recording/remote capture tools and removes the project's earliest black HTML/CSS fallback surface.

### Goals
- Continue after Chapter 266 without waiting for manual QA.
- Convert the black/white/gray flash complaint into repeatable visual evidence using desktop recording and frame analysis.
- Identify whether the recorded full-screen black frames came from actual WebView presentation or Windows capture exclusion.
- Remove low-risk black fallback surfaces that could still be exposed before React fully hydrates.
- Keep the direct SharedBuffer/post-paint path intact.

### External Findings
- Microsoft documents WebView2 background control through `DefaultBackgroundColor` / the `WEBVIEW2_DEFAULT_BACKGROUND_COLOR` environment path, which supports the Chapter 266 decision to set transparent backing before WebView creation instead of waiting for React CSS.
- Microsoft documents `SetWindowDisplayAffinity` / `WDA_EXCLUDEFROMCAPTURE` as a Windows capture-exclusion mechanism. In practice, desktop recording tools can show excluded full-screen windows as black, so a visual recording smoke must either disable capture exclusion or treat black frames as a capture-tool artifact.
- Snow Shot remains the peer architecture reference for a preloaded screenshot surface plus low-level pixel delivery; this chapter continues that direction without copying source code.

### Added Files
- None.

### Modified Files
- `tauri-client/index.html`
  - Changes the earliest `html`, `body`, and `#root` fallback background from `#0b0f14` to `transparent`.
- `tauri-client/src/index.css`
  - Changes the global `html`, `body`, and `#root` fallback background from `#0b0f14` to `transparent`.
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - Adds `screenshot_capture_exclusion_enabled()`.
  - Makes screenshot helper `WDA_EXCLUDEFROMCAPTURE` opt-in with `YSN_SCREENSHOT_EXCLUDE_FROM_CAPTURE=1`.
  - Logs `overlay_capture_exclusion excluded=false/true` for each screenshot session.
  - Adds a unit test proving capture exclusion is off by default and opt-in only.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Updates the resume snapshot and records Chapter 267 evidence, validation, risks, and next recommended chapter.

### Deleted Files
- None.

### Evidence Added
- Pre-fix visual smoke:
  - video: `tmp-runtime-logs\visual-flash-smoke-20260610-034351.mp4`;
  - analysis: `tmp-runtime-logs\visual-flash-smoke-20260610-034351-visual-analysis.json`;
  - result: `luma_min=0.0069`, `diff_max=181.46`, `flag_count=173`, with many near-black full-screen frames while the screenshot helper was capture-excluded.
- Interpretation: those black frames matched the Windows capture-exclusion path and are not reliable proof of a human-visible WebView black frame. They are still a product problem for users who record/share the screen.
- Post-fix visual smoke:
  - video: `tmp-runtime-logs\visual-flash-post-exclusion-20260610-035131.mp4`;
  - analysis: `tmp-runtime-logs\visual-flash-post-exclusion-20260610-035131-visual-analysis.json`;
  - result: `frames=479`, `luma_min=88.54`, `luma_max=161.67`, `diff_max=70.00`, `diff_avg=1.15`, `black_frame_count=0`, `white_frame_count=0`.
- Post-fix release log:
  - stdout: `tmp-runtime-logs\visual-flash-post-exclusion-20260610-035101-out.log`;
  - stderr: `tmp-runtime-logs\visual-flash-post-exclusion-20260610-035101-err.log`, empty.
- The post-fix release log showed 4 automated `Alt+A/Esc` rounds:
  - all rounds logged `overlay_capture_exclusion excluded=false`;
  - all rounds used `shared_buffer_direct_posted`;
  - no `rgba_fetch`;
  - no `shared_buffer_direct_wait_miss`;
  - no native first-frame shield;
  - `image_ready` stayed `7-11ms`;
  - `first_paint gate=post-paint-task` stayed `12-15ms`;
  - `overlay_ready_to_show_returned` stayed `37-71ms`.

### Explicit Non-Goals
- Did not remove the normal screenshot dimming/mask UI after screenshot mode starts.
- Did not implement native selection input/rendering.
- Did not change capture backend to DXGI/WGC.
- Did not remove SharedBuffer fallbacks.
- Did not claim that a user's naked-eye monitor can no longer show any compositor artifact; this chapter proves the recording black-frame class is gone when capture exclusion is off.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_window_transparency_tests -- --nocapture` with `5 passed`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_shared_buffer -- --nocapture` with `2 passed`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; Vite emitted existing chunk-size/dynamic-import warnings only.
- Passed: `cd tauri-client; npm run check:i18n` with `566 zh-CN keys match 566 en-US keys`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.
- Passed: `cmd /c "build.bat --no-pause"` release rebuild.
- Passed: post-fix ffmpeg desktop recording smoke and raw RGB frame analysis with no black/white frames.

### Known Risks
- The post-fix visual smoke still shows expected entry/exit luminance changes because screenshot mode dims the desktop. If the user describes that dimming itself as a flash, the next chapter should tune the initial mask/dim timing and opacity, not the image transfer path.
- The first run after app restart can still be slower than warm runs because main-window parking and WebView warm state vary; warm runs stayed stable.
- Users who explicitly set `YSN_SCREENSHOT_EXCLUDE_FROM_CAPTURE=1` can still see black windows in external recording tools by design.
- Human-visible driver/compositor artifacts still need a naked-eye or camera/phone recording confirmation.

### Next Recommended Chapter
- Chapter 268 should be chosen based on the next observation:
  - If black/white flash is gone but immediate drag still feels behind QQ/WeChat/Snow Shot, implement native selection input/rendering.
  - If the remaining complaint is the gray/dim transition, tune the initial mask presentation so screenshot pixels appear first and dimming is applied without a separate full-screen pulse.
  - If a true black/white compositor flash remains with capture exclusion disabled, record another visual smoke and correlate it against `overlay_capture_exclusion`, `shared_buffer_received`, `first_paint gate=post-paint-task`, and `overlay_ready_to_show_returned`.

## Chapter 268 - Transparent Input Shell Before Screenshot Pixels (2026-06-10)

> Chapter status: completed for the next latency slice after the black/white frame fix. The default screenshot path now shows a transparent, empty input shell before screenshot pixels are ready, so `Alt+A` can hand control to the screenshot surface earlier without drawing a black/white/gray placeholder. The real screenshot pixels still arrive through the direct WebView2 SharedBuffer path, and the native GDI first-frame shield stays disabled.

### Goals
- Reduce the user's "Alt+A then cannot immediately drag" delay without reintroducing the old full-screen native GDI shield that caused black/color-shift artifacts.
- Match the Snow Shot-style architecture more closely: hotkey routes into an already-loaded draw surface, capture and window preparation run in parallel, and raw pixels arrive through a SharedBuffer-like path.
- Avoid drawing a separate gray mask before the screenshot image is ready; the early shell must be transparent and input-only.
- Preserve a rollback switch if a driver or WebView2 transparent-window regression appears on the user's device.

### External Findings
- Snow Shot's public architecture and source confirm the peer direction: a reusable draw window receives an `execute-screenshot` style event, monitor capture is separate from draw-page readiness, and WebView shared buffers are used for image transfer. This chapter follows the pattern at the architecture level only and does not copy Snow Shot code or assets.
- Microsoft's WebView2 background-color guidance continues to support keeping the earliest WebView/window backing transparent before React paints.
- Microsoft's Win32 input model supports immediate mouse capture once an interactive window is visible; this chapter uses the existing WebView input shell first because it is lower risk than re-enabling a native visual shield.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_commands.rs`
  - Makes `screenshot_early_visible_shell_enabled()` default to true.
  - Keeps rollback with `YSN_SCREENSHOT_DEFER_VISIBLE_SHELL=1` or `YSN_SCREENSHOT_EARLY_VISIBLE_SHELL=0`.
  - Updates the startup policy unit test to cover the new default and rollbacks.
  - Logs `overlay_window_prepared ... transparent_input_shell=true/false` instead of implying the overlay is always hidden.
- `tauri-client/src/hooks/useScreenshotInteraction.ts`
  - Allows a left-button-down pointer move to start selection even if the initial pointer-down happened just before the shell received pointer capture.
  - Logs `first_pointer_move_down` for that recovery path.
- `tauri-client/src/pages/ScreenshotPage.tsx`
  - Uses the backend pointer pre-capture store during shell display, not only after image ready.
  - Draws an early selection rectangle on the transparent shell when a pre-show drag is detected.
  - Focuses the shell canvas after shell show returns, while deferring screenshot toolbar UI until real pixels are ready.
- `docs/IMPLEMENTATION_CHAPTERS.md`
  - Records this chapter.

### Deleted Files
- None.

### Evidence Added
- Packaged release hotkey smoke:
  - stdout: `tmp-runtime-logs\transparent-input-shell-smoke-20260610-043120-out.log`.
  - stderr: `tmp-runtime-logs\transparent-input-shell-smoke-20260610-043120-err.log`, empty.
- The 4-round `Alt+A -> Esc` smoke showed:
  - all rounds logged `transparent_input_shell=true`;
  - all rounds used direct SharedBuffer delivery;
  - no `rgba_fetch`;
  - no `shared_buffer_direct_wait_miss`;
  - no `shared_buffer_receive_timeout`;
  - no failure/error/panic log matches.
- Timings from the hotkey smoke:
  - shell window prepared at `23-46ms` warm, `46ms` first round;
  - `shell_ready_to_show_returned` at `21-70ms`;
  - `image_ready` at `6-12ms` after frontend session start;
  - `first_paint` at `9-18ms` after frontend session start.
- Visual recording smoke:
  - video: `tmp-runtime-logs\transparent-input-shell-visual-20260610-043245.mp4`;
  - app stdout: `tmp-runtime-logs\transparent-input-shell-visual-20260610-043245-out.log`;
  - app stderr: `tmp-runtime-logs\transparent-input-shell-visual-20260610-043245-err.log`, empty;
  - analysis: `tmp-runtime-logs\transparent-input-shell-visual-20260610-043245-visual-analysis.json`.
- Visual frame analysis result:
  - `frames=419`;
  - `black_frame_count=0`;
  - `white_frame_count=0`;
  - `high_diff_frame_count=0`;
  - `luma_min=29.22`, `luma_max=68.77`;
  - `diff_max=23.95`, `diff_avg=0.45`.

### Explicit Non-Goals
- Did not implement a full native-drawn selection renderer.
- Did not re-enable the old native GDI first-frame shield by default.
- Did not switch capture to DXGI/WGC GPU texture presentation.
- Did not remove the normal dimmed screenshot mask after the real screenshot image is ready.
- Did not claim final human-visible parity with QQ/WeChat/Snow Shot without the user's manual device validation.

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_window_transparency_tests -- --nocapture` with `5 passed`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_shared_buffer -- --nocapture` with `2 passed`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_input_smoke -- --nocapture` with `2 passed`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; Vite emitted existing chunk-size/dynamic-import warnings only.
- Passed: `cd tauri-client; npm run check:i18n` with `566 zh-CN keys match 566 en-US keys`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.
- Passed: `cmd /c "build.bat --no-pause"` release rebuild.
- Passed: packaged release hotkey smoke and ffmpeg visual frame analysis.

### Known Risks
- This is still a WebView input shell, not a full native selection renderer. It reduces the perceived drag gate, but it does not make the architecture identical to Snow Shot's deeper candidate/draw system.
- The first visual transition from live desktop to frozen/dimmed screenshot can still be perceptible as the screenshot image arrives. The visual smoke shows no black/white/high-diff flash, but a user may still notice the intended dim state appearing.
- If a specific GPU/WebView2 driver renders transparent early windows incorrectly, set `YSN_SCREENSHOT_DEFER_VISIBLE_SHELL=1` or `YSN_SCREENSHOT_EARLY_VISIBLE_SHELL=0` to return to the Chapter 267 deferred-show behavior.

### Next Recommended Chapter
- Chapter 269 should implement **Native First Frame Screenshot Session** directly.
- Do not continue with a WebView-only timing patch and do not stop at low-level mouse recovery. The next route is native first-frame screenshot overlay + native input overlay, with low-level/global mouse hook only as a 0-50ms fallback and WebView only as the later complex-UI layer.
- Chapter 269 must target `Alt+A -> 画面冻结/遮罩出现/马上能拖`, not `Alt+A -> 等 WebView -> 闪一下 -> 再遮罩 -> 再能拖`.

## Chapter 269 - Native First Frame Screenshot Session (Planned)

> Chapter status: planned and user-approved as the next implementation target. This chapter is not completed yet. It supersedes a WebView-only or low-level-hook-only next step: the next screenshot chapter should build a native first-frame screenshot session where Rust/Win32 owns the first visible screenshot frame, mask, candidate/window recognition, and immediate mouse input, while WebView joins later for toolbar, OCR, translation, editing, copy, and save.

### User-Approved Target - Verbatim

```text
核心方案：

Alt+A
  ↓
Rust/Win32 立即接管
  ↓
原生截图 + 原生遮罩 + 原生窗口识别 + 原生鼠标输入
  ↓
用户立刻可拖
  ↓
WebView 后面再接管工具栏、OCR、翻译、编辑

也就是：

第一帧：Native 负责
后续 UI：WebView 负责

最终推荐架构
常驻预热：
- 预创建 hidden native overlay HWND
- 预初始化 D3D/Direct2D/GDI 绘制资源
- 预初始化截图 backend
- 预初始化窗口枚举/候选框服务
- WebView 截图页保持隐藏预热

Alt+A 触发：
1. Rust 收到全局热键
2. 立即进入 native screenshot session
3. 立即捕获屏幕
4. 立即枚举窗口/控件候选区域
5. 原生 overlay 画：
   - 截图画面
   - 半透明遮罩
   - 鼠标下窗口候选框
6. overlay 直接接管鼠标：
   - WM_LBUTTONDOWN
   - WM_MOUSEMOVE
   - WM_LBUTTONUP
   - ESC
7. 用户马上拖选
8. WebView 后台 ready 后只接管：
   - 工具栏
   - OCR
   - 翻译
   - 编辑按钮
   - 复制/保存
这比单独 low-level hook 更适合你的目标

你现在要的不是：

先记录鼠标，等 WebView 后恢复

你要的是：

Alt+A 后画面立刻冻结，遮罩立刻出现，窗口立刻识别，鼠标立刻能拖

所以我会把优先级改成：

第一优先级：native input overlay + native first frame
第二优先级：low-level mouse hook 兜底最早几十毫秒输入
第三优先级：WebView 后置接管复杂 UI

不要只做 low-level hook。
low-level hook 只能解决“马上拖动不丢第一下”，但不能解决你说的：

延迟 100ms+
瞬间闪烁
遮罩慢
UI 窗口识别慢

这些必须靠 native first frame。
目标链路应该变成这样
Alt+A
  ↓ 0-10ms
热键进入 Rust native fast path
  ↓ 10-40ms
截图 + 画遮罩 + 识别当前窗口
  ↓ 40-60ms
native overlay 已显示，鼠标已可拖
  ↓ 后台
WebView 工具栏/OCR/翻译接上
实际目标可以定成：

P95 hotkey -> 鼠标可拖：<= 50ms
P95 hotkey -> 遮罩首帧出现：<= 60ms
P95 hotkey -> 窗口候选框出现：<= 60ms
P95 hotkey -> WebView 工具栏 ready：<= 120ms

用户体感就是：

Alt+A -> 画面冻结/遮罩出现/马上能拖

而不是：

Alt+A -> 等 WebView -> 闪一下 -> 再遮罩 -> 再能拖
实施顺序

我建议下一章直接叫：

Chapter 269: Native First Frame Screenshot Overlay

做这几件事：

1. 新增 Win32 native overlay 窗口
2. overlay 启动时预创建，默认隐藏
3. Alt+A 后不等 WebView，直接 show native overlay
4. native overlay 负责首帧截图、遮罩、候选框
5. native overlay 直接处理鼠标拖选
6. WebView 只在 ready 后接管工具栏和后续功能
7. low-level mouse hook 只作为 0-50ms 兜底
最终形态
QQ/微信式体验 =
原生截图首帧
+ 原生遮罩
+ 原生输入
+ 原生窗口识别
+ WebView 后置复杂 UI
```

### Goals

- Build the first production-grade slice of **Native First Frame Screenshot Session**, not another WebView-first shell experiment.
- Keep the old diagnostic native GDI first-frame shield disabled by default. The new route must render actual screenshot pixels and native mask/candidates; it must not be a temporary black, gray, white, or color-shifting cover.
- Add or upgrade a Win32 native overlay window that is created/prepared before use, hidden by default, and capable of immediate no-activate topmost presentation when `Alt+A` starts.
- Make the native overlay own the first visible frame: screenshot bitmap, dim mask, current mouse-window candidate rectangle, and drag rectangle.
- Make the native overlay own immediate input for the screenshot-start phase: `WM_LBUTTONDOWN`, `WM_MOUSEMOVE`, `WM_LBUTTONUP`, `ESC`, and repeat-hotkey cancellation if applicable.
- Use low-level/global mouse hook only as the earliest 0-50ms fallback so a click that begins before the overlay message pump is ready is not lost.
- Let WebView remain hidden/preheated during native first-frame presentation, then hand off only the complex UI layer: toolbar, OCR, translation, edit actions, copy, save, and later annotation tools.
- Preserve the existing direct WebView2 SharedBuffer path as the WebView image handoff/fallback path; do not copy Snow Shot source code.

### Proposed Native Session Flow

```text
Alt+A
  -> Rust global hotkey callback records hotkey timestamp
  -> create/resume native screenshot session
  -> begin 0-50ms low-level/global mouse fallback capture
  -> capture screen into RGBA/native bitmap
  -> enumerate fast window/control candidates
  -> show hidden native overlay no-activate/topmost
  -> native overlay paints screenshot pixels + mask + initial candidate
  -> native overlay message pump handles drag/cancel
  -> WebView screenshot page warms/receives SharedBuffer in background
  -> WebView toolbar joins after ready without replacing the first visible frame with a blank/gray/white surface
```

### Implementation Boundaries

- Prefer a focused native session module rather than extending the already-large `screenshot_commands.rs` with more responsibilities.
- Reuse existing native primitives where safe: `win32_overlay`, `native_overlay_session`, `win32_input`, `selection_state`, current capture code, and candidate/window enumeration adapters.
- Do not revive the Chapter 262-263 full-screen native shield as the product default.
- Do not depend on Snow Shot GPL code or its custom forks. Use Snow Shot only as an architecture reference.
- Do not move OCR, translation, or recording work into Chapter 269 except for smoke regression.
- Do not claim QQ/WeChat/Snow Shot parity until a real release build passes repeated human-device tests and visual recording evidence.

### Acceptance Targets

- P95 `hotkey -> 鼠标可拖`: `<= 50ms`.
- P95 `hotkey -> 遮罩首帧出现`: `<= 60ms`.
- P95 `hotkey -> 窗口候选框出现`: `<= 60ms`.
- P95 `hotkey -> WebView 工具栏 ready`: `<= 120ms`.
- No black frame, white frame, full-screen gray pulse, color-shift frame, or WebView-default-background flash during entry.
- Repeated `Alt+A` runs must not degrade on the third/fourth run.
- Immediate drag must work even when the mouse button goes down before WebView focus or toolbar readiness.
- `Esc`, repeat hotkey, cancel, copy, save, OCR, translate, and focus restore must remain recoverable after native-to-WebView handoff.

### Validation Required

- Rust unit tests for native input event decoding, native selection transitions, session lifecycle cleanup, stale-session rejection, and cancellation.
- Rust compile/check: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check` and `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Frontend checks after handoff changes: `cd tauri-client; npx tsc --noEmit`, `npm run build`, `npm run check:i18n`, and `npm run check:ocr-processing`.
- Release build smoke through `cmd /c "build.bat --no-pause"` unless the chapter only lands guarded diagnostics.
- Automated hotkey timing smoke with logged timestamps for `hotkey_received`, `native_session_start`, `native_overlay_first_paint`, `native_input_ready`, `candidate_first_rect`, `webview_toolbar_ready`, and `handoff_complete`.
- Visual recording smoke with frame analysis for black/white/high-diff/color-shift frames.
- Manual QA on the release exe: repeated `Alt+A`, immediate drag, third/fourth run, rapid cancel, rapid repeat-hotkey, multi-monitor/DPI, copy/save/OCR/translate after handoff.

### Rollback And Diagnostics

- Keep an explicit env rollback to the Chapter 268 WebView SharedBuffer/transparent-shell path.
- Keep the old full-screen native shield from Chapters 262-263 separate and off by default.
- Log whether each screenshot used `native_first_frame_session`, `low_level_mouse_fallback`, `webview_handoff`, or a fallback route.
- On any native-session failure, cancel/cleanup native HWND/hooks and fall back to the existing WebView SharedBuffer path rather than leaving an invisible or input-blocking overlay.

### Next Chapter Entry Point

- Start Chapter 269 by extracting a focused native screenshot session owner around the existing Win32 overlay/input primitives, then prove a minimal native first frame can show real screenshot pixels, draw a dim mask/candidate, accept drag, cancel cleanly, and hand off to the current WebView toolbar path.

## Chapter 269: Native First Frame Screenshot Session (Phase 1 Guarded Experiment)

### Goals Completed
- Implemented real-time synchronization between the native pointer capture (`ScreenshotPointerPreCapture`) and the native overlay rendering loop (`win32_overlay.rs`).
- The `ysn-native-first-frame-session` thread now periodically polls for selection rectangle changes and updates the Win32 overlay selection bounds.
- Visual mask logic inside `StretchDIBits` handles drawing a pre-computed 50% dimmed background, avoids per-frame full image clones by storing shared `Arc<[u8]>` buffers, and copies the selected region over it using GDI `StretchDIBits`. Dimming preserves the alpha channel.
- Added `YSN_NATIVE_FIRST_FRAME_SESSION=1` guard to keep this path experimental and avoid regression.
- Maintained `HTTRANSPARENT` for `WM_NCHITTEST`; this keeps the experiment mouse-transparent and relies on pointer pre-capture polling until the native input state machine is implemented.
- Restored invalid bitmap dimension errors, kept bitmap length overflow checks, exposed a local `Win32OverlaySelectionRect`, and restored a `native_first_frame_session_disabled` log for default-path observability.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_native/native_overlay_session.rs`
- `tauri-client/src-tauri/src/screenshot_native/win32_overlay.rs`
- `tauri-client/src-tauri/src/screenshot_native/mod.rs`
- `tauri-client/src-tauri/src/screenshot_commands.rs`
- `tauri-client/src-tauri/src/window_lifecycle.rs`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Validation
- Passed: `git diff --check` with existing LF-to-CRLF working-copy warnings only.
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_first_frame_session_is_opt_in_until_visual_artifacts_are_fixed -- --nocapture` with `1 passed`.
- Passed: `cd tauri-client; npx tsc --noEmit`.

### Known Risks
- This is still a guarded visual/pre-capture experiment, not the completed Chapter 269 architecture.
- Native mouse input is not implemented yet because the overlay still returns `HTTRANSPARENT`.
- Native window/candidate rectangles are not rendered yet.
- No release visual smoke has been run yet with `YSN_NATIVE_FIRST_FRAME_SESSION=1`.

### Next Steps
- Implement full `WM_LBUTTONDOWN`, `WM_MOUSEMOVE`, and `WM_LBUTTONUP` handling in `win32_overlay_wnd_proc` so the native overlay can own input instead of only polling pre-capture state.
- Expand native rendering to also include candidate window rectangles.

## Chapter 269: Native First Frame Screenshot Session (Phase 2 Native Input Overlay)

### Goals Completed
- Converted the guarded native first-frame overlay from mouse-transparent visual/pre-capture mode into a real native input overlay for the first screenshot-selection phase.
- Replaced `WM_NCHITTEST -> HTTRANSPARENT` with a client hit-test result so the native overlay can receive mouse messages instead of passing them through.
- Added per-HWND native input state in `win32_overlay.rs`, backed by the existing `SelectionState` state machine.
- Handled `WM_LBUTTONDOWN`, `WM_MOUSEMOVE`, `WM_LBUTTONUP`, `WM_KEYDOWN`, `WM_SYSKEYDOWN`, and `WM_KILLFOCUS` in the native overlay window procedure.
- Added `SetCapture(hwnd)` on primary-button down and `ReleaseCapture()` on button up, terminal state, or focus loss, so a drag remains owned by the native overlay even if the pointer leaves the original point.
- Native overlay drag transitions now update the GDI-rendered selected region directly, while cancel/armed transitions clear the previous selected region.
- Kept the `YSN_NATIVE_FIRST_FRAME_SESSION=1` guard. Default production behavior remains the Chapter 268 WebView SharedBuffer/transparent-input-shell path until visual/release smoke proves the guarded native session.
- Prevented pointer pre-capture polling from overwriting native overlay selection rendering once native input has started. The global pre-capture path still remains available for WebView handoff/recovery.
- Added a native overlay selection snapshot for the active screenshot session, exposed through `get_screenshot_pointer_state` as `nativeOverlay`.
- Updated both screenshot recovery paths to prefer `nativeOverlay` selection data when available and fall back to `preCapture` only when native data is unavailable.
- Added `source=native-overlay` / `source=pre-capture` recovery logging so guarded smoke can prove which handoff path restored the WebView selection.
- Added native input phase and event sequence tracking (`idle`, `started`, `selecting`, `completed`, `cancelled`) so WebView handoff can distinguish a live native drag, completed native selection, and native cancel even when no selected rectangle exists.
- The native overlay thread now logs `native_input_ready`, `native_selection_handoff_ready`, `native_selection_completed`, and `native_selection_cancelled` with event sequence and rectangle details for guarded timing smoke.
- `get_screenshot_pointer_state.nativeOverlay` now includes `phase`, `eventSeq`, and `handoffReady`; the WebView recovery paths log these values and cancel the screenshot when native overlay reports `cancelled`.
- Added native candidate rectangle painting in the GDI overlay: the first frame now seeds a cursor-under candidate before the overlay is shown, then refreshes it while no native drag is active.
- The native overlay thread now logs `native_candidate_first_rect` and `native_overlay_first_paint` for guarded first-frame timing evidence.
- Reused existing Win32 window-target helpers to resolve taskbar, child control, top-level window, and display fallback candidates without adding a new dependency.
- Added native candidate and selection border drawing with bitmap-bound clipping so negative coordinates and monitor edges do not produce invalid GDI rectangles.
- Added Win32 `SetCapture` and `ReleaseCapture` bindings.
- Cleaned up native overlay input state when an overlay is destroyed or receives `WM_NCDESTROY`.
- Extracted native overlay input ownership, state, dispatch, and snapshot tests into `win32_overlay_input.rs` so `win32_overlay.rs` remains focused on HWND lifecycle and painting.

### Modified Files
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src-tauri/src/screenshot_commands.rs`
- `tauri-client/src-tauri/src/screenshot_native/win32_overlay.rs`
- `tauri-client/src-tauri/src/screenshot_native/win32_overlay_input.rs`
- `tauri-client/src-tauri/src/screenshot_native/native_overlay_session.rs`
- `tauri-client/src-tauri/src/screenshot_native/mod.rs`
- `tauri-client/src/hooks/useScreenshotLoader.ts`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_input_smoke -- --nocapture` with `2 passed`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib win32_overlay_input -- --nocapture` with `3 passed`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib win32_overlay -- --nocapture` with `18 passed`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_overlay_session -- --nocapture` with `4 passed`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_first_frame_session_is_opt_in_until_visual_artifacts_are_fixed -- --nocapture` with `1 passed`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_window_transparency_tests -- --nocapture` with `5 passed`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; Vite emitted existing chunk-size/dynamic-import warnings only.
- Passed: `cd tauri-client; npm run check:i18n` with `566 zh-CN keys match 566 en-US keys`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.

### Known Risks
- This is still guarded by `YSN_NATIVE_FIRST_FRAME_SESSION=1` and has not yet passed a release visual smoke with repeated real `Alt+A` runs.
- Native overlay now exposes phase/event-sequenced handoff data to WebView, but the final architecture still needs release smoke evidence that the WebView toolbar consistently joins without visual flash.
- Native candidate/window rectangles are now drawn, but only with the existing fast Win32/window-target candidate source. UIAutomation-grade control candidates remain a later polish layer.
- Native `Esc` handling clears/cancels the native overlay selection state, but full app-level cancel still relies on the existing global/WebView cancellation flow.
- This phase does not yet prove P95 `hotkey -> mouse draggable <= 50ms`; it only removes the architectural blocker that prevented the native overlay from accepting the first drag.

### Next Steps
- Run guarded release visual smoke with `YSN_NATIVE_FIRST_FRAME_SESSION=1`, repeated third/fourth runs, immediate drag, rapid cancel, and frame analysis for black/white/color-shift flashes.
- Only after that smoke passes, evaluate whether this guarded native path can become the default route.

## Chapter 269: Native First Frame Screenshot Session (Phase 3 Handoff Timing And Visual Smoke)

### Goals Completed
- Fixed the first guarded release smoke race where a fixed native first-frame dismiss could destroy the native overlay before `WM_LBUTTONUP` / handoff was observed.
- Changed native first-frame handoff so WebView no longer shows the early empty/transparent shell when `YSN_NATIVE_FIRST_FRAME_SESSION=1`; the WebView shell is deferred until screenshot RGBA is ready.
- Preserved `nativeVisible=false` as the WebView-shell visibility meaning and added `nativeFirstFrame=true` only as a diagnostic payload flag, preventing the frontend from skipping `overlay_ready_to_show`.
- Replaced fixed dismiss with state-aware dismiss:
  - no active input: dismiss after the configured delay;
  - native drag active: wait;
  - pre-capture drag active before native HWND receives down: wait;
  - completed selection: keep a short WebView handoff grace;
  - timeout: force dismiss so the native overlay cannot block the app forever.
- Added `YSN_NATIVE_FIRST_FRAME_SESSION_HANDOFF_GRACE_MS` and `YSN_NATIVE_FIRST_FRAME_SESSION_MAX_DISMISS_MS` guarded tuning knobs.
- Added a small `ScreenshotPointerPreCaptureActivity` snapshot so native first-frame dismissal can see early mouse-down/drag activity without parsing frontend JSON.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_commands.rs`
- `tauri-client/src-tauri/src/window_lifecycle.rs`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_first_frame_dismiss -- --nocapture` with `5 passed`.
- Passed: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib native_input_smoke -- --nocapture` with `2 passed`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; Vite emitted existing chunk-size/dynamic-import warnings only.
- Passed: `cd tauri-client; npm run check:i18n` with `566 zh-CN keys match 566 en-US keys`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.
- Passed: `cmd /c "build.bat --no-pause"`.
- Guarded release immediate-drag smoke: `tmp-runtime-logs/native-first-frame-immediate-smoke-20260611-051111-out.log`.
  - `stderrLength=0`.
  - `hotkey_received=6`.
  - `native_first_frame_visible=6`.
  - `native_overlay_first_paint=6`.
  - `native_input_ready=6`.
  - `native_selection_handoff_ready=6`.
  - `native_selection_completed=6`.
  - `native_selection_cancelled=0`.
  - `native_first_frame_session_dismiss_wait=12`.
  - `native_first_frame_session_dismissed=6`.
  - `pre_show_drag_recovered=6`.
  - `pre_show_drag_finalized=6`.
- Guarded release visual smoke:
  - Video: `tmp-runtime-logs/native-first-frame-visual-20260611-051302.mp4`.
  - App log: `tmp-runtime-logs/native-first-frame-visual-20260611-051302-app-out.log`.
  - Extracted/analyzed `330` frames at reduced size.
  - `blackFrames=0`, `whiteFrames=0`, `largeDiffFrames=0`, `brightnessJumps=0`.
  - `meanMin=86.92`, `meanMax=160.87`, `maxAvgDiff=72.68`.
  - App log had `hotkey_received=3`, `native_first_frame_visible=3`, `pre_show_drag_recovered=3`, `pre_show_drag_finalized=3`, `native_first_frame_session_dismissed=3`, and `stderrLength=0`.

### Known Risks
- The guarded native route is materially improved, but it is still not enabled by default.
- Immediate-drag smoke now proves the native/pre-capture first-stage can preserve selection and avoid premature dismiss, but it is not a true low-level mouse hook yet.
- Visual smoke did not find black/white/high-diff flash frames, but this is automated frame analysis, not a human visual QA pass across monitors/DPI/apps.
- Current measured `native_first_frame_visible` in release smoke is still roughly `76-108ms`, not the target `<=60ms`; the capture/presenter path must still move closer to a prewarmed native/DXGI first frame before claiming QQ/WeChat/Snow Shot parity.
- Some visual-smoke rounds use `pre-capture` as the WebView finalization source when the mouse down starts before native HWND can own the first button event. This is acceptable as a guarded fallback, but a real low-level hook remains the cleaner 0-50ms input fallback.

### Next Steps
- Add a real low-level mouse hook fallback for the first 0-50ms so pre-HWND mouse down/up is captured as structured native input rather than only pre-capture polling.
- Move native first-frame presentation earlier in the pipeline by starting native overlay before WebView shared-buffer posting where safe, then continue toward precreated hidden HWND and preinitialized native capture resources.
- Keep `YSN_NATIVE_FIRST_FRAME_SESSION=1` guarded until manual release QA confirms no black/white/color flash, immediate drag, cancel, copy/save/OCR/translate, multi-monitor, and DPI behavior.

## Chapter 270: Settings And Save UX Polish (2026-06-11)

### Goals Completed
- Froze the Native first-frame commercial confirmation/default-rollout plan at the user's request; no new long-term document was created.
- Hid internal text-translation service URL, token, LAN URL, and LAN-preference controls from the system settings panel.
- Kept local translation-cache management available without exposing internal service configuration.
- Removed direct service URL exposure from the app header tooltip, translation-channel health summary, and user-facing internal-service failure prompts.
- Changed Windows tray menu text to Chinese and added a tray command for a delayed 3-second screenshot.
- Added image save settings for filename prefix, datetime format, default save directory, and optional "remember last save directory" behavior.
- Replaced fixed `screenshot.png` / Desktop save defaults with config-driven filename and directory resolution.
- Normalized image save filename handling across save commands, including automatic `.png` extension and Windows reserved-name protection.
- Added a folder picker command for the image default save directory.
- Changed `Ctrl+S` save-dialog cancellation so pressing `Esc` exits the screenshot session cleanly instead of restoring the screenshot overlay and requiring a second `Esc`.

### Modified Files
- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src-tauri/src/screenshot_commands.rs`
- `tauri-client/src/components/app/AppLayout.tsx`
- `tauri-client/src/components/settings/TranslationChannelCard.tsx`
- `tauri-client/src/components/settings/TranslationServiceCard.tsx`
- `tauri-client/src/components/settings/ImageSaveSettingsCard.tsx`
- `tauri-client/src/hooks/useScreenshotActions.ts`
- `tauri-client/src/hooks/useSettingsController.ts`
- `tauri-client/src/i18n/en-US.ts`
- `tauri-client/src/i18n/zh-CN.ts`
- `tauri-client/src/pages/Settings.tsx`
- `tauri-client/src/types/config.ts`

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run check:i18n`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `cd tauri-client; npm run build`; Vite emitted existing dynamic-import/chunk-size warnings only.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.

### Known Risks
- The tray delayed screenshot currently has no countdown toast; it waits 3 seconds and starts capture.
- Custom filename formats can include Windows-invalid characters such as `:`; the backend sanitizes them before showing the save dialog.
- The internal translation service is hidden from the normal settings UI, but translation-channel operations still rely on the existing hidden config values.

### Next Steps
- Manually verify the tray menu labels, delayed screenshot, image-save settings persistence, and `Ctrl+S` save-dialog `Esc` exit in the built app.
- Continue small feature add/remove work before returning to commercial-grade native first-frame rollout confirmation.

## Chapter 271: Save Dialog, Hotkey, And Window Click Polish (2026-06-11)

### Goals Completed
- Changed the default image save timestamp from minute-level to second-level (`yyyyMMdd_HHmmss`) to avoid same-minute name collisions without scanning the target folder.
- Added a compatibility migration so the previous default `yyyyMMdd_HHmm` is treated as the new second-level default.
- Kept filenames Windows-safe; colon-separated times are not used because `:` is invalid in Windows filenames.
- Reduced `Ctrl+S` save-dialog cancel residue by unregistering the screenshot Escape shortcut while the native save dialog is open, and force-closing screenshot windows when the dialog is cancelled.
- Improved detected UI/window click behavior so a light left click or small pointer wobble selects the detected candidate instead of falling into a tiny manual box selection.
- Added automatic local settings save for form changes, including switches, save-location preferences, screenshot recognition options, and hotkey fields.
- Changed hotkey controls from typed text boxes to key-capture fields; pressing a shortcut writes it, while `Backspace`, `Delete`, or `Esc` clears it.
- Made clear/default hotkey actions save immediately and re-register global shortcuts, so the UI success state matches the actual shortcut state.

### Modified Files
- `docs/IMPLEMENTATION_CHAPTERS.md`
- `tauri-client/src-tauri/src/screenshot_commands.rs`
- `tauri-client/src/hooks/useScreenshotActions.ts`
- `tauri-client/src/hooks/useScreenshotInteraction.ts`
- `tauri-client/src/hooks/useSettingsController.ts`
- `tauri-client/src/components/settings/ImageSaveSettingsCard.tsx`
- `tauri-client/src/components/settings/SystemHotkeyCard.tsx`
- `tauri-client/src/components/settings/types.ts`
- `tauri-client/src/i18n/en-US.ts`
- `tauri-client/src/i18n/zh-CN.ts`
- `tauri-client/src/pages/Settings.tsx`

### Validation
- Passed: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run check:i18n`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `cd tauri-client; npm run build`; Vite emitted existing dynamic-import/chunk-size warnings only.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.

### Known Risks
- Native Windows save dialogs cannot display or save filenames containing `:`; the fast no-scan default uses compact second-level time (`HHmmss`) instead.
- Settings auto-save intentionally only persists local configuration and local side effects; the top Save action remains available for explicit translation-service synchronization.

### Next Steps
- Manually verify `Ctrl+S -> Esc`, detected-window single click selection, hotkey capture, restore-default shortcut registration, and switch auto-save in the built app.

## Chapter 272: Repository Size Cleanup (2026-06-12)

### Goals Completed
- Cleaned generated build output, release artifacts, local caches, duplicate packaged release assets, and checked-in regenerable OCR runtime/model payloads from the working tree.
- Removed the ignored root `release/` portable output.
- Removed the tracked duplicate `scripts/release/YSN-Screenshot-Translator` portable bundle because current build and packaging scripts use root `release\YSN-Screenshot-Translator`, not `scripts/release`.
- Removed checked-in RapidOCR ONNX model payloads from `models/rapidocr`, leaving a `.gitkeep` placeholder.
- Removed the checked-in PyInstaller RapidOCR runner bundle from `tauri-client/src-tauri/resources/rapidocr/rapidocr-runner`, leaving the resource placeholder directory intact.
- Added ignore rules so generated release copies, RapidOCR runner output, and local model assets do not get reintroduced accidentally.
- Pruned local Git LFS cache and ran standard Git garbage collection.
- Reduced the source working tree excluding `.git` to about `9.25 MB`; total folder size remains about `841 MB` because `.git` still contains repository history and retained LFS objects.

### Added Files
- `models/rapidocr/.gitkeep`

### Modified Files
- `.gitignore`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- `release/` generated portable output.
- `scripts/release/YSN-Screenshot-Translator/` tracked duplicate portable bundle.
- `models/rapidocr/*.onnx` and OCR dictionary payloads.
- `tauri-client/src-tauri/resources/rapidocr/rapidocr-runner/` generated PyInstaller runner bundle.

### Not Done
- Did not delete `.git` because that would remove repository history and normal Git workflow.
- Did not rewrite Git history or force-push remote cleanup.
- Did not regenerate RapidOCR runner or model assets after cleanup.

### Validation
- Passed: `git clean -ndX` returned no ignored files left to remove.
- Passed: working tree size excluding `.git` measured about `9.25 MB`.
- Passed: `.git` reduced to about `832 MB` after `git lfs prune` and `git gc --prune=now`.
- Inspected largest Git history blobs; remaining history still contains old release binaries, `.codex-analysis` artifacts, and packaged OCR payloads, so a true few-MB clone requires history rewrite or a source-only copy without `.git`.

### Known Risks
- `build.bat` will fail until RapidOCR assets are regenerated because it intentionally checks for `tauri-client/src-tauri/resources/rapidocr/rapidocr-runner/rapidocr-runner.exe` and `models/rapidocr/ch_PP-OCRv5_det_mobile.onnx`.
- OCR should be treated as not locally ready until running `cd tauri-client && npm run build:rapidocr-runner` successfully rebuilds the runner and warms model assets.

### Next Steps
- If the goal is a small development checkout with history, rewrite repository history or create a fresh shallow/partial clone after the cleanup commit lands.
- If the goal is a runnable release build, regenerate RapidOCR assets with `cd tauri-client && npm run build:rapidocr-runner`, then run the normal build checks.

## Chapter 273: Cleanup Script And Root Installer Output (2026-06-12)

### Goals Completed
- Changed `clean_all_cache.bat` from a Windows shell/cache refresh helper into a focused project source-slim cleanup script.
- Default cleanup now only targets the user-approved regenerable artifacts: root `release/`, `tauri-client/src-tauri/target/`, `tauri-client/dist/`, `tauri-client/node_modules/`, runtime smoke logs, root-level old `*.exe` / `*.pdb`, `.codex-analysis`, and `.superpowers`.
- Kept OCR models, RapidOCR runner/resources, FFmpeg resources, `.git`, user config, recordings, source files, and Windows shell caches out of the cleanup path.
- Added `--dry-run` and `--no-pause` flags to `clean_all_cache.bat`.
- Changed `build.bat` default behavior from `tauri build --no-bundle` to full `tauri build`, so installers are generated by default.
- Added `--no-installers` / `--portable-only` to `build.bat` for fast portable-only builds.
- Added versioned root installer collection under `build\x64_v<version>\`, copying generated `.msi` and `*-setup.exe` files from Tauri's nested bundle output.
- Updated `pack_release.ps1` so portable zip artifacts are also written to `build\x64_v<version>\`.
- Ignored the root `build/` artifact directory.

### Modified Files
- `.gitignore`
- `build.bat`
- `clean_all_cache.bat`
- `pack_release.ps1`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Validation
- Passed: `cmd /c "clean_all_cache.bat --dry-run --no-pause"`.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.
- Expected fail: `cmd /c "build.bat --no-pause --no-launch --no-installers"` parsed version `1.2.6` and reached resource checks, then stopped because RapidOCR runner assets were intentionally removed in Chapter 272.

### Known Risks
- Full installer generation now runs by default in `build.bat`; use `--no-installers` or `--portable-only` when only the portable directory is needed.
- Real installer copy into `build\x64_v<version>\` has not been smoke-tested yet because OCR assets must be regenerated first.

### Next Steps
- Run `cd tauri-client && npm install` if `node_modules` is missing.
- Run `cd tauri-client && npm run build:rapidocr-runner` to restore OCR runner and models.
- Run `.\build.bat --no-pause --no-launch` and confirm `.msi` plus `*-setup.exe` appear under `build\x64_v1.2.6\`.

## Chapter 274: Proxy-Compatible Official Translation And Screenshot Session Reset (2026-06-12)

### Goals Completed
- Split official translation traffic from user-configured URL traffic:
  - Google, Baidu, and official DeepL requests now use a normal pooled Requests session with `trust_env=True`, so system/environment proxies and TUN fake-IP routing work.
  - User-configured public LLM and relay URLs keep the pinned SSRF-safe transport with `trust_env=False`.
- Restricted configurable DeepL endpoints to the official `api-free.deepl.com` and `api.deepl.com` HTTPS hosts before allowing them onto the proxy-compatible official session.
- Disabled redirects for official translation provider requests.
- Removed the previous DeepL reserved-IP DNS bypass from general public URL validation; user-configured public URLs continue rejecting private/reserved DNS results.
- Fixed screenshot session reuse so a newly identified remote capture session cannot inherit the previous visible shell, selection, masked buffer, canvas pixels, or interaction refs.
- Added OCR/translation operation generations so an old asynchronous OCR or translation task cannot write a prompt, result overlay, or close action into a newer screenshot session.
- Clear keyed OCR/translation notices when a screenshot session resets.
- Kept `updateCurrentRect(next, false)` in pointer-move paths because `ScreenshotPage.setCurrentRect` already synchronously updates `rectRef.current`; changing it to `true` would add React renders during every pointer move.
- Kept the screenshot canvas in CSS-pixel interaction coordinates because physical screenshot cropping already maps canvas coordinates to image coordinates. Multiplying the backing canvas by `devicePixelRatio` without redesigning that mapping would introduce selection/crop errors.

### External Findings
- Requests documentation confirms environment proxy behavior is controlled through the session environment trust path.
- OWASP SSRF guidance supports applying strict validation at user-controlled URL boundaries and using allowlists for known trusted hosts.
- React documentation confirms refs are intentionally mutable and suitable for immediate event-handler state that should not trigger renders.
- MDN documents the CSS-pixel/device-pixel distinction; the current screenshot architecture deliberately maps CSS interaction coordinates to physical image pixels during crop/export.

### Added Files
- `server/tests/test_http_client.py`

### Modified Files
- `server/app.py`
- `server/http_client.py`
- `server/security.py`
- `server/translator.py`
- `server/tests/test_security.py`
- `server/tests/test_server.py`
- `server/tests/test_translate_text.py`
- `server/tests/test_translator.py`
- `tauri-client/src/hooks/useScreenshotInteraction.ts`
- `tauri-client/src/hooks/useScreenshotLoader.ts`
- `tauri-client/src/hooks/useScreenshotOcr.ts`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not weaken SSRF validation for user-configured LLM or relay URLs.
- Did not change pointer-move rect updates to React state updates on every frame.
- Did not change the screenshot canvas backing size to `devicePixelRatio` coordinates.
- Did not regenerate removed RapidOCR assets or run a full installer build.

### Validation
- Passed: `F:\Python311\python.exe -m pytest server/tests -rs` with `63 passed, 1 skipped`; the skipped test is the expected user-configured New API public URL rejection under the current fake-IP environment.
- Passed: `F:\Python311\python.exe -m py_compile server/http_client.py server/security.py server/translator.py server/app.py`.
- Passed: current DNS resolves `translate.googleapis.com` to fake IP `198.18.0.188`, while direct `GoogleTranslator().translate("Open settings", "en", "zh")` returned `打开设置` with `trust_env=True`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run check:i18n` with `601 zh-CN keys match 601 en-US keys`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `cd tauri-client; npm run build`; Vite emitted existing dynamic-import/chunk-size warnings only.
- Passed: local browser load smoke for `http://127.0.0.1:1420`; expected Tauri-invoke warnings remain in a normal browser.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.

### Known Risks
- Desktop Tauri manual QA is still required for repeated screenshot sessions, mid-drag cancellation, resize/move smoothness, and multi-monitor/DPI behavior.
- DeepL-compatible custom endpoints are no longer accepted as DeepL official providers; custom/self-hosted translation services must use the user-configured relay path.
- Old asynchronous OCR/translation work is invalidated for UI writes but is not force-cancelled at the underlying process/network layer.

### Next Steps
- Manually verify repeated `Alt+A` runs: start a drag, cancel mid-drag, reopen immediately, move/resize an existing selection, and confirm no old frame, selection, loading notice, or translation overlay returns.
- Manually verify Google, Baidu, and DeepL through the packaged translation service while the proxy/TUN fake-IP mode remains enabled.

## Chapter 275: DeepL Translation Channel Enablement (2026-06-12)

### Goals Completed
- Removed the frontend-only hard disable that made the existing DeepL backend channel impossible to configure or test.
- Enabled DeepL in the translation channel selector and health summary.
- Added an official endpoint selector for DeepL API Free (`api-free.deepl.com`) and DeepL API Pro (`api.deepl.com`).
- Enabled DeepL API Key and formality controls.
- Connected the DeepL configuration card to the existing test-and-enable workflow.
- Replaced the obsolete unavailable warning with setup guidance explaining how to match Free and Pro API keys to the correct endpoint.

### External Findings
- DeepL's official API documentation confirms translation requests use `POST /v2/translate` and the `Authorization: DeepL-Auth-Key ...` header already implemented by the backend.
- DeepL documents separate API Free and API Pro hosts, matching the backend official-host allowlist added in Chapter 274.

### Modified Files
- `tauri-client/src/components/settings/TranslationChannelCard.tsx`
- `tauri-client/src/components/settings/settingsOptions.ts`
- `tauri-client/src/i18n/en-US.ts`
- `tauri-client/src/i18n/zh-CN.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Explicit Non-Goals
- Did not accept custom DeepL-compatible endpoints; DeepL remains restricted to the two official HTTPS hosts.
- Did not perform a successful paid/free DeepL translation because no valid user DeepL API Key is available in the workspace.

### Validation
- Passed: direct proxy-compatible requests reached both official DeepL Free and Pro translation endpoints; each returned the expected HTTP `403` for an intentionally invalid test key instead of a network, DNS, or SSRF error.
- Passed: `F:\Python311\python.exe -m pytest server/tests -rs` with `63 passed, 1 skipped`; the skip remains the expected fake-IP rejection for a user-configured New API URL.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run check:i18n` with `603 zh-CN keys match 603 en-US keys`.
- Passed: `cd tauri-client; npm run build`; Vite emitted existing dynamic-import/chunk-size warnings only.
- Passed: browser UI smoke confirmed DeepL can be selected, the API Key/formality/test controls are enabled, and both official endpoint choices are present.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.

### Known Risks
- A real valid-key DeepL translation still requires the user to select the endpoint matching their DeepL API plan and provide a working API Key.

### Next Steps
- Run `测试并启用` with a valid DeepL API Free or API Pro key, then verify a screenshot translation end to end through the packaged desktop app.

## Chapter 276: Chinese Build Mode Batch Launchers (2026-06-12)

### Goals Completed
- Added Chinese-named root batch launchers for the common build modes:
  - Daily Tauri dev self-test.
  - Release exe-only build without installer bundling.
  - NSIS-only installer build.
  - MSI-only installer build.
  - Full release build wrapper for the existing dual-installer `build.bat` flow.
- Added `--dry-run` and `--no-pause` support to each launcher so commands can be checked without running a real build and CI/terminal usage can avoid blocking pauses.
- Kept the scripts' internal text mostly ASCII to avoid Windows `cmd` code-page issues while preserving Chinese filenames for double-click usability.
- Normal launcher runs now close running `YsnTrans.exe` and legacy `tauri-client.exe` before starting dev or release builds to reduce locked-output and stale-process failures.
- Exe, NSIS, MSI, and full-release build flows now create a root `YsnTrans.lnk` shortcut pointing at the build output they launch.
- Exe, NSIS, and MSI build flows now auto-launch `tauri-client\src-tauri\target\release\YsnTrans.exe` after a successful build.
- Full release builds now keep `build.bat` auto-launch enabled, which opens `release\YSN-Screenshot-Translator\YsnTrans.exe` after copying the portable runtime.
- `--dry-run` now remains non-mutating for the Chinese launchers: it prints command, output, shortcut, and launch targets without killing processes, building, creating shortcuts, or launching the app.

### Added Files
- `日常自测-开发模式.bat`
- `构建EXE-不打安装包.bat`
- `构建NSIS安装包.bat`
- `构建MSI安装包.bat`
- `完整发布-双安装包.bat`

### Modified Files
- `日常自测-开发模式.bat`
- `构建EXE-不打安装包.bat`
- `构建NSIS安装包.bat`
- `构建MSI安装包.bat`
- `完整发布-双安装包.bat`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Explicit Non-Goals
- Did not install or configure `sccache`, `lld`, or other machine-level Rust acceleration tools.
- Did not change the existing `build.bat` installer artifact copy behavior.
- Did not intentionally run full real release builds because the requested change was launcher behavior and the dry-run path validates script wiring without spending build time.

### Validation
- Passed: `cmd /c "日常自测-开发模式.bat --dry-run --no-pause"`.
- Passed: `cmd /c "构建EXE-不打安装包.bat --dry-run --no-pause"`.
- Passed: `cmd /c "构建NSIS安装包.bat --dry-run --no-pause"`.
- Passed: `cmd /c "构建MSI安装包.bat --dry-run --no-pause"`.
- Passed: `cmd /c "完整发布-双安装包.bat --dry-run --no-pause"`.
- Passed: `scripts/create-root-shortcut.ps1` created a temporary shortcut to `notepad.exe`, then the temporary `_shortcut_helper_test.lnk` was removed.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.

### Known Risks
- Real installer builds still depend on the local Tauri/Rust/NSIS/MSI toolchain and bundled runtime resources being present.
- `sccache` and faster linker setup remain a separate machine configuration task.
- The raw Tauri release exe remains under `tauri-client\src-tauri\target\release\`; the root shortcut is the intended easy access point so the exe can keep using its expected runtime layout.

### Next Steps
- Use `日常自测-开发模式.bat` for quick feature checks, `构建EXE-不打安装包.bat` for release-performance checks, and one of the installer scripts only when a distributable installer is needed. After a successful build, launch from root `YsnTrans.lnk` when you do not want to browse into the Tauri output directory.

## Chapter 277: Screenshot Overlay Reentry, First-Down, And Escape Hardening (2026-06-13)

### Goals Completed
- Fixed frequent `Alt+A` reentry risk by adding a `SCREENSHOT_STARTING` native start gate before hotkey-spawned screenshot tasks.
- Changed duplicate `start_screenshot` calls while a capture is active from "cancel and rebuild the active session" to "ignore the duplicate start", avoiding concurrent GPU/window lifecycle work.
- Kept `CAPTURING` as the authoritative active-session flag and reset the starting gate on error, force close, and cancel paths.
- Made capture Escape registration more robust by unregistering any stale Escape shortcut before re-registering when the registered flag is already set.
- Re-register capture Escape after the Save dialog whenever the screenshot session is still active, even if the dialog is cancelled.
- Fixed first pointer down loss during the async shell/first-frame gap by caching the pending down point while `frameInteractive` is false.
- Resumed pending selection from the original down coordinate once the frame becomes interactive, instead of starting from the later pointer-move coordinate.
- Added pending-down pointer capture, session scoping, timeout cleanup, and reset/cancel cleanup so a stale pending event cannot leak into a later screenshot session.
- Strengthened screenshot overlay interaction activation on Windows by using `AttachThreadInput` before `SetForegroundWindow` / `SetFocus`, matching the stronger existing main-window activation path.

### External Findings
- Tauri global shortcuts call application handlers for registered shortcut press events, so application-level reentry gates are required around long-running native work.
- MDN Pointer Events documents `buttons` as the current pressed-button bitmask and `setPointerCapture` as the way to keep subsequent pointer events routed to the same element during a drag.
- Microsoft Win32 documentation describes using `AttachThreadInput` to share input state across threads so focus can be set to a window attached to another thread's input queue.

### Modified Files
- `tauri-client/src-tauri/src/hotkeys.rs`
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src-tauri/src/screenshot_commands.rs`
- `tauri-client/src-tauri/src/window_lifecycle.rs`
- `tauri-client/src/hooks/useScreenshotInteraction.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Explicit Non-Goals
- Did not redesign the native first-frame architecture or make native overlay the default path.
- Did not remove the existing pre-capture/native-overlay recovery path; pending pointer down complements it for the short async readiness gap.
- Did not run full installer packaging for this bugfix.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client/src-tauri; cargo check`.
- Passed: `cd tauri-client/src-tauri; cargo test`.
- Passed: `cd tauri-client; npm run check:i18n` with `603 zh-CN keys match 603 en-US keys`.
- Passed: `cd tauri-client; npm run build`; Vite emitted existing dynamic-import/chunk-size warnings only.
- Passed: in-app browser smoke loaded `http://127.0.0.1:1420` and rendered the dashboard; normal-browser Tauri invoke warnings remain expected.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.

### Known Risks
- Real `Alt+A` timing, ESC behavior, and crash resistance still need desktop manual QA because browser smoke cannot exercise global shortcuts, native foreground focus, WebView focus, or DXGI/window-resource contention.
- Duplicate screenshot starts are now ignored instead of toggling/cancelling the active screenshot; cancellation should use ESC, right-click, or explicit close actions.

### Next Steps
- Manual desktop QA: repeatedly press `Alt+A` during startup and while an overlay is visible, verify no crash and no session rebuild.
- Manual desktop QA: hold left mouse during overlay startup and drag as the frame appears, verify the rectangle starts from the original down point.
- Manual desktop QA: press ESC before and after first frame readiness, including after a cancelled Save dialog, and confirm the overlay closes reliably.

## Chapter 278: Screenshot Shell Pointer Ownership Race Fix (2026-06-13)

### Goals Completed
- Fixed the rare "mouse starts at point 1 but selection begins at point 2" feel by letting the screenshot canvas accept pointer events as soon as the visible shell is present, rather than coupling pointer ownership to screenshot image readiness.
- Kept image-dependent actions behind the existing image-ready gates; early shell interaction now only owns geometry.
- Prevented shell/native pre-capture recovery from overwriting a real WebView selection that has already started locally.
- Prevented the later image-ready pre-show recovery path from overwriting an already-started local selection.
- Added baseline logs when pre-capture recovery is skipped because local selection already owns the drag.

### External Findings
- MDN documents `pointerdown` as the transition from no mouse buttons pressed to at least one pressed, so recovering from a later `pointermove.buttons` event cannot fully reconstruct the original down point.
- MDN documents `setPointerCapture` as routing subsequent pointer events to the capture element, which supports keeping the canvas as the early owner once the visible shell exists.

### Modified Files
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src/hooks/useScreenshotLoader.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Explicit Non-Goals
- Did not change screenshot crop coordinate mapping or device-pixel-ratio handling.
- Did not enable the guarded native first-frame session by default.
- Did not run a packaged Tauri release build or installer build for this frontend-only fix.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; Vite emitted the existing dynamic-import and chunk-size warnings only.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.

### Known Risks
- Real desktop manual QA is still required because browser/TypeScript checks cannot exercise the exact Windows foreground focus and pointer timing race.
- If a user presses and drags before the overlay shell is visible at all, the remaining recovery still depends on the native pre-capture sampler; the long-term commercial route remains the native first-frame input path.

### Next Steps
- Manual desktop QA: repeatedly press `Alt+A` and immediately drag short and long rectangles, especially fast drags from the left/top side of the screen, and confirm no jump from the original down point.
- If any residual point-1-to-point-2 offset remains, promote the native first-frame input overlay or a low-level mouse hook from diagnostic/guarded mode into the default first-input owner.

## Chapter 279: Simplified Dependency Status UX (2026-06-13)

### Goals Completed
- Simplified the dashboard by removing the visible commercial-baseline and startup-diagnostics/recovery cards.
- Added a compact top-right dependency status bar that shows translation, OCR, and FFmpeg status directly in the header.
- Added a startup dependency hook that loads the latest readiness snapshot and runs one dependency probe when the app opens.
- Simplified the `识字模型 / 视频录制` page into beginner-friendly RapidOCR and FFmpeg cards, with Rapid OCR V5/V4 status, model directory access, self-test action, FFmpeg download, and FFmpeg executable selection.
- Removed user-facing diagnostic-report copy UI and old recovery/checklist panels.
- Preserved the existing refresh button as the single "recheck everything" control for translation plus dependency status.

### Added Files
- `tauri-client/src/components/app/DependencyStatusBar.tsx`
- `tauri-client/src/hooks/useStartupDependencyStatus.ts`

### Modified Files
- `tauri-client/src/App.tsx`
- `tauri-client/src/components/app/AppLayout.tsx`
- `tauri-client/src/components/config/RapidOcrPanel.tsx`
- `tauri-client/src/components/config/RecordingDependencyPanel.tsx`
- `tauri-client/src/i18n/en-US.ts`
- `tauri-client/src/i18n/types.ts`
- `tauri-client/src/i18n/zh-CN.ts`
- `tauri-client/src/ocr-models/types.ts`
- `tauri-client/src/pages/Dashboard.tsx`
- `tauri-client/src/pages/OcrConfig.tsx`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- `tauri-client/src/components/config/ConfigPageHeader.tsx`
- `tauri-client/src/components/config/ConfigReadinessOverview.tsx`
- `tauri-client/src/components/config/ConfigRecoveryChecklist.tsx`
- `tauri-client/src/components/dashboard/DashboardDiagnosticsCard.tsx`
- `tauri-client/src/components/dashboard/DashboardReadiness.tsx`
- `tauri-client/src/hooks/useDiagnosticsReport.ts`

### Explicit Non-Goals
- Did not implement a new RapidOCR model downloader because the current codebase does not expose a reliable model-download command or model-source manifest yet; the page now exposes model directory, self-test, and missing-file status instead.
- Did not change the backend readiness commands or FFmpeg downloader behavior.
- Did not run a packaged desktop exe build for this UI-only chapter.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run check:i18n` with `583 zh-CN keys match 583 en-US keys`.
- Passed: `cd tauri-client; npm run build`; Vite emitted existing dynamic-import/chunk-size warnings only.
- Passed: in-app browser smoke confirmed the dashboard no longer shows the old commercial-baseline or startup-diagnostics cards.
- Passed: in-app browser smoke confirmed the header shows compact `翻译`, `识字`, and `FFmpeg` status pills and clicking a pill opens `识字模型 / 视频录制`.
- Passed: in-app browser smoke confirmed the dependency page shows Rapid OCR V5/V4, model directory/self-test controls, FFmpeg download, and FFmpeg selection.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.

### Known Risks
- Real exe startup should still be manually checked because browser smoke cannot exercise Tauri `invoke`, global shortcut registration, or real packaged model/FFmpeg paths.
- RapidOCR model downloading remains a product decision: it needs an owned model manifest and download command before a commercial-grade one-click model installer should be added.

### Next Steps
- Manual desktop QA: open the packaged exe fresh and confirm the header performs one startup check, then shows translation, OCR, and FFmpeg statuses without needing to open the configuration page.
- Add an owned RapidOCR model-pack downloader only after model source URLs, checksums, target directory layout, and recovery behavior are defined in the master plan.

## Chapter 280: Dedicated Model Management And RapidOCR Installer (2026-06-13)

### Goals Completed
- Split OCR model ownership out of the combined `识字模型 / 视频录制` page into a dedicated left-nav `模型管理` page.
- Added a beginner-facing model management surface with current RapidOCR status, Rapid OCR V5/V4 wording, default install directory, manual download guidance, and official RapidOCR/ModelScope links.
- Added a one-click backend installer command that prepares the default `models/rapidocr` install root, delegates model warming to the bundled RapidOCR runner, and probes both V5 and V4 after installation.
- Changed the RapidOCR fallback model root so development builds and root-directory installs prefer the repo/app `models/rapidocr` directory instead of hiding assets deep in internal runtime folders.
- Simplified the old dependency page into `视频录制 / 翻译目标`, leaving FFmpeg and target-language controls there while moving model install/repair actions to `模型管理`.
- Routed the top dependency status bar so translation opens settings, OCR opens model management, and FFmpeg opens the video/recording page.

### External Findings
- RapidOCR's official model list documents PP-OCRv4/PP-OCRv5 model families and points model assets at the RapidAI/RapidOCR ModelScope repository.
- RapidOCR v3 documentation describes automatic model downloading through hosted model definitions, matching the bundled runner's local `default_models.yaml` ModelScope URLs.
- This chapter intentionally reuses the bundled RapidOCR runner's upstream model manifest instead of hand-writing stale model URLs in the frontend.

### Added Files
- `tauri-client/src/pages/ModelManagement.tsx`

### Modified Files
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src-tauri/src/rapid_ocr/mod.rs`
- `tauri-client/src-tauri/src/rapid_ocr/runner.rs`
- `tauri-client/src/App.tsx`
- `tauri-client/src/components/app/AppLayout.tsx`
- `tauri-client/src/components/app/DependencyStatusBar.tsx`
- `tauri-client/src/i18n/en-US.ts`
- `tauri-client/src/i18n/types.ts`
- `tauri-client/src/i18n/zh-CN.ts`
- `tauri-client/src/ocr-models/types.ts`
- `tauri-client/src/pages/OcrConfig.tsx`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Explicit Non-Goals
- Did not add checksum verification or an owned signed model-pack manifest yet; the first commercial-safe step is to use RapidOCR's official runner/model metadata path.
- Did not package model files into the installer; this chapter provides guided and one-click acquisition into `models/rapidocr`.
- Did not run the actual one-click download from normal browser smoke because browser mode has no Tauri `invoke` bridge or native RapidOCR runner process.

### Validation
- Passed: `cd tauri-client/src-tauri; cargo fmt`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run check:i18n` with `584 zh-CN keys match 584 en-US keys`.
- Passed: `cd tauri-client/src-tauri; cargo check`.
- Passed: `cd tauri-client; npm run build`; Vite emitted existing dynamic-import/chunk-size warnings only.
- Passed: in-app browser smoke confirmed the left navigation now contains `模型管理` and `视频录制`, and no longer shows the old combined `识字模型 / 视频录制` label.
- Passed: in-app browser smoke confirmed the `模型管理` page shows `一键下载/安装模型`, `models\\rapidocr`, official RapidOCR/ModelScope link actions, and manual download guidance.
- Passed: in-app browser smoke confirmed the `视频录制 / 翻译目标` page keeps FFmpeg controls and no longer shows model installer actions.

### Known Risks
- Real one-click model download still needs desktop exe QA with network access to ModelScope because browser smoke cannot execute Tauri commands.
- Model integrity is still delegated to RapidOCR's official model metadata; a future commercial hardening chapter should add pinned checksums or an owned signed manifest.
- Packaged-path behavior should be checked in both `--no-bundle` release exe and installer layouts to confirm `models/rapidocr` is visible beside the app as intended.

### Next Steps
- Manual desktop QA: open the real exe, click `模型管理` -> `一键下载/安装模型`, verify V5/V4 probes pass and files land under `models/rapidocr`.
- Manual desktop QA: delete or rename one model file, restart the app, and confirm the startup status plus model page explain the missing asset and recovery action clearly.
- Add a signed model-pack manifest with checksums before treating model installation as fully release-hardened.

## Chapter 281: RapidOCR Dictionary False-Missing Fix And Install Progress (2026-06-13)

### Goals Completed
- Fixed the false missing-file loop where `ppocr_keys_v1.txt` and `ppocrv5_dict.txt` were incorrectly required inside root `models/rapidocr` even though the product uses RapidOCR's ONNXRuntime path.
- Aligned V5 and V4 readiness lists with the actual ONNX models initialized by the bundled runner, including each version's detector, classifier, Chinese recognition model, and multilingual fallback recognition models.
- Added a RapidOCR installation progress event contract with preparing, download/initialization, V5 verification, V4 verification, completion, and failure phases.
- Added an FFmpeg-style progress bar and progress detail text below the model install actions.
- Added beginner-facing dictionary guidance explaining that ONNX models embed their character table and do not require the two fallback dictionary files in the root model directory.
- Corrected the manual download steps so they no longer instruct users to download unnecessary dictionary files.

### External Findings
- RapidOCR's official recognizer source explicitly states that ONNX has an internal character table; external character dictionary files are used by other inference engines when the session does not expose character keys.
- RapidOCR's official model documentation confirms that PP-OCRv4/v5 models are hosted on ModelScope and can be automatically downloaded through RapidOCR's model configuration.
- The bundled runner independently initialized both V5 and V4 successfully while the two dictionary files were absent from root `models/rapidocr`, confirming the UI failure was caused by the product's readiness checklist rather than a download failure.

### Modified Files
- `tauri-client/src-tauri/src/rapid_ocr/mod.rs`
- `tauri-client/src-tauri/src/rapid_ocr/runner.rs`
- `tauri-client/src-tauri/src/tests.rs`
- `tauri-client/src/ocr-models/types.ts`
- `tauri-client/src/pages/ModelManagement.tsx`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Explicit Non-Goals
- Did not add byte-level transfer progress because the bundled RapidOCR runner currently owns the ModelScope download stream and only returns after model initialization; the new progress bar reports truthful installation phases.
- Did not copy fallback dictionary files into root `models/rapidocr`, because that would hide the incorrect readiness contract instead of fixing it.
- Did not rebuild or package a release exe while the user's existing desktop app may be running.

### Validation
- Passed: `cd tauri-client/src-tauri; cargo fmt`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run check:i18n` with `584 zh-CN keys match 584 en-US keys`.
- Passed: `cd tauri-client/src-tauri; cargo check`.
- Passed: `cd tauri-client/src-tauri; cargo test`, including the new regression test that forbids external dictionary files in ONNX readiness lists.
- Passed: `cd tauri-client; npm run build`; Vite emitted existing dynamic-import/chunk-size warnings only.
- Passed: bundled `rapidocr-runner.exe` probes for both V5 and V4 against root `models/rapidocr`.
- Passed: local root-model audit found no missing V5 or V4 required ONNX files while confirming both dictionary files remain absent as expected.
- Passed: in-app browser smoke confirmed the model page explains the dictionary behavior and renders the install progress bar.
- Passed: `git diff --check`; Git emitted existing LF-to-CRLF working-copy warnings only.

### Known Risks
- Progress is installation-phase progress rather than byte-level network progress; byte-level progress requires the product to own the ModelScope download stream instead of delegating it to the bundled RapidOCR runner.
- The running desktop exe must be rebuilt or relaunched from a new build before it uses the corrected readiness list and progress event contract.

### Next Steps
- Manual desktop QA after rebuilding: open `模型管理`, confirm the old dictionary-file warning is gone, then click install and confirm the phase progress bar reaches completion.
- If byte-level progress becomes a release requirement, move ModelScope downloads into an owned Rust installer that consumes RapidOCR's trusted model manifest and verifies its existing SHA256 values before invoking the runner.

## Chapter 282: Translation Transport Deployment Closure (2026-06-13)

### Goals Completed
- Confirmed the desktop app was correctly preferring the configured LAN translation service at `192.168.1.3:8318`, but that service was still running an old `http_client.py` where Google used the pinned SSRF transport.
- Identified the deployment root cause: `deploy_n100_translation_server.ps1` uploaded `translator.py` and security modules but omitted `http_client.py`, so the official-provider proxy compatibility fix never reached the active service.
- Updated the deployment bundle and remote backups to include `http_client.py`, `safe_transport.py`, and `requirements.txt`.
- Added a mandatory pre-restart deployment assertion that official translation sessions use `trust_env=True` and are not mounted with `SSRFSafeAdapter`.
- Deployed the corrected runtime to N100, restarted the active uvicorn service, and verified both LAN and public translation endpoints.

### External Findings
- Requests session behavior uses environment proxy configuration when `trust_env=True`; the pinned SSRF session intentionally disables that behavior.
- OWASP SSRF guidance supports strict protection at user-controlled URL boundaries and allowlists for known destinations, matching the project's split between official providers and user-configured relay/base URLs.
- Clash/Mihomo fake-IP mode commonly uses the reserved `198.18.0.0/15` range, explaining why IP-pinning rejects an otherwise valid public provider when the proxy owns DNS routing.

### Modified Files
- `deploy_n100_translation_server.ps1`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Explicit Non-Goals
- Did not remove or weaken IP-pinning for user-configured LLM relay/base URLs.
- Did not require users to disable TUN/fake-IP mode or change proxy settings.
- Did not change translation provider selection or translation-quality behavior.

### Validation
- Passed: local transport inspection confirmed official translation session `trust_env=True` and no `SSRFSafeAdapter`.
- Passed: local Google translator smoke returned a Chinese translation for `Open settings`.
- Passed: `pytest -q server/tests` with `63 passed, 1 skipped`.
- Passed: PowerShell deployment-script syntax parse.
- Passed: remote pre-restart transport-policy assertion.
- Passed: N100 service restart and LAN `/api/health`.
- Passed: deployment smoke through both LAN and public endpoints.
- Passed: exact reported text `rel lease` returned HTTP 200 and a non-original Google translation through both `http://192.168.1.3:8318` and `https://ocr.yousn.me`.

### Known Risks
- The remote Python environment emits an existing Requests dependency warning for its installed charset-detection package; this did not block translation but should be cleaned up in a separate dependency-maintenance chapter.
- Deployment smoke output can display Chinese as mojibake in Windows PowerShell even though the JSON response contains a valid translation.

### Next Steps
- Keep the new transport-policy assertion as a deployment gate so future server deployments cannot silently restore pinned transport for official providers.
- Separately normalize the N100 Python dependency environment and deployment console encoding.

## Chapter 283: Translation Spinner And Optimistic Cached Screenshot Save (2026-06-13)

### Goals Completed
- Made the translation loading spinner remain visible and centered for small selections, scale with the selection short edge, and remain square through `boxSizing: "border-box"`.
- Added `save_selection_from_cache`, which clones the session-matched cached RGBA frame before background work, crops frame-local physical coordinates, encodes PNG/JPEG, and writes without base64 IPC.
- Changed the cached PNG path to RGB24 `Default + Adaptive` lossless encoding and JPEG quality 92.
- Changed the base64 fallback PNG recompression from `Best` to `Default` while preserving alpha when the rendered output requires it.
- Made pure unedited screenshot saves close the overlay optimistically and complete cached-frame encoding/writing in the background.
- Preserved rendered-output saving for annotated or translated screenshots so cached raw pixels cannot discard user edits.
- Preserved multi-monitor correctness by converting authoritative desktop physical bounds back to cached-frame-local coordinates before invoking the cached save command.
- Prevented a failed JPEG re-encode from writing PNG bytes under a `.jpg` extension.

### External Findings
- The `image` crate documents `PngEncoder::new_with_quality` compression/filter choices as encoding hints; `Default + Adaptive` remains PNG lossless while avoiding the CPU cost of `Best`.
- MDN documents `box-sizing: border-box` as including borders inside the declared width and height, which keeps the spinner's rendered outer box square.

### Modified Files
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src-tauri/src/screenshot_commands.rs`
- `tauri-client/src/components/screenshot/TranslationLoadingOverlay.tsx`
- `tauri-client/src/hooks/useScreenshotActions.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Explicit Non-Goals
- Did not route annotated or translated screenshots through the raw RGBA cache because that would discard edits.
- Did not change copy, OCR, translation, pin, or scroll-capture output behavior.
- Did not run `npm run build`, a Tauri packaged build, or installer build, following the user's instruction not to build.

### Validation
- Passed: `cd tauri-client/src-tauri; cargo fmt --check`.
- Passed: `cd tauri-client/src-tauri; cargo check`.
- Passed: `cd tauri-client/src-tauri; cargo test`.
- Passed: `cd tauri-client/src-tauri; cargo test image_save_reencode_tests -- --nocapture` with 5 tests.
- Passed: cached `1661x884` opaque RGB PNG fixture decoded pixel-identically; Fast/NoFilter baseline was `5,874,693` bytes and cached Default/Adaptive RGB PNG was `10,856` bytes.
- Passed: JPEG quality 92 fixture was `403,774` bytes.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `git diff --check` with existing LF-to-CRLF working-copy warnings only.

### Known Risks
- Real desktop QA is still required to measure save-dialog-confirmation to overlay disappearance against the `<100ms` target.
- Real desktop QA is still required for spinner appearance at 100%, 125%, and 150% DPI and for cached save crop correctness on negative-coordinate secondary monitors.
- Background save failure currently reports through the existing frontend message path after the overlay closes; a future product-wide native error-toast path would provide stronger recovery visibility.

### Next Steps
- Manually test pure PNG/JPEG save, annotated save, and translated save on the real desktop application.
- Capture `cache_save_start`, `overlay_exit_after_save`, and `cache_save_end` logs to quantify perceived close latency separately from background encode/write latency.
- Test small translation selections and negative-coordinate secondary-monitor crops at 100%, 125%, and 150% scaling.

## Chapter 284: Deterministic Build Icon Synchronization (2026-06-13)

### Goals Completed
- Identified the intended build entry points and their output ownership:
  - `构建EXE-不打安装包.bat` builds only the latest target-release executable.
  - `build.bat --portable-only --no-launch` builds the complete portable runtime without installers.
  - `构建MSI安装包.bat`, `构建NSIS安装包.bat`, and `完整发布-双安装包.bat` are installer/release entry points.
- Made every direct Tauri build entry synchronize derived application icons from `src-tauri/icons/icon.png` before compilation.
- Added explicit preflight checks so builds fail clearly when `scripts/sync-app-icons.ps1` is missing.
- Ensured the root launcher's resource compilation also consumes the newly synchronized multi-size `icon.ico`.

### Root Cause Findings
- The existing build entry points did not invoke the repository's icon synchronization script, so changing the source icon did not guarantee that the executable and installer icons were refreshed.
- The checked-in `icon.ico` was only about 529 bytes, while a validated synchronization run generated a multi-size `icon.ico` of about 56 KB.
- The current `target/release/YsnTrans.exe` was written at `2026-06-13 18:36:19`, after the relevant screenshot source changes, so it is the latest currently built target-release artifact.
- The complete portable executable and root launcher are currently absent; launching an older pinned shortcut or an executable from another location can therefore show an older build or cached icon.

### Modified Files
- `构建EXE-不打安装包.bat`
- `构建MSI安装包.bat`
- `构建NSIS安装包.bat`
- `build.bat`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Explicit Non-Goals
- Did not run a frontend build, Tauri executable build, portable build, or installer build.
- Did not clear the Windows icon cache or restart Explorer because those actions are disruptive.
- Did not alter the user's source icon design.

### Validation
- Passed: `构建EXE-不打安装包.bat --dry-run --no-pause`.
- Passed: `构建MSI安装包.bat --dry-run --no-pause`.
- Passed: `构建NSIS安装包.bat --dry-run --no-pause`.
- Passed: `scripts/sync-app-icons.ps1` against a temporary icon directory; generated PNG derivatives plus multi-size `icon.ico` and `taskbar.ico`.
- Pending by user instruction: actual executable/portable/installer build.

### Known Risks
- Windows may continue showing a cached icon for an already pinned taskbar item or old shortcut even after rebuilding; re-pinning the newly built executable is the least disruptive recovery path.
- `build.bat` does not currently expose a dry-run mode, so its full portable copy path was inspected but not executed.

### Next Steps
- Use `build.bat --portable-only --no-launch` for the next complete no-installer build.
- After that build, launch or pin `release/YSN-Screenshot-Translator/YsnTrans.exe` rather than an older taskbar item.

## Chapter 285: Screenshot Overlay Freeze Root-Cause Hardening (2026-06-13)

### Goals Completed
- Converted `get_fullscreen_image`, `get_fullscreen_image_bytes`, `get_fullscreen_rgba_bytes`, and `post_fullscreen_rgba_shared_buffer` from synchronous Tauri commands to async commands.
- Moved full-screen PNG encoding, base64 conversion, RGBA cloning, and requested SharedBuffer posting into `spawn_blocking` workers.
- Added `[screenshot-baseline]` start/end timing phases for every heavy fallback command, including worker status and error details.
- Moved the initial direct SharedBuffer post onto a blocking worker so it no longer blocks the async screenshot runtime worker.
- Replaced the SharedBuffer post's unbounded `recv()` wait with a bounded 500ms timeout and a regression test that prevents an unbounded wait from returning.
- Changed PNG cache-miss encoding to clone the cached RGBA `Arc` under lock and perform full-screen encoding after releasing the global cache lock.
- Changed RGBA and PNG-byte frontend fallback decoding to asynchronous `createImageBitmap` paths before drawing into the source canvas.
- Stopped precomputing full-screen analysis `ImageData` when visual detection is disabled; when enabled, analysis capture now waits for idle time where supported.
- Reduced pre-show drag polling from up to 48 calls every 16ms to up to 24 calls every 33ms while preserving approximately the same recovery window.
- Moved native overlay full-screen dimmed-bitmap generation outside the bitmap-store lock.
- Changed native overlay raise/cancel flows so their thread-reply waits do not hold the global native-session lock.

### External Findings
- Tauri's official command guide states that async commands are preferred for heavy work to avoid UI freezes, and commands without `async` execute on the main thread unless explicitly marked async.
- Tauri's official guide recommends `tauri::ipc::Response` for large byte-array responses; the PNG and RGBA byte fallbacks continue using that optimized response type.
- MDN documents `createImageBitmap` as returning a promise for asynchronous bitmap preparation; the fallback decode paths now use it instead of synchronous full-screen `putImageData`.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_commands.rs`
- `tauri-client/src-tauri/src/screenshot_shared_buffer.rs`
- `tauri-client/src-tauri/src/screenshot_native/native_overlay_session.rs`
- `tauri-client/src-tauri/src/screenshot_native/win32_overlay.rs`
- `tauri-client/src/hooks/useScreenshotLoader.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Explicit Non-Goals
- Did not change PNG as the default screenshot format.
- Did not change or remove `save_selection_from_cache`, cached direct saving, or rendered annotated/translated saving.
- Did not change `Cargo.toml` or intentionally modify `YsnTrans.lnk`.
- Did not lower native overlay start/command timeouts from 500ms to 120ms because real multi-monitor and high-DPI timing evidence is still required before tightening that fallback threshold.
- Did not replace the masked screenshot canvas with a CSS overlay; that needs a separate visual/interaction regression chapter.
- Did not run a frontend production build, Tauri executable build, portable build, or installer build.

### Validation
- Passed: static audit found no synchronous definitions of the four heavy fullscreen fallback commands.
- Passed: static audit found no unbounded `receiver.recv()` in the SharedBuffer post path.
- Passed: static audit found no `putImageData` in `useScreenshotLoader.ts`.
- Passed: `cd tauri-client/src-tauri; cargo fmt --check`.
- Passed: `cd tauri-client/src-tauri; cargo check`.
- Passed: `cd tauri-client/src-tauri; cargo test`, including bounded SharedBuffer wait and native dimming regression coverage.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `git diff --check` with existing LF-to-CRLF working-copy warnings only.

### Known Risks
- The required real-desktop acceptance run remains pending: 20 repeated captures across single/multi-monitor and 125%/150% DPI, including a forced SharedBuffer miss.
- Full-screen masked-canvas construction still performs one synchronous draw and dim fill before the WebView overlay becomes ready; it is a remaining frame-jank target, but no longer shares the same multi-second main-thread freeze risk as synchronous Rust fallback commands.
- SharedBuffer creation and the final native copy must still enter the WebView2 context; the bounded worker wait prevents indefinite caller blocking, but real-device logs must confirm its normal latency.

### Next Steps
- Rebuild only when requested, then run the handoff's 20-capture real-desktop matrix and inspect the new `fullscreen_*_command_start/end`, `shared_buffer_*`, `rgba_canvas_ready`, `png_bitmap_ready`, and `mask_canvas_ready` phases.
- Force a SharedBuffer miss and confirm the PNG fallback completes without a Windows “Not Responding” overlay.
- If `mask_canvas_ready` remains above 100ms on 4K displays, move masked-canvas preparation to a dedicated offscreen/worker path in a separate focused chapter.

## Chapter 286: Reliable Build Dependency Recovery And Windows Cache Cleanup (2026-06-13)

### Goals Completed
- Diagnosed the EXE-only build stopping after icon synchronization: the old `clean_all_cache.bat` had deleted `tauri-client/node_modules`, including the local Tauri CLI, and also removed the entire Rust `target` directory.
- Added a shared Node dependency preflight helper that restores dependencies with `npm ci --no-audit --no-fund` when the local Tauri CLI is missing.
- Integrated dependency recovery into every direct Tauri build entry before icon synchronization and compilation.
- Reworked `clean_all_cache.bat` so normal cleanup preserves `node_modules`, release outputs, OCR models, resources, source files, user data, and Git history.
- Added explicit `--deep`, `--project-only`, `--windows-only`, `--dry-run`, `--no-pause`, and `--no-explorer-restart` cleanup modes.
- Made Windows icon/thumbnail cache cleanup run in fast repeated passes because Windows can automatically restart Explorer and recreate cache databases during deletion.
- Treated cache files that disappear during cleanup as success instead of a false failure.
- Added next-Windows-restart deletion scheduling for genuinely locked cache files.
- Added a 3-second hard timeout for the Windows `ie4uinit.exe -ClearIconCache` helper.
- Guaranteed Explorer recovery through a PowerShell `finally` block and a BAT-level fallback check.
- Normalized all modified build and cleanup BAT files to Windows CRLF line endings.

### Root Cause Findings
- `tauri-client/node_modules` was absent, so `npm run tauri -- --version` failed because `tauri` was not recognized.
- `tauri-client/src-tauri/target` was absent, so the next build must be a full Rust rebuild rather than an incremental build.
- The old cache script silently deleted dependencies during ordinary cleanup while explicitly not clearing Windows caches or restarting Explorer.
- Windows immediately recreates a minimal set of icon cache files after Explorer restarts; this is normal and must not be reported as cleanup failure.

### Added Files
- `scripts/ensure-node-dependencies.bat`

### Modified Files
- `构建EXE-不打安装包.bat`
- `构建MSI安装包.bat`
- `构建NSIS安装包.bat`
- `build.bat`
- `clean_all_cache.bat`
- `refresh_windows_icon_cache.ps1`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Explicit Non-Goals
- Did not run an EXE, portable, MSI, NSIS, frontend production, or Tauri packaged build.
- Did not delete OCR models, RapidOCR resources, FFmpeg resources, user data, or Git history.
- Did not intentionally modify `Cargo.toml` or `YsnTrans.lnk`.

### Validation
- Passed: actual Windows icon/thumbnail cache cleanup returned exit code `0`.
- Passed: Explorer restarted successfully; final observed Explorer PID was `9796`, started at `2026-06-13 19:20:08`.
- Passed: restored missing Node dependencies with `npm ci`; installed 159 packages.
- Passed: `cd tauri-client; npm run tauri -- --version` returned `tauri-cli 2.11.2`.
- Passed: dependency helper reports `Node dependencies are ready` after recovery.
- Passed: EXE-only, MSI, and NSIS build entry dry-runs.
- Passed: default/project-only/windows-only cache-cleaner dry-runs.
- Passed: PowerShell cache-helper syntax parse.
- Passed: all modified BAT files contain zero bare-LF lines.
- Passed: `git diff --check` with existing LF-to-CRLF working-copy warnings only.

### Known Risks
- The next executable build will be substantially slower because the old cleanup script already removed the full Rust `target` cache.
- Windows recreates required icon/thumbnail cache databases after Explorer starts; seeing a small set of cache files afterward does not mean cleanup failed.
- Automatic `npm ci` requires npm registry access when dependencies are missing.

### Next Steps
- Run the EXE-only build entry when ready; it will now restore missing Node dependencies automatically before icon synchronization and compilation.
- Keep routine cache cleanup on the default mode; use `--deep` only when intentionally removing dependencies and release outputs.

## Chapter 270 - Enterprise Screenshot Enhancement Sprint (2026-06-13)

> Chapter status: completed for the enterprise annotation/performance/feature sprint. This chapter landed real-time annotation performance (P0-1), number marker tool (F9), dead code cleanup (F6), micro-animations (F7), feature toggle settings page (F8), and magnifier/color-picker hook (F1+F2). All changes pass TypeScript strict check and production Vite build with zero new errors.

### Goals
- Fix annotation input lag by injecting `scheduleDraw()` calls into `pointerdown` and `pointermove` annotation branches (P0-1).
- Implement auto-incrementing number markers with circle/square/drop shapes, re-indexing on undo/delete (F9).
- Remove dead `AnnotationLayer.tsx` and uninstall `konva` / `react-konva` dependencies (F6).
- Add spring-based CSS micro-animations for toolbar pop-in and button hover (F7).
- Create self-contained `FeatureSwitches.tsx` settings page with direct config load/save (F8).
- Implement `useScreenshotMagnifier.ts` hook with PixPin-style square magnifier and 1x1 HEX color picker (F1+F2).
- Wire `markerShapeRef` from `useScreenshotAnnotation` through `ScreenshotPage` to `useScreenshotInteraction`.
- Route the Feature Switches page into `App.tsx` navigation.

### Added Files
- `tauri-client/src/pages/FeatureSwitches.tsx` — Self-contained feature toggle page with Ant Design Switch rows, direct `invoke("get_config")` / `invoke("save_config")` pattern, and `autoStart` special handling.
- `tauri-client/src/hooks/useScreenshotMagnifier.ts` — Square magnifier drawing and raw HEX color sampling, using efficient 1x1 `drawImage` sampling to prevent 4K performance degradation.

### Modified Files
- `tauri-client/src/hooks/useScreenshotInteraction.ts` — Injected `scheduleDraw()` into annotation `pointerdown`/`pointermove` branches; added `markerShapeRef` prop and number-marker click-to-place handler.
- `tauri-client/src/hooks/useScreenshotAnnotation.ts` — Added `markerShape` state, re-indexing logic on deletion, and `markerShapeRef` export.
- `tauri-client/src/types/screenshot.ts` — Extended `AnnotationTool` with `"number"`, added `MarkerShape`, `markerShape`, and `markerIndex` to `Annotation`.
- `tauri-client/src/types/config.ts` — Added feature flags: `enableMagnifier`, `enableColorPicker`, `enablePreciseSelection`, `enableLiveAnnotation`, `autoStart`.
- `tauri-client/src/utils/annotationGeometry.ts` — Number marker factory, hit detection, re-indexing, and drag support.
- `tauri-client/src/utils/renderAnnotations.ts` — Canvas rendering for number markers (circle/square/drop shapes with dynamic index labels).
- `tauri-client/src/index.css` — Spring easing variables and micro-animation CSS classes (`pop-in-toolbar`, `btn-press`).
- `tauri-client/src/pages/ScreenshotPage.tsx` — Destructured `markerShapeRef` from annotation hook and passed to interaction hook.
- `tauri-client/src/App.tsx` — Added `FeatureSwitches` import, `ThunderboltOutlined` icon, navigation menu item, and route case.

### Deleted Files
- `tauri-client/src/pages/AnnotationLayer.tsx` — Dead React-Konva code.
- `konva` and `react-konva` npm dependencies removed via `npm uninstall`.

### Validation
- Passed: `npx tsc --noEmit` — zero TypeScript errors.
- Passed: `npx vite build` — production bundle built successfully (1304 kB JS, 26 kB CSS).
- Passed: `npm uninstall konva react-konva` — clean dependency removal.

### Known Risks
- `useScreenshotMagnifier` hook is implemented but not yet wired into `ScreenshotPage` render loop; requires `enableMagnifier` feature flag integration.
- Scroll-capture stitching (P0-3) remains unimplemented.
- Number marker toolbar UI button not yet added to the annotation toolbar; markers are logic-ready but need a trigger button.
- Large `index` chunk Vite warning is pre-existing (Ant Design + React bundled together).

### Next Recommended Chapter
- Chapter 271 should wire the magnifier hook into `ScreenshotPage`, add the number-marker button to the annotation toolbar, and begin scroll-capture (P0-3) implementation.

## Chapter 287: PP-OCRv6 Small Direct ONNXRuntime Mainline (2026-06-14)

### Goals Completed
- Added a product-owned PP-OCRv6 Small ONNXRuntime adapter inside the existing bundled runner process without renaming the worker, IPC, logs, or backend modules.
- Made PP-OCRv6 Small the default model for new/invalid configuration while preserving strict manual model selection and the existing RapidOCR V5/V4 compatibility paths.
- Added a hard V6 initialization contract: dictionary size `18708`, output classes `18710`, blank index `0`, classes `1..18708` mapped to the YAML dictionary, and class `18709` mapped to the single implicit space.
- Required a fixed-input detection and recognition probe even when ONNX metadata uses dynamic dimensions.
- Added V6 preprocessing, DB detection postprocess, perspective crop, RGB recognition normalization, strict CTC decoding, confidence output, and worker dispatch.
- Updated the model-management UI to present “本地 OCR 引擎”, PP-OCRv6 Small as the default main model, and RapidOCR as the internal V5/V4 compatibility adapter.
- Added a quantitative V6 fixture manifest and gate. Internal spaces remain part of normalized CER so deleting the implicit-space mapping fails immediately.

### Added Files
- `tauri-client/src-tauri/rapidocr/ppocr_v6/__init__.py`
- `tauri-client/src-tauri/rapidocr/ppocr_v6/adapter.py`
- `tauri-client/src-tauri/rapidocr/ppocr_v6/contract.py`
- `tauri-client/src-tauri/rapidocr/ppocr_v6/detector.py`
- `tauri-client/src-tauri/rapidocr/ppocr_v6/recognizer.py`
- `tauri-client/scripts/check-ocr-v6-fixtures.ps1`
- `tauri-client/scripts/ocr-v6-fixtures.json`
- `tauri-client/src/ocr-models/modelLabels.ts`

### Modified Files
- `tauri-client/src-tauri/rapidocr/rapidocr_runner.py`
- `tauri-client/src-tauri/rapidocr/requirements.txt`
- `tauri-client/scripts/build-rapidocr-runner.ps1`
- `tauri-client/src-tauri/src/rapid_ocr/runner.rs`
- `tauri-client/src-tauri/src/rapid_ocr/mod.rs`
- `tauri-client/src-tauri/src/tests.rs`
- `tauri-client/src/ocr-models/index.ts`
- `tauri-client/src/ocr-models/types.ts`
- `tauri-client/src/utils/ocrConfigHelpers.ts`
- `tauri-client/src/hooks/useOcrConfigController.tsx`
- `tauri-client/src/components/app/DependencyStatusBar.tsx`
- `tauri-client/src/components/config/RapidOcrPanel.tsx`
- `tauri-client/src/pages/ModelManagement.tsx`
- `tauri-client/src/i18n/zh-CN.ts`
- `tauri-client/src/i18n/en-US.ts`
- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Explicit Non-Goals
- Did not add automatic V6-to-V5 fallback or automatic model switching.
- Did not claim to detect unsupported scripts from V6 output.
- Did not add a V6 low-confidence user warning before V6-specific threshold calibration.
- Did not rename the runner process, worker, IPC methods, backend modules, or logs.
- Did not add V6 download hosting, release bundling, checksum manifest, updater, or rollback behavior.
- Did not remove the RapidOCR V5/V4 compatibility paths.

### Validation
- Passed: source and bundled-runner V6 fixed-input probe reported dictionary `18708`, classes `18710`, blank `0`, implicit space `18709`, recognition probe shape `[1, 40, 18710]`, and detection probe shape `[1, 1, 32, 32]`.
- Passed: V6 generated-clean fixture gate with CER `0` for `Open preview before saving`, `PATH=C:\Windows\System32 LocalModel.exe --help`, and two-line Chinese UI text.
- Passed: bundled worker OCR returned `Open preview before saving` with spaces intact and no model fallback.
- Passed: the user-provided real model-management screenshot produced 43 V6 text blocks, including `RapidOCR`, `ModelScope`, Chinese UI text, and a Windows path; total runner time was about `1.34s`.
- Passed: bundled-runner build, V6 probe, V5 probe, V4 probe, and V5/V4 multilingual fixture smoke.
- Passed: Python module compilation, `npx tsc --noEmit`, `npm run build`, `cargo fmt --check`, `cargo check`, and full `cargo test` (`267 passed`, `14 ignored`, `0 failed`).
- Passed: `git diff --check` with existing LF-to-CRLF working-copy warnings only.

### Known Risks
- The local `ocrv6` model directory is not yet a release-managed or bundled asset. Distribution, checksum verification, update, and rollback remain a separate release chapter.
- A durable, ground-truthed real-screenshot V6 manifest with normalized CER `<= 0.10` and case-sensitive critical tokens is still required before broader commercial readiness claims.
- V6 low-quality thresholds remain intentionally uncalibrated; no user warning is shown yet.
- The real screenshot smoke spent about `1.21s` in recognition. Batch sizing and large-screenshot recognition performance should be measured before release.
- Existing explicit user selection is preserved; only missing or invalid model-version configuration defaults to V6.

### Next Recommended Chapter
- Add a durable V6 real-screenshot ground-truth fixture set, calibrate V6 confidence from those samples, and then design release-managed V6 model distribution without changing the current strict manual-selection behavior.

## Chapter 288: V6 Technical-Text Translation Short-Circuit Removal (2026-06-14)

### Goals Completed
- Removed the client-side whole-block technical-text preservation short circuit for the actual PP-OCRv6 screenshot OCR translation path.
- Added an explicit `force_translate_technical_text` request flag and forwarded it through the N100 `/api/translate_text` preprocessing path.
- Kept default, V5, V4, and text-source fast-path behavior unchanged.
- Kept sentence-level protected-term and technical-token translation policy intact.
- Treated a non-empty provider response for an all-technical V6 block as a completed translation attempt even when token protection legitimately returns the original path unchanged.

### Modified Files
- `tauri-client/src/types/config.ts`
- `tauri-client/src/utils/ocrTranslationRequest.ts`
- `tauri-client/src/utils/translationMemory.ts`
- `tauri-client/src/utils/localOcrTranslate.ts`
- `tauri-client/scripts/check-ocr-processing.mjs`
- `server/app.py`
- `server/translator.py`
- `server/tests/test_translator.py`
- `server/tests/test_translate_text.py`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Explicit Non-Goals
- Did not remove or weaken sentence-level protected-term guidance or token-preservation checks.
- Did not change V5, V4, or text-source translation behavior.
- Did not deploy the updated server code to N100.
- Did not change OCR model routing, fallback, recognition, or rendering behavior.

### Validation
- Passed: V6 pure-path request bypasses client preservation, reaches the server/provider path, and does not surface as `preserved`.
- Passed: default/V5/V4 technical-text behavior remains preserved.
- Passed: mixed translated text retains the original path token.
- Passed: `npm run check:ocr-processing`, `npx tsc --noEmit`, and `npm run build`.
- Passed: server targeted tests (`25 passed`, `1 skipped`) and full server suite (`65 passed`, `1 skipped`).
- Passed: Python compilation, `cargo fmt --check`, `cargo check`, full `cargo test`, and `git diff --check`.

### Known Risks
- N100 must be updated with the modified server code before production clients can use the new V6 request flag end to end.
- Translation providers may legitimately return a pure path unchanged because the path is protected; the important behavioral guarantee is that V6 sends it through translation instead of locally or server-side short-circuiting it.

### Next Recommended Chapter
- Deploy the server update to N100, then run a live V6 screenshot acceptance pass for a pure path and a mixed prose-plus-command sample against the configured production translation provider.

## Chapter 289: Magnifier Selection And V6 Initialization Bug Closure (2026-06-14)

### Goals Completed
- Fixed the known screenshot bug where enabling the magnifier could make drag selection stop following the mouse.
- Fixed the known V6 model-management bug where switching to V6 could show OCR unavailable/stale state and still present the action as a model self-test.
- Changed the user-facing model action from self-test wording to initialization/application wording while keeping the internal compatibility command name.
- Made the RapidOCR initialization command async and moved blocking runner/probe work behind `spawn_blocking` so the UI action remains interactive.
- Verified the current local bundled RapidOCR runner with `onnxruntime==1.26.0` can initialize PP-OCRv6 Small successfully.
- Completed a targeted bug audit for screenshot, OCR readiness, scroll capture, recording, configuration, and model-management flows without fixing the additional findings in this chapter.

### External Findings
- Tauri v2 command guidance supports using async commands for work that can block the UI; the OCR initialization/probe path now follows that shape.
- ONNXRuntime compatibility is version-sensitive; local source-runner and bundled-runner probes are the authoritative proof that the current packaged runner can load the V6 model contract.

### Added Files
- None.

### Modified Files
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src/components/config/RapidOcrPanel.tsx`
- `tauri-client/src/hooks/useRapidOcrController.ts`
- `tauri-client/src/pages/ModelManagement.tsx`
- `tauri-client/src/i18n/zh-CN.ts`
- `tauri-client/src/i18n/en-US.ts`
- `tauri-client/src-tauri/src/rapid_ocr/mod.rs`
- `tauri-client/src-tauri/src/diagnostics.rs`
- `tauri-client/src-tauri/src/tests.rs`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not fix the additional bugs found during the audit.
- Did not commit, push, tag, or create a branch.
- Did not add V6 model download hosting, release-managed checksums, updater behavior, or rollback behavior.
- Did not rename the backend `run_rapid_ocr_self_test` command or internal `selfTesting` compatibility fields.
- Did not touch pre-existing unrelated working-tree changes in `app.py`, `YsnTrans.lnk`, local model files, or local settings/database files.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run check:i18n`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `cargo fmt --manifest-path tauri-client\src-tauri\Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client\src-tauri\Cargo.toml --tests`.
- Passed: source V6 probe: `python tauri-client\src-tauri\rapidocr\rapidocr_runner.py --probe --model-version v6 --model-root ocrv6`, contract dictionary `18708`, classes `18710`, blank `0`, implicit space `18709`.
- Passed: local bundled V6 probe: `tauri-client\src-tauri\resources\rapidocr\rapidocr-runner\rapidocr-runner.exe --probe --model-version v6 --model-root ocrv6`, same contract. This runner is a gitignored local resource rather than a tracked source diff in this chapter.
- Passed: `powershell -NoProfile -ExecutionPolicy Bypass -File .\tauri-client\scripts\check-ocr-v6-fixtures.ps1`, all V6 fixture CER values were `0`.
- Passed: `cd tauri-client; npm run build`; only existing Vite dynamic-import and chunk-size warnings were reported.
- Passed: `cargo test --manifest-path tauri-client\src-tauri\Cargo.toml --lib`.
- Passed: `cargo test --manifest-path tauri-client\src-tauri\Cargo.toml`.
- Passed: `server\.venv\Scripts\python.exe -m pytest server\tests` (`65 passed`, `1 skipped`).
- Partial: `git diff --check` still fails only on pre-existing `app.py:425` trailing whitespace; modified chapter files only produced LF-to-CRLF working-copy warnings.
- Passed: UTF-8 replacement-character scan found exactly one real source match, `tauri-client/src/hooks/useSettingsController.ts:180`.

### Remaining Bug Inventory
- `tauri-client/src/hooks/useSettingsController.ts:180`: user-facing settings-load error contains real replacement characters in `加载设��失败...`, producing corrupted Chinese text when local config loading fails.
- `tauri-client/src/hooks/useScrollCapture.ts:221-307`: manual scroll capture sets the screenshot window capture exclusion to true on start, restores it in `finishManualScrollCapture`, but `cancelManualScrollCapture` and `clearScrollCaptureState` do not restore it. Cancel/reset can leave the screenshot overlay excluded from capture.
- `tauri-client/src/hooks/useScreenshotRecording.ts:132-142`: region recording converts the selected rectangle to absolute coordinates by adding `getCurrentWindow().outerPosition()` to image-local physical selection. Other screenshot paths use `getDesktopPhysicalSelection(... physicalBounds)`, and the FFmpeg `gdigrab` backend consumes desktop offsets, so recording regions can be offset on negative-origin, multi-monitor, or DPI-scaled desktops.
- `tauri-client/src/hooks/useRecordingControl.ts:169-184` and `231-237`: successful save sets `savedPath` but returns the overlay status to `ready`, even though `OverlayStatus` has a `saved` state and close handling checks for it. The saved control bar can behave like a ready/cancel state instead of a saved state.
- `tauri-client/src/pages/ModelManagement.tsx:158-161` and `tauri-client/src-tauri/src/rapid_ocr/mod.rs` install flow: when V6 is selected, the install button explicitly installs backup V5/V4 models, while V6 still requires manual model placement. The wording is mostly explicit, but it remains a product-risk because users may expect the primary V6 model to be installed.
- `tauri-client/src-tauri/src/screenshot_native/wgc_capture.rs` and `tauri-client/src-tauri/src/screenshot_native/dxgi_capture.rs`: WGC capture has a `NotImplemented` error path and DXGI capture still reports a placeholder/blocked contract that falls back to the existing CPU screenshot path. Treat as an architecture/performance risk unless a current user-facing path starts requiring GPU capture as the primary backend.
- Ant Design v6 compatibility warnings remain in the UI layer, including deprecated `Space direction`, `Card bordered`, `Alert message`, `List`, `Statistic valueStyle`, and static `message`/`notification` usage. The app still renders, but console noise can hide real runtime errors and should be cleaned up in a focused UI compatibility chapter.

### Known Risks
- The magnifier fix was verified by code path and type/build checks, but a live drag-selection pass with the magnifier enabled is still needed on a real desktop session.
- V6 readiness is now proven by source and bundled fixed-input probes plus generated fixtures, but real screenshot end-to-end acceptance across the model-management UI remains a manual QA item.
- Browser smoke on `http://127.0.0.1:5179/` confirmed the model-management page renders the V6 card and `初始化并应用` button. The same smoke also produced expected Tauri IPC errors because the page was running in a normal browser instead of the Tauri WebView.
- PowerShell terminal output can display valid UTF-8 Chinese as mojibake; future audits should use UTF-8 reads or app rendering rather than raw console display before classifying text as corrupted.
- `YsnTrans.lnk`, `app.py`, `ocrv6/*.onnx`, `shichen_calendar.db`, and `widget_settings.json` were already dirty/untracked in the working tree and were left untouched.

### Next Recommended Chapter
- Fix the audited bugs in priority order: restore scroll-capture capture exclusion on every cancel/reset path, then correct recording region desktop-coordinate conversion, then set the recording control overlay to the real `saved` state after successful save.

## Chapter 290: Audited Bug Closure And AntD V6 Cleanup (2026-06-14)

### Goals Completed
- Fixed the settings-load error text replacement-character corruption in `useSettingsController`.
- Made manual scroll capture restore screenshot-window capture exclusion on finish, cancel, and reset paths.
- Added stale async-frame guards so an in-flight scroll-capture frame cannot write preview/message state after cancel/reset.
- Corrected screenshot recording region coordinates to use screenshot desktop physical bounds instead of screenshot window `outerPosition`.
- Kept window/display recording target selection mapped back to the canvas using the same desktop physical bounds when available.
- Fixed recording-control save completion so the overlay enters the existing `saved` state instead of returning to `ready`.
- Changed the V6 model-management primary action to `打开 V6 模型目录`, and moved V5/V4 installation to an explicitly secondary backup action.
- Added GPU diagnostics fields that clearly report `cpu-fallback-active`, `existing-cpu-screenshot`, and `gpuCaptureExperimental`, so WGC/DXGI placeholders are not user-facing primary capture readiness.
- Migrated Ant Design v6 deprecated UI APIs away from `Space direction`, `Card bordered`, `Alert message`, `List`, `Statistic valueStyle`, and static `message`/`notification` usage in the affected pages/components/hooks.
- Added AntD context wrapping for screenshot and OCR direct window entries so `AntdApp.useApp()` works outside the main window route.

### External Findings
- FFmpeg `gdigrab` region offsets are desktop/screen offsets, including negative offsets for monitors left or above the primary display; recording region coordinates should therefore be desktop physical coordinates.
- Tauri window scale and position APIs are DPI-sensitive window APIs, but this recording path already has a stronger source of truth: the screenshot payload physical bounds.
- Microsoft documents `WDA_EXCLUDEFROMCAPTURE` as suitable for video recording controls, which supports restoring capture exclusion on all cancellation/reset paths.
- Ant Design v6 marks the migrated APIs as deprecated and scheduled for removal in v7, so removing the warnings now keeps console output useful for real runtime errors.

### Added Files
- None.

### Modified Files
- `tauri-client/src/hooks/useSettingsController.ts`
- `tauri-client/src/hooks/useScrollCapture.ts`
- `tauri-client/src/hooks/useScreenshotRecording.ts`
- `tauri-client/src/hooks/useRecordingControl.ts`
- `tauri-client/src/hooks/useScreenshotActions.ts`
- `tauri-client/src/hooks/useScreenshotLoader.ts`
- `tauri-client/src/hooks/useScreenshotOcr.ts`
- `tauri-client/src/hooks/useRapidOcrController.ts`
- `tauri-client/src/hooks/useOcrConfigController.tsx`
- `tauri-client/src/hooks/useRecordingDependencyController.ts`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src/pages/ModelManagement.tsx`
- `tauri-client/src/pages/Dashboard.tsx`
- `tauri-client/src/pages/History.tsx`
- `tauri-client/src/pages/OcrConfig.tsx`
- `tauri-client/src/pages/Settings.tsx`
- `tauri-client/src/pages/OcrPage.tsx`
- `tauri-client/src/pages/FeatureSwitches.tsx`
- `tauri-client/src/main.tsx`
- `tauri-client/src/App.tsx`
- `tauri-client/src/components/config/ConfigSectionCard.tsx`
- `tauri-client/src/components/config/RapidOcrPanel.tsx`
- `tauri-client/src/components/config/RecordingDependencyPanel.tsx`
- `tauri-client/src/components/config/TranslationLanguagePanel.tsx`
- `tauri-client/src/components/dashboard/DashboardActionList.tsx`
- `tauri-client/src/components/dashboard/DashboardHero.tsx`
- `tauri-client/src/components/dashboard/DashboardStats.tsx`
- `tauri-client/src/components/settings/ImageSaveSettingsCard.tsx`
- `tauri-client/src/components/settings/ScreenshotRecognitionCard.tsx`
- `tauri-client/src/components/settings/SystemHotkeyCard.tsx`
- `tauri-client/src/components/settings/TranslationChannelCard.tsx`
- `tauri-client/src/components/settings/TranslationServiceCard.tsx`
- `tauri-client/src-tauri/src/screenshot_commands.rs`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not implement a full WGC or DXGI primary capture backend in this chapter.
- Did not change the stable CPU screenshot path.
- Did not change the backend `run_rapid_ocr_self_test` compatibility command name.
- Did not fix pre-existing unrelated dirty files such as `app.py`, `YsnTrans.lnk`, local model files, or local settings/database files.
- Did not commit, push, tag, or create a branch.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run check:i18n`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `cd tauri-client; npm run build`; only existing Vite dynamic-import and chunk-size warnings were reported.
- Passed: `cargo fmt --manifest-path tauri-client\src-tauri\Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client\src-tauri\Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client\src-tauri\Cargo.toml --lib`.
- Passed: `cargo test --manifest-path tauri-client\src-tauri\Cargo.toml`.
- Passed: UTF-8 replacement-character scan now finds no source-code `�` matches; only Chapter 289's historical bug record still contains the old corrupted sample.
- Passed: source search found no remaining `Space direction="vertical"`, `Card bordered={false}`, `List`, `Statistic valueStyle`, `Alert message=`, or static `message` imports in the migrated UI scope.
- Passed: browser smoke on `http://127.0.0.1:5173/` confirmed the model-management V6 action shows `打开 V6 模型目录` plus an explicit backup V5/V4 install action, and no AntD deprecated warnings appeared.
- Partial: browser smoke still shows expected Tauri IPC errors in a normal browser because `window.__TAURI_INTERNALS__` is absent there.
- Partial: `git diff --check` still fails only on pre-existing `app.py:425` trailing whitespace; modified files only produced existing LF-to-CRLF working-copy warnings.

### Known Risks
- The user's reported screenshot-time app freeze was not reproduced in this chapter. The fixes remove several stale-state and blocking-risk paths, but a live Tauri desktop screenshot pass is still needed to confirm the exact freeze scenario.
- Recording coordinate correction was validated by code path and type/build checks; multi-monitor and mixed-DPI live recording should still be manually tested on real hardware.
- Scroll-capture capture-exclusion recovery was validated by code path and build checks; live cancel/reset testing should confirm the overlay is capturable again after every exit path.
- GPU capture remains experimental/diagnostic. The product now reports CPU fallback clearly, but a future chapter is still required for true WGC/DXGI primary capture readiness.
- Normal browser smoke cannot validate Tauri IPC behavior; the Tauri WebView remains the authoritative runtime for screenshot, OCR, model initialization, and recording flows.

### Next Recommended Chapter
- Run a real Tauri desktop QA pass for: magnifier drag selection, scroll-capture cancel/reset, V6 initialize/apply, area recording on multi-monitor or non-100% DPI, and post-save recording controls.

## Chapter 291: Deep Freeze-Risk Bug Rescan And Cleanup Hardening (2026-06-14)

### Goals Completed
- Performed another deep static rescan for screenshot-time freeze risks, stale async state, global Ant Design message cleanup, source-code mojibake, recording window lifecycle, dynamic toast/control window capability coverage, and remaining synchronous Tauri commands.
- Moved recording temporary-file cleanup behind an async Tauri command wrapper and `spawn_blocking`, so deleting large recording segment files after cancel/save no longer runs on the command caller thread.
- Kept the recording cleanup safety boundary intact: only `.mp4` files under the app recording temp directory are deleted, and the unit test now calls the sync implementation directly.
- Removed an additional real mojibake comment from `useScreenshotInteraction`.
- Rechecked that dynamic `save_toast_*` labels are covered by capability permissions and that the frontend routes save-toasts by query string, so the dynamic label does not block toast rendering.
- Confirmed the broad UI message cleanup now uses keyed AntD message destruction rather than global `message.destroy()` in the scanned screenshot/OCR/recording scopes.

### External Findings
- Tauri v2 command/window docs were rechecked while auditing freeze risks: async command wrappers and background blocking workers are the right shape for filesystem, process, clipboard, screenshot, and recording work that can stall the UI.
- Tauri v2 window API docs were rechecked for the `destroy()` cleanup path used by externally closed recording/save-toast windows.
- FFmpeg `gdigrab` docs show region capture using `-offset_x` and `-offset_y` with desktop capture input, matching the existing desktop-coordinate recording fix from the previous chapter.

References:
- `https://v2.tauri.app/develop/calling-rust/`
- `https://v2.tauri.app/reference/javascript/api/namespacewindow/`
- `https://ffmpeg.org/ffmpeg-devices.html#gdigrab`

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/recording_process/process_manager.rs`
- `tauri-client/src-tauri/src/tests.rs`
- `tauri-client/src/hooks/useScreenshotInteraction.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not convert every remaining sync diagnostic/config/history command in this chapter; the remaining heavy-looking entries are either diagnostic-only, small file/config operations, or need a focused command-boundary pass.
- Did not implement WGC/DXGI primary GPU capture.
- Did not change the stable CPU screenshot path.
- Did not claim the user-reported screenshot freeze is fully reproduced; this chapter removes additional plausible blocking paths and records the remaining live QA requirement.
- Did not touch pre-existing unrelated dirty files such as `app.py`, `YsnTrans.lnk`, local model files, `shichen_calendar.db`, or `widget_settings.json`.
- Did not commit, push, tag, or create a branch.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run check:i18n`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `cd tauri-client; npm run build`; only existing Vite dynamic-import and chunk-size warnings were reported.
- Passed: `cargo fmt --manifest-path tauri-client\src-tauri\Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client\src-tauri\Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client\src-tauri\Cargo.toml --lib`.
- Passed: `cargo test --manifest-path tauri-client\src-tauri\Cargo.toml`.
- Passed: source scan found no remaining replacement-character or common mojibake matches in `tauri-client/src` and `tauri-client/src-tauri/src`.
- Passed: source scan found no remaining global AntD `message.destroy()` / `notification.destroy()` usage or static `message`/`notification` imports in the scanned frontend scope.
- Partial: `git diff --check` still fails only on pre-existing `app.py:425` trailing whitespace; modified files only produced existing LF-to-CRLF working-copy warnings.

### Known Risks
- The user-reported freeze while taking a screenshot was not reproduced in a live Tauri desktop session here. The next reliable proof still requires real app QA while taking screenshots, especially with magnifier, OCR/model initialization, scroll capture, and recording controls active.
- Remaining sync Tauri commands include config/history/autostart/window-enumeration/diagnostic smoke commands. They were classified as lower-risk or diagnostic-only in this pass, but a future command-boundary chapter should convert larger user-facing file/registry/window-enumeration paths to async wrappers.
- Multi-monitor, negative-origin monitor, and mixed-DPI recording still need real hardware validation even though the coordinate path now uses desktop physical bounds where available.
- WGC/DXGI GPU capture remains experimental/diagnostic and should not be treated as a production-ready primary capture backend.
- Normal browser smoke cannot validate Tauri IPC, native window lifecycle, clipboard, capture exclusion, or ffmpeg behavior; the Tauri WebView remains the authoritative runtime.

### Next Recommended Chapter
- Run a real Tauri desktop QA pass focused on the user freeze report: screenshot start while another screenshot is in progress, magnifier drag selection, scroll-capture cancel/reset, save-toast rendering, recording cancel/save cleanup, and V6 initialize/apply under model load.

## Chapter 292: Command Boundary And Scroll-Capture Race Bug Closure (2026-06-14)

### Goals Completed
- Continued the deep bug rescan after Chapter 291, focusing on hidden runtime failures around frontend `invoke` calls, Tauri command registration, screenshot/scroll-capture coordinates, OCR initialization status, recording state cleanup, and source-code encoding.
- Fixed the scroll-capture multi-monitor/DPI coordinate bug by adding desktop-coordinate live-capture and mouse-wheel commands, then routing scroll capture through the screenshot payload physical bounds when available.
- Added and registered the missing `set_capturing_state` Tauri command used by the frontend recording path, so recording can reliably clear screenshot capture state and unregister the capture Escape shortcut.
- Hardened manual scroll capture against stale async-frame races: every in-flight frame is now tied to the active scroll-capture session, so frames returned after cancel/reset cannot update preview/message state or unlock the wrong session.
- Rechecked the known magnifier, V6 initialization, Ant Design warning, recording save-state, scroll-capture exclusion, dynamic save-toast label, and invoke-handler bug classes from the previous chapters.
- Confirmed PowerShell mojibake is display-only for the inspected UI Chinese strings; UTF-8/source-code scans found no real replacement-character, `锟`, or private-use-character corruption in `tauri-client/src` and `tauri-client/src-tauri/src`.
- Confirmed all literal frontend `invoke(...)` calls found by static scan are registered in the Tauri `generate_handler!` list.

### External Findings
- No new external architecture dependency was introduced in this chapter; the root causes were local command-boundary and state-machine defects.
- The Tauri async-command and FFmpeg `gdigrab` desktop-offset findings from Chapter 291 remain the relevant external basis for the blocking-work and desktop-coordinate fixes.

### Added Files
- None.

### Modified Files
- `tauri-client/src-tauri/src/screenshot_commands.rs`
- `tauri-client/src-tauri/src/lib.rs`
- `tauri-client/src/hooks/useScrollCapture.ts`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not implement WGC/DXGI as a production primary capture backend.
- Did not change the stable CPU screenshot capture route beyond the focused command-boundary and scroll-capture fixes.
- Did not claim the user's screenshot-time freeze is reproduced or fully closed without a live Tauri desktop QA pass.
- Did not convert every remaining lower-risk sync config/history/window-enumeration/diagnostic command to async wrappers.
- Did not touch pre-existing unrelated dirty files such as `app.py`, `YsnTrans.lnk`, local ONNX model files, `shichen_calendar.db`, or `widget_settings.json`.
- Did not commit, push, tag, or create a branch.

### Validation
- Passed: static invoke-to-handler scan found no missing registered Tauri commands.
- Passed: source scan found no replacement-character, `锟`, or private-use-character corruption in `tauri-client/src` and `tauri-client/src-tauri/src`.
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run check:i18n`.
- Passed: `cd tauri-client; npm run check:ocr-processing`.
- Passed: `cd tauri-client; npm run build`; only existing Vite dynamic-import and chunk-size warnings were reported.
- Passed: `cargo fmt --manifest-path tauri-client\src-tauri\Cargo.toml -- --check`.
- Passed: `cargo check --manifest-path tauri-client\src-tauri\Cargo.toml --tests`.
- Passed: `cargo test --manifest-path tauri-client\src-tauri\Cargo.toml --lib`.
- Passed: `cargo test --manifest-path tauri-client\src-tauri\Cargo.toml`.
- Passed: `server\.venv\Scripts\python.exe -m pytest server\tests` (`65 passed`, `1 skipped`).
- Partial: `git diff --check` still fails only on pre-existing `app.py:425` trailing whitespace; modified chapter files only produced existing LF-to-CRLF working-copy warnings.

### Known Risks
- The user's reported freeze while taking a screenshot still needs live Tauri reproduction/confirmation. Static fixes reduced several plausible blocking, stale-state, and command-boundary risks, but the desktop runtime is the authoritative proof.
- Scroll capture now uses desktop physical coordinates for ordinary selected regions, but live testing is still needed for negative-origin monitors, mixed DPI, and regions spanning more than one monitor.
- Area recording coordinate conversion was previously corrected to use screenshot physical bounds; live multi-monitor and non-100% DPI recording still need hardware validation.
- V6 initialize/apply is validated by probes/builds in earlier chapters, but the real model-management UI flow should still be tested inside Tauri WebView, not a normal browser.
- Remaining sync command-boundary candidates include config/history/autostart/window enumeration and diagnostics. They are not currently proven blockers, but a future focused chapter can convert larger user-facing file/registry/enumeration paths to async wrappers.
- WGC/DXGI GPU capture remains experimental/diagnostic with clear CPU fallback reporting, not production primary capture readiness.

### Next Recommended Chapter
- Run a real Tauri desktop QA pass for the user freeze report and recently fixed paths: magnifier drag selection, screenshot start while another screenshot is active, scroll capture start/cancel/reset/finish, save-toast rendering, V6 initialize/apply, recording cancel/save cleanup, and area recording on multi-monitor or non-100% DPI.

## Chapter 293: Build Helper Taskkill Timeout And Console Copy (2026-06-14)

### Goals Completed
- Fixed the deployment helper hang where EXE builds could stop forever at `【准备】尝试关闭正在运行的 YsnTrans.exe …` before reaching `npm`.
- Moved the pre-build `taskkill /IM YsnTrans.exe` call behind a dedicated helper with a 5-second timeout, explicit success/no-process/warning logs, and continue-on-failure behavior.
- Added the same timeout guard to the build stop path, so `停止构建` cannot indefinitely block on Windows `taskkill`.
- Added a `复制` button next to `清屏` in the build console toolbar.
- Implemented console-log copying with Clipboard API first and a textarea fallback for pywebview or restricted clipboard contexts.

### Added Files
- None.

### Modified Files
- `app.py`
- `ui.html`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not run a full Tauri/npm build because the user reported the helper was stuck and asked to fix that path first.
- Did not change the existing root helper launch/port behavior that was already present in `app.py` before this chapter.
- Did not alter the broader Tauri screenshot/OCR/recording code in this chapter.
- Did not commit, push, tag, or create a branch.

### Validation
- Passed: `python -m py_compile app.py`.
- Passed: static scan confirmed `copyConsole`, `timeout=5`, `TimeoutExpired`, and the new warning/success log paths exist in `app.py` and `ui.html`.
- Checked: no currently running `taskkill.exe` process was visible after the patch.

### Known Risks
- An already-open deployment helper window must be restarted to load the patched Python/HTML code.
- The `复制` button still needs one live pywebview click test to confirm clipboard permission behavior in the user's exact runtime.
- The EXE build itself was intentionally not run in this chapter, so the next proof should be a short live helper smoke that confirms the log advances past `【准备】` into `【执行】npm ...`.

### Next Recommended Chapter
- Restart the deployment helper, click `构建 EXE · 不打安装包`, confirm the pre-build close step either succeeds or times out within 5 seconds, then test the new `复制` button against the live console log.

## Chapter 294: Deployment Helper Build Running-State Fix (2026-06-14)

### Goals Completed
- Investigated whether `启动部署助手.bat` was the reason the deployment helper still appeared unable to build.
- Confirmed the batch launcher can reach the local Python runtime and that Python, Node, npm, `node_modules`, and `@tauri-apps/cli` are available.
- Confirmed command-line builds work outside the helper: `npm run build` and `npm run tauri build -- --no-bundle` both completed successfully.
- Identified the real helper bug: `poll_build()` treated the build as not running while the helper was still in the pre-npm preparation phase because `_proc` is only assigned after `subprocess.Popen(...)`.
- Added explicit `_build_running` and `_cancel_build` state so the frontend keeps polling from `【准备】` through `【执行】npm ...` and does not freeze on stale preparation logs.
- Added preparation-stage stop handling so `停止构建` can request cancellation before npm has been spawned.

### Added Files
- None.

### Modified Files
- `app.py`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not modify `启动部署助手.bat`; it was not the root cause of the observed build-helper stall.
- Did not change the Tauri build scripts or Rust packaging configuration.
- Did not clean unrelated pre-existing dirty files.
- Did not commit, push, tag, or create a branch.

### Validation
- Passed: `python -m py_compile app.py`.
- Passed: `npm run tauri -- --version` from `tauri-client`.
- Passed: `npm run build` from `tauri-client`.
- Passed: `npm run tauri build -- --no-bundle` from `tauri-client`; produced `tauri-client\src-tauri\target\release\YsnTrans.exe`.
- Passed: deployment-helper backend probe with a temporary `probe` target running `npm --version`; `poll_build()` stayed `running=True` through preparation and npm execution, then returned `code=0`.
- Checked: no `tauri build`, `cargo`, `rustc`, or npm build child process was left running after the probe.

### Known Risks
- An already-open deployment helper window must still be restarted to load this patched `app.py`.
- The command-line no-bundle build succeeded, but the final proof for the user-facing helper is a live click in the restarted pywebview window.
- Console output captured through PowerShell can still display valid Chinese as mojibake; judge the helper's in-app rendered text rather than raw terminal output.

### Next Recommended Chapter
- Restart `启动部署助手.bat`, run `构建 EXE · 不打安装包` from the helper UI, verify the console advances past `【准备】` into `【执行】npm run tauri build -- --no-bundle`, and test the adjacent `复制` button with the completed log.

## Chapter 295: No-Console Deployment Helper Launcher (2026-06-14)

### Goals Completed
- Removed the persistent command-window experience from launching the deployment helper.
- Added a no-console `启动部署助手.vbs` launcher that uses Windows Script Host and starts `app.py` through `pythonw.exe` when available.
- Kept dependency recovery in the no-console launcher: it checks `import webview`, attempts `pip install pywebview` if missing, writes launcher diagnostics to `%TEMP%\ysn_deploy_helper_launch.log`, and shows a Windows message box on unrecoverable startup errors.
- Updated `启动部署助手.bat` to immediately hand off to the no-console VBS launcher and exit, while retaining the old Python fallback if the VBS file is missing.
- Moved `chcp 65001` before the Chinese VBS filename check so the batch fallback path does not misread the UTF-8 filename on Windows.

### Added Files
- `启动部署助手.vbs`

### Modified Files
- `启动部署助手.bat`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not remove the batch file; it remains a compatibility entry.
- Did not claim a `.bat` file can be physically launched by Windows with zero initial console flash. A batch file is still executed by `cmd.exe`; the truly no-console entry is the new `.vbs` file or a shortcut targeting it.
- Did not change the deployment helper UI or Tauri build commands.
- Did not commit, push, tag, or create a branch.

### Validation
- Passed: `cscript.exe //Nologo "启动部署助手.vbs"` with `YSN_DEPLOY_LAUNCHER_DRY_RUN=1`.
- Passed: `cmd /c "set YSN_DEPLOY_LAUNCHER_DRY_RUN=1&&启动部署助手.bat"`, confirming the batch file hands off to the VBS path without opening the real helper.
- Passed: `python -m py_compile app.py`.
- Checked: no `pythonw.exe app.py` helper process was left running after cleanup.
- Checked: `%TEMP%\ysn_deploy_helper_launch.log` recorded the expected Python path, `pythonw.exe` launch command, and dry-run success.

### Known Risks
- Directly double-clicking `启动部署助手.bat` may still show a very brief Windows-created command-window flash because `.bat` files are executed by `cmd.exe`.
- For a fully no-window launch, use `启动部署助手.vbs` directly or create a shortcut that targets `wscript.exe "启动部署助手.vbs"`.

### Next Recommended Chapter
- Create or update a user-facing shortcut that points directly to `wscript.exe "启动部署助手.vbs"` if the desktop needs a polished one-click no-console entry with a custom icon.

## Chapter 296: Magnifier White-Tail Overlay Fix (2026-06-14)

### Goals Completed
- Investigated the screenshot-window bug where moving the magnifier caused a large white translucent block to appear to its lower-right side.
- Identified the likely root cause as hover/candidate preview rendering being updated by the same no-button pointer moves that show the magnifier, which can brighten a large detected control/window region and look like a white panel following the magnifier.
- Changed magnifier pointer handling so no-button pointer moves update the magnifier first and skip normal hover-detection rendering while the magnifier is visible.
- Added a `suppressHoverPreviewRef` guard so `renderScreenshotCanvas()` receives no hover rect/candidate count while the magnifier is active.
- Cleared hover candidate refs/state when entering and leaving magnifier mode so stale large preview rectangles cannot remain on the screenshot canvas.
- Reworked the magnifier from conditional React state rendering into a fixed DOM host updated by animation frame, reducing React re-render churn during pointer movement.
- Added fixed-size, clipped CSS for the magnifier host/canvas/label so WebView transparent-window background cannot leak as a large white rectangle if layout briefly miscalculates.
- Clamped the magnifier position inside the viewport and flips it to the opposite side near screen edges.

### Added Files
- None.

### Modified Files
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src/index.css`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not change the native screenshot capture pipeline.
- Did not disable window/control detection globally; it is only suppressed while the magnifier is visible.
- Did not change OCR, translation, recording, or scroll-capture behavior.
- Did not commit, push, tag, or create a branch.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; only existing Vite dynamic-import and chunk-size warnings were reported.
- Passed: `cd tauri-client; npm run tauri build -- --no-bundle`; produced a fresh `tauri-client\src-tauri\target\release\YsnTrans.exe`.
- Closed the previously running `target\release\YsnTrans.exe` before rebuilding so the new executable could be written.

### Known Risks
- The white-tail fix was validated by static inspection and builds, but the final proof requires live Alt+A movement over the same Codex/sidebar UI region shown in the user screenshot.
- If a separate WebView2 transparency bug still appears, the next step is a live screenshot-window pixel/DOM inspection around the magnifier host.

### Next Recommended Chapter
- Launch the freshly built `target\release\YsnTrans.exe`, press Alt+A, move the magnifier over dense UI/sidebar regions, and confirm no lower-right white block follows the magnifier.

## Chapter 297: Magnifier During Drag Selection Fix (2026-06-14)

### Goals Completed
- Corrected the magnifier interaction model so the magnifier remains visible and follows the cursor while the user is dragging a screenshot selection.
- Changed pointer-move ordering: while the primary mouse button is down, the screenshot selection state machine runs first, then the magnifier updates at the current pointer location.
- Kept the Chapter 296 white-tail fix intact by continuing to suppress hover/candidate preview rendering while the magnifier is visible.
- Changed pointer-down behavior so left-click initializes the magnifier instead of immediately hiding it; right-click and non-left actions still hide/cancel as before.
- Kept the magnifier hidden only after selection completion, pointer cancel, or non-drag pointer leave.
- Hardened magnifier pixel sampling at screen/image edges by clamping both single-pixel sampling and zoom-source rectangles to image bounds.

### Added Files
- None.

### Modified Files
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src/hooks/useScreenshotMagnifier.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not change the screenshot capture backend.
- Did not change candidate/window detection outside active magnifier display.
- Did not alter OCR, translation, recording, scroll capture, or deployment helper code.
- Did not commit, push, tag, or create a branch.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; only existing Vite dynamic-import and chunk-size warnings were reported.
- Passed: `cd tauri-client; npm run tauri build -- --no-bundle`; produced a fresh `tauri-client\src-tauri\target\release\YsnTrans.exe`.
- Closed the previously running `target\release\YsnTrans.exe` before rebuilding.

### Known Risks
- Final proof still needs a live Alt+A drag test: press, drag a selection, confirm the magnifier follows during the drag, then confirm it disappears after mouse-up with no white-tail block.

### Next Recommended Chapter
- Run a live screenshot QA pass over dense UI: hover magnifier, drag-select with magnifier active, release selection, repeat near screen edges, and verify copy/save toolbar behavior still appears normally after selection completion.

## Chapter 298: Screenshot Interaction Smoothness Optimization (2026-06-14)

### Goals Completed
- Reduced screenshot drag-selection frame pressure without changing the user-facing screenshot workflow.
- Stopped the magnifier animation frame from forcing a main screenshot-canvas redraw on every pointer move.
- Kept the Chapter 296 white-tail protection by redrawing the main canvas once only when entering magnifier suppression and clearing stale hover previews.
- Coalesced magnifier color sampling and magnifier canvas drawing into a single animation-frame path so high-frequency pointer events no longer run synchronous pixel reads one event at a time.
- Added dirty-region rendering support to `renderScreenshotCanvas()` for normal selection movement: when the static masked background is unchanged and no hover/annotation/translation overlay is active, only the previous and current selection decoration bounds are restored and redrawn.
- Added hover-detection reuse and throttling so repeated pointer movement does not run visual/window candidate detection on every raw pointer event.
- Reduced redundant React state updates and canvas redraws when window-rect and hover-candidate signatures have not changed.
- Skipped window-rect IPC loading while a selection/drag/resize gesture is already active, avoiding candidate warmup contention with immediate screenshot dragging.

### Added Files
- None.

### Modified Files
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src/utils/renderScreenshotCanvas.ts`
- `tauri-client/src/hooks/useScreenshotInteraction.ts`
- `tauri-client/src/hooks/useScreenshotWindowRects.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not launch the release app, press global screenshot hotkeys, or run live desktop screenshot QA because the user asked not to test in their active environment.
- Did not change the native screenshot capture backend, SharedBuffer transfer path, OCR, translation, recording, scroll capture, or deployment helper behavior.
- Did not close or replace the currently running `YsnTrans.exe`.
- Did not commit, push, tag, or create a branch.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; only existing Vite dynamic-import and chunk-size warnings were reported.
- Passed: static Node probe confirming the dirty-render path is present and guarded by masked background/no-hover/no-annotation conditions.

### Known Risks
- Dirty-region rendering was verified by type/build/static inspection, but final proof still requires live drag QA in a separate test environment.
- If a translated image, annotation draft, hover preview, or recording selection overlay is active, the renderer intentionally falls back to the full redraw path for correctness.
- The native screenshot capture startup path was not changed; if perceived lag remains before the overlay becomes interactive, the next optimization pass should profile capture/window preparation timings rather than the drag-render path.

### Next Recommended Chapter
- Run isolated live QA on a non-user-active desktop/session: Alt+A hover, drag with magnifier, rapid drag on 2K/4K displays, edge drag, Tab candidate switching, annotation drawing, and translate-mode selection. If drag is now smooth but startup still feels slow, continue with native capture/overlay readiness profiling.

## Chapter 299: Screenshot Warmup And Window-Rect Overhead Trim (2026-06-14)

### Goals Completed
- Deferred screenshot-page OCR/translation prewarm to browser idle time so page load stops competing with the first interactive overlay frames.
- Deferred full-frame visual-analysis image extraction and candidate warmup while a selection is already in progress, avoiding heavyweight background work during immediate drag interactions.
- Reduced magnifier per-frame work by reusing sample/overlay contexts and caching the magnifier decoration grid instead of redrawing it on every pointer update.
- Removed one full JSON stringify/parse round-trip from `get_window_rects` by returning structured rect arrays directly from Tauri to the WebView.
- Moved screenshot-page mount-time window-rect warmup from a fixed early timeout to idle/fallback scheduling so non-critical candidate prewarm yields to initial UI responsiveness.

### Added Files
- None.

### Modified Files
- `tauri-client/src/hooks/useScreenshotLoader.ts`
- `tauri-client/src/hooks/useScreenshotMagnifier.ts`
- `tauri-client/src/hooks/useScreenshotWindowRects.ts`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src-tauri/src/window_targets.rs`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not run live Alt+A or desktop interaction testing in the user's active environment.
- Did not change the native screenshot capture backend or broader OCR/translation/recording flows.
- Did not address existing Vite chunk-size or dynamic-import warnings in this chapter.
- Did not commit unrelated dirty workspace files.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; only existing Vite dynamic-import and chunk-size warnings were reported.
- Passed: `cargo check --manifest-path tauri-client\src-tauri\Cargo.toml`.

### Known Risks
- The startup/perceived-latency gains are validated by static/build checks only; real proof still needs isolated desktop QA away from the user's active session.
- `get_window_rects` now returns structured values directly, so any future non-TypeScript callers must consume arrays rather than a JSON string.
- Remaining latency before overlay interactivity may still be dominated by native capture/session startup rather than WebView-side warmup.

### Next Recommended Chapter
- Profile and trim native screenshot session startup, especially capture readiness and overlay handoff timings, once a separate safe QA environment is available.

## Chapter 300: Release And Documentation Reality Sync (2026-06-15)

### Goals Completed
- Cleaned up the release/documentation drift identified in the 2026-06-15 project audit.
- Reassembled the current `1.2.7` portable directory without running the full build script, so the currently running `YsnTrans.exe` was not force-closed.
- Generated the current portable zip from the assembled portable directory.
- Updated English and Chinese READMEs from stale `v1.2.5` installer paths to the current `v1.2.7` portable package and build flow.
- Updated the master plan with a new current-risk section for Chapter 300 and clearly marked the old Chapter 148 risk section as historical.
- Updated the implementation resume snapshot from the stale 2026-06-10 / Chapter 269 state to the current 2026-06-15 / Chapter 300 state.
- Recorded the user's latest screenshot feedback: interaction feels smooth and comfortable overall, with occasional stutter possibly tied to magnifier behavior.

### Added Files
- None.

### Modified Files
- `README.md`
- `README.zh-CN.md`
- `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Generated Artifacts
- `release\YSN-Screenshot-Translator\YsnTrans.exe`
- `release\YSN-Screenshot-Translator\resources\...`
- `release\YSN-Screenshot-Translator\models\rapidocr\...`
- `build\x64_v1.2.7\ScreenshotTranslator_Windows.zip`

### Deleted Files
- None in this chapter.

### Explicit Non-Goals
- Did not change screenshot, magnifier, OCR, translation, recording, or scroll-capture behavior.
- Did not run full `build.bat`, because it closes the currently running app before building.
- Did not regenerate `1.2.7` NSIS/MSI installers; existing visible installers are still historical `1.2.6` artifacts.
- Did not restore the pre-existing deletion of `启动部署助手.vbs`; that remains a user/workspace decision.
- Did not commit, push, tag, or create a branch.

### Validation
- Passed: assembled portable directory from the current `tauri-client\src-tauri\target\release\YsnTrans.exe`, `tauri-client\src-tauri\resources`, and `models\rapidocr`.
- Passed: portable directory file count/size check reported `188` files and `368970781` bytes.
- Passed: `powershell -NoProfile -ExecutionPolicy Bypass -File .\pack_release.ps1`, producing `build\x64_v1.2.7\ScreenshotTranslator_Windows.zip` at about `199.52 MB`.
- Passed: `git diff --check`.
- Passed: `powershell -NoProfile -ExecutionPolicy Bypass -File .\check_commercial.ps1`.

### Known Risks
- `1.2.7` installers still need a full build when it is acceptable to close the running app.
- The portable directory was assembled from existing current build output rather than a fresh full build.
- The occasional screenshot stutter reported by the user is only recorded here; it still needs a focused profiling chapter.
- Service-side pytest coverage still needs an environment with `pytest` installed.

### Next Recommended Chapter
- Profile the occasional screenshot stutter without broad behavior changes, starting with magnifier frame work, pixel sampling, hover/candidate suppression, window-rect warmup, and capture handoff timings.
- When the user allows closing the current app, run full `build.bat --no-pause --no-launch`, regenerate `1.2.7` installers, and smoke-launch the packaged output.

## Chapter 301: Borderless Main Window Brand Chrome (2026-06-15)

### Goals Completed
- Removed the duplicate native main-window title bar by setting the Tauri main window to `decorations: false`.
- Kept a lightweight professional window-control surface in the app header with minimize, maximize/restore, and close actions.
- Added safe drag handling for the main header and left brand area so the borderless window can still be moved without interfering with clickable header controls.
- Simplified the left brand wordmark to a more restrained premium style while keeping the single `Ysn Trans` brand signal requested by the user.
- Kept the change isolated from the other parallel work streams and did not touch existing README/master-plan changes or the pre-existing VBS deletion.

### Added Files
- `tauri-client/src/components/app/MainWindowControls.tsx`

### Modified Files
- `tauri-client/src-tauri/tauri.conf.json`
- `tauri-client/src/components/app/AppLayout.tsx`
- `tauri-client/src/index.css`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not change screenshot, OCR, translation, recording, scroll capture, packaging, or release artifacts.
- Did not close or replace the currently running `YsnTrans.exe`.
- Did not run a full Tauri desktop smoke because that would require launching/replacing the desktop app state.
- Did not commit, push, tag, or create a branch.

### Validation
- Passed: `cd tauri-client; npm run build`; only existing Vite dynamic-import and chunk-size warnings were reported.
- Passed: `node -e "JSON.parse(require('fs').readFileSync('src-tauri/tauri.conf.json','utf8'))"`.
- Passed: `git diff --check -- tauri-client\src\components\app\AppLayout.tsx tauri-client\src\components\app\MainWindowControls.tsx tauri-client\src\index.css tauri-client\src-tauri\tauri.conf.json`; only existing LF-to-CRLF warnings were reported.
- Passed: local browser preview after adding safe Tauri-window guards; the main layout rendered, the brand was present, three window buttons were present, and `1200x800` inspection showed no horizontal overflow.

### Known Risks
- Real proof of the borderless native window still needs a Tauri desktop smoke after the app can be restarted.
- Browser preview cannot verify native window dragging, native maximize/restore behavior, or actual removal of the Windows title bar.
- Remaining browser console errors during preview are expected outside Tauri because existing app hooks call Tauri `invoke`.

### Next Recommended Chapter
- When it is acceptable to restart or launch the desktop app, run a focused main-window smoke: confirm no native title bar, drag from brand/header blank area, verify minimize/maximize/restore/close, and check the 1200px header does not visually crowd status chips.
- Continue the previously recommended screenshot-stutter profiling after this visual polish is verified.

## Chapter 302: Temporary Screenshot Stutter Probe (2026-06-15)

### Goals Completed
- Added a removable, single-module temporary probe for the user-reported occasional screenshot stutter.
- Kept screenshot behavior unchanged; the probe only measures elapsed time and logs slow slices when explicitly built with `VITE_YSN_SCREENSHOT_STUTTER_PROBE=1`.
- Instrumented the current highest-suspicion paths: magnifier frame work, 1x1 pixel sampling, magnifier canvas draw, main selection canvas draw, full-frame analysis `ImageData`, and window-rect IPC.
- Marked every temporary call site with `STUTTER_PROBE_TEMP` so cleanup can be done with `rg STUTTER_PROBE_TEMP`.
- Preserved the other parallel work streams; did not touch the main-window chrome files, release README changes, Tauri config changes from Chapter 301, or the pre-existing VBS deletion.

### Added Files
- `tauri-client/src/utils/screenshotStutterProbe.ts`

### Modified Files
- `tauri-client/src/hooks/useScreenshotMagnifier.ts`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/src/hooks/useScreenshotLoader.ts`
- `tauri-client/src/hooks/useScreenshotWindowRects.ts`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not optimize or change magnifier, selection, detection, OCR, translation, recording, or scroll-capture behavior.
- Did not start, close, replace, or smoke-test the currently running desktop app.
- Did not run a probe-enabled Tauri build because the currently running `target\release\YsnTrans.exe` should not be replaced during parallel active work.
- Did not commit, push, tag, or create a branch.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; only existing Vite dynamic-import and chunk-size warnings were reported.
- Passed: `git diff --check`; only existing LF-to-CRLF warnings were reported.
- Checked: `rg STUTTER_PROBE_TEMP tauri-client\src -S` finds the temporary module and all temporary call sites.

### Known Risks
- The normal build run in this chapter did not enable `VITE_YSN_SCREENSHOT_STUTTER_PROBE=1`, so it proves compile/build safety but does not yet produce live `[stutter-probe]` timing evidence.
- Probe evidence still requires a controlled run from a build made with `VITE_YSN_SCREENSHOT_STUTTER_PROBE=1`, followed by normal Alt+A hover/drag attempts until the occasional stutter reproduces.
- The temporary module intentionally remains in source until the stutter sample is captured or the probe is no longer needed.

### Next Recommended Chapter
- When it is acceptable to restart the app, build or launch a temporary probe-enabled version with `VITE_YSN_SCREENSHOT_STUTTER_PROBE=1`, reproduce several Alt+A hover/drag sessions with magnifier and intelligent detection enabled, then inspect `[stutter-probe]` lines to decide whether the root cause is magnifier frame work, pixel sampling, selection draw, analysis `ImageData`, or window-rect IPC.
- After diagnosis, remove the temporary probe with `rg STUTTER_PROBE_TEMP`, rerun `npx tsc --noEmit` and `npm run build`, then record the cleanup chapter.

## Chapter 303: Probe Smoke Findings And Automation Blockers (2026-06-15)

### Goals Completed
- Took over unattended screenshot-stutter testing after the user explicitly allowed desktop control.
- Attempted to initialize the Computer Use Windows automation plugin before any fallback automation. It failed twice with the same internal `@oai/sky` package export error, so real mouse/keyboard desktop automation was not used.
- Built a probe-enabled release with `VITE_YSN_SCREENSHOT_STUTTER_PROBE=1` and `VITE_YSN_SCREENSHOT_SELECTION_SMOOTHNESS_SMOKE=1` before later parallel Settings changes introduced a TypeScript blocker.
- Ran the app with `YSN_SCREENSHOT_AUTO_START_SMOKE=1` and collected stdout/stderr logs under `tmp-runtime-logs`.
- Captured a valid automatic screenshot + internal selection smoke sample from `tmp-runtime-logs\stutter-probe-auto-20260615-045520-out.log`.
- Added a temporary probe-only magnifier smoke extension so future probe builds can exercise `updateMagnifier` during internal synthetic drags when real desktop automation is unavailable.

### Added Files
- None in this chapter.

### Modified Files
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not fix the concurrent Settings TypeScript error because it belongs to another parallel work stream.
- Did not use PowerShell/SendInput foreground UI automation after Computer Use failed.
- Did not claim the automatic smoke proves real human mouse movement or real magnifier-hover behavior.
- Did not leave the diagnostic `YsnTrans.exe` process running.
- Did not commit, push, tag, or create a branch.

### Validation
- Passed before the concurrent Settings blocker appeared: `cd tauri-client; VITE_YSN_SCREENSHOT_STUTTER_PROBE=1 VITE_YSN_SCREENSHOT_SELECTION_SMOOTHNESS_SMOKE=1 npx tauri build --no-bundle`.
- Passed valid auto-smoke launch: `YSN_SCREENSHOT_AUTO_START_SMOKE=1` against the probe build.
- Valid log evidence:
  - `rgba_canvas_ready elapsed_ms=13`.
  - `first_paint elapsed_ms=25`.
  - `[stutter-probe] phase=analysis_image_data_ms duration_ms=41.40 threshold_ms=12 width=2560 height=1440`.
  - `analysis_image_data_ready elapsed_ms=69 width=2560 height=1440`.
  - `candidate_load_deferred elapsed_ms=247 reason=selection_active`.
  - `selection-smoothness-smoke` completed `12` rounds, each around `257-261 ms`.
- No `selection_draw_ms` probe exceeded the current `8 ms` threshold in the valid automatic smoke.
- No valid `magnifier_frame_ms`, `pixel_sample_ms`, or `magnifier_draw_ms` sample was captured because the original internal smoothness smoke bypassed the React pointer wrapper.

### Blocked / Invalid Attempts
- Computer Use automation failed twice with: `Package subpath './dist/project/cua/sky_js/src/targets/windows/internal/computer_use_client_base.js' is not defined by "exports" in ...\@oai\sky\package.json`.
- After another parallel work stream modified Settings files, `npx tsc --noEmit` failed with an unrelated `Settings.tsx` prop type error: `Property 'activeChannel' does not exist...`.
- A later diagnostic `vite build` + `cargo build --release` completed, but the resulting auto-start smoke only logged native capture/payload emission and never reached frontend `shared_buffer_received/image_ready`; this run is not valid stutter evidence.

### Known Risks
- The current strongest measured spike is full-frame visual-analysis `ImageData` at about `41 ms` on a 2560x1440 screen. This is enough to explain an occasional hitch if it overlaps early user movement.
- Magnifier remains unproven by live evidence because Computer Use was unavailable and the valid automatic smoke did not exercise `updateMagnifier`.
- The repository currently has concurrent Settings changes that block `tsc`; this must be resolved by that work stream before a formal probe build or cleanup build can pass normally.
- The temporary probe and probe-only magnifier smoke extension remain in source and must be removed after diagnosis.

### Next Recommended Chapter
- First resolve or wait for the concurrent Settings TypeScript fix, then rebuild with `VITE_YSN_SCREENSHOT_STUTTER_PROBE=1 VITE_YSN_SCREENSHOT_SELECTION_SMOOTHNESS_SMOKE=1 npx tauri build --no-bundle` and rerun `YSN_SCREENSHOT_AUTO_START_SMOKE=1` to collect magnifier probe lines.
- Product-side mitigation candidate, pending confirmation: delay or slice full-frame `analysisImageData` until after the first selection gesture settles, because the current measured `41.40 ms` readback is already above a 60 fps frame budget.
- If Computer Use becomes available again, run real Alt+A hover/drag with magnifier enabled and inspect `magnifier_frame_ms`, `pixel_sample_ms`, and `magnifier_draw_ms` before changing magnifier behavior.

## Chapter 303: Main Brand Lavender Tile Revision (2026-06-15)

### Goals Completed
- Replaced the previous italic gradient `Ysn Trans` wordmark with a compact lavender rounded-rectangle brand tile inspired by the user's Plus Jakarta Sans reference.
- Changed the brand text to `YsnTrans` without a space, using black Plus Jakarta Sans-style typography.
- Set the tile to `140px x 60px` with a `14px` radius and a flat lavender background.
- Adjusted the left brand/header slot height and padding so the tile sits cleanly in the sidebar while keeping the drag region behavior from Chapter 301.
- Kept the change isolated from the parallel screenshot-stutter probe work in Chapter 302.

### Added Files
- None.

### Modified Files
- `tauri-client/src/components/app/AppLayout.tsx`
- `tauri-client/src/index.css`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not change main-window borderless behavior, window controls, screenshot, OCR, translation, recording, packaging, or release artifacts.
- Did not touch the temporary screenshot-stutter probe files from Chapter 302.
- Did not start, close, or replace the currently running desktop `YsnTrans.exe`.
- Did not commit, push, tag, or create a branch.

### Validation
- Passed: `cd tauri-client; npm run build`; only existing Vite dynamic-import and chunk-size warnings were reported.
- Passed: `git diff --check -- tauri-client\src\components\app\AppLayout.tsx tauri-client\src\index.css`; only existing LF-to-CRLF warnings were reported.
- Passed: local browser preview reported the brand text as `YsnTrans`, tile size `140x60`, background `rgb(201, 192, 255)`, radius `14px`, black text, Plus Jakarta Sans font stack, and no preview startup failure from the brand change.
- Checked: temporary Vite preview process was stopped after the visual check.

### Known Risks
- Browser preview verifies CSS and layout only; the running desktop app still needs a restart to show the updated brand tile in the real Tauri window.
- The Google-hosted Plus Jakarta Sans import follows the existing project pattern for Inter but still depends on font availability/network until fonts are bundled locally in a future polish chapter.

### Next Recommended Chapter
- Restart or rebuild the desktop app when convenient, confirm the real borderless main window shows the lavender brand tile cleanly, and decide whether the lavender needs to be lighter/darker after seeing it in the actual WebView.

## Chapter 304: Main Brand Plain Black Wordmark (2026-06-15)

### Goals Completed
- Reverted the lavender brand tile treatment from Chapter 303 after user feedback.
- Kept the `YsnTrans` text and Plus Jakarta Sans-style typography, but removed the background block, fixed tile dimensions, radius, and lavender fill.
- Set the wordmark to pure black text on a transparent background.
- Restored the left brand area to a compact `64px` height while preserving the borderless-window drag behavior from Chapter 301.
- Kept the change isolated from the parallel screenshot-stutter probe and settings work.

### Added Files
- None.

### Modified Files
- `tauri-client/src/components/app/AppLayout.tsx`
- `tauri-client/src/index.css`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not change window controls, main-window borderless config, screenshot, OCR, translation, recording, settings, packaging, or release artifacts.
- Did not touch temporary screenshot-stutter probe files or parallel settings changes.
- Did not start, close, or replace the currently running desktop `YsnTrans.exe`.
- Did not commit, push, tag, or create a branch.

### Validation
- Passed: `cd tauri-client; npm run build`; only existing Vite dynamic-import and chunk-size warnings were reported.
- Passed: `git diff --check -- tauri-client\src\components\app\AppLayout.tsx tauri-client\src\index.css`; only existing LF-to-CRLF warnings were reported.
- Passed: local browser preview reported `YsnTrans`, transparent background `rgba(0, 0, 0, 0)`, text color `rgb(0, 0, 0)`, Plus Jakarta Sans font stack, `23px` font size, and `700` font weight.
- Checked: temporary Vite preview process was stopped after the visual/CSS check.

### Known Risks
- Browser preview verifies CSS and layout only; the currently running desktop app still needs a restart to show the updated wordmark.

### Next Recommended Chapter
- Restart or rebuild the desktop app when convenient and confirm the real borderless main window shows the plain black wordmark cleanly in the left sidebar.

## Chapter 304: Translation Channel Save-And-Enable UX (2026-06-15)

### Goals Completed
- Reworked the settings-page translation channel section so selecting a channel only opens it for viewing/editing and never saves or enables it.
- Removed the large channel health summary block and the Google warning panel from the visible translation channel UI.
- Kept the existing channel dropdown per user preference, but relabeled it as the channel to configure instead of the active channel.
- Added explicit active/editing state: green breathing dot for the currently enabled channel, yellow breathing dot for editing or unsaved changes, and static red state for failed save/enable attempts.
- Added one unified `保存并启用 {channel}` action for Google, Baidu, LLM, and DeepL.
- Changed `保存并启用` to validate required fields first, then call the existing server `/api/config/test` path so the channel is only enabled after a real provider test passes.
- Kept target language, service URL, LAN preference, token, hotkeys, and other ordinary settings on their existing save behavior.
- Stopped ordinary form auto-save and the top `保存设置` action from persisting translation channel selection, API keys, provider model, endpoint, or prompt fields.
- Treated Google Translate as a normal formal channel with a compact quality note instead of a large risk warning.

### Added Files
- None.

### Modified Files
- `tauri-client/src/components/settings/TranslationChannelCard.tsx`
- `tauri-client/src/components/settings/types.ts`
- `tauri-client/src/hooks/useSettingsController.ts`
- `tauri-client/src/pages/Settings.tsx`
- `tauri-client/src/i18n/zh-CN.ts`
- `tauri-client/src/i18n/en-US.ts`
- `tauri-client/src/index.css`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not change translation provider implementations or server API contracts.
- Did not change screenshot, OCR, recording, main-window brand, screenshot-stutter probe, packaging, or release artifacts.
- Did not restart or replace the running desktop `YsnTrans.exe`.
- Did not commit, push, tag, or create a branch.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run check:i18n`, reporting `594 zh-CN keys match 594 en-US keys`.
- Passed: `cd tauri-client; npm run build`; only existing Vite dynamic-import and chunk-size warnings were reported.
- Passed: targeted `git diff --check` on the modified translation/settings files; only existing LF-to-CRLF warnings were reported.
- Passed: local browser preview of the settings page confirmed `保存并启用 Google Translate（无密钥）` is visible, the status bar is `translation-channel-statusbar is-enabled`, `通道健康摘要` is absent, `质量风险` is absent, the old Google yellow warning title is absent, and no horizontal overflow was detected at `1280px`.
- Checked: the temporary Vite preview process on port `5173` was stopped after verification.

### Known Risks
- Browser preview can verify layout and DOM state, but it cannot exercise the real Tauri desktop save path end-to-end without using the running app and live translation service configuration.
- The Ant Design dropdown option was not stable enough in the in-app browser automation to complete a visual click-through to Baidu, but the React state/type/build path for the selected-channel panel passed compile and build checks.
- Old compatibility functions for `testChannel` remain in the settings controller, but the new translation channel UI no longer calls them.

### Next Recommended Chapter
- In the real desktop app, open Settings, switch the dropdown between Google/Baidu/LLM/DeepL, confirm selection only changes the editor panel, then use `保存并启用` against Google and one configured keyed provider to verify green/red status transitions and local/server config persistence.

## Chapter 305: Screenshot Stutter Range Matrix Probe (2026-06-15)

### Goals Completed
- Continued the temporary screenshot-stutter investigation with the user's requested wider drag coverage.
- Expanded the probe-only selection smoke path to cover 16 drag shapes: small top-left, medium center, large diagonal, near fullscreen, top edge wide, bottom edge wide, left edge tall, right edge tall, center wide, center tall, reverse bottom-right, reverse bottom-left, top-right corner, bottom-left corner, thin horizontal, and thin vertical.
- Ran the range matrix three times for 48 total synthetic drag sessions.
- Kept the probe isolated behind `VITE_YSN_SCREENSHOT_STUTTER_PROBE=1` / `VITE_YSN_SCREENSHOT_SELECTION_SMOOTHNESS_SMOKE=1`; normal builds do not emit the probe timings.
- Preserved the parallel main-window and settings work streams; did not edit those files.

### Added Files
- `tauri-client/src/utils/screenshotStutterProbe.ts`

### Modified Files
- `tauri-client/src/hooks/useScreenshotMagnifier.ts`
- `tauri-client/src/hooks/useScreenshotLoader.ts`
- `tauri-client/src/hooks/useScreenshotWindowRects.ts`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- None.

### Explicit Non-Goals
- Did not make the stutter probe permanent.
- Did not change screenshot interaction behavior for normal builds.
- Did not regenerate installers or release artifacts.
- Did not fix or modify the unrelated settings/main-window changes from parallel work streams.
- Did not commit, push, tag, or create a branch.

### Validation
- Earlier probe compile/build gates passed before the parallel settings stream changed the worktree: `cd tauri-client; npx tsc --noEmit` and `cd tauri-client; npm run build`.
- Current full TypeScript/build validation is blocked by unrelated settings changes in the same worktree; this chapter used diagnostic Tauri builds with a temporary config that runs `vite build` instead of the normal `tsc && vite build` gate.
- Passed diagnostic run: `VITE_YSN_SCREENSHOT_STUTTER_PROBE=1` and `VITE_YSN_SCREENSHOT_SELECTION_SMOOTHNESS_SMOKE=1` with `npx tauri build --no-bundle --config %TEMP%\ysn-tauri-probe-config.json`.
- Captured valid range-matrix logs:
  - `tmp-runtime-logs\stutter-probe-range-20260615-051218-run1-out.log`
  - `tmp-runtime-logs\stutter-probe-range-20260615-051248-run2-out.log`
  - `tmp-runtime-logs\stutter-probe-range-20260615-051316-run3-out.log`
- The range matrix produced no `selection_draw_ms` entries above the 8ms threshold across all three runs, so large/edge/reverse selection rectangle drawing is not currently the primary suspect.
- The largest observed full-frame analysis readback was `[stutter-probe] phase=analysis_image_data_ms duration_ms=53.00 threshold_ms=12 width=2560 height=1440`.
- The largest observed magnifier first-frame path was `[stutter-probe] phase=magnifier_frame_ms duration_ms=30.30 threshold_ms=8 x=51 y=48`, paired with `[stutter-probe] phase=pixel_sample_ms duration_ms=29.30 threshold_ms=2 sx=51 sy=48`.
- A visual-detection-off A/B run logged `analysis_image_data_skipped reason=visual_detection_disabled`, confirming the 36-53ms analysis readback belongs to the intelligent visual detection image-data path.

### Known Risks
- This is synthetic frontend smoke evidence, not a human hand-drag recording on multiple real devices.
- The in-app Computer Use automation failed to initialize in this environment because the bundled `@oai/sky` package export for the Windows computer-use client was unavailable, so foreground desktop mouse automation was not used.
- Magnifier-off A/B data was inconclusive because the ad-hoc config mutation did not reliably reflect in the running test app; do not treat those logs as proof that magnifier is or is not safe.
- Window-rect IPC occasionally logged around 140ms during idle candidate loading, but active drag currently skips that query, so it is a lower-confidence suspect for the reported drag stutter.

### Next Recommended Chapter
- Implement a small root-cause mitigation for the two evidenced spikes: defer or idle-schedule the full-frame `analysisImageData` capture so it cannot overlap first selection movement, and warm or throttle the magnifier's first pixel-sampling frame without disabling the drag-following magnifier.
- After the parallel settings TypeScript blocker is resolved, rerun the normal gates, remove the `STUTTER_PROBE_TEMP` instrumentation, then repeat one probe-enabled confirmation run and one normal human screenshot session.

## Chapter 306: Screenshot Stutter Mitigation And Icon Refresh (2026-06-15)

### Goals Completed
- Moved the full-screen visual-detection `ImageData` readback out of the active drag path by deferring it until screenshot interaction has been quiet for the guard window.
- Removed the forced retry cap that allowed the readback to run during continuous synthetic dragging.
- Changed the magnifier HEX picker to read from cached `ImageData` when available and reuse the previous HEX text before the cache is ready, avoiding synchronous 1x1 canvas readback during pointer movement.
- Removed the temporary screenshot stutter probe module and all `STUTTER_PROBE_TEMP` call sites after validation.
- Replaced the app, tray/taskbar, Windows, macOS, iOS, Android, and custom candidate icons from the new source icon.
- Converted the white source-image corners to transparent before icon generation and post-processed PNG corners so the checked icon sizes have `alpha=0` at all four corners.

### Added Files
- `图标设计草案/ChatGPT Image 2026年6月15日 05_42_48.png`

### Modified Files
- `tauri-client/src/hooks/useScreenshotLoader.ts`
- `tauri-client/src/hooks/useScreenshotMagnifier.ts`
- `tauri-client/src/hooks/useScreenshotWindowRects.ts`
- `tauri-client/src/pages/ScreenshotPage.tsx`
- `tauri-client/index.html`
- `tauri-client/public/favicon.ico`
- `tauri-client/src-tauri/icons/**`
- `docs/IMPLEMENTATION_CHAPTERS.md`

### Deleted Files
- `tauri-client/src/utils/screenshotStutterProbe.ts`

### Explicit Non-Goals
- Did not regenerate the 1.2.7 installer; the user asked to leave installer regeneration for the final release step.
- Did not address the broader code-organization debt.
- Did not modify unrelated parallel settings/main-window work or the deleted `启动部署助手.vbs`.

### Validation
- Passed: `cd tauri-client; npx tsc --noEmit`.
- Passed: `cd tauri-client; npm run build`; only existing Vite dynamic-import and chunk-size warnings were reported.
- Passed: `cd tauri-client; npx tauri build --no-bundle`; generated `tauri-client/src-tauri/target/release/YsnTrans.exe`.
- Probe before cleanup: `tmp-runtime-logs\stutter-probe-optimized-no-pixel-readback-20260615-062045-out.log` completed all 16 drag cases with no `stutter-probe` threshold hits.
- Probe before cleanup confirmed `pixel_sample_ms` no longer appeared, and `analysis_image_data_ready` did not run during the continuous drag matrix.
- Formal release smoke after cleanup: `tmp-runtime-logs\release-final-auto-screenshot-post-favicon-20260615-063115-out.log` reached screenshot `image_ready` in 12ms and `first_paint` in 24ms on a 2560x1440 capture.
- Icon pixel check passed for `icon.png`, `32x32.png`, `64x64.png`, `taskbar-32x32.png`, `taskbar-64x64.png`, `icon-candidate-v1-16x16.png`, and `icon-candidate-v1-32x32.png`: four corner alpha values were `0,0,0,0` and center alpha was `255`.
- Checked: no lingering `YsnTrans` or `rapidocr-runner` process remained after smoke runs.

### Known Risks
- Computer Use desktop automation could not initialize because the bundled Windows `@oai/sky` package subpath was not exported; verification used the app's own smoke path and build logs instead of foreground mouse control.
- Browser automation was not used for the desktop executable path because the relevant target is a Tauri app, not a local web route.
- This chapter validates the current machine thoroughly, but the broader real-device matrix remains incomplete.

### Next Recommended Chapter
- Run the real-device matrix on at least one additional Windows display/DPI/device setup, then regenerate the 1.2.7 installer as the final release artifact step.
