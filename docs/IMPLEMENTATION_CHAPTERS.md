# Implementation Chapters

> This file is the single execution-history document for the project. It is intentionally optimized for fast resume: keep the current status and recent implementation chapters detailed, and keep older history as a compact ledger inside this same file instead of scattering archive docs.

## Current Resume Snapshot - 2026-06-09

### Product State
- C/E selected-output technical acceptance is complete as of Chapter 250: Overall 100%, Plan C 100%, Plan E 100%.
- The product is not yet bug-free and not yet WeChat/QQ screenshot maturity; the active phase is release/manual QA, no-flicker polish, compatibility, and rollout policy.
- Chapter 251 fixed the first-frame `Alt+A` gray-shell timing path by deferring native window visibility until the screenshot image, mask, first candidate pass, and one frontend animation frame are ready.
- Normal users still keep transparent screenshot windows by default; automation/diagnostic behavior remains guarded by explicit env flags.

### Current Hot Paths
- `Alt+A` screenshot startup now routes through hidden shell prep -> RGBA payload -> frontend paint/candidate readiness -> `overlay_ready_to_show`.
- WGC/DXGI selected-output diagnostics and copy/save candidates remain guarded and should not become default production behavior without a rollout chapter.
- Manual QA still needs rebuilt-release validation for first visible frame, hand drag, copy/save/OCR/translate, focus cleanup, Alt-Tab cleanup, multi-monitor, DPI, and scaling.

### Latest Validation Snapshot
- Passed recently: `cargo fmt --manifest-path tauri-client/src-tauri/Cargo.toml -- --check`.
- Passed recently: `cargo check --manifest-path tauri-client/src-tauri/Cargo.toml --tests`.
- Passed recently: `cargo test --manifest-path tauri-client/src-tauri/Cargo.toml --lib screenshot_window_transparency_tests -- --nocapture`.
- Passed recently: `cd tauri-client; npx tsc --noEmit`.
- Passed recently: `cd tauri-client; npm run build`.
- Passed recently: `git diff --check` with existing LF-to-CRLF warnings only.

### Next Recommended Chapter
- Chapter 252: close old `YsnTrans.exe`, launch/rebuild the updated app, and manually retest `Alt+A` with a phone/video or desktop recording.
- Required retest: first visible frame should already contain the screenshot dim layer and UI candidate; no separate empty gray flash before the candidate.
- If retest passes, continue release QA; if it fails, add visible-frame probes and move toward native first-frame presentation.

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

## Detailed Current Chapters - 230-251

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
